import { streamChatFallback } from "./fallback.ts";
import type { AgyRequest, AgyResponse, AgyStreamEvent } from "./types.ts";

export type * from "./types.ts";
export * from "./fallback.ts";

export interface ClientOptions {
  /** 远程私有节点地址，如 "http://100.64.0.1:8970" 或 "https://agent.yourdomain.com" */
  baseUrl?: string;
  /** 访问私有节点所需的安全 Bearer Token */
  authToken?: string;
  /** 当未配置远程节点或连接失败时，是否自动回退至本地沙箱直调 */
  autoFallback?: boolean;
}

export class AgyClient {
  private baseUrl?: string;
  private authToken?: string;
  private autoFallback: boolean;

  constructor(options?: ClientOptions) {
    this.baseUrl = options?.baseUrl?.replace(/\/+$/, "");
    this.authToken = options?.authToken;
    this.autoFallback = options?.autoFallback ?? true;
  }

  /**
   * 发起流式对话
   */
  async *streamChat(request: AgyRequest): AsyncGenerator<AgyStreamEvent, void, unknown> {
    // 1. 若配置了远程节点，优先通过 HTTP/SSE 请求远程 Agent 节点
    if (this.baseUrl) {
      try {
        const headers: Record<string, string> = {
          "Content-Type": "application/json",
        };
        if (this.authToken) {
          headers["Authorization"] = `Bearer ${this.authToken}`;
        }

        const res = await fetch(`${this.baseUrl}/api/v1/stream`, {
          method: "POST",
          headers,
          body: JSON.stringify(request),
        });

        if (!res.ok) {
          throw new Error(`Remote node returned status ${res.status}: ${res.statusText}`);
        }

        if (!res.body) {
          throw new Error("No response body received from remote agent node");
        }

        const reader = res.body.getReader();
        const decoder = new TextDecoder();
        let buffer = "";

        while (true) {
          const { done, value } = await reader.read();
          if (done) break;

          buffer += decoder.decode(value, { stream: true });
          const lines = buffer.split("\n");
          buffer = lines.pop() || "";

          for (const line of lines) {
            const trimmed = line.trim();
            if (trimmed.startsWith("data:")) {
              const dataStr = trimmed.slice(5).trim();
              if (dataStr) {
                try {
                  const ev = JSON.parse(dataStr) as AgyStreamEvent;
                  yield ev;
                } catch {
                  // 忽略非 JSON 数据行
                }
              }
            }
          }
        }
        return;
      } catch (err) {
        if (!this.autoFallback) {
          yield {
            type: "agy:error",
            sessionId: request.sessionId || "unknown",
            errorCode: "REMOTE_NODE_FAILED",
            errorMessage: err instanceof Error ? err.message : String(err),
            isRetryable: false,
          };
          return;
        }
        // 自动降级至本地 Fallback
      }
    }

    // 2. 本地沙箱 Worker 兜底
    if (this.autoFallback) {
      yield* streamChatFallback(request);
      return;
    }

    yield {
      type: "agy:error",
      sessionId: request.sessionId || "unknown",
      errorCode: "NO_BACKEND_AVAILABLE",
      errorMessage: "未配置远程节点且未启用 autoFallback",
      isRetryable: false,
    };
  }

  /**
   * 一次性获取完整推导响应
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
