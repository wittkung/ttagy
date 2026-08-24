# Tasks: Subagent Mesh & Distributed Workspace Orchestration

**Feature**: [`specs/005-subagent-mesh-and-workspaces`](file:///Users/kevintung/Documents/dev/infra/ttagy/specs/005-subagent-mesh-and-workspaces/spec.md)
**Plan**: [`plan.md`](./plan.md)
**Status**: `Completed & Verified`

---

## Phase 1: Workspace Isolation Engine (`crates/ttagyd/src/workspace_manager.rs`)

- [x] T001 [P] Create `crates/ttagyd/src/workspace_manager.rs` with `WorkspaceManager` and `WorkspaceGuard`
- [x] T002 Implement startup orphan reconciliation and RAII GC queue in `workspace_manager.rs`

---

## Phase 2: Actor Message Bus & Subagent Mesh (`crates/ttagyd`)

- [x] T003 [P] Create `crates/ttagyd/src/message_bus.rs` with bounded inboxes and DLQ
- [x] T004 [P] Create `crates/ttagyd/src/subagent_mesh.rs` with `WaitForGraph` cycle detection and cascade kill

---

## Phase 3: Control Plane Endpoints (`crates/ttagyd/src/v1/routes.rs`)

- [x] T005 Integrate `WorkspaceManager`, `MessageBus`, and `SubagentMesh` into `AppState` in `crates/ttagyd/src/main.rs`
- [x] T006 Add `/subagents/*` routes in `crates/ttagyd/src/v1/routes.rs`

---

## Phase 4: SDK Alignment

- [x] T007 [P] Add subagent helper methods in `crates/ttagy-client/src/client.rs`
- [x] T008 [P] Add subagent helper methods in `packages/ttagy-client/src/client.ts`

---

## Phase 5: CI/CD Quality Gate & Verification

- [x] T009 Run `bash scripts/local-ci.sh` verifying 100% PASS with 0 cloud tokens consumed
