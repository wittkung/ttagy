# Tasks: Enterprise Observability, W3C Tracing & Prometheus Metrics

**Feature**: [`specs/006-observability-and-telemetry`](file:///Users/kevintung/Documents/dev/infra/ttagy/specs/006-observability-and-telemetry/spec.md)
**Plan**: [`plan.md`](./plan.md)
**Status**: `Completed & Verified`

---

## Phase 1: Telemetry Core Engine (`crates/ttagyd/src/telemetry/`)

- [x] T001 [P] Create `crates/ttagyd/src/telemetry/metrics.rs` with Prometheus exposition formatter
- [x] T002 [P] Create `crates/ttagyd/src/telemetry/tracer.rs` with W3C TraceContext parser
- [x] T003 [P] Create `crates/ttagyd/src/telemetry/redaction.rs` with secret sanitization regexes
- [x] T004 Create `crates/ttagyd/src/telemetry/mod.rs` module export

---

## Phase 2: Routes & Control Plane Integration

- [x] T005 Integrate `TelemetryEngine` into `AppState` in `crates/ttagyd/src/main.rs`
- [x] T006 Add `GET /metrics` and `GET /api/v1/telemetry/*` in `crates/ttagyd/src/v1/routes.rs`

---

## Phase 3: CI/CD Quality Gate & Verification

- [x] T007 Run `bash scripts/local-ci.sh` verifying 100% PASS with 0 cloud tokens consumed
