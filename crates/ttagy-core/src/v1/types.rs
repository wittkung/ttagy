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
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default, alias = "conversationId")]
    pub conversation_id: Option<String>,
    #[serde(default, alias = "continueLast")]
    pub continue_last: Option<bool>,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default, alias = "addDirs")]
    pub add_dirs: Vec<String>,
    #[serde(default)]
    pub sandbox: Option<bool>,
    #[serde(default, alias = "dangerouslySkipPermissions")]
    pub dangerously_skip_permissions: Option<bool>,
    #[serde(default, alias = "disableSlashCommands")]
    pub disable_slash_commands: Option<bool>,
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
            agent: None,
            mode: None,
            conversation_id: None,
            continue_last: None,
            project: None,
            add_dirs: Vec::new(),
            sandbox: None,
            dangerously_skip_permissions: None,
            disable_slash_commands: None,
            timeout_secs: default_timeout_secs(),
        }
    }
}

impl TtagyRequest {
    pub fn builder(prompt: impl Into<String>) -> TtagyRequestBuilder {
        TtagyRequestBuilder::new(prompt)
    }
}

pub struct TtagyRequestBuilder {
    req: TtagyRequest,
}

impl TtagyRequestBuilder {
    pub fn new(prompt: impl Into<String>) -> Self {
        let mut req = TtagyRequest::default();
        req.prompt = prompt.into();
        Self { req }
    }

    pub fn session_id(mut self, sid: impl Into<String>) -> Self {
        self.req.session_id = sid.into();
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.req.model = Some(model.into());
        self
    }

    pub fn effort(mut self, effort: impl Into<String>) -> Self {
        self.req.effort = Some(effort.into());
        self
    }

    pub fn temperature(mut self, temp: f32) -> Self {
        self.req.temperature = Some(temp);
        self
    }

    pub fn system_instruction(mut self, instruction: impl Into<String>) -> Self {
        self.req.system_instruction = Some(instruction.into());
        self
    }

    pub fn json_schema(mut self, schema: impl Into<String>) -> Self {
        self.req.json_schema = Some(schema.into());
        self
    }

    pub fn agent(mut self, agent: impl Into<String>) -> Self {
        self.req.agent = Some(agent.into());
        self
    }

    pub fn mode(mut self, mode: impl Into<String>) -> Self {
        self.req.mode = Some(mode.into());
        self
    }

    pub fn conversation_id(mut self, cid: impl Into<String>) -> Self {
        self.req.conversation_id = Some(cid.into());
        self
    }

    pub fn continue_last(mut self, c: bool) -> Self {
        self.req.continue_last = Some(c);
        self
    }

    pub fn project(mut self, proj: impl Into<String>) -> Self {
        self.req.project = Some(proj.into());
        self
    }

    pub fn add_dir(mut self, dir: impl Into<String>) -> Self {
        self.req.add_dirs.push(dir.into());
        self
    }

    pub fn sandbox(mut self, s: bool) -> Self {
        self.req.sandbox = Some(s);
        self
    }

    pub fn dangerously_skip_permissions(mut self, d: bool) -> Self {
        self.req.dangerously_skip_permissions = Some(d);
        self
    }

    pub fn disable_slash_commands(mut self, d: bool) -> Self {
        self.req.disable_slash_commands = Some(d);
        self
    }

    pub fn timeout_secs(mut self, t: u64) -> Self {
        self.req.timeout_secs = t;
        self
    }

    pub fn build(self) -> TtagyRequest {
        self.req
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
