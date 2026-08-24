import 'dart:convert';

String extractStructuredJson(String raw) {
  final trimmed = raw.trim();

  try {
    jsonDecode(trimmed);
    return trimmed;
  } catch (_) {}

  final mdRegex = RegExp(r'```(?:json)?\s*([\s\S]*?)\s*```', caseSensitive: false);
  final match = mdRegex.firstMatch(trimmed);
  String text = trimmed;
  if (match != null && match.group(1) != null) {
    final candidate = match.group(1)!.trim();
    try {
      jsonDecode(candidate);
      return candidate;
    } catch (_) {}
    text = candidate;
  }

  bool inString = false;
  bool escape = false;
  int depth = 0;
  int startIndex = -1;

  for (int i = 0; i < text.length; i++) {
    final char = text[i];
    if (escape) {
      escape = false;
      continue;
    }
    if (char == r'\') {
      escape = true;
      continue;
    }
    if (char == '"') {
      inString = !inString;
      continue;
    }
    if (inString) continue;

    if (char == '{' || char == '[') {
      if (depth == 0) startIndex = i;
      depth++;
    } else if (char == '}' || char == ']') {
      if (depth > 0) {
        depth--;
        if (depth == 0 && startIndex != -1) {
          final candidate = text.substring(startIndex, i + 1);
          try {
            jsonDecode(candidate);
            return candidate;
          } catch (_) {}
        }
      }
    }
  }

  throw const FormatException('Failed to extract valid structured JSON.');
}
