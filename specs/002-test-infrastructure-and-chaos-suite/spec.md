# Specification: TTAgy Deterministic Testing & Chaos Suite (CTS)

**Classification**: `[Full SDD]`
**Status**: `Draft`
**Created**: 2026-08-24
**Feature Directory**: `specs/002-test-infrastructure-and-chaos-suite`

---

## 1. Executive Summary & Problem Statement

TTAgy serves as the foundational local AI bridge infrastructure for multiple mission-critical host applications (TTSubs, TTZip). As demonstrated during the architecture audit, traditional shallow unit tests created a false illusion of "100% PASS" while completely missing system-level defects:
1. **Kernel Pipe Deadlocks (64KB Stderr Saturation)**: Untested because tests never flooded subprocess pipes.
2. **Concurrency Semaphore Leaks & Zombie Workers**: Untested because tests never simulated client connection drops under race conditions.
3. **Contract Drift**: Static JSON schemas were syntax-checked, but actual runtime traffic from Rust, TypeScript, and Python was never validated against Draft-07 contracts.
4. **Cloud Quota & Network Fragility**: Testing against real `agy` CLI consumed cloud credits, was non-deterministic, and failed in offline CI environments.

This specification defines the functional requirements and design for an enterprise-grade, 4-tier testing infrastructure for TTAgy:
- **Tier 1: Deterministic Mock CLI Simulator (`mock-agy`)**: A zero-cloud-quota, zero-network, sub-millisecond CLI test double with configurable fault-injection scenarios.
- **Tier 2: Runtime Contract Invariant Gate**: Real-time validation of all IPC/network payloads against Draft-07 schemas.
- **Tier 3: Chaos & Boundary Stress Suite**: Automated injection of 10MB stderr floods, 100x rapid abort connection churn, and sandbox leakage assertions.
- **Tier 4: Cross-Language Conformance Test Suite (CTS)**: A unified test harness asserting behavioral and contract parity across Rust, TypeScript, and Python SDKs.

---

## 2. User Scenarios & Personas

### Persona 1: CI/CD Pipeline & Quality Gate Engine
- **Goal**: Run the full test suite on any local machine or offline CI runner in $\le 5\text{s}$ with 0 API tokens consumed, achieving 100% deterministic green/red signals.
- **Flow**: Executes `bash scripts/local-ci.sh`; compiles `mock-agy`; executes CTS and Chaos suites; validates runtime contracts.

### Persona 2: Multi-Language SDK Developer
- **Goal**: Add a new feature or model parameter and verify that Rust, TypeScript, and Python implementations produce identical event streams, error codes, and structured JSON outputs.
- **Flow**: Adds a test fixture in `tests/conformance/fixtures/`; runs CTS; CTS automatically drives Rust, TS, and Python SDKs against `mock-agy` and reports parity.

### Persona 3: Systems & Chaos Engineer
- **Goal**: Ensure the daemon and client drivers are completely immune to OS-level resource exhaustion, process leaks, and pipe buffer deadlocks.
- **Flow**: Runs the Chaos suite; floods stderr with 10MB data; triggers 100 rapid concurrent client aborts; verifies 0 zombie processes (`pgrep` count == 0) and immediate semaphore permit recovery.

---

## 3. Scope & System Boundaries

### In Scope
- **`mock-agy` Binary (`crates/mock-agy`)**: Lightweight Rust binary supporting scenario flags (`--scenario <name>`) and custom payload generation.
- **Runtime Schema Validator**: In-memory JSON Schema Draft-07 validator integrated into integration test pipelines.
- **Chaos Test Engine**: Automated stress tests for pipe saturation, high-concurrency connection aborts, and temporary sandbox garbage collection.
- **Cross-Language Conformance Suite (CTS)**: Automated test runner executing shared scenario fixtures against Rust (`ttagy-client`), TypeScript (`@ttagy/client`), and Python (`python/ttagy`).
- **CI/CD Integration**: Seamless inclusion in `scripts/local-ci.sh`.

### Out of Scope (Non-Goals)
- Testing third-party proprietary LLM weights or cloud model generation quality.
- Emulating full interactive TUI terminal interfaces of the Antigravity IDE.

---

## 4. Functional Requirements

### FR-01: Deterministic Mock CLI Simulator (`mock-agy`)
- **FR-01.1**: The project MUST provide a standalone executable `mock-agy` compiled in the workspace.
- **FR-01.2**: `mock-agy` MUST support the following deterministic scenarios via `--scenario <name>` or environment variable `MOCK_AGY_SCENARIO`:
  1. `stream_normal`: Streams standard `step_update` (thought + content) and terminal `result` with token usage.
  2. `stderr_flood`: Emits 10MB of continuous debug logs to `stderr` while simultaneously streaming valid NDJSON to `stdout`.
  3. `malformed_ndjson`: Emits unescaped markdown blocks, partial JSON fragments, and raw text to test parser resilience.
  4. `abort_hang`: Ignores SIGTERM and simulates a slow infinite loop to test parent process `kill_on_drop` / `SIGKILL` timeout.
  5. `quota_error`: Outputs standard top-level CLI error envelope (`{"type":"error","error":"Quota exceeded"}`) with exit code 1.
  6. `empty_output`: Emits stderr logs but produces 0 bytes on stdout with non-zero exit code.
- **FR-01.3**: `mock-agy` MUST support simulating configurable token emission latency (`--delay-ms <ms>`).

### FR-02: Runtime Schema Invariant Gate
- **FR-02.1**: All integration tests MUST validate every captured `TtagyRequest`, `TtagyResponse`, and `TtagyStreamEvent` against `specs/contracts/v1/*.json` schemas.
- **FR-02.2**: If any emitted event violates the Draft-07 contract (e.g., missing required fields, illegal enum values), the test MUST fail immediately with detailed schema validation errors.

### FR-03: Chaos & Boundary Stress Suite
- **FR-03.1**: The Chaos suite MUST assert that streaming a 10MB stderr flood from `mock-agy` completes without thread hanging or pipe buffer deadlock.
- **FR-03.2**: The Chaos suite MUST launch 100 concurrent requests to `ttagyd` and randomly abort 100% of them within 5ms~50ms, asserting:
  1. Residual `agy` / `mock-agy` process count is 0 within 100ms.
  2. `ttagyd` available semaphore permits recover to `max_concurrency` (100% permit recovery).
  3. Residual sandbox directories in `/tmp/local_ai_sandboxes/` is 0.

### FR-04: Cross-Language Conformance Test Suite (CTS)
- **FR-04.1**: CTS MUST define canonical scenario fixtures in JSON format covering unary chat, streaming chat, structured JSON extraction, and error handling.
- **FR-04.2**: CTS MUST execute each fixture across Rust, TypeScript, and Python SDKs, asserting that:
  1. Event sequences match.
  2. Extracted JSON objects match.
  3. Error codes and retryability flags match.

---

## 5. Non-Functional Requirements & Success Criteria

| Metric | Target | Measurement Method |
| :--- | :--- | :--- |
| **Cloud Token / Cost Consumption** | **0 tokens ($0.00)** | Full test execution in offline mode (`--offline`) |
| **Full Test Suite Execution Time** | $\le 5.0\text{s}$ | `time bash scripts/local-ci.sh` |
| **Deadlock Regression Detection** | 100% detection rate | Assert failure if StderrDrainer is disabled |
| **Process / Sandbox Leak Detection** | 100% detection rate | Assert failure if `kill_on_drop` is disabled |
| **Cross-Language Parity** | 100% contract match | CTS execution across Rust, TS, and Python |

---

## 6. Assumptions & Dependencies

- **Assumption 1**: Local system has Rust (Cargo), Node.js, and Python 3.10+ installed.
- **Assumption 2**: `mock-agy` can be placed in a temporary directory and pointed to via `AGY_PATH` environment variable during tests.
