"""
TTAgy NDJSON 流式行事件解析器 (Python 实现与 Rust 100% 对齐)
"""
import json
from typing import List, Optional, Union, Dict, Any

class ParsedThinkingDelta:
    def __init__(self, delta: str):
        self.delta = delta
    def __repr__(self):
        return f"ParsedThinkingDelta({self.delta!r})"
    def __eq__(self, other):
        return isinstance(other, ParsedThinkingDelta) and self.delta == other.delta

class ParsedContentDelta:
    def __init__(self, delta: str):
        self.delta = delta
    def __repr__(self):
        return f"ParsedContentDelta({self.delta!r})"
    def __eq__(self, other):
        return isinstance(other, ParsedContentDelta) and self.delta == other.delta

class ParsedDone:
    def __init__(self, content: str, thinking_content: Optional[str] = None, prompt_tokens: Optional[int] = None, output_tokens: Optional[int] = None):
        self.content = content
        self.thinking_content = thinking_content
        self.prompt_tokens = prompt_tokens
        self.output_tokens = output_tokens
    def __repr__(self):
        return f"ParsedDone(content={self.content!r}, prompt_tokens={self.prompt_tokens}, output_tokens={self.output_tokens})"
    def __eq__(self, other):
        return (isinstance(other, ParsedDone) and self.content == other.content and 
                self.thinking_content == other.thinking_content and 
                self.prompt_tokens == other.prompt_tokens and 
                self.output_tokens == other.output_tokens)

class ParsedError:
    def __init__(self, code: str, message: str):
        self.code = code
        self.message = message
    def __repr__(self):
        return f"ParsedError(code={self.code!r}, message={self.message!r})"
    def __eq__(self, other):
        return isinstance(other, ParsedError) and self.code == other.code and self.message == other.message

ParsedItem = Union[ParsedThinkingDelta, ParsedContentDelta, ParsedDone, ParsedError]

class NdjsonParser:
    @staticmethod
    def parse_line_items(line: str) -> List[ParsedItem]:
        trimmed = line.strip()
        if not trimmed:
            return []
        try:
            val = json.loads(trimmed)
        except Exception:
            return []

        items: List[ParsedItem] = []
        ev_name = val.get("event") or val.get("type") or ""

        if ev_name == "step_update" and isinstance(val.get("step_update"), dict):
            step = val["step_update"]
            thought = step.get("thought_delta") or step.get("reasoning_delta") or step.get("thinking_delta")
            if thought and isinstance(thought, str):
                items.append(ParsedThinkingDelta(thought))
            text = step.get("text_delta") or step.get("content_delta")
            if text and isinstance(text, str):
                items.append(ParsedContentDelta(text))

        elif ev_name == "content":
            text = val.get("content") or val.get("text")
            if text and isinstance(text, str):
                items.append(ParsedContentDelta(text))

        elif ev_name == "message":
            msg = val.get("message")
            text = val.get("content") or val.get("text")
            if not text and isinstance(msg, dict):
                text = msg.get("content") or msg.get("text")
            if text and isinstance(text, str):
                items.append(ParsedContentDelta(text))

        elif ev_name in ("result", "done"):
            res_obj = val.get("result") if isinstance(val.get("result"), dict) else val
            status = res_obj.get("status", "")
            if status == "ERROR" or "error" in res_obj:
                err = res_obj.get("error")
                err_msg = err.get("message", "AGY worker error") if isinstance(err, dict) else str(err or "AGY worker error")
                items.append(ParsedError("AGY_ERROR", err_msg))
                return items

            content = (res_obj.get("response") or res_obj.get("content") or 
                       res_obj.get("text") or res_obj.get("structured_output") or "")
            if not isinstance(content, str):
                content = json.dumps(content) if content is not None else ""

            thinking = res_obj.get("thinking_content") or res_obj.get("thought")
            if thinking is not None and not isinstance(thinking, str):
                thinking = str(thinking)

            usage = res_obj.get("usage")
            prompt_tokens = None
            output_tokens = None
            if isinstance(usage, dict):
                prompt_tokens = usage.get("prompt_tokens")
                output_tokens = usage.get("completion_tokens") or usage.get("output_tokens")

            items.append(ParsedDone(
                content=content,
                thinking_content=thinking,
                prompt_tokens=prompt_tokens,
                output_tokens=output_tokens,
            ))

        elif ev_name == "error":
            err = val.get("error") or val.get("message") or "Unknown CLI error"
            err_msg = err.get("message", "Unknown error") if isinstance(err, dict) else str(err)
            items.append(ParsedError("CLI_ERROR", err_msg))

        return items
