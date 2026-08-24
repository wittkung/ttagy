# Research: Deterministic Testing Infrastructure & Chaos Suite

**Feature**: [`specs/002-test-infrastructure-and-chaos-suite`](file:///Users/kevintung/Documents/dev/infra/ttagy/specs/002-test-infrastructure-and-chaos-suite/spec.md)
**Status**: `Completed`
**Created**: 2026-08-24

---

## 1. Research Topic 1: Deterministic CLI Simulation (`mock-agy`)

### Problem Statement
Integration tests relying on the real `agy` CLI binary incur network latency, depend on live credentials, consume cloud credits, and cannot deterministically reproduce edge-case failures (e.g., 10MB stderr flooding, SIGKILL hangs, malformed NDJSON lines, quota errors).

### Decision
Build a lightweight, zero-dependency Rust binary `crates/mock-agy` that behaves as a drop-in replacement for `agy`:
- Configured via CLI flag `--scenario <name>` or environment variable `MOCK_AGY_SCENARIO`.
- Scenarios:
  1. `stream_normal`: Emits `step_update` (thinking, text), `step_update` (text), and terminal `result` with usage metadata.
  2. `stderr_flood`: Streams 10MB of stderr data in 4KB chunks concurrently with stdout NDJSON events.
  3. `malformed_ndjson`: Emits raw non-JSON text, broken markdown fences, and valid JSON lines.
  4. `abort_hang`: Sleeps indefinitely in a loop to test parent process SIGKILL enforcement on cancellation.
  5. `quota_error`: Emits top-level error JSON and exits with code 1.
  6. `empty_output`: Emits stderr logs with 0 stdout bytes and exits with code 1.

### Rationale
- Zero cloud quota consumption ($0.00 cost).
- Offline test capability on any standard CI runner.
- Sub-millisecond execution for rapid test feedback.

---

## 2. Research Topic 2: Chaos & Concurrency Race Testing

### Problem Statement
Shallow unit tests do not test concurrent load, sudden connection termination (RST / socket close), or kernel pipe saturation.

### Decision
Implement `crates/ttagyd/tests/chaos_suite.rs` containing 3 dedicated chaos scenarios:
1. **Pipe Saturation Stress Test**: Executes `mock-agy` with `stderr_flood` and asserts that the parent process receives and parses all stdout NDJSON events in <100ms without blocking.
2. **100x Abort Race Test**: Spawns 100 concurrent requests against `ttagyd`, reads 1 initial event, and abruptly drops the connection stream; asserts that within 200ms:
   - 0 lingering `mock-agy` processes exist.
   - Semaphore permits recover 100% to `max_concurrency`.
3. **Sandbox Leak Assertion**: Asserts that all created temporary directories under `/tmp/local_ai_sandboxes/` are completely removed.

### Rationale
- Defends against regressions of the 5 critical defects audited in feature 001.

---

## 3. Research Topic 3: Cross-Language Conformance Test Suite (CTS)

### Problem Statement
Rust, TypeScript, and Python SDKs have different language idioms and parsing implementations, creating risk of behavioral divergence.

### Decision
1. Define shared scenario fixtures in JSON (`tests/conformance/fixtures/`):
   - `basic_stream.json`
   - `thinking_stream.json`
   - `structured_json.json`
   - `quota_error.json`
2. Implement automated CTS drivers in Rust, TypeScript, and Python, asserting:
   - Identical event type sequences (`agy:init` -> `agy:thinking_delta` -> `agy:content_delta` -> `agy:done`).
   - Identical structured JSON extraction outputs.
   - Identical error codes and message handling.

### Rationale
- Guarantees 100% feature and behavioral parity across all supported SDKs.
