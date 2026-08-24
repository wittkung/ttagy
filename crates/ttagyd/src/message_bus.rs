//! TTAgy Actor Message Bus & Reactive Broker

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorMessage {
    pub message_id: String,
    pub sender_id: String,
    pub recipient_id: String,
    pub content: String,
    pub reply_to: Option<String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeadLetter {
    pub message_id: String,
    pub sender_id: String,
    pub recipient_id: String,
    pub content: String,
    pub reason: String,
    pub failed_at: u64,
}

#[derive(Clone)]
pub struct MessageBus {
    registry: Arc<RwLock<HashMap<String, mpsc::Sender<ActorMessage>>>>,
    dlq: Arc<RwLock<Vec<DeadLetter>>>,
    default_timeout: Duration,
}

impl Default for MessageBus {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageBus {
    pub fn new() -> Self {
        Self {
            registry: Arc::new(RwLock::new(HashMap::new())),
            dlq: Arc::new(RwLock::new(Vec::new())),
            default_timeout: Duration::from_secs(5),
        }
    }

    pub async fn register_inbox(
        &self,
        agent_id: impl Into<String>,
        capacity: usize,
    ) -> mpsc::Receiver<ActorMessage> {
        let (tx, rx) = mpsc::channel(capacity);
        let mut reg = self.registry.write().await;
        reg.insert(agent_id.into(), tx);
        rx
    }

    pub async fn unregister_inbox(&self, agent_id: &str) {
        let mut reg = self.registry.write().await;
        reg.remove(agent_id);
    }

    pub async fn send_message(&self, message: ActorMessage) -> Result<(), String> {
        let recipient_id = message.recipient_id.clone();
        let maybe_tx = {
            let reg = self.registry.read().await;
            reg.get(&recipient_id).cloned()
        };

        let tx = match maybe_tx {
            Some(tx) => tx,
            None => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let dl = DeadLetter {
                    message_id: message.message_id,
                    sender_id: message.sender_id,
                    recipient_id: message.recipient_id,
                    content: message.content,
                    reason: "RecipientNotFound".to_string(),
                    failed_at: now,
                };
                self.dlq.write().await.push(dl);
                return Err(format!("Recipient '{}' not found in Actor registry", recipient_id));
            }
        };

        match tokio::time::timeout(self.default_timeout, tx.send(message.clone())).await {
            Ok(Ok(())) => Ok(()),
            Ok(Err(_)) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let dl = DeadLetter {
                    message_id: message.message_id,
                    sender_id: message.sender_id,
                    recipient_id: message.recipient_id,
                    content: message.content,
                    reason: "RecipientTerminated".to_string(),
                    failed_at: now,
                };
                self.dlq.write().await.push(dl);
                Err(format!("Recipient '{}' inbox closed", recipient_id))
            }
            Err(_) => {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let dl = DeadLetter {
                    message_id: message.message_id,
                    sender_id: message.sender_id,
                    recipient_id: message.recipient_id,
                    content: message.content,
                    reason: "MailboxFullTimeout".to_string(),
                    failed_at: now,
                };
                self.dlq.write().await.push(dl);
                Err(format!("Recipient '{}' inbox full and timed out", recipient_id))
            }
        }
    }

    pub async fn get_dlq(&self) -> Vec<DeadLetter> {
        self.dlq.read().await.clone()
    }
}
