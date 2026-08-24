# Feature Specification: 005 Subagent Mesh & Distributed Workspace Orchestration

**Feature**: `005-subagent-mesh-and-workspaces`
**Type**: `[Full SDD]`
**Status**: `Specified`
**Created**: 2026-08-24

---

## 1. Problem Statement & Motivation

While TTAgy supports high-speed single-turn and multi-turn conversations through its Warm Worker Pool and Stateful Session Store, modern AI development (especially within the Antigravity architecture) relies heavily on **autonomous multi-agent collaboration (Subagent Mesh)**:
1. **Concurrency & Specialization**: Breaking down complex engineering tasks across specialized agents (e.g. `Codebase Researcher`, `Test Engineer`, `Security Auditor`) running concurrently.
2. **Workspace Contention**: Multiple subagents modifying the same working directory concurrently leads to file overwrite collisions, broken builds, and dirty git working trees.
3. **Inter-Agent Communication & Deadlocks**: Subagents communicating via peer-to-peer messaging require an asynchronous, non-blocking message broker with deadlock prevention (detecting cyclical wait-for-message graphs).

Feature 005 adds the **Subagent Mesh Runtime** to `ttagyd`, providing virtualized subagent lifecycle management, three-state workspace isolation (`inherit`, `branch`, `share`), and an asynchronous Actor message bus.

---

## 2. User Scenarios & Functional Requirements

### 2.1 User Scenario 1: Parallel Subagent Delegation & Batch Invocation
- **Given** a parent client request requiring 3 distinct audits (e.g. architecture, tests, security).
- **When** calling `POST /api/v1/subagents/invoke` with an array of Subagent specifications (`TypeName`, `Role`, `Prompt`, `Workspace`, `Model`).
- **Then** `ttagyd` spawns and orchestrates 3 isolated subagents concurrently, returning unique `conversation_id` handles and streaming events asynchronously.

### 2.2 User Scenario 2: Branched Workspace Isolation via Git Worktree
- **Given** a subagent invoked with `Workspace: "branch"`.
- **When** the subagent begins execution.
- **Then** `ttagyd` dynamically provisions an isolated `git worktree` at a temporary path (e.g. `~/.ttagy/workspaces/<uuid>`), allowing the subagent to make filesystem edits and run tests without polluting the parent developer workspace.

### 2.3 User Scenario 3: Inter-Agent Point-to-Point Messaging (Actor Inbox)
- **Given** active subagents `subagent-A` and `subagent-B`.
- **When** `subagent-A` sends a message to `subagent-B` via `POST /api/v1/subagents/message`.
- **Then** `ttagyd` routes the message into `subagent-B`'s inbox, wake up `subagent-B` reactively, and records the interaction in the global DAG trajectory.

### 2.4 User Scenario 4: Deadlock Prevention & Cascade Subagent Disposal
- **Given** a tree of spawned subagents.
- **When** the parent conversation completes or is aborted by the client.
- **Then** `ttagyd` cleanly cascades termination (`kill_all`) to all child and descendant subagents, immediately reclaiming worktrees, temp files, and worker pool permits.

---

## 3. Non-Functional Requirements & Safety Boundaries

1. **Workspace Safety**: Branched worktrees must be cleaned up automatically on subagent exit or error.
2. **Actor Message Isolation**: Inboxes must be bounded (e.g. max 128 queued messages) to prevent memory ballooning under high message volume.
3. **Deadlock Detection**: Cyclical message waiting graphs (A waits on B while B waits on A) must be detected with timeout errors within $\le 5\text{s}$.
4. **Deterministic Local Simulation**: All subagent mesh behaviors must be testable 100% offline via `mock-agy` within local CI.
