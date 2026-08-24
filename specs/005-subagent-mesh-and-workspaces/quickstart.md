# Quickstart: Subagent Mesh & Workspace Orchestration

**Feature**: [`specs/005-subagent-mesh-and-workspaces`](file:///Users/kevintung/Documents/dev/infra/ttagy/specs/005-subagent-mesh-and-workspaces/spec.md)
**Status**: `Ready for Verification`
**Created**: 2026-08-24

---

## 1. Batch Subagent Delegation in Rust

```rust
use ttagy_client::{ClientConfig, TtagyClient, SubagentSpec, WorkspaceMode};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = TtagyClient::new(ClientConfig::default());

    // Dispatch 2 specialized subagents concurrently
    let subagents = vec![
        SubagentSpec {
            subagent_id: None,
            role: "researcher".to_string(),
            prompt: "Analyze codebase dependencies and memory safety".to_string(),
            model: "gemini-3.7-flash-high".to_string(),
            workspace_mode: WorkspaceMode::Inherit,
        },
        SubagentSpec {
            subagent_id: None,
            role: "tester".to_string(),
            prompt: "Write unit tests in an isolated branch".to_string(),
            model: "gemini-3.7-flash-high".to_string(),
            workspace_mode: WorkspaceMode::Branch,
        },
    ];

    let res = client.invoke_subagents("parent_session_123", subagents).await?;
    println!("Spawned Subagents: {:?}", res.spawned_subagents);
    Ok(())
}
```

---

## 2. Inter-Agent Point-to-Point Messaging

```bash
curl -X POST http://127.0.0.1:8970/api/v1/subagents/message \
  -H "Content-Type: application/json" \
  -d '{
    "sender_id": "subagent-1",
    "recipient_id": "subagent-2",
    "content": "Finished research report. Proceed with test generation.",
    "is_blocking_wait": false
  }'
```
