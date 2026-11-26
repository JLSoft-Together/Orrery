import 'dart:convert';

import 'protocol.dart';

const int maxClipBytes = 64 * 1024;
const String clipCap = 'clip';

enum ClipPushResult { sent, empty, tooLarge, secret, denied, offline, failed }

typedef ClipCommander = Future<JsonMap> Function(
  String type, [
  JsonMap? fields,
]);

String? clipEventText(JsonMap event) {
  if (event['t'] != 'clip') return null;
  final text = event['text'];
  if (text is! String || text.isEmpty) return null;
  if (utf8.encode(text).length > maxClipBytes) return null;
  return text;
}

Future<ClipPushResult> pushClip(ClipCommander command, String? text) async {
  if (text == null || text.isEmpty) return ClipPushResult.empty;
  if (utf8.encode(text).length > maxClipBytes) return ClipPushResult.tooLarge;
  try {
    await command('clip.push', {'text': text});
    return ClipPushResult.sent;
  } on RemoteCommandException catch (error) {
    return switch (error.code) {
      'secret' => ClipPushResult.secret,
      'limit' => ClipPushResult.tooLarge,
      'denied' => ClipPushResult.denied,
      'offline' || 'timeout' => ClipPushResult.offline,
      _ => ClipPushResult.failed,
    };
  } on Object {
    return ClipPushResult.failed;
  }
}
