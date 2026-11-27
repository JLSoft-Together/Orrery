import 'dart:async';

import 'package:flutter/foundation.dart';

import '../remote/protocol.dart';
import 'prompt_model.dart';
import 'prompts_api.dart';

enum PromptsStatus { loading, ready, failed }

class PromptsController extends ChangeNotifier {
  PromptsController({required this.api, required this.paneKey}) {
    _sub = api.events.listen(_onEvent);
    load();
  }

  final PromptsApi api;
  final String paneKey;
  late final StreamSubscription<JsonMap> _sub;
  PromptsStatus status = PromptsStatus.loading;
  PromptList list = const PromptList();
  int _gen = 0;
  bool _disposed = false;

  void _onEvent(JsonMap event) {
    final t = event['t'];
    if (t == 'prompts' || t == 'hello') load();
  }

  Future<void> load() async {
    final gen = ++_gen;
    try {
      final next = await api.list(paneKey);
      if (_disposed || gen != _gen) return;
      list = next;
      status = PromptsStatus.ready;
    } on Object {
      if (_disposed || gen != _gen) return;
      status = PromptsStatus.failed;
    }
    notifyListeners();
  }

  Future<void> save(PromptDraft draft) async {
    await api.save(draft);
    await load();
  }

  Future<void> remove(String id) async {
    await api.delete(id);
    await load();
  }

  @override
  void dispose() {
    _disposed = true;
    _sub.cancel();
    super.dispose();
  }
}
