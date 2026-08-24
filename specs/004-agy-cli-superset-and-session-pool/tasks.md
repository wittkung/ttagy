# Tasks: AGY CLI Superset Evolution, Warm Worker Pool & Stateful Session Store

**Feature**: [`specs/004-agy-cli-superset-and-session-pool`](file:///Users/kevintung/Documents/dev/infra/ttagy/specs/004-agy-cli-superset-and-session-pool/spec.md)
**Plan**: [`plan.md`](./plan.md)
**Status**: `Completed & Verified`

---

## Phase 1: Core Type Expansion (`crates/ttagy-core`)

- [x] T001 Expand `TtagyRequest` struct with full AGY CLI parameters in `crates/ttagy-core/src/v1/types.rs`
- [x] T002 Update `TtagyRequest::builder()` methods for fluent API in `crates/ttagy-core/src/v1/types.rs`
- [x] T003 Update fallback driver arguments in `crates/ttagy-client/src/fallback.rs`

---

## Phase 2: Warm Worker Pool (`crates/ttagyd/src/worker_pool.rs`)

- [x] T004 [P] Create `crates/ttagyd/src/worker_pool.rs` with `WorkerInstance` and `WorkerPool`
- [x] T005 Integrate `WorkerPool` into `crates/ttagyd/src/main.rs` and `crates/ttagyd/src/v1/routes.rs`

---

## Phase 3: Stateful Session Store & Virtualized APIs (`crates/ttagyd`)

- [x] T006 [P] Implement `SessionStore` (LRU + WAL) in `crates/ttagyd/src/session_store.rs`
- [x] T007 [P] Implement `McpManager` in `crates/ttagyd/src/mcp_manager.rs`
- [x] T008 Add `/models`, `/agents`, `/mcp/servers`, and `/sessions` handlers in `crates/ttagyd/src/v1/routes.rs`

---

## Phase 4: Multi-Language SDK Alignment

- [x] T009 [P] Update TS SDK request types in `packages/ttagy-client/src/types.ts`
- [x] T010 [P] Update Python SDK request types in `python/ttagy/types.py`
- [x] T011 [P] Update Go SDK request types in `golang/ttagy/types.go`
- [x] T012 [P] Update Dart SDK request types in `dart/ttagy/lib/src/types.dart`
- [x] T013 [P] Update C header in `crates/ttagy-ffi/include/ttagy.h`

---

## Phase 5: CI/CD Quality Gate & Verification

- [x] T014 Run `bash scripts/local-ci.sh` verifying 100% PASS with 0 cloud tokens consumed
