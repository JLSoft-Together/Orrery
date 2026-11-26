import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../remote/secure_kv.dart';

const String onlineSpeechKey = 'media.onlineSpeech';
const String ignoreAskedKey = 'media.inboxIgnoreAsked';

class MediaPrefs {
  MediaPrefs(this.kv);

  final SecureKv kv;

  Future<bool> onlineSpeechAllowed() async =>
      (await kv.read(onlineSpeechKey)) == 'on';

  Future<void> allowOnlineSpeech() => kv.write(onlineSpeechKey, 'on');

  Future<bool> ignoreAsked(String root) async {
    final raw = await kv.read(ignoreAskedKey);
    return raw != null && raw.split('\n').contains(_norm(root));
  }

  Future<void> markIgnoreAsked(String root) async {
    final raw = await kv.read(ignoreAskedKey);
    final list = raw == null || raw.isEmpty ? <String>[] : raw.split('\n');
    final key = _norm(root);
    if (list.contains(key)) return;
    list.add(key);
    final kept = list.length > 64 ? list.sublist(list.length - 64) : list;
    await kv.write(ignoreAskedKey, kept.join('\n'));
  }

  static String _norm(String root) =>
      root.replaceAll('\\', '/').replaceAll(RegExp(r'/+$'), '').toLowerCase();
}

final mediaPrefsProvider = Provider<MediaPrefs>(
  (ref) => MediaPrefs(FlutterSecureKv()),
);
