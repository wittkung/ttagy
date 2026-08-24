import * as fs from "node:fs";
import { streamChatFallback } from "./fallback.ts";
import type { AgyRequest, AgyResponse, AgyStreamEvent } from "./types.ts";

export type * from "./types.ts";
export * from "./fallback.ts";

export interface ClientOptions {
  socketPath?: string;
  httpUrl?: string;
  autoFallback?: boolean;
}

export class AgyClient {
  private socketPath: string;
  private autoFallback: boolean;

  constructor(options?: ClientOptions) {
    this.socketPath = options?.socketPath || "/tmp/local_ai_daemon.sock";
    this.autoFallback = options?.autoFallback ?? true;
  }

  /**
   * 发起流式对话
   */
  async *streamChat(request: AgyRequest): AsyncGenerator<AgyStreamEvent, void, unknown> {
    const isDaemonLive = fs.existsSync(this.socketPath);

    if (isDaemonLive) {
      // TODO: 连接 Daemon IPC 流
    }

    if (this.autoFallback) {
      yield* streamChatFallback(request);
      return;
    }

    yield {
      type: "agy:error",
      sessionId: request.sessionId || "unknown",
      errorCode: "DAEMON_UNAVAILABLE",
      errorMessage: "Local AI Daemon 未运行且未启用 autoFallback",
      isRetryable: false,
    };
  }

  /**
   * 一次性获取完整响应
   */
  async chat(request: AgyRequest): Promise<AgyResponse> {
    const startTime = Date.now();
    const sessionId = request.sessionId || `session_${startTime}`;
    let fullContent = "";
    let thinkingContent = "";

    for await (const event of this.streamChat(request)) {
      if (event.type === "agy:content_delta") {
        fullContent += event.textDelta;
      } else if (event.type === "agy:thinking_delta") {
        thinkingContent += event.textDelta;
      } else if (event.type === "agy:done") {
        fullContent = event.fullContent;
        if (event.thinkingContent) {
          thinkingContent = event.thinkingContent;
        }
      } else if (event.type === "agy:error") {
        return {
          sessionId,
          status: "error",
          content: "",
          elapsedMs: Date.now() - startTime,
          errorMessage: event.errorMessage,
        };
      }
    }

    return {
      sessionId,
      status: "success",
      content: fullContent,
      thinkingContent: thinkingContent || undefined,
      model: request.model || "gemini-3.7-flash",
      elapsedMs: Date.now() - startTime,
    };
  }
}
