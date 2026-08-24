import 'dart:async';
import 'dart:collection';
import 'dart:convert';
import 'dart:io';

import 'parser.dart';
import 'types.dart';

HttpClient createUdsHttpClient(String socketPath) {
  final client = HttpClient();
  client.connectionFactory = (Uri uri, String? proxyHost, int? proxyPort) {
    final address = InternetAddress(socketPath, type: InternetAddressType.unix);
    return Socket.startConnect(address, 0);
  };
  return client;
}

class TtagyClient {
  final String? baseUrl;
  final String? socketPath;
  final String? authToken;
  final bool autoFallback;
  late final HttpClient? _httpClient;

  TtagyClient({
    this.baseUrl,
    this.socketPath = '/tmp/ttagy.sock',
    this.authToken,
    this.autoFallback = true,
  }) {
    if (socketPath != null && File(socketPath!).existsSync()) {
      _httpClient = createUdsHttpClient(socketPath!);
    } else if (baseUrl != null) {
      _httpClient = HttpClient();
    } else {
      _httpClient = null;
    }
  }

  Stream<TtagyStreamEvent> streamChat(TtagyRequest request) {
    if (_httpClient != null) {
      final uri = Uri.parse(baseUrl != null ? '$baseUrl/api/v1/stream' : 'http://localhost/api/v1/stream');
      return _createSseStream(uri, request);
    }

    if (autoFallback) {
      return _createProcessFallbackStream(request);
    }

    return Stream.error(StateError('No daemon available and autoFallback is false.'));
  }

  Stream<TtagyStreamEvent> _createSseStream(Uri uri, TtagyRequest request) {
    late StreamController<TtagyStreamEvent> controller;
    HttpClientRequest? activeRequest;
    StreamSubscription? responseSub;

    controller = StreamController<TtagyStreamEvent>(
      onListen: () async {
        try {
          activeRequest = await _httpClient!.postUrl(uri);
          activeRequest!.headers.contentType = ContentType.json;
          if (authToken != null && authToken!.isNotEmpty) {
            activeRequest!.headers.set(HttpHeaders.authorizationHeader, 'Bearer $authToken');
          }

          activeRequest!.write(jsonEncode(request.toJson()));
          final response = await activeRequest!.close();

          if (response.statusCode != HttpStatus.ok) {
            controller.addError(HttpException(
              'Remote daemon returned status ${response.statusCode}',
              uri: uri,
            ));
            await controller.close();
            return;
          }

          responseSub = response
              .transform(utf8.decoder)
              .transform(const LineSplitter())
              .listen(
            (line) {
              final trimmed = line.trim();
              if (trimmed.startsWith('data:')) {
                final payload = trimmed.substring(5).trim();
                if (payload.isNotEmpty) {
                  try {
                    final map = jsonDecode(payload) as Map<String, dynamic>;
                    controller.add(TtagyStreamEvent.fromJson(map));
                  } catch (_) {}
                }
              }
            },
            onError: (err, stack) => controller.addError(err, stack),
            onDone: () => controller.close(),
            cancelOnError: true,
          );
        } catch (err, stack) {
          if (!controller.isClosed) {
            controller.addError(err, stack);
            await controller.close();
          }
        }
      },
      onCancel: () async {
        await responseSub?.cancel();
        activeRequest?.abort();
      },
    );

    return controller.stream;
  }

  Stream<TtagyStreamEvent> _createProcessFallbackStream(TtagyRequest request) {
    late StreamController<TtagyStreamEvent> controller;
    Process? process;
    Directory? sandboxDir;
    final stderrQueue = ListQueue<int>();
    const maxStderrBytes = 64 * 1024;

    controller = StreamController<TtagyStreamEvent>(
      onListen: () async {
        final binary = _findAgyBinary();
        final sid = request.sessionId ?? 'session_${DateTime.now().millisecondsSinceEpoch}';

        if (binary == null) {
          controller.add(TtagyStreamEvent.error(
            sessionId: sid,
            errorCode: 'BINARY_NOT_FOUND',
            errorMessage: 'Antigravity CLI (agy) binary not found on host.',
            isRetryable: false,
          ));
          await controller.close();
          return;
        }

        sandboxDir = Directory.systemTemp.createTempSync('ttagy_dart_');
        final logPath = '${sandboxDir!.path}/agy.log';

        final args = [
          '-p', request.prompt,
          '--output-format', 'stream-json',
          '--log-file', logPath,
        ];
        if (request.model != null) args.addAll(['--model', request.model!]);
        if (request.effort != null && request.effort != 'none') args.addAll(['--effort', request.effort!]);
        if (request.agent != null) args.addAll(['--agent', request.agent!]);
        if (request.mode != null) args.addAll(['--mode', request.mode!]);
        if (request.conversationId != null) args.addAll(['--conversation', request.conversationId!]);
        if (request.continueLast == true) args.add('--continue');
        if (request.project != null) args.addAll(['--project', request.project!]);
        for (final dir in request.addDirs) {
          args.addAll(['--add-dir', dir]);
        }
        if (request.sandbox == true) args.add('--sandbox');
        if (request.jsonSchema != null) args.addAll(['--json-schema', request.jsonSchema!]);
        if (request.disableSlashCommands ?? true) args.add('--disable-slash-commands');
        if (request.dangerouslySkipPermissions ?? true) args.add('--dangerously-skip-permissions');

        try {
          process = await Process.start(binary, args, workingDirectory: sandboxDir!.path);

          controller.add(TtagyStreamEvent.init(
            sessionId: sid,
            model: request.model ?? 'gemini-3.7-flash',
            effort: request.effort ?? 'low',
            backendMode: 'fallback_direct_spawn',
          ));

          process!.stderr.listen((chunk) {
            stderrQueue.addAll(chunk);
            while (stderrQueue.length > maxStderrBytes) {
              stderrQueue.removeFirst();
            }
          });

          bool hasError = false;
          final startTime = DateTime.now().millisecondsSinceEpoch;

          process!.stdout
              .transform(utf8.decoder)
              .transform(const LineSplitter())
              .listen((line) {
            final trimmed = line.trim();
            if (trimmed.isEmpty) return;

            try {
              final map = jsonDecode(trimmed) as Map<String, dynamic>;
              final evType = map['type'] ?? map['event'] ?? '';
              final elapsed = (DateTime.now().millisecondsSinceEpoch - startTime).toDouble();

              if (evType == 'step_update' && map['step_update'] is Map) {
                final step = map['step_update'] as Map<String, dynamic>;
                final thought = step['thought_delta'] ?? step['reasoning_delta'] ?? step['thinking_delta'];
                if (thought is String && thought.isNotEmpty) {
                  controller.add(TtagyStreamEvent.thinkingDelta(
                    sessionId: sid,
                    textDelta: thought,
                    elapsedMs: elapsed,
                  ));
                }
                final text = step['text_delta'] ?? step['content_delta'];
                if (text is String && text.isNotEmpty) {
                  controller.add(TtagyStreamEvent.contentDelta(
                    sessionId: sid,
                    textDelta: text,
                    accumulatedChars: text.length,
                    elapsedMs: elapsed,
                  ));
                }
              } else if (evType == 'result' || evType == 'done') {
                final res = map['result'] is Map ? map['result'] as Map<String, dynamic> : map;
                final content = res['content'] ?? res['response'] ?? res['text'] ?? '';
                final usage = res['usage'] is Map ? res['usage'] as Map<String, dynamic> : null;
                controller.add(TtagyStreamEvent.done(
                  sessionId: sid,
                  fullContent: content.toString(),
                  elapsedMs: elapsed,
                  promptTokens: usage?['prompt_tokens'] as int?,
                  outputTokens: (usage?['completion_tokens'] ?? usage?['output_tokens']) as int?,
                ));
              } else if (evType == 'error') {
                hasError = true;
                final err = map['error'] ?? map['message'] ?? 'Unknown CLI error';
                controller.add(TtagyStreamEvent.error(
                  sessionId: sid,
                  errorCode: 'CLI_ERROR',
                  errorMessage: err.toString(),
                  isRetryable: false,
                ));
              }
            } catch (_) {}
          }, onDone: () async {
            final exitCode = await process!.exitCode;
            if (exitCode != 0 && !hasError && stderrQueue.isNotEmpty) {
              final errText = utf8.decode(stderrQueue.toList(), allowMalformed: true);
              controller.add(TtagyStreamEvent.error(
                sessionId: sid,
                errorCode: 'CLI_EXIT_NONZERO',
                errorMessage: errText,
                isRetryable: false,
              ));
            }
            await controller.close();
          }, onError: (e) {
            controller.addError(e);
          });
        } catch (e, st) {
          controller.addError(e, st);
          await controller.close();
        }
      },
      onCancel: () async {
        process?.kill(ProcessSignal.sigkill);
        if (sandboxDir != null && sandboxDir!.existsSync()) {
          try {
            sandboxDir!.deleteSync(recursive: true);
          } catch (_) {}
        }
      },
    );

    return controller.stream;
  }

  Future<TtagyResponse> chat(TtagyRequest request) async {
    final sw = Stopwatch()..start();
    final sid = request.sessionId ?? 'session_${DateTime.now().millisecondsSinceEpoch}';
    final fullBuf = StringBuffer();
    final thinkBuf = StringBuffer();

    await for (final event in streamChat(request)) {
      if (event is TtagyContentDeltaEvent) {
        fullBuf.write(event.textDelta);
      } else if (event is TtagyThinkingDeltaEvent) {
        thinkBuf.write(event.textDelta);
      } else if (event is TtagyDoneEvent) {
        return TtagyResponse(
          sessionId: sid,
          status: 'success',
          content: event.fullContent.isNotEmpty ? event.fullContent : fullBuf.toString(),
          thinkingContent: event.thinkingContent ?? (thinkBuf.isNotEmpty ? thinkBuf.toString() : null),
          model: request.model ?? 'gemini-3.7-flash',
          elapsedMs: sw.elapsedMilliseconds.toDouble(),
          promptTokens: event.promptTokens,
          outputTokens: event.outputTokens,
        );
      } else if (event is TtagyErrorEvent) {
        return TtagyResponse(
          sessionId: sid,
          status: 'error',
          content: '',
          model: request.model ?? 'gemini-3.7-flash',
          elapsedMs: sw.elapsedMilliseconds.toDouble(),
          errorMessage: event.errorMessage,
        );
      }
    }

    return TtagyResponse(
      sessionId: sid,
      status: 'success',
      content: fullBuf.toString(),
      thinkingContent: thinkBuf.isNotEmpty ? thinkBuf.toString() : null,
      model: request.model ?? 'gemini-3.7-flash',
      elapsedMs: sw.elapsedMilliseconds.toDouble(),
    );
  }

  Future<T> runJson<T>(TtagyRequest request, T Function(dynamic json) fromJson) async {
    final resp = await chat(request);
    if (resp.status != 'success') {
      throw StateError('Ttagy error: ${resp.errorMessage}');
    }
    final jsonStr = extractStructuredJson(resp.content);
    return fromJson(jsonDecode(jsonStr));
  }

  String? _findAgyBinary() {
    final envPath = Platform.environment['AGY_PATH'];
    if (envPath != null && File(envPath).existsSync()) return envPath;

    final home = Platform.environment['HOME'] ?? '';
    final candidates = [
      '$home/.local/bin/agy',
      '$home/bin/agy',
      '/usr/local/bin/agy',
      '/opt/homebrew/bin/agy',
    ];
    for (final c in candidates) {
      if (File(c).existsSync()) return c;
    }
    return null;
  }
}
