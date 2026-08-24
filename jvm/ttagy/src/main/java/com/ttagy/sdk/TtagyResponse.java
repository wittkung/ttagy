package com.ttagy.sdk;

public record TtagyResponse(
    String sessionId,
    String status,
    String content,
    String thinkingContent,
    String model,
    double elapsedMs,
    Integer promptTokens,
    Integer outputTokens,
    String errorMessage
) {}
