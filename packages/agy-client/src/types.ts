/**
 * Local AI 强类型契约定义 (Draft-07 对齐)
 */

export interface AgyRequest {
  sessionId?: string;
  prompt: string;
  model?: string;
  effort?: "low" | "medium" | "high" | "none";
  temperature?: number;
  systemInstruction?: string;
  jsonSchema?: string;
  timeoutSecs?: number;
}

export interface AgyResponse {
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

export type AgyStreamEvent =
  | {
      type: "agy:init";
      sessionId: string;
      model: string;
      effort: string;
      backendMode: "daemon_ipc" | "fallback_direct_spawn";
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
