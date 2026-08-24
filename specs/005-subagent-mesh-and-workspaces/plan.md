# Implementation Plan: Subagent Mesh & Distributed Workspace Orchestration

**Feature**: [`specs/005-subagent-mesh-and-workspaces`](file:///Users/kevintung/Documents/dev/infra/ttagy/specs/005-subagent-mesh-and-workspaces/spec.md)
**Status**: `Planned`
**Created**: 2026-08-24
**Branch**: `main`

---

## 1. Technical Context & Objectives

This plan implements the complete multi-agent subagent mesh runtime, including git worktree workspace isolation (`WorkspaceManager`), the asynchronous Actor message broker (`MessageBus`), DAG topology with microsecond cycle detection (`WaitForGraph`), and `/api/v1/subagents` Axum REST endpoints.

---

## 2. Tasks Breakdown

- **Phase 1: Workspace Isolation Engine (`crates/ttagyd/src/workspace_manager.rs`)**
  - Implement `WorkspaceMode` (`Inherit`, `Branch`, `Share`).
  - Implement `WorkspaceGuard` with RAII `Drop` async cleanup.
  - Implement `WorkspaceManager` with `reconcile_orphans()` and `provision()`.

- **Phase 2: Actor Message Bus & Deadlock Detector (`crates/ttagyd/src/message_bus.rs`, `crates/ttagyd/src/subagent_mesh.rs`)**
  - Implement bounded private inboxes with `tokio::sync::mpsc::channel(128)`.
  - Implement `WaitForGraph` with incremental reachability checks ($\le 10\mu\text{s}$) and 5-second timeout熔断.
  - Implement `SubagentMesh` manager with depth limit ($D \le 3$) and two-phase cascade kill.

- **Phase 3: Subagent Control Plane API (`crates/ttagyd/src/v1/routes.rs` & `crates/ttagyd/src/main.rs`)**
  - Add `/api/v1/subagents/invoke`, `/api/v1/subagents/message`, `/api/v1/subagents/wait`, `/api/v1/subagents/kill_all`, `/api/v1/subagents/:id`, `/api/v1/subagents/:id/kill`.

- **Phase 4: Multi-Language SDK Alignment & CTS Tests**
  - Expand client SDKs with `invoke_subagents` and `send_message`.

- **Phase 5: Local CI Verification**
  - Run `bash scripts/local-ci.sh` verifying 100% PASS with 0 cloud tokens.
