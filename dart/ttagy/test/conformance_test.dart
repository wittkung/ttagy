import 'dart:io';
import 'package:test/test.dart';
import 'package:ttagy/ttagy.dart';

void main() {
  setUpAll(() {
    final root = Directory.current.parent.parent.path;
    final mockPath = '$root/target/debug/mock-agy';
    Platform.environment['AGY_PATH'] = mockPath;
  });

  test('Dart: extractStructuredJson handles markdown fences', () {
    const raw = 'Output:\n```json\n{\n  "status": "ok",\n  "count": 42\n}\n```\nDone';
    final extracted = extractStructuredJson(raw);
    expect(extracted, contains('"status": "ok"'));
  });

  test('Dart CTS: stream_normal parity', () async {
    final client = TtagyClient(autoFallback: true);
    final events = <TtagyStreamEvent>[];
    final contentBuf = StringBuffer();

    await for (final ev in client.streamChat(const TtagyRequest(
      prompt: 'scenario:stream_normal',
      model: 'gemini-3.7-flash',
      effort: 'low',
    ))) {
      events.add(ev);
      if (ev is TtagyContentDeltaEvent) {
        contentBuf.write(ev.textDelta);
      }
    }

    final types = events.map((e) => e.type).toList();
    expect(types, containsAllInOrder(['agy:init', 'agy:thinking_delta', 'agy:content_delta', 'agy:done']));
    expect(contentBuf.toString(), contains('Antigravity AI 助手'));
  });

  test('Dart CTS: structured_json parity', () async {
    final client = TtagyClient(autoFallback: true);
    final result = await client.runJson(
      const TtagyRequest(
        prompt: 'scenario:structured_json',
        model: 'gemini-3.7-flash',
        effort: 'low',
      ),
      (json) => json as Map<String, dynamic>,
    );

    expect(result['status'], equals('success'));
    expect(result['task'], equals('compression'));
    expect(result['files_count'], equals(3));
  });

  test('Dart CTS: quota_error parity', () async {
    final client = TtagyClient(autoFallback: true);
    final events = <TtagyStreamEvent>[];

    await for (final ev in client.streamChat(const TtagyRequest(
      prompt: 'scenario:quota_error',
      model: 'gemini-3.7-flash',
      effort: 'low',
    ))) {
      events.add(ev);
    }

    expect(events.length, equals(2));
    expect(events[0], isA<TtagyInitEvent>());
    expect(events[1], isA<TtagyErrorEvent>());
    final err = events[1] as TtagyErrorEvent;
    expect(err.errorMessage, contains('Resource quota exceeded'));
  });
}
