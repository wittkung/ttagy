use serde::{Deserialize, Serialize};

/// Antigravity CLI 官方支持的原生模型全量权威索引
pub const AGY_SUPPORTED_MODELS: &[&str] = &[
    "gemini-3.7-flash-high",
    "gemini-3.7-flash-medium",
    "gemini-3.7-flash-low",
    "gemini-3.6-flash-high",
    "gemini-3.6-flash-medium",
    "gemini-3.6-flash-low",
    "gemini-3.5-flash-high",
    "gemini-3.5-flash-medium",
    "gemini-3.5-flash-low",
    "gemini-3.1-pro-high",
    "gemini-3.1-pro-low",
    "claude-sonnet-4-6",
    "claude-opus-4-6-thinking",
    "gpt-oss-120b-medium",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TtagyRequest {
    #[serde(default = "generate_session_id")]
    pub session_id: String,
    pub prompt: String,
    #[serde(default = "default_model")]
    pub model: Option<String>,
    #[serde(default = "default_effort")]
    pub effort: Option<String>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default)]
    pub system_instruction: Option<String>,
    #[serde(default)]
    pub json_schema: Option<String>,
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
}

impl Default for TtagyRequest {
    fn default() -> Self {
        Self {
            session_id: generate_session_id(),
            prompt: String::new(),
            model: default_model(),
            effort: default_effort(),
            temperature: None,
            system_instruction: None,
            json_schema: None,
            timeout_secs: default_timeout_secs(),
        }
    }
}

fn generate_session_id() -> String {
    format!("session_{}", std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis())
}
fn default_model() -> Option<String> { Some("gemini-3.7-flash".to_string()) }
fn default_effort() -> Option<String> { Some("high".to_string()) }
fn default_timeout_secs() -> u64 { 60 }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TtagyResponse {
    pub session_id: String,
    pub status: String,
    pub content: String,
    #[serde(default)]
    pub thinking_content: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    pub elapsed_ms: f64,
    #[serde(default)]
    pub prompt_tokens: Option<usize>,
    #[serde(default)]
    pub output_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
}

/// 强类型流式推导事件 (符合 Draft-07 Discriminated Union 规范)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum TtagyStreamEvent {
    #[serde(rename = "agy:init")]
    Init {
        session_id: String,
        model: String,
        effort: String,
        backend_mode: String,
    },
    #[serde(rename = "agy:thinking_delta")]
    ThinkingDelta {
        session_id: String,
        text_delta: String,
        elapsed_ms: f64,
    },
    #[serde(rename = "agy:content_delta")]
    ContentDelta {
        session_id: String,
        text_delta: String,
        accumulated_chars: usize,
        elapsed_ms: f64,
    },
    #[serde(rename = "agy:done")]
    Done {
        session_id: String,
        full_content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        thinking_content: Option<String>,
        elapsed_ms: f64,
        #[serde(skip_serializing_if = "Option::is_none")]
        prompt_tokens: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        output_tokens: Option<usize>,
    },
    #[serde(rename = "agy:error")]
    Error {
        session_id: String,
        error_code: String,
        error_message: String,
        is_retryable: bool,
    },
}

impl TtagyStreamEvent {
    pub fn event_type(&self) -> &'static str {
        match self {
            Self::Init { .. } => "agy:init",
            Self::ThinkingDelta { .. } => "agy:thinking_delta",
            Self::ContentDelta { .. } => "agy:content_delta",
            Self::Done { .. } => "agy:done",
            Self::Error { .. } => "agy:error",
        }
    }
}
