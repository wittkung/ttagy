# Feature Specification: 006 Enterprise Observability, W3C Tracing & Prometheus Metrics

**Feature**: `006-observability-and-telemetry`
**Type**: `[Full SDD]`
**Status**: `Specified`
**Created**: 2026-08-24

---

## 1. Problem Statement & Motivation

As TTAgy scales to support concurrent multi-agent meshes, pre-warmed worker pools, and stateful multi-turn session storage, observability becomes a critical operational requirement:
1. **Distributed Tracing & Causality**: Complex tasks involve nested subagents, P2P Actor messages, and tool invocations. Without W3C Distributed Tracing (`traceparent` / `tracestate`), identifying bottlenecks or failure roots across the DAG is impossible.
2. **Prometheus Metrics Standard**: Production deployments require standard Prometheus metrics (`/metrics`) for alerting on token usage, latency percentiles (P50/P95/P99), pool saturation, and deadlocks.
3. **Security & Privacy Redaction**: Production logs must never leak bearer tokens, API keys (`sk-...`, `ghp-...`), or sensitive user data into WAL journals or disk logs.

Feature 006 implements the **Enterprise Observability & Telemetry Engine** in `ttagyd`.

---

## 2. User Scenarios & Functional Requirements

### 2.1 User Scenario 1: End-to-End W3C Distributed Tracing
- **Given** an incoming client request with or without a `traceparent` header (e.g. `00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01`).
- **When** the request flows through `ttagyd`, worker pool acquisition, subagent dispatch, and tool execution.
- **Then** each span records parent-child relationships, elapsed timing, token counts, and is retrievable via `GET /api/v1/telemetry/traces/:trace_id`.

### 2.2 User Scenario 2: Prometheus Metrics Scraping
- **Given** a Prometheus scraper or monitoring agent (e.g. Grafana Agent / Datadog).
- **When** scraping `GET /metrics`.
- **Then** `ttagyd` returns standard Prometheus text format metrics with counters, gauges, and histograms.

### 2.3 User Scenario 3: Real-Time Telemetry Stats Dashboard API
- **Given** a web dashboard, CLI TUI, or developer inspecting node health.
- **When** querying `GET /api/v1/telemetry/stats`.
- **Then** `ttagyd` returns a real-time JSON snapshot of token usage, active workers, semaphore permits, memory LRU hit rate, and DLQ errors.

### 2.4 User Scenario 4: Automated Credential & Token Redaction
- **Given** prompts, stderr logs, or tool results containing secrets (`Authorization: Bearer ...`, `ghp_...`, `AIza...`, `sk-...`).
- **When** writing audit logs or persisting WAL snapshots.
- **Then** the redaction engine sanitizes the sensitive strings with `[REDACTED:...]` tokens without corrupting JSON structures.

---

## 3. Non-Functional Requirements & Safety Boundaries

1. **Zero Runtime Overhead**: In-memory metric updates (atomic counters and fixed buckets) must add $<1\mu\text{s}$ per request turn.
2. **Standard Prometheus Text Format**: `/metrics` must strictly conform to OpenMetrics / Prometheus Exposition Format.
3. **Memory Safety**: Trace storage must be bounded (e.g. LRU cache of last 1,000 traces) to prevent unbounded memory growth.
4. **Deterministic Local Simulation**: Telemetry endpoints and metrics must be 100% testable locally in CI with zero cloud calls.
