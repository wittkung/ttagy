"""
TTAgy Python SDK 强类型契约模型
"""
from dataclasses import dataclass, field
from typing import Optional, Any, Dict, List, Literal

AGY_SUPPORTED_MODELS: List[str] = [
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
]

@dataclass
class TtagyRequest:
    prompt: str
    session_id: Optional[str] = None
    model: Optional[str] = "gemini-3.7-flash"
    effort: Optional[str] = "low"
    temperature: Optional[float] = None
    system_instruction: Optional[str] = None
    json_schema: Optional[str] = None
    schema_path: Optional[str] = None
    timeout_secs: int = 60
    retries: int = 2

    def to_dict(self) -> Dict[str, Any]:
        data = {
            "prompt": self.prompt,
            "session_id": self.session_id,
            "model": self.model,
            "effort": self.effort,
            "timeout_secs": self.timeout_secs,
        }
        if self.temperature is not None:
            data["temperature"] = self.temperature
        if self.system_instruction:
            data["system_instruction"] = self.system_instruction
        if self.json_schema:
            data["json_schema"] = self.json_schema
        return {k: v for k, v in data.items() if v is not None}

@dataclass
class TtagyResponse:
    session_id: str
    status: Literal["success", "error", "aborted"]
    content: str
    thinking_content: Optional[str] = None
    model: Optional[str] = None
    elapsed_ms: float = 0.0
    prompt_tokens: Optional[int] = None
    output_tokens: Optional[int] = None
    error_message: Optional[str] = None

@dataclass
class InitEvent:
    type: Literal["agy:init"]
    session_id: str
    model: str
    effort: str
    backend_mode: str

@dataclass
class ThinkingDeltaEvent:
    type: Literal["agy:thinking_delta"]
    session_id: str
    text_delta: str
    elapsed_ms: float

@dataclass
class ContentDeltaEvent:
    type: Literal["agy:content_delta"]
    session_id: str
    text_delta: str
    accumulated_chars: int
    elapsed_ms: float

@dataclass
class DoneEvent:
    type: Literal["agy:done"]
    session_id: str
    full_content: str
    thinking_content: Optional[str]
    elapsed_ms: float
    prompt_tokens: Optional[int] = None
    output_tokens: Optional[int] = None

@dataclass
class ErrorEvent:
    type: Literal["agy:error"]
    session_id: str
    error_code: str
    error_message: str
    is_retryable: bool

TtagyStreamEvent = InitEvent | ThinkingDeltaEvent | ContentDeltaEvent | DoneEvent | ErrorEvent
