# Data Model: AGY CLI Superset & Session Pool

**Feature**: [`specs/004-agy-cli-superset-and-session-pool`](file:///Users/kevintung/Documents/dev/infra/ttagy/specs/004-agy-cli-superset-and-session-pool/spec.md)
**Status**: `Completed`
**Created**: 2026-08-24

---

## 1. Domain Entity Relationships

```mermaid
classDiagram
    class TtagyRequest {
        +String prompt
        +Option~String~ sessionId
        +Option~String~ model
        +Option~String~ effort
        +Option~f64~ temperature
        +Option~String~ systemInstruction
        +Option~String~ jsonSchema
        +Option~String~ agent
        +Option~String~ mode
        +Option~String~ conversationId
        +Option~bool~ continueLast
        +Option~String~ project
        +Vec~String~ addDirs
        +Option~bool~ sandbox
        +Option~bool~ dangerouslySkipPermissions
        +Option~bool~ disableSlashCommands
        +u64 timeoutSecs
    }

    class SessionMetadata {
        +String sessionId
        +String agentId
        +String model
        +String status
        +u64 createdAt
        +u64 lastAccessedAt
        +u64 ttlSecs
        +usize turnCount
        +usize totalTokens
    }

    class SessionMessage {
        +String id
        +String role
        +String content
        +Option~String~ thinkingContent
        +Vec~Value~ toolCalls
        +Vec~Value~ toolResults
        +u64 createdAt
        +bool pinned
        +Option~usize~ tokenCount
    }

    class McpServerConfig {
        +String name
        +String transport
        +Option~String~ command
        +Option~Vec~String~~ args
        +Option~HashMap~String, String~~ env
        +Option~String~ url
        +bool autoRestart
    }

    class ModelCapability {
        +String id
        +String name
        +String provider
        +usize contextWindow
        +usize maxOutputTokens
        +Vec~String~ supportedEfforts
        +bool supportsStreaming
        +bool supportsToolCalling
        +bool supportsJsonSchema
    }

    SessionMetadata --> SessionMessage
    TtagyRequest --> SessionMetadata
    McpServerConfig --> ModelCapability
```
