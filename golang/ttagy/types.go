package ttagy

// Request 请求契约模型 (对齐 JSON Schema v1 & Superset)
type Request struct {
	SessionID                  string   `json:"session_id,omitempty"`
	Prompt                     string   `json:"prompt"`
	Model                      string   `json:"model,omitempty"`
	Effort                     string   `json:"effort,omitempty"`
	Temperature                float64  `json:"temperature,omitempty"`
	SystemInstruction          string   `json:"system_instruction,omitempty"`
	JSONSchema                 string   `json:"json_schema,omitempty"`
	Agent                      string   `json:"agent,omitempty"`
	Mode                       string   `json:"mode,omitempty"`
	ConversationID             string   `json:"conversation_id,omitempty"`
	ContinueLast               bool     `json:"continue_last,omitempty"`
	Project                    string   `json:"project,omitempty"`
	AddDirs                    []string `json:"add_dirs,omitempty"`
	Sandbox                    bool     `json:"sandbox,omitempty"`
	DangerouslySkipPermissions bool     `json:"dangerously_skip_permissions,omitempty"`
	DisableSlashCommands       bool     `json:"disable_slash_commands,omitempty"`
	TimeoutSecs                int      `json:"timeout_secs,omitempty"`
}

// Response 响应聚合模型
type Response struct {
	SessionID       string  `json:"session_id"`
	Status          string  `json:"status"` // "success" | "error" | "aborted"
	Content         string  `json:"content"`
	ThinkingContent *string `json:"thinking_content,omitempty"`
	Model           string  `json:"model"`
	ElapsedMs       float64 `json:"elapsed_ms"`
	PromptTokens    *int    `json:"prompt_tokens,omitempty"`
	OutputTokens    *int    `json:"output_tokens,omitempty"`
	ErrorMessage    *string `json:"error_message,omitempty"`
}

type StreamEventType string

const (
	EventInit          StreamEventType = "agy:init"
	EventThinkingDelta StreamEventType = "agy:thinking_delta"
	EventContentDelta  StreamEventType = "agy:content_delta"
	EventDone          StreamEventType = "agy:done"
	EventError         StreamEventType = "agy:error"
)

// StreamEvent 流式事件模型
type StreamEvent struct {
	Type             StreamEventType `json:"type"`
	SessionID        string          `json:"session_id"`
	Model            string          `json:"model,omitempty"`
	Effort           string          `json:"effort,omitempty"`
	BackendMode      string          `json:"backend_mode,omitempty"`
	TextDelta        string          `json:"text_delta,omitempty"`
	AccumulatedChars int             `json:"accumulated_chars,omitempty"`
	FullContent      string          `json:"full_content,omitempty"`
	ThinkingContent  *string         `json:"thinking_content,omitempty"`
	ElapsedMs        float64         `json:"elapsed_ms,omitempty"`
	PromptTokens     *int            `json:"prompt_tokens,omitempty"`
	OutputTokens     *int            `json:"output_tokens,omitempty"`
	ErrorCode        string          `json:"error_code,omitempty"`
	ErrorMessage     string          `json:"error_message,omitempty"`
	IsRetryable      bool            `json:"is_retryable,omitempty"`
}

// ClientConfig 客户端配置
type ClientConfig struct {
	BaseURL      string
	SocketPath   string
	AuthToken    string
	AutoFallback bool
}
