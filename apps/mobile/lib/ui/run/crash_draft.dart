import 'package:flutter_riverpod/flutter_riverpod.dart';

class CrashDraft extends Notifier<Map<String, String>> {
  @override
  Map<String, String> build() => const {};

  void put(String paneKey, String text) => state = {...state, paneKey: text};

  String? take(String paneKey) {
    final text = state[paneKey];
    if (text == null) return null;
    state = {...state}..remove(paneKey);
    return text;
  }
}

final crashDraftProvider = NotifierProvider<CrashDraft, Map<String, String>>(
  CrashDraft.new,
);
