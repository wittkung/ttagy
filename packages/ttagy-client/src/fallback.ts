import { spawn } from "node:child_process";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import * as readline from "node:readline";
import type { TtagyRequest, TtagyStreamEvent } from "./types.ts";

/**
 * 自动发现本地 agy 可执行文件路径
 */
export function findAgyBinary(): string | null {
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
  return null;
}

/**
 * TypeScript 进程内 Fallback 流式推导器
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
    `ts_fallback_${Date.now()}`
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

  const args = [
    "-p",
    request.prompt,
    "--model",
    modelName,
    "--output-format",
    "stream-json",
    "--disable-slash-commands",
    "--dangerously-skip-permissions",
    "--log-file",
    logFile,
  ];

  if (effort && effort !== "none") {
    args.push("--effort", effort);
  }

  const child = spawn(binary, args, {
    cwd: sandboxDir,
    stdio: ["pipe", "pipe", "pipe"],
  });

  const rl = readline.createInterface({
    input: child.stdout,
    crlfDelay: Infinity,
  });

  const startTime = Date.now();
  let fullContent = "";
  let thinkingContent = "";

  try {
    for await (const line of rl) {
      const trimmed = line.trim();
      if (!trimmed) continue;

      try {
        const val = JSON.parse(trimmed);
        const evType = val.type || val.event;

        if (evType === "step_update" && val.step_update) {
          const step = val.step_update;
          if (step.thought_delta || step.reasoning_delta) {
            const thought = step.thought_delta || step.reasoning_delta;
            thinkingContent += thought;
            yield {
              type: "agy:thinking_delta",
              sessionId,
              textDelta: thought,
              elapsedMs: Date.now() - startTime,
            };
          }
          if (step.text_delta) {
            fullContent += step.text_delta;
            yield {
              type: "agy:content_delta",
              sessionId,
              textDelta: step.text_delta,
              accumulatedChars: fullContent.length,
              elapsedMs: Date.now() - startTime,
            };
          }
        } else if (evType === "content" || evType === "message") {
          const chunk = val.content || val.text || "";
          if (chunk) {
            fullContent += chunk;
            yield {
              type: "agy:content_delta",
              sessionId,
              textDelta: chunk,
              accumulatedChars: fullContent.length,
              elapsedMs: Date.now() - startTime,
            };
          }
        } else if (evType === "result" && val.result) {
          if (!fullContent && val.result.content) {
            fullContent = val.result.content;
          }
        }
      } catch {
        // 忽略非 JSON 行
      }
    }

    yield {
      type: "agy:done",
      sessionId,
      fullContent,
      thinkingContent: thinkingContent || undefined,
      elapsedMs: Date.now() - startTime,
    };
  } finally {
    // 清理临时沙箱
    try {
      fs.rmSync(sandboxDir, { recursive: true, force: true });
    } catch {
      // 忽略清理失败
    }
  }
}
