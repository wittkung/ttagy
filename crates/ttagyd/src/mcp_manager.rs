//! TTAgy Virtualized MCP (Model Context Protocol) Server & Tool Manager

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpServerConfig {
    pub name: String,
    pub transport: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub args: Option<Vec<String>>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default = "default_auto_restart")]
    pub auto_restart: bool,
}

fn default_auto_restart() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpToolInfo {
    pub server_name: String,
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpServerStatus {
    pub config: McpServerConfig,
    pub status: String,
    pub tools_count: usize,
    pub last_error: Option<String>,
}

struct McpServerHandle {
    config: McpServerConfig,
    status: String,
    tools: Vec<McpToolInfo>,
    last_error: Option<String>,
}

pub struct McpManager {
    servers: Arc<RwLock<HashMap<String, McpServerHandle>>>,
}

impl Default for McpManager {
    fn default() -> Self {
        Self::new()
    }
}

impl McpManager {
    pub fn new() -> Self {
        Self {
            servers: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_server(&self, config: McpServerConfig) -> Result<(), String> {
        let mut map = self.servers.write().await;
        let handle = McpServerHandle {
            config: config.clone(),
            status: "connected".to_string(),
            tools: vec![
                McpToolInfo {
                    server_name: config.name.clone(),
                    name: "default_tool".to_string(),
                    description: format!("Default virtual tool for {}", config.name),
                    input_schema: serde_json::json!({
                        "type": "object",
                        "properties": {
                            "input": { "type": "string" }
                        }
                    }),
                }
            ],
            last_error: None,
        };
        map.insert(config.name.clone(), handle);
        Ok(())
    }

    pub async fn unregister_server(&self, name: &str) -> Result<(), String> {
        let mut map = self.servers.write().await;
        if map.remove(name).is_some() {
            Ok(())
        } else {
            Err(format!("MCP Server '{}' not found", name))
        }
    }

    pub async fn list_servers(&self) -> Vec<McpServerStatus> {
        let map = self.servers.read().await;
        map.values()
            .map(|h| McpServerStatus {
                config: h.config.clone(),
                status: h.status.clone(),
                tools_count: h.tools.len(),
                last_error: h.last_error.clone(),
            })
            .collect()
    }

    pub async fn list_tools(&self) -> Vec<McpToolInfo> {
        let map = self.servers.read().await;
        map.values().flat_map(|s| s.tools.clone()).collect()
    }
}
