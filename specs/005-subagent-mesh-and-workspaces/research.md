# Research: Subagent Mesh, Workspace Isolation & Deadlock Detection

**Feature**: [`specs/005-subagent-mesh-and-workspaces`](file:///Users/kevintung/Documents/dev/infra/ttagy/specs/005-subagent-mesh-and-workspaces/spec.md)
**Status**: `Completed & Audited by Subagents`
**Created**: 2026-08-24

---

## 1. Git Worktree & Workspace Sandboxing

- **Mechanism**: Dedicated `git worktree add <path> -b <branch> HEAD` using shared `.git/objects` for zero-copy file trees and isolated index staging locks.
- **Tri-State Semantics**:
  - `inherit`: Inherits parent directory without creating branch or directory (ideal for read-only research).
  - `branch`: Creates isolated temporary branch `ttagy/sandbox/<agent_id>` at `~/.ttagy/workspaces/<uuid>` with automatic RAII deletion.
  - `share`: Joins shared collaborative group branch `ttagy/shared/<group_id>`.
- **RAII Lifecycle & Crash Reconciliation**:
  - `WorkspaceGuard` implements `Drop` sending cleanup task to background GC queue (`git worktree remove --force` + `git branch -D`).
  - Startup reconciliation scans and prunes leftover `ttagy/sandbox/*` branches and orphaned worktree directories.

---

## 2. Actor Message Bus & Reactive Wakeup

- **Bounded Inboxes**: Each subagent owns a private bounded `tokio::sync::mpsc::channel(128)` inbox.
- **Zero-Latency Reactive Wakeup**: `tokio::select!` over `inbox.recv()`, `cancellation_token.cancelled()`, and timeout futures eliminates all polling/sleep overhead.
- **Dead Letter Queue (DLQ)**: Handles unroutable, full, or terminated recipient messages with error status.

---

## 3. Subagent DAG Topology, Deadlock Prevention & Cascade Disposal

- **DAG Hierarchy**:
  - `max_subagent_depth = 3`: Prevents recursive prompt injection fork bombs.
  - `max_children_per_parent = 8`: Bounds subagent fan-out.
  - Global `Semaphore(32)`: Controls maximum host concurrency.
- **Wait-For Graph (WFG) Cycle Detection**:
  - Microsecond incremental reachability check upon wait edge addition.
  - 5-second ticker circuit-breaker selecting leaf-most victim to abort cycle.
- **Two-Phase Cascade Kill**:
  - BFS traversal of descendant nodes.
  - Phase 1: Parallel `CancellationToken` soft broadcast.
  - Phase 2: Parallel `SIGKILL` + Git Worktree removal + WFG edge clearing.
