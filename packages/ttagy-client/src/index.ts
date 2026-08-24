import { streamChatFallback } from "./fallback.ts";
import type { TtagyRequest, TtagyResponse, TtagyStreamEvent } from "./types.ts";

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

export class TtagyClient {
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
  async *streamChat(request: TtagyRequest): AsyncGenerator<TtagyStreamEvent, void, unknown> {
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
                  const ev = JSON.parse(dataStr) as TtagyStreamEvent;
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
  async chat(request: TtagyRequest): Promise<TtagyResponse> {
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

  /**
   * 强类型 JSON 确定性推导执行器 (带自动代码块剥离与重试)
   */
  async runJson<T = any>(request: TtagyRequest): Promise<T> {
    const retries = request.retries ?? 2;
    let lastError: Error | null = null;

    for (let attempt = 1; attempt <= retries + 1; attempt++) {
      try {
        const response = await this.chat({
          ...request,
          sessionId: `json_${Date.now()}_att${attempt}`,
        });

        if (response.status === "error") {
          throw new Error(response.errorMessage || "Ttagy returned error status");
        }

        let raw = response.content.trim();

        // 1. 尝试直接解析
        try {
          return JSON.parse(raw) as T;
        } catch {}

        // 2. 剥离 Markdown 代码块 ```json ... ```
        const markdownMatch = raw.match(/```(?:json)?\s*([\s\S]*?)\s*```/);
        if (markdownMatch && markdownMatch[1]) {
          try {
            return JSON.parse(markdownMatch[1].trim()) as T;
          } catch {}
        }

        // 3. 提取首尾大括号
        const firstBrace = raw.indexOf("{");
        const lastBrace = raw.lastIndexOf("}");
        if (firstBrace !== -1 && lastBrace > firstBrace) {
          const candidate = raw.slice(firstBrace, lastBrace + 1);
          return JSON.parse(candidate) as T;
        }

        throw new Error(`无法从 ttagy 输出中提取合法 JSON (长度: ${raw.length})`);
      } catch (err: any) {
        lastError = err;
        if (attempt <= retries) {
          await new Promise((r) => setTimeout(r, 1000 * attempt));
        }
      }
    }

    throw lastError || new Error("ttagy runJson failed after retries");
  }
}
