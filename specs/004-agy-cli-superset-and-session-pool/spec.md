# Feature Specification: 004 AGY CLI Superset Evolution, Warm Worker Pool & Stateful Session Store

**Feature**: `004-agy-cli-superset-and-session-pool`
**Type**: `[Full SDD]`
**Status**: `Specified`
**Created**: 2026-08-24

---

## 1. Problem Statement & Motivation

TTAgy has established a robust multi-language SDK ecosystem and deterministic chaos test suite. However, currently `ttagyd` still spawns a fresh `agy` process per request (340ms ~ 800ms cold start latency), has no multi-turn stateful session storage, lacks virtualized MCP management endpoints, and does not fully expose the complete parameter set of the Google Antigravity CLI (`agy`).

By transforming TTAgy into a **strict superset** of `agy` CLI, TTAgy will provide:
1. **Parameter Supersymmetry**: 100% full coverage of all native `agy` CLI arguments across all 8 language SDKs.
2. **Warm Worker Pool**: Standby pre-warmed worker pool communicating via full-duplex `stream-json` pipes, achieving $\le 1.8\text{ms}$ sub-2ms response latency.
3. **Stateful Multi-Turn Session Store**: Memory LRU (`moka`) + Disk WAL journal for multi-turn conversation caching, context window sliding/compaction, and instant crash recovery.
4. **Virtualized MCP & Catalog Control Plane**: Dynamic `/api/v1/models`, `/api/v1/agents`, `/api/v1/mcp/servers`, and `/api/v1/sessions` REST/SSE + UDS endpoints.

---

## 2. User Scenarios & Functional Requirements

### 2.1 User Scenario 1: Sub-2ms Hot Inference via Warm Worker Pool
- **Given** a long-running `ttagyd` daemon with `min_idle = 2` pre-forked workers.
- **When** any client SDK sends a chat request over UDS (`/tmp/ttagy.sock`) or TCP (`:8970`).
- **Then** `ttagyd` acquires an idle worker from the pool, pipes the request frame over stdin in $<0.2\text{ms}$, streams tokens immediately ($\le 1.8\text{ms}$ TTFT), and automatically recycles the worker upon completion.

### 2.2 User Scenario 2: Stateful Multi-Turn Conversation & Sliding Window Compaction
- **Given** a multi-turn conversation with `session_id = "ses_..."`.
- **When** the conversation token count approaches the high watermark (75% of context window).
- **Then** `ttagyd` automatically triggers background compaction (pruning old tool outputs and generating hierarchical summaries), appending to WAL without interrupting ongoing turns.

### 2.3 User Scenario 3: Virtualized Dynamic MCP Tool Server Management
- **Given** a running `ttagyd` daemon.
- **When** a user or client issues `POST /api/v1/mcp/servers` with an MCP server configuration (command/args or SSE URL).
- **Then** `ttagyd` hot-plugs the MCP server, negotiates tool definitions, and immediately exposes them to all subsequent Agent turns without restarting the daemon.

### 2.4 User Scenario 4: Parameter Supersymmetry Across All 8 SDKs
- **Given** requests in any of the 8 SDKs (Rust, TS, Python, Go, Dart, C-FFI, Java, C#).
- **When** specifying advanced CLI parameters (`agent`, `mode`, `conversation_id`, `continue_last`, `project`, `add_dirs`, `system_instruction`, `temperature`, `sandbox`, `dangerously_skip_permissions`).
- **Then** the request is validated against Draft-07 contracts and passed transparently to the execution engine.

---

## 3. Non-Functional Requirements & Safety Boundaries

1. **Latency SLA**: Warm pool request acquisition and first token dispatch must be $\le 2\text{ms}$ under standard load.
2. **Crash Resilience & Memory Isolation**:
   - Single worker crash must NOT affect `ttagyd` or other workers; replacement worker is automatically replenished.
   - Worker processes must be recycled after $N$ turns (default 100) or when RSS memory exceeds threshold (default 512MB).
3. **Data Integrity**: All session state mutations must be atomically written to append-only WAL logs before acknowledging turns to the client.
4. **Zero-Cloud-Quota Local Determinism**: All new APIs and pool behaviors must be fully testable offline using `mock-agy` within local CI in $\le 2\text{s}$.
