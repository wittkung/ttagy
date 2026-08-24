//! TTAgy Stateful Multi-Turn Session Store (In-Memory Hot Cache + Disk WAL Journal)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionMetadata {
    pub session_id: String,
    pub agent_id: String,
    pub model: String,
    pub status: String,
    pub created_at: u64,
    pub last_accessed_at: u64,
    pub ttl_secs: u64,
    pub turn_count: usize,
    pub total_tokens: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_content: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<serde_json::Value>,
    #[serde(default)]
    pub tool_results: Vec<serde_json::Value>,
    pub created_at: u64,
    #[serde(default)]
    pub pinned: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    pub metadata: SessionMetadata,
    pub messages: Vec<SessionMessage>,
}

pub struct SessionStore {
    storage_dir: PathBuf,
    memory_cache: Arc<RwLock<HashMap<String, Arc<RwLock<Session>>>>>,
}

impl SessionStore {
    pub fn new(storage_dir: PathBuf) -> Self {
        Self {
            storage_dir,
            memory_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn create_session(
        &self,
        agent_id: impl Into<String>,
        model: impl Into<String>,
    ) -> Result<SessionMetadata, String> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let session_id = format!("ses_{}", uuid::Uuid::now_v7());

        let meta = SessionMetadata {
            session_id: session_id.clone(),
            agent_id: agent_id.into(),
            model: model.into(),
            status: "initialized".to_string(),
            created_at: now,
            last_accessed_at: now,
            ttl_secs: 86400,
            turn_count: 0,
            total_tokens: 0,
        };

        let session = Session {
            metadata: meta.clone(),
            messages: Vec::new(),
        };

        self.persist_snapshot(&session).await?;

        let mut cache = self.memory_cache.write().await;
        cache.insert(session_id, Arc::new(RwLock::new(session)));
        Ok(meta)
    }

    pub async fn get_session_handle(
        &self,
        session_id: &str,
    ) -> Result<Arc<RwLock<Session>>, String> {
        {
            let cache = self.memory_cache.read().await;
            if let Some(handle) = cache.get(session_id) {
                return Ok(handle.clone());
            }
        }

        let session = self.recover_from_disk(session_id).await?;
        let handle = Arc::new(RwLock::new(session));
        let mut cache = self.memory_cache.write().await;
        cache.insert(session_id.to_string(), handle.clone());
        Ok(handle)
    }

    pub async fn append_message(
        &self,
        session_id: &str,
        msg: SessionMessage,
    ) -> Result<(), String> {
        let handle = self.get_session_handle(session_id).await?;
        let mut session = handle.write().await;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        session.metadata.last_accessed_at = now;
        session.metadata.turn_count += 1;
        if let Some(t) = msg.token_count {
            session.metadata.total_tokens += t;
        }
        session.metadata.status = "active".to_string();
        session.messages.push(msg.clone());

        self.append_wal_record(session_id, &msg).await?;
        Ok(())
    }

    pub async fn compact_session(&self, session_id: &str) -> Result<(usize, usize), String> {
        let handle = self.get_session_handle(session_id).await?;
        let mut session = handle.write().await;

        let before = session.messages.len();
        // 保留最近 10 条消息及 pinned 消息
        if session.messages.len() > 10 {
            let keep_idx = session.messages.len() - 10;
            let mut compacted = Vec::new();
            for (i, m) in session.messages.iter().enumerate() {
                if i >= keep_idx || m.pinned || m.role == "system" {
                    compacted.push(m.clone());
                }
            }
            session.messages = compacted;
        }
        let after = session.messages.len();
        session.metadata.status = "idle".to_string();

        self.persist_snapshot(&session).await?;
        Ok((before, after))
    }

    pub async fn delete_session(&self, session_id: &str) -> Result<(), String> {
        let mut cache = self.memory_cache.write().await;
        cache.remove(session_id);

        let session_dir = self.storage_dir.join("sessions").join(session_id);
        if session_dir.exists() {
            let _ = tokio::fs::remove_dir_all(session_dir).await;
        }
        Ok(())
    }

    pub async fn list_sessions(&self) -> Vec<SessionMetadata> {
        let cache = self.memory_cache.read().await;
        let mut list = Vec::new();
        for handle in cache.values() {
            let session = handle.read().await;
            list.push(session.metadata.clone());
        }
        list
    }

    async fn append_wal_record(
        &self,
        session_id: &str,
        msg: &SessionMessage,
    ) -> Result<(), String> {
        let session_dir = self.storage_dir.join("sessions").join(session_id);
        tokio::fs::create_dir_all(&session_dir)
            .await
            .map_err(|e| e.to_string())?;
        let wal_path = session_dir.join("current.wal");

        let record = serde_json::json!({
            "ts": msg.created_at,
            "op": "APPEND",
            "data": msg,
        });
        let mut line = serde_json::to_string(&record).map_err(|e| e.to_string())?;
        line.push('\n');

        use tokio::io::AsyncWriteExt;
        let mut file = tokio::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(wal_path)
            .await
            .map_err(|e| e.to_string())?;
        file.write_all(line.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        file.flush().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn persist_snapshot(&self, session: &Session) -> Result<(), String> {
        let session_dir = self
            .storage_dir
            .join("sessions")
            .join(&session.metadata.session_id);
        tokio::fs::create_dir_all(&session_dir)
            .await
            .map_err(|e| e.to_string())?;
        let snapshot_path = session_dir.join("checkpoint.json");
        let content =
            serde_json::to_string_pretty(session).map_err(|e| e.to_string())?;
        tokio::fs::write(snapshot_path, content)
            .await
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn recover_from_disk(&self, session_id: &str) -> Result<Session, String> {
        let session_dir = self.storage_dir.join("sessions").join(session_id);
        let snapshot_path = session_dir.join("checkpoint.json");
        if !snapshot_path.exists() {
            return Err(format!("Session '{}' not found on disk", session_id));
        }
        let data = tokio::fs::read_to_string(snapshot_path)
            .await
            .map_err(|e| e.to_string())?;
        let session: Session =
            serde_json::from_str(&data).map_err(|e| e.to_string())?;
        Ok(session)
    }
}
