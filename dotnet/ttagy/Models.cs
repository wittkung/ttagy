using System.Text.Json.Serialization;

namespace Ttagy.Sdk;

public class TtagyRequest
{
    [JsonPropertyName("session_id")]
    public string? SessionId { get; set; }

    [JsonPropertyName("prompt")]
    public required string Prompt { get; set; }

    [JsonPropertyName("model")]
    public string Model { get; set; } = "gemini-3.7-flash";

    [JsonPropertyName("effort")]
    public string Effort { get; set; } = "low";

    [JsonPropertyName("temperature")]
    public double? Temperature { get; set; }

    [JsonPropertyName("system_instruction")]
    public string? SystemInstruction { get; set; }

    [JsonPropertyName("json_schema")]
    public string? JsonSchema { get; set; }

    [JsonPropertyName("timeout_secs")]
    public int TimeoutSecs { get; set; } = 60;
}

public class TtagyResponse
{
    [JsonPropertyName("session_id")]
    public required string SessionId { get; set; }

    [JsonPropertyName("status")]
    public required string Status { get; set; } // "success" | "error" | "aborted"

    [JsonPropertyName("content")]
    public required string Content { get; set; }

    [JsonPropertyName("thinking_content")]
    public string? ThinkingContent { get; set; }

    [JsonPropertyName("model")]
    public required string Model { get; set; }

    [JsonPropertyName("elapsed_ms")]
    public double ElapsedMs { get; set; }

    [JsonPropertyName("prompt_tokens")]
    public int? PromptTokens { get; set; }

    [JsonPropertyName("output_tokens")]
    public int? OutputTokens { get; set; }

    [JsonPropertyName("error_message")]
    public string? ErrorMessage { get; set; }
}

[JsonPolymorphic(TypeDiscriminatorPropertyName = "type")]
[JsonDerivedType(typeof(AgyInitEvent), "agy:init")]
[JsonDerivedType(typeof(AgyThinkingDeltaEvent), "agy:thinking_delta")]
[JsonDerivedType(typeof(AgyContentDeltaEvent), "agy:content_delta")]
[JsonDerivedType(typeof(AgyDoneEvent), "agy:done")]
[JsonDerivedType(typeof(AgyErrorEvent), "agy:error")]
public abstract record TtagyStreamEvent
{
    [JsonPropertyName("session_id")]
    public required string SessionId { get; init; }
}

public sealed record AgyInitEvent : TtagyStreamEvent
{
    [JsonPropertyName("model")] public required string Model { get; init; }
    [JsonPropertyName("effort")] public required string Effort { get; init; }
    [JsonPropertyName("backend_mode")] public required string BackendMode { get; init; }
}

public sealed record AgyThinkingDeltaEvent : TtagyStreamEvent
{
    [JsonPropertyName("text_delta")] public required string TextDelta { get; init; }
    [JsonPropertyName("elapsed_ms")] public required double ElapsedMs { get; init; }
}

public sealed record AgyContentDeltaEvent : TtagyStreamEvent
{
    [JsonPropertyName("text_delta")] public required string TextDelta { get; init; }
    [JsonPropertyName("accumulated_chars")] public required long AccumulatedChars { get; init; }
    [JsonPropertyName("elapsed_ms")] public required double ElapsedMs { get; init; }
}

public sealed record AgyDoneEvent : TtagyStreamEvent
{
    [JsonPropertyName("full_content")] public required string FullContent { get; init; }
    [JsonPropertyName("thinking_content")] public string? ThinkingContent { get; init; }
    [JsonPropertyName("elapsed_ms")] public required double ElapsedMs { get; init; }
    [JsonPropertyName("prompt_tokens")] public int? PromptTokens { get; init; }
    [JsonPropertyName("output_tokens")] public int? OutputTokens { get; init; }
}

public sealed record AgyErrorEvent : TtagyStreamEvent
{
    [JsonPropertyName("error_code")] public required string ErrorCode { get; init; }
    [JsonPropertyName("error_message")] public required string ErrorMessage { get; init; }
    [JsonPropertyName("is_retryable")] public required bool IsRetryable { get; init; }
}
