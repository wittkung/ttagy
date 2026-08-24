"""
Python In-Process Direct Spawn 兜底引擎 (带非阻塞 stderr 异步排空与生命周期安全清理)
"""
import asyncio
import os
import shutil
import tempfile
import time
from typing import AsyncGenerator

from .detector import find_agy_binary
from .parser import NdjsonParser, ParsedThinkingDelta, ParsedContentDelta, ParsedDone, ParsedError
from .types import (
    TtagyRequest,
    TtagyStreamEvent,
    InitEvent,
    ThinkingDeltaEvent,
    ContentDeltaEvent,
    DoneEvent,
    ErrorEvent,
)

async def stream_chat_fallback(request: TtagyRequest) -> AsyncGenerator[TtagyStreamEvent, None]:
    binary = find_agy_binary()
    session_id = request.session_id or f"session_py_{int(time.time() * 1000)}"

    if not binary:
        yield ErrorEvent(
            type="agy:error",
            session_id=session_id,
            error_code="BINARY_NOT_FOUND",
            error_message="未检测到 Antigravity CLI (agy) 二进制，请先在终端安装并认证",
            is_retryable=False,
        )
        return

    sandbox_dir = tempfile.mkdtemp(prefix="ttagy_py_sandbox_")
    log_file = os.path.join(sandbox_dir, "agy.log")
    model_name = request.model or "gemini-3.7-flash"
    effort = request.effort or "low"

    yield InitEvent(
        type="agy:init",
        session_id=session_id,
        model=model_name,
        effort=effort,
        backend_mode="fallback_direct_spawn",
    )

    args = [
        binary,
        "-p",
        request.prompt,
        "--model",
        model_name,
        "--output-format",
        "stream-json",
        "--disable-slash-commands",
        "--dangerously-skip-permissions",
        "--log-file",
        log_file,
    ]
    if effort and effort != "none":
        args.extend(["--effort", effort])
    if request.json_schema:
        args.extend(["--json-schema", request.json_schema])

    proc = await asyncio.create_subprocess_exec(
        *args,
        cwd=sandbox_dir,
        stdout=asyncio.subprocess.PIPE,
        stderr=asyncio.subprocess.PIPE,
    )

    start_time = time.time()
    stderr_logs = []

    async def drain_stderr():
        if proc.stderr:
            while True:
                chunk = await proc.stderr.read(4096)
                if not chunk:
                    break
                stderr_logs.append(chunk.decode(errors="ignore"))
                if len(stderr_logs) > 30:
                    stderr_logs.pop(0)

    stderr_task = asyncio.create_task(drain_stderr())
    accumulated_text = ""
    accumulated_thinking = ""

    try:
        while True:
            try:
                line_bytes = await asyncio.wait_for(proc.stdout.readline(), timeout=request.timeout_secs)
            except asyncio.TimeoutError:
                yield ErrorEvent(
                    type="agy:error",
                    session_id=session_id,
                    error_code="TIMEOUT",
                    error_message=f"Request timed out after {request.timeout_secs} seconds",
                    is_retryable=True,
                )
                break

            if not line_bytes:
                # EOF
                elapsed = (time.time() - start_time) * 1000.0
                err_text = "".join(stderr_logs).strip()
                if not accumulated_text and err_text:
                    yield ErrorEvent(
                        type="agy:error",
                        session_id=session_id,
                        error_code="EXECUTION_EMPTY_OUTPUT",
                        error_message=f"Worker produced empty stdout. Stderr: {err_text}",
                        is_retryable=False,
                    )
                else:
                    yield DoneEvent(
                        type="agy:done",
                        session_id=session_id,
                        full_content=accumulated_text,
                        thinking_content=accumulated_thinking or None,
                        elapsed_ms=elapsed,
                    )
                break

            line_str = line_bytes.decode(errors="ignore")
            elapsed = (time.time() - start_time) * 1000.0
            items = NdjsonParser.parse_line_items(line_str)

            for item in items:
                if isinstance(item, ParsedThinkingDelta):
                    accumulated_thinking += item.delta
                    yield ThinkingDeltaEvent(
                        type="agy:thinking_delta",
                        session_id=session_id,
                        text_delta=item.delta,
                        elapsed_ms=elapsed,
                    )
                elif isinstance(item, ParsedContentDelta):
                    accumulated_text += item.delta
                    yield ContentDeltaEvent(
                        type="agy:content_delta",
                        session_id=session_id,
                        text_delta=item.delta,
                        accumulated_chars=len(accumulated_text),
                        elapsed_ms=elapsed,
                    )
                elif isinstance(item, ParsedDone):
                    if item.content:
                        accumulated_text = item.content
                    final_thinking = item.thinking_content or (accumulated_thinking or None)
                    yield DoneEvent(
                        type="agy:done",
                        session_id=session_id,
                        full_content=accumulated_text,
                        thinking_content=final_thinking,
                        elapsed_ms=elapsed,
                        prompt_tokens=item.prompt_tokens,
                        output_tokens=item.output_tokens,
                    )
                    return
                elif isinstance(item, ParsedError):
                    yield ErrorEvent(
                        type="agy:error",
                        session_id=session_id,
                        error_code=item.code,
                        error_message=item.message,
                        is_retryable=True,
                    )
                    return

    finally:
        stderr_task.cancel()
        if proc.returncode is None:
            try:
                proc.kill()
                await proc.wait()
            except ProcessLookupError:
                pass
        shutil.rmtree(sandbox_dir, ignore_errors=True)
