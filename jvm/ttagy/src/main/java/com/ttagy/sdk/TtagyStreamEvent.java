package com.ttagy.sdk;

public sealed interface TtagyStreamEvent permits
    TtagyStreamEvent.Init,
    TtagyStreamEvent.ThinkingDelta,
    TtagyStreamEvent.ContentDelta,
    TtagyStreamEvent.Done,
    TtagyStreamEvent.Error {

    String sessionId();

    record Init(String sessionId, String model, String effort, String backendMode) implements TtagyStreamEvent {}
    record ThinkingDelta(String sessionId, String textDelta, double elapsedMs) implements TtagyStreamEvent {}
    record ContentDelta(String sessionId, String textDelta, long accumulatedChars, double elapsedMs) implements TtagyStreamEvent {}
    record Done(String sessionId, String fullContent, String thinkingContent, double elapsedMs, Integer promptTokens, Integer outputTokens) implements TtagyStreamEvent {}
    record Error(String sessionId, String errorCode, String errorMessage, boolean isRetryable) implements TtagyStreamEvent {}
}
