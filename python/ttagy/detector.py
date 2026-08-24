"""
TTAgy CLI 二进制探查器 (Python SDK)
"""
import os
import shutil
from typing import Optional

def find_agy_binary() -> Optional[str]:
    env_path = os.environ.get("AGY_PATH")
    if env_path and os.path.isfile(env_path):
        return env_path
    which_path = shutil.which("agy")
    if which_path and os.path.isfile(which_path):
        return which_path
    home = os.path.expanduser("~")
    candidates = [
        os.path.join(home, ".local/bin/agy"),
        os.path.join(home, "bin/agy"),
        "/usr/local/bin/agy",
        "/opt/homebrew/bin/agy",
    ]
    for p in candidates:
        if os.path.isfile(p):
            return p
    return None
