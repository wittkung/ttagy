import 'package:ttagy/ttagy.dart';

void main() async {
  final client = TtagyClient(
    socketPath: '/tmp/ttagy.sock',
    autoFallback: true,
  );

  print('⚡ Streaming from Dart / Flutter SDK:');

  final stream = client.streamChat(const TtagyRequest(
    prompt: 'How to handle state in Flutter with InheritedWidget?',
    model: 'gemini-3.7-flash',
    effort: 'low',
  ));

  await for (final event in stream) {
    if (event is ContentDeltaEvent) {
      print(event.textDelta);
    } else if (event is DoneEvent) {
      print('\n\n✅ Done in ${event.elapsedMs}ms');
    }
  }
}
