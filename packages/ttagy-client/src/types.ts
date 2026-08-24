/**
 * Antigravity CLI 官方支持的原生模型全量权威索引 (2026 最新)
 */
export const AGY_SUPPORTED_MODELS = [
  "gemini-3.7-flash-high",
  "gemini-3.7-flash-medium",
  "gemini-3.7-flash-low",
  "gemini-3.6-flash-high",
  "gemini-3.6-flash-medium",
  "gemini-3.6-flash-low",
  "gemini-3.5-flash-high",
  "gemini-3.5-flash-medium",
  "gemini-3.5-flash-low",
  "gemini-3.1-pro-high",
  "gemini-3.1-pro-low",
  "claude-sonnet-4-6",
  "claude-opus-4-6-thinking",
  "gpt-oss-120b-medium",
] as const;

export type AgySupportedModel = typeof AGY_SUPPORTED_MODELS[number];

export interface TtagyRequest {
  sessionId?: string;
  prompt: string;
  model?: string;
  effort?: "low" | "medium" | "high" | "none";
  temperature?: number;
  systemInstruction?: string;
  jsonSchema?: string;
  schemaPath?: string;
  timeoutSecs?: number;
  retries?: number;
  /** 可选的 AbortSignal 用于即时取消推理与销毁子进程 */
  signal?: AbortSignal;
}

export interface TtagyResponse {
  sessionId: string;
  status: "success" | "error" | "aborted";
  content: string;
  thinkingContent?: string;
  model?: string;
  elapsedMs: number;
  promptTokens?: number;
  outputTokens?: number;
  errorMessage?: string;
}

export type TtagyStreamEvent =
  | {
      type: "agy:init";
      sessionId: string;
      model: string;
      effort: string;
      backendMode: "daemon_uds" | "daemon_tcp" | "daemon_ipc" | "fallback_direct_spawn";
    }
  | {
      type: "agy:thinking_delta";
      sessionId: string;
      textDelta: string;
      elapsedMs: number;
    }
  | {
      type: "agy:content_delta";
      sessionId: string;
      textDelta: string;
      accumulatedChars: number;
      elapsedMs: number;
    }
  | {
      type: "agy:done";
      sessionId: string;
      fullContent: string;
      thinkingContent?: string;
      elapsedMs: number;
      promptTokens?: number;
      outputTokens?: number;
    }
  | {
      type: "agy:error";
      sessionId: string;
      errorCode: string;
      errorMessage: string;
      isRetryable: boolean;
    };
