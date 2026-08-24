import { spawn, execSync } from "node:child_process";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import type { TtagyRequest, TtagyStreamEvent } from "./types";

/**
 * 自动发现本地 agy 可执行文件路径
 */
export function findAgyBinary(): string | null {
  if (process.env.AGY_PATH && fs.existsSync(process.env.AGY_PATH)) {
    return process.env.AGY_PATH;
  }
  const home = os.homedir();
  const candidates = [
    path.join(home, ".local/bin/agy"),
    path.join(home, "bin/agy"),
    "/usr/local/bin/agy",
    "/opt/homebrew/bin/agy",
  ];
  for (const p of candidates) {
    if (fs.existsSync(p)) return p;
  }
  try {
    const which = execSync("which agy", { encoding: "utf-8" }).trim();
    if (which && fs.existsSync(which)) return which;
  } catch {
    // which failed
  }
  return null;
}

/**
 * TypeScript 进程内 Fallback 流式与即时 JSON 推导器 (TTSubs 早期流截断引擎)
 */
export async function* streamChatFallback(
  request: TtagyRequest
): AsyncGenerator<TtagyStreamEvent, void, unknown> {
  const binary = findAgyBinary();
  if (!binary) {
    yield {
      type: "agy:error",
      sessionId: request.sessionId || "unknown",
      errorCode: "BINARY_NOT_FOUND",
      errorMessage: "未找到 Antigravity CLI (agy) 二进制，请先安装并认证",
      isRetryable: false,
    };
    return;
  }

  // 创建临时隔离沙箱
  const sandboxDir = path.join(
    os.tmpdir(),
    "local_ai_sandboxes",
    `ts_fallback_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`
  );
  fs.mkdirSync(sandboxDir, { recursive: true });
  const logFile = path.join(sandboxDir, "agy.log");

  const sessionId = request.sessionId || `session_${Date.now()}`;
  const modelName = request.model || "gemini-3.7-flash";
  const effort = request.effort || "high";

  yield {
    type: "agy:init",
    sessionId,
    model: modelName,
    effort,
    backendMode: "fallback_direct_spawn",
  };

  const args: string[] = [
    "--print",
    request.prompt,
    "--model",
    modelName,
    "--disable-slash-commands",
    "--dangerously-skip-permissions",
    "--log-file",
    logFile,
  ];

  if (effort && effort !== "none") {
    args.push("--effort", effort);
  }

  if (request.schemaPath && fs.existsSync(request.schemaPath)) {
    args.push("--json-schema", request.schemaPath);
  }

  const child = spawn(binary, args, {
    cwd: sandboxDir,
    stdio: ["pipe", "pipe", "pipe"],
  });

  const startTime = Date.now();
  let stdoutBuffer = "";
  let fullContent = "";
  let thinkingContent = "";
  let resolved = false;

  const timeoutMs = (request.timeoutSecs || 60) * 1000;

  try {
    const streamPromise = new Promise<void>((resolve, reject) => {
      const timer = setTimeout(() => {
        if (!resolved) {
          try {
            child.kill("SIGTERM");
          } catch {}
          reject(new Error(`[ttagy] Fallback timeout after ${timeoutMs}ms`));
        }
      }, timeoutMs);

      child.stdout.on("data", (chunk: Buffer) => {
        const text = chunk.toString();
        stdoutBuffer += text;

        // 1. TTSubs Early stream JSON closure resolution
        const firstBrace = stdoutBuffer.indexOf("{");
        const lastBrace = stdoutBuffer.lastIndexOf("}");
        if (firstBrace !== -1 && lastBrace > firstBrace) {
          const candidate = stdoutBuffer.slice(firstBrace, lastBrace + 1);
          try {
            const parsed = JSON.parse(candidate);
            if (parsed && (parsed.status === "SUCCESS" || parsed.structured_output || parsed.response || parsed.paragraphs || parsed.items || parsed.glossary || parsed.concepts)) {
              resolved = true;
              clearTimeout(timer);
              if (parsed.structured_output) {
                fullContent = typeof parsed.structured_output === "string" ? parsed.structured_output : JSON.stringify(parsed.structured_output);
              } else if (parsed.response) {
                fullContent = parsed.response;
              } else {
                fullContent = candidate;
              }
              try {
                child.kill("SIGTERM");
              } catch {}
              resolve();
              return;
            }
          } catch {
            // Not a complete JSON envelope yet
          }
        }
      });

      child.stderr.on("data", (chunk: Buffer) => {
        const errText = chunk.toString();
        if (errText.includes("Thinking:") || errText.includes("thought_delta")) {
          thinkingContent += errText;
        }
      });

      child.on("close", () => {
        clearTimeout(timer);
        if (!resolved) {
          fullContent = stdoutBuffer.trim();
          resolve();
        }
      });

      child.on("error", (err) => {
        clearTimeout(timer);
        reject(err);
      });
    });

    await streamPromise;

    yield {
      type: "agy:done",
      sessionId,
      fullContent,
      thinkingContent: thinkingContent || undefined,
      elapsedMs: Date.now() - startTime,
    };
  } catch (err: any) {
    yield {
      type: "agy:error",
      sessionId,
      errorCode: "SPAWN_EXEC_FAILED",
      errorMessage: err.message || String(err),
      isRetryable: true,
    };
  } finally {
    try {
      if (!child.killed) child.kill("SIGKILL");
    } catch {}
    try {
      fs.rmSync(sandboxDir, { recursive: true, force: true });
    } catch {}
  }
}
