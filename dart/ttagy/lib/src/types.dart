// 强类型契约模型 (对齐 JSON Schema v1)

class TtagyRequest {
  final String? sessionId;
  final String prompt;
  final String? model;
  final String? effort;
  final double? temperature;
  final String? systemInstruction;
  final String? jsonSchema;
  final String? agent;
  final String? mode;
  final String? conversationId;
  final bool? continueLast;
  final String? project;
  final List<String> addDirs;
  final bool? sandbox;
  final bool? dangerouslySkipPermissions;
  final bool? disableSlashCommands;
  final int timeoutSecs;

  const TtagyRequest({
    this.sessionId,
    required this.prompt,
    this.model = 'gemini-3.7-flash',
    this.effort = 'low',
    this.temperature,
    this.systemInstruction,
    this.jsonSchema,
    this.agent,
    this.mode,
    this.conversationId,
    this.continueLast,
    this.project,
    this.addDirs = const [],
    this.sandbox,
    this.dangerouslySkipPermissions,
    this.disableSlashCommands,
    this.timeoutSecs = 60,
  });

  Map<String, dynamic> toJson() => {
        if (sessionId != null) 'session_id': sessionId,
        'prompt': prompt,
        if (model != null) 'model': model,
        if (effort != null) 'effort': effort,
        if (temperature != null) 'temperature': temperature,
        if (systemInstruction != null) 'system_instruction': systemInstruction,
        if (jsonSchema != null) 'json_schema': jsonSchema,
        if (agent != null) 'agent': agent,
        if (mode != null) 'mode': mode,
        if (conversationId != null) 'conversation_id': conversationId,
        if (continueLast != null) 'continue_last': continueLast,
        if (project != null) 'project': project,
        if (addDirs.isNotEmpty) 'add_dirs': addDirs,
        if (sandbox != null) 'sandbox': sandbox,
        if (dangerouslySkipPermissions != null) 'dangerously_skip_permissions': dangerouslySkipPermissions,
        if (disableSlashCommands != null) 'disable_slash_commands': disableSlashCommands,
        'timeout_secs': timeoutSecs,
      };
}

class TtagyResponse {
  final String sessionId;
  final String status;
  final String content;
  final String? thinkingContent;
  final String model;
  final double elapsedMs;
  final int? promptTokens;
  final int? outputTokens;
  final String? errorMessage;

  const TtagyResponse({
    required this.sessionId,
    required this.status,
    required this.content,
    this.thinkingContent,
    required this.model,
    required this.elapsedMs,
    this.promptTokens,
    this.outputTokens,
    this.errorMessage,
  });
}

abstract class TtagyStreamEvent {
  final String type;
  final String sessionId;

  const TtagyStreamEvent({required this.type, required this.sessionId});

  factory TtagyStreamEvent.init({
    required String sessionId,
    required String model,
    required String effort,
    required String backendMode,
  }) = TtagyInitEvent;

  factory TtagyStreamEvent.thinkingDelta({
    required String sessionId,
    required String textDelta,
    required double elapsedMs,
  }) = TtagyThinkingDeltaEvent;

  factory TtagyStreamEvent.contentDelta({
    required String sessionId,
    required String textDelta,
    required int accumulatedChars,
    required double elapsedMs,
  }) = TtagyContentDeltaEvent;

  factory TtagyStreamEvent.done({
    required String sessionId,
    required String fullContent,
    String? thinkingContent,
    required double elapsedMs,
    int? promptTokens,
    int? outputTokens,
  }) = TtagyDoneEvent;

  factory TtagyStreamEvent.error({
    required String sessionId,
    required String errorCode,
    required String errorMessage,
    required bool isRetryable,
  }) = TtagyErrorEvent;

  factory TtagyStreamEvent.fromJson(Map<String, dynamic> json) {
    final type = json['type'] as String? ?? '';
    final sid = json['session_id'] as String? ?? '';
    switch (type) {
      case 'agy:init':
        return TtagyInitEvent(
          sessionId: sid,
          model: json['model'] as String? ?? '',
          effort: json['effort'] as String? ?? '',
          backendMode: json['backend_mode'] as String? ?? '',
        );
      case 'agy:thinking_delta':
        return TtagyThinkingDeltaEvent(
          sessionId: sid,
          textDelta: json['text_delta'] as String? ?? '',
          elapsedMs: (json['elapsed_ms'] as num?)?.toDouble() ?? 0.0,
        );
      case 'agy:content_delta':
        return TtagyContentDeltaEvent(
          sessionId: sid,
          textDelta: json['text_delta'] as String? ?? '',
          accumulatedChars: json['accumulated_chars'] as int? ?? 0,
          elapsedMs: (json['elapsed_ms'] as num?)?.toDouble() ?? 0.0,
        );
      case 'agy:done':
        return TtagyDoneEvent(
          sessionId: sid,
          fullContent: json['full_content'] as String? ?? '',
          thinkingContent: json['thinking_content'] as String?,
          elapsedMs: (json['elapsed_ms'] as num?)?.toDouble() ?? 0.0,
          promptTokens: json['prompt_tokens'] as int?,
          outputTokens: json['output_tokens'] as int?,
        );
      case 'agy:error':
        return TtagyErrorEvent(
          sessionId: sid,
          errorCode: json['error_code'] as String? ?? 'UNKNOWN',
          errorMessage: json['error_message'] as String? ?? '',
          isRetryable: json['is_retryable'] as bool? ?? false,
        );
      default:
        throw FormatException('Unknown TtagyStreamEvent type: $type');
    }
  }
}

class TtagyInitEvent extends TtagyStreamEvent {
  final String model;
  final String effort;
  final String backendMode;
  const TtagyInitEvent({
    required super.sessionId,
    required this.model,
    required this.effort,
    required this.backendMode,
  }) : super(type: 'agy:init');
}

class TtagyThinkingDeltaEvent extends TtagyStreamEvent {
  final String textDelta;
  final double elapsedMs;
  const TtagyThinkingDeltaEvent({
    required super.sessionId,
    required this.textDelta,
    required this.elapsedMs,
  }) : super(type: 'agy:thinking_delta');
}

class TtagyContentDeltaEvent extends TtagyStreamEvent {
  final String textDelta;
  final int accumulatedChars;
  final double elapsedMs;
  const TtagyContentDeltaEvent({
    required super.sessionId,
    required this.textDelta,
    required this.accumulatedChars,
    required this.elapsedMs,
  }) : super(type: 'agy:content_delta');
}

class TtagyDoneEvent extends TtagyStreamEvent {
  final String fullContent;
  final String? thinkingContent;
  final double elapsedMs;
  final int? promptTokens;
  final int? outputTokens;
  const TtagyDoneEvent({
    required super.sessionId,
    required this.fullContent,
    this.thinkingContent,
    required this.elapsedMs,
    this.promptTokens,
    this.outputTokens,
  }) : super(type: 'agy:done');
}

class TtagyErrorEvent extends TtagyStreamEvent {
  final String errorCode;
  final String errorMessage;
  final bool isRetryable;
  const TtagyErrorEvent({
    required super.sessionId,
    required this.errorCode,
    required this.errorMessage,
    required this.isRetryable,
  }) : super(type: 'agy:error');
}
