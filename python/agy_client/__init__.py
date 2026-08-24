"""
AGY Bridge Python Client SDK
"""
import asyncio
import os
import shutil
import tempfile
from typing import AsyncGenerator, Dict, Any, Optional

def find_agy_binary() -> Optional[str]:
    which_path = shutil.which("agy")
    if which_path:
        return which_path
    candidates = [
        os.path.expanduser("~/.local/bin/agy"),
        os.path.expanduser("~/bin/agy"),
        "/usr/local/bin/agy",
        "/opt/homebrew/bin/agy",
    ]
    for p in candidates:
        if os.path.isfile(p):
            return p
    return None

class AgyClient:
    def __init__(self, socket_path: str = "/tmp/agy_daemon.sock", auto_fallback: bool = True):
        self.socket_path = socket_path
        self.auto_fallback = auto_fallback

    async def stream_chat(self, prompt: str, model: str = "gemini-3.7-flash", effort: str = "high") -> AsyncGenerator[Dict[str, Any], None]:
        binary = find_agy_binary()
        if not binary:
            yield {"type": "agy:error", "error_message": "未找到 agy 二进制"}
            return

        sandbox_dir = tempfile.mkdtemp(prefix="agy_py_sandbox_")
        log_file = os.path.join(sandbox_dir, "agy.log")
        
        args = [
            binary, "-p", prompt,
            "--model", model,
            "--output-format", "stream-json",
            "--disable-slash-commands",
            "--dangerously-skip-permissions",
            "--log-file", log_file,
        ]
        if effort and effort != "none":
            args.extend(["--effort", effort])

        proc = await asyncio.create_subprocess_exec(
            *args,
            cwd=sandbox_dir,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )

        try:
            while True:
                line = await proc.stdout.readline()
                if not line:
                    break
                # 解析输出
                yield {"type": "agy:content_delta", "raw": line.decode().strip()}
        finally:
            shutil.rmtree(sandbox_dir, ignore_errors=True)
