# Tasks: TTAgy Architecture Evolution & Deep Remediation

**Feature**: [`specs/001-architecture-audit-evolution`](file:///Users/kevintung/Documents/dev/infra/ttagy/specs/001-architecture-audit-evolution/spec.md)
**Plan**: [`plan.md`](./plan.md)
**Status**: `Completed`

---

## Phase 1: Setup & Workspace Dependencies

- [X] T001 Update workspace dependencies in `crates/ttagyd/Cargo.toml` to add `tokio-util` and `hyper-util`
- [X] T002 [P] Synchronize JSON Schema contracts in `specs/001-architecture-audit-evolution/contracts/` and `specs/contracts/v1/`

---

## Phase 2: Foundational Core Infrastructure (`ttagy-core`)

- [X] T003 Implement bounded ring buffer `RollingBuffer` and async `StderrDrainer` in `crates/ttagy-core/src/v1/stderr_drainer.rs`
- [X] T004 [P] Implement `resolve_model_name` with safe charset regex, canonical alias table, and transparent passthrough in `crates/ttagy-core/src/v1/model.rs`
- [X] T005 Refactor `NdjsonParser` in `crates/ttagy-core/src/v1/parser.rs` to return `Vec<ParsedStreamItem>` supporting composite deltas, nested message unnesting, top-level errors, and token usage
- [X] T006 Update type definitions (`backend_mode` variants, `UsageMetadata`) in `crates/ttagy-core/src/v1/types.rs` and re-export in `crates/ttagy-core/src/v1/mod.rs` and `crates/ttagy-core/src/lib.rs`
- [X] T007 Add core unit tests in `crates/ttagy-core/tests/test_core.rs` verifying 1MB stderr flood drainage, composite NDJSON parsing, and model passthrough

---

## Phase 3: [US1] Zero-Deadlock Subprocess Execution & Stderr Drainage

- [X] T008 [US1] Integrate `StderrDrainer` into daemon request pipeline in `crates/ttagyd/src/v1/routes.rs` to eliminate pipe deadlocks and capture diagnostics
- [X] T009 [P] [US1] Integrate `StderrDrainer` and diagnostic error propagation into `crates/ttagy-client/src/fallback.rs`
- [X] T010 [US1] Add integration test in `crates/ttagyd/tests/consumer_compat.rs` asserting zero deadlocks and successful stream completion under heavy stderr output

---

## Phase 4: [US2] Dual Transport UDS + TCP Server & Client Adapters

- [X] T011 [US2] Implement `--socket` CLI option and dual `tokio::net::UnixListener` + `tokio::net::TcpListener` concurrent binding via `hyper_util` in `crates/ttagyd/src/main.rs`
- [X] T012 [P] [US2] Update `crates/ttagy-client/src/client.rs` to support `TransportTarget::Uds` and auto-negotiation (`UDS -> TCP -> In-Process Fallback`)
- [X] T013 [P] [US2] Update `packages/ttagy-client/src/index.ts` to support `socketPath` in `ClientOptions` via Node.js `node:http` or `undici.Agent`
- [X] T014 [US2] Add dual transport integration test in `crates/ttagyd/tests/consumer_compat.rs` testing health and stream endpoints over UDS socket

---

## Phase 5: [US3] Stream Lifecycle Cancellation & Zombie Prevention

- [X] T015 [US3] Implement `GuardedStream` with `CancellationToken` on `PinnedDrop` in `crates/ttagyd/src/v1/routes.rs`
- [X] T016 [US3] Refactor worker loop in `crates/ttagyd/src/v1/routes.rs` with `tokio::select!`, `cmd.kill_on_drop(true)`, `process_group(0)`, and RAII `_permit` release on drop
- [X] T017 [P] [US3] Add `signal?: AbortSignal` support in `packages/ttagy-client/src/types.ts`, `packages/ttagy-client/src/index.ts`, and `packages/ttagy-client/src/fallback.ts`
- [X] T018 [US3] Add integration test in `crates/ttagyd/tests/consumer_compat.rs` verifying instant `SIGKILL` and immediate permit reclamation upon client stream abort

---

## Phase 6: [US4] Domain-Agnostic Stream Parser & Structured JSON Engine

- [X] T019 [US4] Remove hardcoded TTSubs domain keys (`paragraphs`, `items`, `glossary`, `concepts`) from `packages/ttagy-client/src/fallback.ts` and add `--output-format stream-json`
- [X] T020 [P] [US4] Implement balanced-brace state machine `extractStructuredJson` and streaming auto-repairer `repairIncompleteJson` in `packages/ttagy-client/src/fallback.ts` and `index.ts`
- [X] T021 [US4] Add unit tests in `packages/ttagy-client/src/__tests__/client.test.mjs` verifying domain-agnostic structured JSON extraction and streaming delta dispatches

---

## Phase 7: [US5] Model Passthrough Security Validator & Python SDK Full Parity

- [X] T022 [US5] Replace `normalize_model_name` with `resolve_model_name` in `crates/ttagyd/src/v1/routes.rs`
- [X] T023 [P] [US5] Implement dataclass models in `python/ttagy/types.py`
- [X] T024 [P] [US5] Implement NDJSON parser in `python/ttagy/parser.py` matching Rust `NdjsonParser`
- [X] T025 [P] [US5] Implement binary detector in `python/ttagy/detector.py`
- [X] T026 [US5] Implement in-process fallback with async stderr draining and `SIGKILL` safety in `python/ttagy/fallback.py`
- [X] T027 [US5] Implement unified `TtagyClient` supporting UDS / TCP HTTP/SSE via `httpx`, local fallback, `stream_chat()`, `chat()`, and `run_json()` in `python/ttagy/client.py` and `python/ttagy/__init__.py`
- [X] T028 [US5] Create unit tests in `python/tests/test_parser.py` and `python/tests/test_client.py`

---

## Phase 8: Polish, Compiler Warning Cleanup & CI/CD Quality Gate

- [X] T029 Expose `max_concurrency` in `/api/v1/health` to resolve struct field warning in `crates/ttagyd/src/v1/routes.rs`
- [X] T030 [P] Clean up unused imports and unused variables in `crates/ttagy-client/tests/test_client.rs`
- [X] T031 Update `scripts/local-ci.sh` to add `[5/5]` Python SDK test execution
- [X] T032 Run `bash scripts/local-ci.sh` and ensure 100% PASS across all 5 tiers with zero compiler warnings
