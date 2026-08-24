//! TTAgy Subagent Mesh DAG Topology, Cycle Detection & Cascading Termination

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tokio_util::sync::CancellationToken;
use crate::workspace_manager::WorkspaceMode;

pub const MAX_SUBAGENT_DEPTH: usize = 3;
pub const MAX_CHILDREN_PER_PARENT: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentSpec {
    pub subagent_id: Option<String>,
    pub role: String,
    pub prompt: String,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_effort")]
    pub effort: String,
    #[serde(default = "default_workspace_mode")]
    pub workspace_mode: WorkspaceMode,
}

fn default_model() -> String {
    "gemini-3.7-flash-high".to_string()
}
fn default_effort() -> String {
    "high".to_string()
}
fn default_workspace_mode() -> WorkspaceMode {
    WorkspaceMode::Branch
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentNodeInfo {
    pub id: String,
    pub parent_id: Option<String>,
    pub depth: usize,
    pub role: String,
    pub status: String,
    pub workspace_mode: WorkspaceMode,
    pub workspace_path: Option<PathBuf>,
    pub children: Vec<String>,
    pub created_at: u64,
}

pub struct SubagentNode {
    pub id: String,
    pub parent_id: Option<String>,
    pub depth: usize,
    pub role: String,
    pub status: Arc<RwLock<String>>,
    pub cancel_token: CancellationToken,
    pub children: Arc<RwLock<HashSet<String>>>,
    pub workspace_mode: WorkspaceMode,
    pub workspace_path: Option<PathBuf>,
    pub created_at: u64,
}

#[derive(Default)]
pub struct WaitForGraph {
    pub edges: HashMap<String, HashSet<String>>,
}

impl WaitForGraph {
    pub fn is_reachable(&self, from: &str, to: &str) -> bool {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(from.to_string());
        visited.insert(from.to_string());

        while let Some(curr) = queue.pop_front() {
            if curr == to {
                return true;
            }
            if let Some(neighbors) = self.edges.get(&curr) {
                for next in neighbors {
                    if visited.insert(next.clone()) {
                        queue.push_back(next.clone());
                    }
                }
            }
        }
        false
    }

    pub fn add_wait_edge(&mut self, waiter: &str, waitee: &str) -> Result<(), String> {
        if waiter == waitee {
            return Err(format!("Self-dependency deadlock: agent '{}' cannot wait on itself", waiter));
        }
        if self.is_reachable(waitee, waiter) {
            return Err(format!(
                "Deadlock cycle detected: adding wait edge '{}' -> '{}' creates cyclical dependency",
                waiter, waitee
            ));
        }
        self.edges.entry(waiter.to_string()).or_default().insert(waitee.to_string());
        Ok(())
    }

    pub fn remove_wait_edge(&mut self, waiter: &str, waitee: &str) {
        if let Some(set) = self.edges.get_mut(waiter) {
            set.remove(waitee);
            if set.is_empty() {
                self.edges.remove(waiter);
            }
        }
    }
}

pub struct SubagentMesh {
    nodes: Arc<RwLock<HashMap<String, Arc<SubagentNode>>>>,
    wait_graph: Arc<RwLock<WaitForGraph>>,
    semaphore: Arc<Semaphore>,
}

impl SubagentMesh {
    pub fn new(max_concurrency: usize) -> Self {
        Self {
            nodes: Arc::new(RwLock::new(HashMap::new())),
            wait_graph: Arc::new(RwLock::new(WaitForGraph::default())),
            semaphore: Arc::new(Semaphore::new(max_concurrency)),
        }
    }

    pub async fn invoke_subagents(
        &self,
        parent_id: Option<String>,
        specs: Vec<SubagentSpec>,
    ) -> Result<Vec<String>, String> {
        let mut nodes = self.nodes.write().await;
        let parent_depth = match &parent_id {
            Some(pid) => {
                let parent = nodes.get(pid).ok_or_else(|| format!("Parent agent '{}' not found", pid))?;
                let children = parent.children.read().await;
                if children.len() + specs.len() > MAX_CHILDREN_PER_PARENT {
                    return Err(format!(
                        "Max children quota exceeded (max: {}) for parent '{}'",
                        MAX_CHILDREN_PER_PARENT, pid
                    ));
                }
                parent.depth
            }
            None => 0,
        };

        let target_depth = parent_depth + 1;
        if target_depth > MAX_SUBAGENT_DEPTH {
            return Err(format!(
                "Max subagent nesting depth ({}) exceeded. Attempted depth: {}",
                MAX_SUBAGENT_DEPTH, target_depth
            ));
        }

        let mut spawned_ids = Vec::new();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        for spec in specs {
            if self.semaphore.available_permits() == 0 {
                return Err("Global subagent concurrency quota exhausted".to_string());
            }

            let agent_id = spec
                .subagent_id
                .unwrap_or_else(|| format!("subagent-{}", uuid::Uuid::new_v4()));

            let node = Arc::new(SubagentNode {
                id: agent_id.clone(),
                parent_id: parent_id.clone(),
                depth: target_depth,
                role: spec.role,
                status: Arc::new(RwLock::new("running".to_string())),
                cancel_token: CancellationToken::new(),
                children: Arc::new(RwLock::new(HashSet::new())),
                workspace_mode: spec.workspace_mode,
                workspace_path: None,
                created_at: now,
            });

            if let Some(ref pid) = parent_id {
                if let Some(parent) = nodes.get(pid) {
                    parent.children.write().await.insert(agent_id.clone());
                }
            }

            nodes.insert(agent_id.clone(), node);
            spawned_ids.push(agent_id);
        }

        Ok(spawned_ids)
    }

    pub async fn wait_for_peer(&self, waiter: &str, waitee: &str) -> Result<(), String> {
        let mut wfg = self.wait_graph.write().await;
        wfg.add_wait_edge(waiter, waitee)
    }

    pub async fn release_wait_for_peer(&self, waiter: &str, waitee: &str) {
        let mut wfg = self.wait_graph.write().await;
        wfg.remove_wait_edge(waiter, waitee);
    }

    pub async fn kill_cascade(&self, target_id: &str) -> Vec<String> {
        let mut nodes_write = self.nodes.write().await;
        if !nodes_write.contains_key(target_id) {
            return Vec::new();
        }

        let mut kill_targets = Vec::new();
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();

        queue.push_back(target_id.to_string());
        visited.insert(target_id.to_string());

        while let Some(curr_id) = queue.pop_front() {
            kill_targets.push(curr_id.clone());
            if let Some(node) = nodes_write.get(&curr_id) {
                let children = node.children.read().await;
                for child_id in children.iter() {
                    if visited.insert(child_id.clone()) {
                        queue.push_back(child_id.clone());
                    }
                }
            }
        }

        for target in &kill_targets {
            if let Some(node) = nodes_write.remove(target) {
                node.cancel_token.cancel();
            }
        }

        let mut wfg = self.wait_graph.write().await;
        for target in &kill_targets {
            wfg.edges.remove(target);
        }

        kill_targets
    }

    pub async fn kill_all(&self) -> Vec<String> {
        let mut nodes = self.nodes.write().await;
        let mut killed = Vec::new();

        for (id, node) in nodes.drain() {
            node.cancel_token.cancel();
            killed.push(id);
        }

        let mut wfg = self.wait_graph.write().await;
        wfg.edges.clear();

        killed
    }

    pub async fn list_subagents(&self) -> Vec<SubagentNodeInfo> {
        let nodes = self.nodes.read().await;
        let mut list = Vec::new();

        for node in nodes.values() {
            let status = node.status.read().await.clone();
            let children: Vec<String> = node.children.read().await.iter().cloned().collect();
            list.push(SubagentNodeInfo {
                id: node.id.clone(),
                parent_id: node.parent_id.clone(),
                depth: node.depth,
                role: node.role.clone(),
                status,
                workspace_mode: node.workspace_mode.clone(),
                workspace_path: node.workspace_path.clone(),
                children,
                created_at: node.created_at,
            });
        }
        list
    }

    pub async fn get_subagent(&self, id: &str) -> Option<SubagentNodeInfo> {
        let nodes = self.nodes.read().await;
        let node = nodes.get(id)?;
        let status = node.status.read().await.clone();
        let children: Vec<String> = node.children.read().await.iter().cloned().collect();

        Some(SubagentNodeInfo {
            id: node.id.clone(),
            parent_id: node.parent_id.clone(),
            depth: node.depth,
            role: node.role.clone(),
            status,
            workspace_mode: node.workspace_mode.clone(),
            workspace_path: node.workspace_path.clone(),
            children,
            created_at: node.created_at,
        })
    }
}
