import { spawn, execSync } from "node:child_process";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import type { TtagyRequest, TtagyStreamEvent } from "./types.ts";

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
 * 从文本中精确提取最外层有效 JSON 字符串 (语法感知平衡括号状态机)
 */
export function extractStructuredJson(raw: string): string {
  const trimmed = raw.trim();

  // 1. 尝试直接解析
  try {
    JSON.parse(trimmed);
    return trimmed;
  } catch {}

  // 2. 剥离 Markdown 代码块 ```json ... ```
  const codeBlockMatch = trimmed.match(/```(?:json)?\s*([\s\S]*?)\s*```/i);
  if (codeBlockMatch && codeBlockMatch[1]) {
    const candidate = codeBlockMatch[1].trim();
    try {
      JSON.parse(candidate);
      return candidate;
    } catch {}
  }

  // 3. 平衡括号状态机查找最外层平衡的大括号 {} 或中括号 []
  const text = codeBlockMatch ? codeBlockMatch[1] : trimmed;
  let inString = false;
  let escape = false;
  let depth = 0;
  let startIndex = -1;

  for (let i = 0; i < text.length; i++) {
    const char = text[i];

    if (escape) {
      escape = false;
      continue;
    }
    if (char === "\\") {
      escape = true;
      continue;
    }
    if (char === '"') {
      inString = !inString;
      continue;
    }
    if (inString) continue;

    if (char === "{" || char === "[") {
      if (depth === 0) {
        startIndex = i;
      }
      depth++;
    } else if (char === "}" || char === "]") {
      if (depth > 0) {
        depth--;
        if (depth === 0 && startIndex !== -1) {
          const candidate = text.slice(startIndex, i + 1);
          try {
            JSON.parse(candidate);
            return candidate;
          } catch {
            // 继续扫描
          }
        }
      }
    }
  }

  throw new Error(`无法从响应内容中提取合法结构化 JSON (原始长度: ${raw.length})`);
}

/**
 * 针对流式传输中的不完整 JSON 片段进行最佳努力闭合修复 (Streaming JSON Auto-repairer)
 */
export function repairIncompleteJson(partial: string): string {
  let s = partial.trim();
  if (!s) return "{}";

  s = s.replace(/^```(?:json)?\s*/i, "").replace(/\s*```$/, "");

  let inString = false;
  let escape = false;
  const stack: ("{" | "[")[] = [];

  for (let i = 0; i < s.length; i++) {
    const char = s[i];
    if (escape) {
      escape = false;
      continue;
    }
    if (char === "\\") {
      escape = true;
      continue;
    }
    if (char === '"') {
      inString = !inString;
      continue;
    }
    if (inString) continue;

    if (char === "{" || char === "[") {
      stack.push(char);
    } else if (char === "}" || char === "]") {
      stack.pop();
    }
  }

  if (inString) {
    s += '"';
  }

  s = s.replace(/,\s*$/, "").replace(/:\s*$/, ": null");

  while (stack.length > 0) {
    const top = stack.pop();
    if (top === "{") {
      s += "}";
    } else if (top === "[") {
      s += "]";
    }
  }

  return s;
}

/**
 * TypeScript 进程内 Fallback 流式推导器 (通用、解耦、支持 AbortSignal 与双流安全排空)
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

  const sandboxDir = path.join(
    os.tmpdir(),
    "local_ai_sandboxes",
    `ts_fallback_${Date.now()}_${Math.random().toString(36).slice(2, 7)}`
  );
  fs.mkdirSync(sandboxDir, { recursive: true });
  const logFile = path.join(sandboxDir, "agy.log");

  const sessionId = request.sessionId || `session_${Date.now()}`;
  const modelName = request.model || "gemini-3.7-flash";
  const effort = request.effort || "low";

  yield {
    type: "agy:init",
    sessionId,
    model: modelName,
    effort,
    backendMode: "fallback_direct_spawn",
  };

  const args: string[] = [
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

  const schema = request.jsonSchema || (request.schemaPath && fs.existsSync(request.schemaPath) ? fs.readFileSync(request.schemaPath, "utf-8") : undefined);
  if (schema) {
    args.push("--json-schema", schema);
  }

  const child = spawn(binary, args, {
    cwd: sandboxDir,
    stdio: ["pipe", "pipe", "pipe"],
  });

  const abortHandler = () => {
    try {
      if (!child.killed) child.kill("SIGKILL");
    } catch {}
  };

  if (request.signal) {
    if (request.signal.aborted) {
      abortHandler();
      return;
    }
    request.signal.addEventListener("abort", abortHandler, { once: true });
  }

  const startTime = Date.now();
  let stderrBuffer = "";
  let fullContent = "";
  let thinkingContent = "";

  const timeoutMs = (request.timeoutSecs || 60) * 1000;

  try {
    let stdoutBuffer = "";

    // 监听 stderr 避免管道死锁
    child.stderr.on("data", (chunk: Buffer) => {
      stderrBuffer += chunk.toString();
      if (stderrBuffer.length > 64 * 1024) {
        stderrBuffer = stderrBuffer.slice(-64 * 1024);
      }
    });

    const eventQueue: TtagyStreamEvent[] = [];
    let streamDone = false;
    let streamError: Error | null = null;
    let wakeResolve: (() => void) | null = null;

    const notify = () => {
      if (wakeResolve) {
        const resolve = wakeResolve;
        wakeResolve = null;
        resolve();
      }
    };

    const timer = setTimeout(() => {
      if (!streamDone) {
        streamError = new Error(`[ttagy] Fallback timeout after ${timeoutMs}ms. Stderr: ${stderrBuffer.slice(-500)}`);
        try {
          child.kill("SIGKILL");
        } catch {}
        notify();
      }
    }, timeoutMs);

    let hasParsedError = false;
    child.stdout.on("data", (chunk: Buffer) => {
      stdoutBuffer += chunk.toString();
      const lines = stdoutBuffer.split("\n");
      stdoutBuffer = lines.pop() || "";

      for (const line of lines) {
        const trimmed = line.trim();
        if (!trimmed) continue;

        try {
          const val = JSON.parse(trimmed);
          const evType = val.event || val.type || "";

          if (evType === "step_update" && val.step_update) {
            const step = val.step_update;
            const thought = step.thought_delta || step.reasoning_delta || step.thinking_delta;
            if (thought) {
              thinkingContent += thought;
              eventQueue.push({
                type: "agy:thinking_delta",
                sessionId,
                textDelta: thought,
                elapsedMs: Date.now() - startTime,
              });
            }
            const text = step.text_delta || step.content_delta;
            if (text) {
              fullContent += text;
              eventQueue.push({
                type: "agy:content_delta",
                sessionId,
                textDelta: text,
                accumulatedChars: fullContent.length,
                elapsedMs: Date.now() - startTime,
              });
            }
          } else if (evType === "content" || evType === "message") {
            const text = val.content || val.text || (val.message && (val.message.content || val.message.text));
            if (text) {
              fullContent += text;
              eventQueue.push({
                type: "agy:content_delta",
                sessionId,
                textDelta: text,
                accumulatedChars: fullContent.length,
                elapsedMs: Date.now() - startTime,
              });
            }
          } else if (evType === "result" || evType === "done") {
            const resObj = val.result || val;
            const content = resObj.response || resObj.content || resObj.text || resObj.structured_output;
            if (typeof content === "string" && content) {
              fullContent = content;
            }
          } else if (evType === "error") {
            hasParsedError = true;
            const err = val.error || val.message || "Unknown error";
            const errMsg = typeof err === "object" ? err.message || JSON.stringify(err) : String(err);
            eventQueue.push({
              type: "agy:error",
              sessionId,
              errorCode: "CLI_ERROR",
              errorMessage: errMsg,
              isRetryable: false,
            });
          }
        } catch {
          // 忽略非 JSON 行
        }
      }
      notify();
    });

    child.on("close", (code) => {
      clearTimeout(timer);
      if (code !== 0 && !hasParsedError && !fullContent && stderrBuffer) {
        eventQueue.push({
          type: "agy:error",
          sessionId,
          errorCode: "CLI_EXIT_NONZERO",
          errorMessage: stderrBuffer.slice(-500),
          isRetryable: false,
        });
      }
      streamDone = true;
      notify();
    });

    child.on("error", (err) => {
      clearTimeout(timer);
      streamError = err;
      notify();
    });

    let emittedError = false;
    while (!streamDone || eventQueue.length > 0) {
      if (eventQueue.length > 0) {
        const ev = eventQueue.shift()!;
        if (ev.type === "agy:error") {
          emittedError = true;
        }
        yield ev;
      } else if (streamError) {
        throw streamError;
      } else if (!streamDone) {
        await new Promise<void>((r) => {
          wakeResolve = r;
        });
      }
    }

    if (streamError) {
      throw streamError;
    }

    if (!emittedError) {
      yield {
        type: "agy:done",
        sessionId,
        fullContent,
        thinkingContent: thinkingContent || undefined,
        elapsedMs: Date.now() - startTime,
      };
    }
  } catch (err: any) {
    yield {
      type: "agy:error",
      sessionId,
      errorCode: "SPAWN_EXEC_FAILED",
      errorMessage: err.message || String(err),
      isRetryable: true,
    };
  } finally {
    if (request.signal) {
      request.signal.removeEventListener("abort", abortHandler);
    }
    try {
      if (!child.killed) child.kill("SIGKILL");
    } catch {}
    try {
      fs.rmSync(sandboxDir, { recursive: true, force: true });
    } catch {}
  }
}
