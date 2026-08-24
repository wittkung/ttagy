# Enterprise Observability & Prometheus Metrics Guide

## 1. Prometheus Scraping (`GET /metrics`)

TTAgy exposes standard OpenMetrics endpoints at `http://127.0.0.1:8970/metrics`.

### Core Metrics Table

| Metric Name | Type | Description |
| :--- | :--- | :--- |
| `ttagy_uptime_seconds` | Gauge | Daemon uptime in seconds |
| `ttagy_requests_total` | Counter | Total chat requests processed |
| `ttagy_tokens_total` | Counter | Tokens consumed with label `type="prompt|output|thinking"` |
| `ttagy_workers` | Gauge | Number of pre-forked workers with label `state="active|idle"` |
| `ttagy_concurrency_available_permits` | Gauge | Available semaphore concurrency permits |
| `ttagy_deadlocks_prevented_total` | Counter | Deadlock wait-dependency cycles detected and broken |

---

## 2. W3C Distributed Tracing (`traceparent`)

TTAgy natively parses incoming `traceparent` headers:
```text
traceparent: 00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01
```
Spans are propagated across daemon handlers, worker subprocesses, subagents, and tools.
Query full trace call trees via:
```bash
curl http://127.0.0.1:8970/api/v1/telemetry/traces/4bf92f3577b34da6a3ce929d0e0e4736
```
