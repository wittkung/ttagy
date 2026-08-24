# Quickstart: AGY CLI Superset & Session Pool

**Feature**: [`specs/004-agy-cli-superset-and-session-pool`](file:///Users/kevintung/Documents/dev/infra/ttagy/specs/004-agy-cli-superset-and-session-pool/spec.md)
**Status**: `Ready for Verification`
**Created**: 2026-08-24

---

## 1. Advanced Stateful Conversation in Rust

```rust
use ttagy_client::{ClientConfig, TtagyClient, TtagyRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = TtagyClient::new(ClientConfig::default());

    // Turn 1: Specify agent mode, persona, and add workspace directory
    let req1 = TtagyRequest::builder("Initialize project architecture")
        .agent("codebase-architect")
        .mode("plan")
        .add_dir("/Users/kevintung/dev/my-project")
        .build();

    let resp1 = client.chat(req1).await?;
    println!("Turn 1 Response: {}", resp1.content);

    // Turn 2: Continue the conversation with stateful session_id
    let req2 = TtagyRequest::builder("Implement the data layer")
        .conversation_id(&resp1.session_id)
        .build();

    let resp2 = client.chat(req2).await?;
    println!("Turn 2 Response: {}", resp2.content);
    Ok(())
}
```

---

## 2. Dynamic MCP Server Mount via HTTP API

```bash
curl -X POST http://127.0.0.1:8970/api/v1/mcp/servers \
  -H "Content-Type: application/json" \
  -d '{
    "name": "filesystem-mcp",
    "transport": "stdio",
    "command": "npx",
    "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
  }'
```
