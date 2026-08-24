# Quickstart: Enterprise Observability & Telemetry

**Feature**: [`specs/006-observability-and-telemetry`](file:///Users/kevintung/Documents/dev/infra/ttagy/specs/006-observability-and-telemetry/spec.md)
**Status**: `Ready for Verification`
**Created**: 2026-08-24

---

## 1. Scraping Prometheus Metrics

```bash
curl http://127.0.0.1:8970/metrics
```

Example output:
```text
# HELP ttagy_requests_total Total number of chat requests processed
# TYPE ttagy_requests_total counter
ttagy_requests_total{model="gemini-3.7-flash",status="success"} 42

# HELP ttagy_tokens_total Total tokens consumed
# TYPE ttagy_tokens_total counter
ttagy_tokens_total{model="gemini-3.7-flash",type="prompt"} 12400
ttagy_tokens_total{model="gemini-3.7-flash",type="output"} 8900

# HELP ttagy_worker_pool_active Current active worker count
# TYPE ttagy_worker_pool_active gauge
ttagy_worker_pool_active 2
```

---

## 2. Fetching Real-Time Node Statistics

```bash
curl http://127.0.0.1:8970/api/v1/telemetry/stats
```
