# Data Model: Subagent Mesh & Workspace Orchestration

**Feature**: [`specs/005-subagent-mesh-and-workspaces`](file:///Users/kevintung/Documents/dev/infra/ttagy/specs/005-subagent-mesh-and-workspaces/spec.md)
**Status**: `Completed`
**Created**: 2026-08-24

---

## 1. Subagent Mesh Domain Entities

```mermaid
classDiagram
    class SubagentNode {
        +String id
        +Option~String~ parentId
        +usize depth
        +SubagentRole role
        +SubagentStatus status
        +WorkspaceMode workspaceMode
        +Option~PathBuf~ workspacePath
        +HashSet~String~ children
        +u64 createdAt
    }

    class WorkspaceGuard {
        +String id
        +WorkspaceMode mode
        +PathBuf path
        +Option~String~ branchName
        +cleanupNow()
    }

    class ActorMessage {
        +String messageId
        +String senderId
        +String recipientId
        +MessagePayload payload
        +Option~String~ correlationId
        +u64 createdAt
    }

    class WaitForGraph {
        +HashMap~String, HashSet~String~~ edges
        +HashMap~Tuple, Instant~ waitStartTimes
        +addWaitEdge(waiter, waitee)
        +removeWaitEdge(waiter, waitee)
        +isReachable(from, to)
    }

    SubagentNode --> WorkspaceGuard
    SubagentNode --> ActorMessage
    SubagentNode --> WaitForGraph
```
