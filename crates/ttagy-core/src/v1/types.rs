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
    #[serde(default = "generate_session_id", alias = "sessionId")]
    pub session_id: String,
    pub prompt: String,
    #[serde(default = "default_model")]
    pub model: Option<String>,
    #[serde(default = "default_effort")]
    pub effort: Option<String>,
    #[serde(default)]
    pub temperature: Option<f32>,
    #[serde(default, alias = "systemInstruction")]
    pub system_instruction: Option<String>,
    #[serde(default, alias = "jsonSchema", alias = "schemaPath")]
    pub json_schema: Option<String>,
    #[serde(default = "default_timeout_secs", alias = "timeoutSecs")]
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
fn default_effort() -> Option<String> { Some("low".to_string()) }
fn default_timeout_secs() -> u64 { 60 }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TtagyResponse {
    #[serde(alias = "sessionId")]
    pub session_id: String,
    pub status: String,
    pub content: String,
    #[serde(default, alias = "thinkingContent")]
    pub thinking_content: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(alias = "elapsedMs")]
    pub elapsed_ms: f64,
    #[serde(default, alias = "promptTokens")]
    pub prompt_tokens: Option<usize>,
    #[serde(default, alias = "outputTokens")]
    pub output_tokens: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "errorMessage")]
    pub error_message: Option<String>,
}

/// 强类型流式推导事件 (兼容 camelCase 与 snake_case)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum TtagyStreamEvent {
    #[serde(rename = "agy:init")]
    Init {
        #[serde(alias = "sessionId")]
        session_id: String,
        model: String,
        effort: String,
        #[serde(alias = "backendMode")]
        backend_mode: String,
    },
    #[serde(rename = "agy:thinking_delta")]
    ThinkingDelta {
        #[serde(alias = "sessionId")]
        session_id: String,
        #[serde(alias = "textDelta")]
        text_delta: String,
        #[serde(alias = "elapsedMs")]
        elapsed_ms: f64,
    },
    #[serde(rename = "agy:content_delta")]
    ContentDelta {
        #[serde(alias = "sessionId")]
        session_id: String,
        #[serde(alias = "textDelta")]
        text_delta: String,
        #[serde(alias = "accumulatedChars")]
        accumulated_chars: usize,
        #[serde(alias = "elapsedMs")]
        elapsed_ms: f64,
    },
    #[serde(rename = "agy:done")]
    Done {
        #[serde(alias = "sessionId")]
        session_id: String,
        #[serde(alias = "fullContent")]
        full_content: String,
        #[serde(skip_serializing_if = "Option::is_none", alias = "thinkingContent")]
        thinking_content: Option<String>,
        #[serde(alias = "elapsedMs")]
        elapsed_ms: f64,
        #[serde(skip_serializing_if = "Option::is_none", alias = "promptTokens")]
        prompt_tokens: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none", alias = "outputTokens")]
        output_tokens: Option<usize>,
    },
    #[serde(rename = "agy:error")]
    Error {
        #[serde(alias = "sessionId")]
        session_id: String,
        #[serde(alias = "errorCode")]
        error_code: String,
        #[serde(alias = "errorMessage")]
        error_message: String,
        #[serde(alias = "isRetryable")]
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
