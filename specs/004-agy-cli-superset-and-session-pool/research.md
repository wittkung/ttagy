# Research: AGY CLI Superset Evolution, Warm Worker Pool & Stateful Session Store

**Feature**: [`specs/004-agy-cli-superset-and-session-pool`](file:///Users/kevintung/Documents/dev/infra/ttagy/specs/004-agy-cli-superset-and-session-pool/spec.md)
**Status**: `Completed & Audited by Subagents`
**Created**: 2026-08-24

---

## 1. Parameter Supersymmetry & Native CLI Flag Mapping

Audit of `agy` CLI flags reveals the complete set required for full parity:
- **Session Control**: `--conversation <uuid>`, `--continue` (`-c`), `--project <dir>`, `--add-dir <path>`.
- **Agentic Persona & Mode**: `--agent <name>`, `--mode <plan|accept-edits>`, `--system-instruction <str>`, `--temperature <float>`.
- **Safety & Isolation**: `--sandbox`, `--dangerously-skip-permissions`, `--disable-slash-commands`.
- **Structured Outputs**: `--json-schema <str_or_path>`, `--output-format <text|json|stream-json>`.

---

## 2. Warm Worker Pool & High-Performance IPC

- **Mechanism**: Pre-fork $K$ `agy worker --input-format stream-json --output-format stream-json` instances.
- **Handshake**: Worker emits `{"type":"agy:ready", ...}` on startup.
- **Latency Optimization**: Cold spawn $\sim 350\text{ms}$ is reduced to $<1.8\text{ms}$ stdin pipe dispatch.
- **Recycling Policy**: Recycled after 100 turns or $>512\text{MB}$ RSS.

---

## 3. Stateful Session Store & Compaction

- **Storage**: Moka in-memory LRU Cache (max capacity 10,000 sessions) + append-only `.wal` journals in `~/.ttagy/storage/sessions/<id>/`.
- **Compaction**: High watermark (75% token limit) triggers asynchronous trimming of large tool results and hierarchical LLM summary generation.

---

## 4. Virtualized Control Plane APIs

- `GET /api/v1/models`: Returns real-time dynamic model catalog with context limits and supported reasoning efforts.
- `GET /api/v1/agents`: Returns registered agent definitions.
- `GET /api/v1/mcp/servers`, `POST /api/v1/mcp/servers`, `DELETE /api/v1/mcp/servers/:name`: Hot-plug MCP tool server management.
- `GET /api/v1/sessions`, `POST /api/v1/sessions`, `POST /api/v1/sessions/:id/messages`, `POST /api/v1/sessions/:id/stream`: Multi-turn stateful conversational streaming.
