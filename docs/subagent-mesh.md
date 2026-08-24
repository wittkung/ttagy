# Subagent Mesh & Workspace Orchestration Guide

## 1. Tri-State Workspace Sandboxing

When subagents execute file modifications or run tests concurrently, file collisions can corrupt workspaces. TTAgy introduces three workspace isolation modes:

| Mode | Mechanism | Behavior |
| :--- | :--- | :--- |
| `inherit` | Direct filesystem pass-through | Reuses parent directory (read-only tasks like research & analysis) |
| `branch` | Lightweight `git worktree` | Provisions `~/.ttagy/workspaces/<uuid>` with dedicated `HEAD` and staging index |
| `share` | Group shared worktree | Shares collaborative branch `ttagy/shared/<group_id>` across pairing agents |

---

## 2. Asynchronous Actor Message Bus

- **Private Bounded Inboxes**: Each subagent owns an inbox with `capacity = 128`.
- **Zero-Polling Wakeup**: When an agent waits on messages, `tokio::select!` wakes the task immediately upon message arrival.
- **Dead Letter Queue (DLQ)**: Failed, timed out, or unroutable messages are safely stored in the DLQ for auditing.

---

## 3. Deadlock Detection in Wait-For Graph (WFG)

- **Microsecond Cycle Prevention**: When agent $A$ waits on agent $B$, an incremental reachability check ($B \rightsquigarrow A$) intercepts cyclical waits in $\le 10\mu\text{s}$.
- **5-Second Timeout Circuit Breaker**: Background scanner breaks hung dependency chains by selecting the leaf-most victim agent.
- **Two-Phase Cascade Kill**: BFS traversal broadcasts `CancellationToken` soft cancellation followed by hard `SIGKILL` and worktree reclamation.
