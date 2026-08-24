# Data Model: Enterprise Observability & Telemetry

**Feature**: [`specs/006-observability-and-telemetry`](file:///Users/kevintung/Documents/dev/infra/ttagy/specs/006-observability-and-telemetry/spec.md)
**Status**: `Completed`
**Created**: 2026-08-24

---

## 1. Observability Domain Entities

```mermaid
classDiagram
    class TraceSpan {
        +String traceId
        +String spanId
        +Option~String~ parentSpanId
        +String name
        +String kind
        +u64 startTimeUnixMs
        +u64 durationMs
        +HashMap~String, String~~ attributes
        +String status
    }

    class MetricsSnapshot {
        +u64 totalRequests
        +u64 totalPromptTokens
        +u64 totalOutputTokens
        +u64 totalThinkingTokens
        +usize activeWorkers
        +usize idleWorkers
        +usize availablePermits
        +usize deadlocksPrevented
        +f64 p50DurationMs
        +f64 p95DurationMs
        +f64 p99DurationMs
    }

    class RedactionRule {
        +String name
        +String pattern
        +String replacement
        +sanitize(input) String
    }

    TraceSpan --> MetricsSnapshot
```
