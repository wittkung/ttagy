# Specification: TTAgy Architecture Audit & System Evolution

**Classification**: `[Full SDD]`
**Status**: `Draft`
**Created**: 2026-08-24
**Feature Directory**: `specs/001-architecture-audit-evolution`

---

## 1. Executive Summary & Problem Statement

TTAgy is designed to serve as the unified, high-performance local AI agent infrastructure bridge and multi-language SDK (`ttagy-core`, `ttagy-client`, `ttagyd`, `@ttagy/client`, `python/ttagy`) for host applications (e.g., TTSubs, TTZip, terminal CLI utilities).

Following a systematic, line-by-line architectural and source code audit across all modules, several critical architectural gaps, performance bottlenecks, and paradigm mismatches were identified between the declared architecture and actual implementation:

1. **Transport & Latency Discrepancy (README vs. Implementation)**:
   - **Declared**: Low-latency Unix Domain Socket (UDS) resident daemon with $\le 2\text{ms}$ dispatch latency.
   - **Actual**: `ttagyd` runs an Axum HTTP/1.1 TCP listener (`127.0.0.1:8970`); zero UDS socket code exists. Every incoming request cold-spawns a new `agy` CLI subprocess (`cmd.spawn()`), resulting in typical cold-start latency of $100\text{ms} \sim 400\text{ms}$.
2. **Subprocess I/O Deadlock Vulnerability**:
   - Both `ttagyd` (`routes.rs`) and `ttagy-client` (`fallback.rs`) spawn `agy` with `.stdout(Stdio::piped()).stderr(Stdio::piped())` but only drain `stdout`. If `agy` outputs heavy diagnostic logs to `stderr` (>64KB pipe buffer), the kernel blocks write calls, causing an unrecoverable process deadlock and timeout.
3. **Connection Cancellation & Resource Leakage**:
   - In `ttagyd`, if a client disconnects or aborts an HTTP SSE stream prematurely, Axum terminates the HTTP connection, but the background `tokio::spawn` task is not aborted. It continues running the `agy` child process to completion, holding the concurrency semaphore permit and wasting CPU/RAM.
   - In `ttagy-client` (Rust) and `@ttagy/client` (TypeScript), aborting consumer streams does not reliably trigger immediate child termination.
4. **Domain Coupling in Generic SDK**:
   - `@ttagy/client/src/fallback.ts` contains hardcoded business domain matching (`parsed.paragraphs`, `parsed.items`, `parsed.glossary` from TTSubs) within a generic infrastructure library, which prematurely terminates execution upon partial JSON heuristic match.
5. **Multi-Language SDK Feature Divergence**:
   - `python/ttagy` is an incomplete stub lacking HTTP/SSE remote client capabilities, typed stream events, and proper error handling.
   - TypeScript SDK lacks UDS transport support and true async cancellation signals (`AbortSignal`).

This specification defines the functional requirements and success criteria to remediate these critical defects, eliminate transport discrepancies, and elevate TTAgy into a resilient, zero-overhead, multi-transport AI bridge infrastructure.

---

## 2. User Scenarios & Personas

### Persona 1: Desktop Application Engineer (e.g., TTSubs / TTZip)
- **Goal**: Execute sub-second local AI inferences (subtitling proofreading, translation, code generation) with minimum resource footprint and immediate cancellation support.
- **Flow**: Connects via zero-overhead Unix Domain Socket (macOS/Linux) or named pipes (Windows); receives streaming deltas (`thinking_delta`, `content_delta`); cancels inferences instantaneously when the user switches tabs or stops the job.

### Persona 2: Backend & Microservice Developer (Remote Agent Node)
- **Goal**: Offload heavy agent workloads to a dedicated host node running `ttagyd` over authenticated HTTP/SSE or UDS.
- **Flow**: Configures `baseUrl` and `authToken`; streams NDJSON / SSE responses; relies on robust concurrency rate-limiting and graceful backpressure without server pipe deadlocks.

### Persona 3: Python / Data Science Integrator
- **Goal**: Seamlessly invoke local or remote Antigravity models in async Python pipelines.
- **Flow**: Installs `ttagy`, imports `TtagyClient`, and consumes standard async generators yielding strongly typed stream events identical to Rust and TypeScript SDKs.

---

## 3. Scope & System Boundaries

### In Scope
- **Dual Transport Layer**: Seamless support for Unix Domain Socket (UDS) IPC (`/tmp/ttagy.sock` or custom path) and HTTP/1.1 / HTTP/2 TCP REST+SSE on `ttagyd`.
- **Subprocess Robustness & Async Stderr Draining**: Continuous non-blocking consumption of `stderr` into an in-memory ring buffer to prevent OS pipe buffer saturation and expose diagnostic traces upon failure.
- **Graceful Lifecycle & Cancellation Propagation**: Bidirectional cancellation binding between client connection drop (HTTP disconnect / UDS close / `AbortSignal`) and immediate `child.kill(SIGKILL)` to prevent orphaned zombie workers.
- **SDK Parity**: Full feature and contract parity across Rust (`ttagy-client`), TypeScript (`@ttagy/client`), and Python (`python/ttagy`).
- **Domain Decoupling**: Complete removal of domain-specific heuristics from generic fallback engines.
- **Worker Management Optimization**: Implementation of a warm process / worker pool abstraction to achieve true sub-millisecond dispatch capability.

### Out of Scope (Non-Goals)
- Re-implementing Antigravity CLI internals or LLM weight inference.
- Managing remote cloud API keys (authentication remains delegated to `agy` CLI's own local auth subsystem).

---

## 4. Functional Requirements

### FR-01: Dual Transport Architecture (UDS + TCP)
- **FR-01.1**: The daemon (`ttagyd`) MUST support binding to a Unix Domain Socket path (defaulting to `/tmp/ttagy.sock` or user-specified path via `--socket`) alongside or instead of TCP host/port.
- **FR-01.2**: All client SDKs (Rust, TypeScript, Python) MUST support UDS connection mode as the primary local IPC transport before falling back to TCP or in-process spawn.
- **FR-01.3**: UDS communication MUST transmit NDJSON or framed SSE messages with sub-millisecond IPC serialization overhead ($\le 1\text{ms}$).

### FR-02: Zero-Deadlock Subprocess Execution & Stderr Drainage
- **FR-02.1**: Every module executing the `agy` CLI (`ttagyd`, Rust fallback driver, TypeScript fallback, Python client) MUST concurrently drain both `stdout` and `stderr` asynchronously.
- **FR-02.2**: `stderr` output MUST be buffered into a bounded circular ring buffer (e.g., 64KB) per execution session.
- **FR-02.3**: If the process exits with a non-zero exit code or error event, the captured `stderr` buffer MUST be formatted into the `error_message` payload of `TtagyStreamEvent::Error`.

### FR-03: Strict Cancellation & Zombie Process Prevention
- **FR-03.1**: When a client terminates an HTTP connection, closes a UDS socket, or triggers an `AbortController` signal, the associated server worker task MUST abort immediately and terminate the spawned `agy` process with `SIGKILL`.
- **FR-03.2**: All temporary sandbox directories created for the session MUST be safely unlinked upon cancellation or completion without leaking temporary files in `/tmp`.

### FR-04: Domain-Agnostic Stream & JSON Extraction Engine
- **FR-04.1**: Generic SDK fallback code MUST NOT contain application-specific JSON key checks (e.g., `paragraphs`, `glossary`, `items`).
- **FR-04.2**: Structured JSON extraction MUST rely strictly on:
  1. CLI-level `--json-schema` enforcement when provided.
  2. Protocol-level `ParsedChunk::Result` / `agy:done` event envelopes.
  3. Clean markdown fence stripping (```json ... ```) and standard balanced-brace parsing.

### FR-05: Multi-Language SDK Parity
- **FR-05.1**: `python/ttagy` MUST provide an async client matching `TtagyRequest`, `TtagyResponse`, and `TtagyStreamEvent` models, supporting both remote HTTP/SSE and local direct spawn.
- **FR-05.2**: `@ttagy/client` MUST support `AbortSignal` in `TtagyRequest` and `streamChat`.
- **FR-05.3**: `ttagy-client` (Rust) MUST eliminate unused variable/import warnings and provide unified UDS + TCP client configuration.

### FR-06: Process Pool & Warm Execution Architecture
- **FR-06.1**: `ttagyd` SHOULD support an optional warm worker mode where standby worker handles or pre-warmed sandbox contexts reduce process invocation jitter.

---

## 5. Non-Functional Requirements & Success Criteria

| Metric | Target | Measurement Method |
| :--- | :--- | :--- |
| **Local UDS IPC Latency** | $\le 2\text{ms}$ time-to-first-byte over socket | Automated benchmark measuring ping-to-init event |
| **Pipe Deadlock Resistance** | 0 deadlocks under 1MB `stderr` flood | Integration test asserting process exit under high stderr output |
| **Cancellation Response Time** | $\le 50\text{ms}$ process termination upon abort | Integration test asserting process tree termination on drop |
| **Sandbox Cleanliness** | 0 leaked sandbox directories after 1000 runs | Directory count check in `/tmp/local_ai_sandboxes` |
| **Local CI Quality Gate** | 100% PASS across Rust, TS, Python, Schema | `bash scripts/local-ci.sh` execution |

---

## 6. Assumptions & Dependencies

- **Assumption 1**: The host operating system provides POSIX-compliant UDS support (macOS, Linux). On Windows, TCP or named pipes will be utilized.
- **Assumption 2**: `agy` CLI executable is installed and authenticated in standard binary paths (`$HOME/.local/bin/agy`, `/usr/local/bin/agy`, `/opt/homebrew/bin/agy`, or via `PATH`).
- **Dependency 1**: Rust 1.80+ (2021 edition), Node.js 18+, Python 3.10+.
