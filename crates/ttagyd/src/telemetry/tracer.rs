//! TTAgy W3C Distributed Tracing Engine

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSpan {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub start_time_ms: u64,
    pub duration_ms: f64,
    pub attributes: HashMap<String, String>,
    pub status: String,
}

pub struct TraceManager {
    spans: Arc<RwLock<Vec<TraceSpan>>>,
    max_spans: usize,
}

impl Default for TraceManager {
    fn default() -> Self {
        Self::new(1000)
    }
}

impl TraceManager {
    pub fn new(max_spans: usize) -> Self {
        Self {
            spans: Arc::new(RwLock::new(Vec::with_capacity(max_spans))),
            max_spans,
        }
    }

    pub fn parse_traceparent(header_val: &str) -> Option<(String, String)> {
        let parts: Vec<&str> = header_val.trim().split('-').collect();
        if parts.len() == 4 && parts[0] == "00" && parts[1].len() == 32 && parts[2].len() == 16 {
            Some((parts[1].to_string(), parts[2].to_string()))
        } else {
            None
        }
    }

    pub fn generate_traceparent() -> (String, String, String) {
        let trace_id = format!("{:032x}", uuid::Uuid::new_v4().as_u128());
        let span_id = format!("{:016x}", (uuid::Uuid::new_v4().as_u128() & 0xFFFFFFFFFFFFFFFF));
        let header = format!("00-{}-{}-01", trace_id, span_id);
        (trace_id, span_id, header)
    }

    pub async fn record_span(&self, span: TraceSpan) {
        let mut list = self.spans.write().await;
        if list.len() >= self.max_spans {
            list.remove(0);
        }
        list.push(span);
    }

    pub async fn get_trace_spans(&self, trace_id: &str) -> Vec<TraceSpan> {
        let list = self.spans.read().await;
        list.iter().filter(|s| s.trace_id == trace_id).cloned().collect()
    }

    pub async fn get_recent_spans(&self, limit: usize) -> Vec<TraceSpan> {
        let list = self.spans.read().await;
        let start = list.len().saturating_sub(limit);
        list[start..].to_vec()
    }
}
