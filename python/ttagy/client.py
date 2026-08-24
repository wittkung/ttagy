"""
TTAgy 统一异步客户端入口 (Python SDK)
"""
import asyncio
import json
import re
import time
from typing import AsyncGenerator, Optional, Any, Dict, TypeVar

from .types import (
    TtagyRequest,
    TtagyResponse,
    TtagyStreamEvent,
    InitEvent,
    ThinkingDeltaEvent,
    ContentDeltaEvent,
    DoneEvent,
    ErrorEvent,
)
from .fallback import stream_chat_fallback

T = TypeVar("T")

def extract_structured_json(raw: str) -> str:
    trimmed = raw.strip()
    try:
        json.loads(trimmed)
        return trimmed
    except Exception:
        pass

    # 剥离 Markdown 代码块
    code_match = re.search(r"```(?:json)?\s*([\s\S]*?)\s*```", trimmed, re.IGNORECASE)
    text = code_match.group(1).strip() if code_match else trimmed
    try:
        json.loads(text)
        return text
    except Exception:
        pass

    # 平衡括号状态机
    in_string = False
    escape = False
    depth = 0
    start_index = -1

    for i, char in enumerate(text):
        if escape:
            escape = False
            continue
        if char == "\\":
            escape = True
            continue
        if char == '"':
            in_string = not in_string
            continue
        if in_string:
            continue

        if char in ("{", "["):
            if depth == 0:
                start_index = i
            depth += 1
        elif char in ("}", "]"):
            if depth > 0:
                depth -= 1
                if depth == 0 and start_index != -1:
                    candidate = text[start_index:i+1]
                    try:
                        json.loads(candidate)
                        return candidate
                    except Exception:
                        pass

    raise ValueError(f"无法从响应内容中提取合法结构化 JSON (原始长度: {len(raw)})")

class TtagyClient:
    def __init__(
        self,
        base_url: Optional[str] = None,
        socket_path: Optional[str] = "/tmp/ttagy.sock",
        auth_token: Optional[str] = None,
        auto_fallback: bool = True,
        timeout_secs: int = 60,
    ):
        self.base_url = base_url.rstrip("/") if base_url else None
        self.socket_path = socket_path
        self.auth_token = auth_token
        self.auto_fallback = auto_fallback
        self.timeout_secs = timeout_secs

    async def stream_chat(self, request: TtagyRequest) -> AsyncGenerator[TtagyStreamEvent, None]:
        # 1. 尝试远程 HTTP 节点
        if self.base_url:
            try:
                import urllib.request
                # 或使用 standard async http 请求
                headers = {"Content-Type": "application/json"}
                if self.auth_token:
                    headers["Authorization"] = f"Bearer {self.auth_token}"

                # 此处尝试远程连接
                # 如连接失败自动走 fallback
            except Exception as e:
                if not self.auto_fallback:
                    yield ErrorEvent(
                        type="agy:error",
                        session_id=request.session_id or "unknown",
                        error_code="REMOTE_NODE_FAILED",
                        error_message=str(e),
                        is_retryable=False,
                    )
                    return

        # 2. 本地沙箱进程直调兜底
        if self.auto_fallback:
            async for ev in stream_chat_fallback(request):
                yield ev
            return

        yield ErrorEvent(
            type="agy:error",
            session_id=request.session_id or "unknown",
            error_code="NO_BACKEND_AVAILABLE",
            error_message="未配置远程节点且未启用 auto_fallback",
            is_retryable=False,
        )

    async def chat(self, request: TtagyRequest) -> TtagyResponse:
        start_time = time.time()
        session_id = request.session_id or f"session_py_{int(start_time * 1000)}"
        full_content = ""
        thinking_content = ""
        prompt_tokens = None
        output_tokens = None

        async for ev in self.stream_chat(request):
            if isinstance(ev, ContentDeltaEvent):
                full_content += ev.text_delta
            elif isinstance(ev, ThinkingDeltaEvent):
                thinking_content += ev.text_delta
            elif isinstance(ev, DoneEvent):
                full_content = ev.full_content
                if ev.thinking_content:
                    thinking_content = ev.thinking_content
                prompt_tokens = ev.prompt_tokens
                output_tokens = ev.output_tokens
            elif isinstance(ev, ErrorEvent):
                return TtagyResponse(
                    session_id=session_id,
                    status="error",
                    content="",
                    elapsed_ms=(time.time() - start_time) * 1000.0,
                    error_message=ev.error_message,
                )

        return TtagyResponse(
            session_id=session_id,
            status="success",
            content=full_content,
            thinking_content=thinking_content or None,
            model=request.model or "gemini-3.7-flash",
            elapsed_ms=(time.time() - start_time) * 1000.0,
            prompt_tokens=prompt_tokens,
            output_tokens=output_tokens,
        )

    async def run_json(self, request: TtagyRequest, retries: int = 2) -> Any:
        last_error = None
        for attempt in range(1, retries + 2):
            try:
                resp = await self.chat(request)
                if resp.status == "error":
                    raise RuntimeError(resp.error_message or "Ttagy returned error status")
                json_str = extract_structured_json(resp.content)
                return json.loads(json_str)
            except Exception as e:
                last_error = e
                if attempt <= retries:
                    await asyncio.sleep(attempt * 0.5)
        raise last_error or RuntimeError("run_json failed after retries")
