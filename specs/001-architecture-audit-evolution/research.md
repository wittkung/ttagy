# Research: TTAgy System Evolution & Deep Remediation

**Feature**: [`specs/001-architecture-audit-evolution`](file:///Users/kevintung/Documents/dev/infra/ttagy/specs/001-architecture-audit-evolution/spec.md)
**Status**: `Completed`
**Created**: 2026-08-24

---

## 1. Research Topic 1: Subprocess Stderr Async Drainage & Bounded Ring Buffer

### Problem Statement
In `ttagyd` and `ttagy-client`, child processes were spawned with `stderr(Stdio::piped())` without draining. When `stderr` output exceeded the OS kernel pipe buffer (64KB on Linux, 16KB~64KB on macOS), the child process blocked on `write(2)`, causing an unrecoverable cross-stream deadlock with the parent process.

### Decision
Implement `RollingBuffer` and `StderrDrainer` in `ttagy-core`:
- `RollingBuffer`: A bounded FIFO ring buffer (`VecDeque<u8>`) with fixed memory cap (64KB default). Older bytes are dropped when full; tracks `total_bytes_dropped`.
- `StderrDrainer`: Spawns a dedicated Tokio task that reads chunks into the `RollingBuffer` asynchronously until EOF.
- Error enrichment: When child process exits abnormally, outputs empty stdout, or times out, the captured `stderr` buffer is attached to `TtagyStreamEvent::Error`.

### Rationale
- Completely eliminates OS pipe write blocking regardless of stderr flood size.
- Strictly bounds memory to 64KB per session, eliminating OOM denial-of-service vectors.
- Transforms silent failure into transparent diagnostics.

### Alternatives Considered
- `Stdio::null()`: Discards deadlock, but throws away all diagnostic information and backtraces.
- Unbounded `read_to_end()`: Vulnerable to memory exhaustion (OOM) if child loops on error logging.

### Source References
- POSIX.1-2017 `pipe(7)` buffer capacity standards.
- Tokio `tokio::process::ChildStderr` asynchronous I/O documentation.

---

## 2. Research Topic 2: Unix Domain Socket (UDS) & TCP Dual-Transport in Axum 0.7

### Problem Statement
`README.md` declared UDS support with $\le 2\text{ms}$ latency, but `ttagyd` only listened on TCP `127.0.0.1:8970`. Python SDK's `socket_path` was a dummy argument. Cold start process spawning incurred 160ms~460ms latency.

### Decision
1. **Server Dual Transport**:
   - `ttagyd` concurrently binds to `tokio::net::TcpListener` (for remote agent nodes) and `tokio::net::UnixListener` (for zero-network-stack local IPC).
   - Utilizes `hyper_util::rt::TokioIo` and `hyper_util::server::conn::auto::Builder` to serve the same Axum `Router` across both transports.
   - Sets Unix socket file permissions to `0600` for local user security.
2. **Client Auto-Negotiation Hierarchy**:
   - Priority 1: Unix Domain Socket (`/tmp/ttagy.sock`) for local IPC ($\le 1\text{ms}$ serialization, 0 TCP overhead).
   - Priority 2: Remote TCP HTTP/SSE (`http://...:8970`) with Bearer token authentication.
   - Priority 3: In-process direct sandbox spawn fallback.
3. **Warm Worker Pool Architecture**:
   - `ttagyd` manages a pool of pre-warmed sandbox handles and standby workers to eliminate cold-start dynamic link and filesystem overhead.

### Rationale
- Delivers the promised $\le 2\text{ms}$ dispatch latency for local host applications (TTSubs, TTZip).
- Maintains seamless remote host node capability for multi-machine setups.
- Aligns all 3 client SDKs (Rust, TypeScript, Python) to the identical negotiation hierarchy.

### Alternatives Considered
- Raw custom binary protocol over UDS: High development cost, breaks compatibility with standard HTTP/SSE tooling. Axum HTTP/1.1 over UDS provides the optimal balance of standard tooling and sub-millisecond IPC.

---

## 3. Research Topic 3: Stream Lifecycle Binding & Cancellation Propagation

### Problem Statement
`stream_handler` spawned detached `tokio::spawn` tasks. When a client disconnected or aborted, the background task continued running the `agy` process, holding the semaphore permit and causing complete denial of service (HTTP 429) after a few aborted requests.

### Decision
1. **`GuardedStream` with `PinnedDrop`**:
   - Wraps the returned SSE `ReceiverStream` using `pin-project-lite`.
   - On drop (when HTTP connection terminates or client disconnects), `cancel_token.cancel()` is immediately triggered.
2. **Reactive `tokio::select!` Loop**:
   - Task loop concurrently watches:
     1. `cancel_token.cancelled()`
     2. `tx.closed()`
     3. `reader.next_line()`
   - Any cancellation immediately issues `child.kill(SIGKILL)` and breaks the loop.
3. **Process Group Isolation**:
   - Unix `cmd.process_group(0)` + `cmd.kill_on_drop(true)` ensures child and all descendant processes are cleanly terminated.
4. **RAII Permit & Sandbox Release**:
   - Exiting the task scope naturally drops `_permit` and `SandboxGuard`, ensuring 0ms permit reclamation.
5. **Client `AbortSignal` Support**:
   - TS SDK: `TtagyRequest.signal?: AbortSignal` forwarded to `fetch` or fallback spawn.
   - Python SDK: Explicit `proc.kill()` in generator `finally` blocks.

### Rationale
- Zero zombie worker processes.
- Zero concurrency permit leakage; eliminates HTTP 429 cascade.
- Sub-50ms cancellation responsiveness.

---

## 4. Research Topic 4: Domain Decoupled Streaming & JSON Engine

### Problem Statement
`packages/ttagy-client/src/fallback.ts` contained hardcoded TTSubs subtitle domain keys (`paragraphs`, `items`, `glossary`, `concepts`) and lacked `--output-format stream-json`. Any non-subtitle AI output matching these keys was killed prematurely, destroying data integrity. `NdjsonParser` also had exclusive return bugs dropping text deltas.

### Decision
1. **Three-Tier Decoupled Architecture**:
   - Layer 1 (Runtime): Standardized CLI invocation with `-p <prompt> --output-format stream-json [--json-schema <path>]`.
   - Layer 2 (Protocol Parser): `NdjsonParser` returns `Vec<ParsedStreamItem>`, supporting concurrent thinking + content deltas in a single line, nested `message` payloads, top-level errors, and usage tokens.
   - Layer 3 (Structured JSON Engine): Generic balanced-brace state machine (`extractStructuredJson`) with escape-character awareness and progressive streaming JSON repairer (`repairIncompleteJson`).
2. **Domain Logic Clean-up**:
   - Completely eradicate all application-specific field names from generic SDK code.

### Rationale
- 100% domain-agnostic; safe for any LLM task (code generation, structured extraction, chat).
- Robust handling of markdown code fences and mixed stream tokens.

---

## 5. Research Topic 5: Transparent Model Name Security Validation & Python SDK Parity

### Problem Statement
`normalize_model_name` used lossy substring matching (`contains("3.7")` forced `claude-3.7-sonnet` to `gemini-3.7-flash`). Python SDK was a 66-line stub lacking remote SSE, NDJSON parsing, typed models, and error handling.

### Decision
1. **Model Name Policy**:
   - Security check: Validate against safe charset `^[a-zA-Z0-9_.:-]+$`.
   - Canonical exact aliasing: Map standard aliases (`default`, `gemini`, `sonnet`, `opus`) to standard identifiers.
   - Transparent passthrough: Any valid custom model string is passed directly to `agy --model <name>` without silent mutation.
2. **Python SDK Modular Architecture**:
   - Modularized into `types.py`, `parser.py`, `detector.py`, `fallback.py`, and `client.py`.
   - Full support for remote HTTP/SSE (with UDS transport via `httpx`), typed stream events, `chat()`, and `run_json()`.
3. **Compiler Warnings & CI Gate**:
   - Expose concurrency metrics in `/api/v1/health` to resolve unused field warnings.
   - Add Python SDK test suite to `scripts/local-ci.sh`.
