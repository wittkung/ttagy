# Implementation Plan: Enterprise Observability, W3C Tracing & Prometheus Metrics

**Feature**: [`specs/006-observability-and-telemetry`](file:///Users/kevintung/Documents/dev/infra/ttagy/specs/006-observability-and-telemetry/spec.md)
**Status**: `Planned`
**Created**: 2026-08-24
**Branch**: `main`

---

## 1. Technical Context & Objectives

This plan implements Prometheus metrics export (`/metrics`), W3C distributed tracing with bounded in-memory span buffering, node statistics (`/api/v1/telemetry/stats`), and automated credential redaction.

---

## 2. Tasks Breakdown

- **Phase 1: Telemetry Core Engine (`crates/ttagyd/src/telemetry/`)**
  - Implement `metrics.rs` (atomic counters, gauges, Prometheus exposition formatter).
  - Implement `tracer.rs` (W3C TraceContext header parser and ring-buffered span store).
  - Implement `redaction.rs` (fast regex-based credential sanitizer).

- **Phase 2: Routes & Middleware Integration (`crates/ttagyd/src/v1/routes.rs`, `crates/ttagyd/src/main.rs`)**
  - Add `GET /metrics` Prometheus endpoint.
  - Add `GET /api/v1/telemetry/stats` and `GET /api/v1/telemetry/traces/:id`.
  - Instrument `stream_handler` and `subagent_mesh` to record metrics and trace spans.

- **Phase 3: CI Quality Gate & Verification**
  - Run `bash scripts/local-ci.sh` verifying 100% PASS with 0 cloud tokens.
