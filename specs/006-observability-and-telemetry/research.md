# Research: Enterprise Observability, W3C Tracing & Prometheus Metrics

**Feature**: [`specs/006-observability-and-telemetry`](file:///Users/kevintung/Documents/dev/infra/ttagy/specs/006-observability-and-telemetry/spec.md)
**Status**: `Completed`
**Created**: 2026-08-24

---

## 1. W3C Distributed Tracing Standard

- **Format**: `traceparent: {version}-{trace_id}-{parent_id}-{trace_flags}`
  - `version`: `00`
  - `trace_id`: 32-hex-character global UUID
  - `parent_id`: 16-hex-character span ID
  - `trace_flags`: `01` (recorded)
- **Propagation**: Injected in HTTP/SSE headers, UDS metadata, and P2P Actor messages.

---

## 2. Prometheus Exposition Format & Metric Definitions

- `ttagy_requests_total{model, status}`: Total chat requests processed.
- `ttagy_request_duration_seconds{model, le}`: Histogram with buckets `[0.002, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 30.0]`.
- `ttagy_tokens_total{type, model}`: Total prompt, thinking, and output tokens.
- `ttagy_worker_pool_active`: Active workers.
- `ttagy_worker_pool_idle`: Idle warm workers.
- `ttagy_deadlocks_prevented_total`: Count of deadlock wait-cycles broken.

---

## 3. Secret Redaction & Sanitization Engine

- **Patterns**:
  - `Bearer [a-zA-Z0-9_\-\.]+` $\to$ `Bearer [REDACTED:AUTH_TOKEN]`
  - `(sk|ghp|AIza|xoxb)-[a-zA-Z0-9_\-]+` $\to$ `[REDACTED:API_KEY]`
  - Password / Secret regex patterns in structured JSON.
