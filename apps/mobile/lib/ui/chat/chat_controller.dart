import 'dart:async';

import 'package:flutter/foundation.dart';

import '../../remote/chat_protocol.dart';
import '../../remote/protocol.dart';
import '../machine_api.dart';

enum OutboxState { sending, sent, unsent }

class OutboxItem {
  OutboxItem(this.id, this.text, this.at, this.seen);

  final int id;
  final Set<String> seen;
  final String text;
  final DateTime at;
  OutboxState state = OutboxState.sending;
  String? error;
}

class ChatLogCache {
  ChatLogCache([this.max = 8]);

  final int max;
  final Map<String, ChatLog> _logs = {};

  ChatLog take(String key, String paneKey) {
    final hit = _logs.remove(key);
    return hit ?? ChatLog(paneKey);
  }

  void keep(String key, ChatLog log) {
    if (log.gen == null) return;
    _logs.remove(key);
    _logs[key] = log;
    while (_logs.length > max) {
      _logs.remove(_logs.keys.first);
    }
  }

  void clear() => _logs.clear();
}

final ChatLogCache chatLogCache = ChatLogCache();

class ChatController extends ChangeNotifier {
  ChatController({
    required this.api,
    required this.paneKey,
    this.cacheKey,
    this.sentTtl = const Duration(seconds: 20),
    DateTime Function()? now,
  }) : log = cacheKey == null
           ? ChatLog(paneKey)
           : chatLogCache.take(cacheKey, paneKey),
       _now = now ?? DateTime.now {
    if (log.gen != null) loading = false;
  }

  final String? cacheKey;

  final MachineApi api;
  final String paneKey;
  final Duration sentTtl;
  final ChatLog log;
  final DateTime Function() _now;

  final List<OutboxItem> outbox = [];
  StreamSubscription<JsonMap>? _sub;
  bool loading = true;
  bool loadingOlder = false;
  bool unsupported = false;
  String? error;
  bool _disposed = false;
  bool _watching = false;
  bool _paused = false;
  int _nextId = 1;
  final Map<int, Timer> _timers = {};

  List<ChatItem> get items => log.view;

  void start() {
    _sub ??= api.events.listen(_onEvent);
    unawaited(watch());
  }

  Future<void> watch() async {
    if (_watching || _disposed || _paused) return;
    _watching = true;
    try {
      final reply = await api.command('chat.watch', {
        'paneKey': paneKey,
        'limit': ChatLimits.page,
      });
      final page = ChatPage.parse(reply);
      if (page != null) log.addPage(page);
      error = null;
      unsupported = false;
    } on RemoteCommandException catch (e) {
      if (e.code == 'denied') {
        unsupported = true;
      } else {
        error = e.code;
      }
    } on Object {
      error = 'internal';
    } finally {
      _watching = false;
      loading = false;
      _dropEchoed();
      _notify();
    }
  }

  Future<void> loadOlder() async {
    final before = log.older;
    if (before == null || loadingOlder || _disposed) return;
    loadingOlder = true;
    _notify();
    try {
      final reply = await api.command('chat.page', {
        'paneKey': paneKey,
        'gen': ?log.gen,
        'before': before,
        'limit': ChatLimits.page,
      });
      final page = ChatPage.parse(reply);
      if (page != null) log.addPage(page);
    } on Object {
      loadingOlder = false;
      _notify();
      return;
    }
    loadingOlder = false;
    _notify();
  }

  void pause() {
    if (_paused || _disposed) return;
    _paused = true;
    _unwatch();
  }

  void resume() {
    if (!_paused || _disposed) return;
    _paused = false;
    unawaited(watch());
  }

  void _unwatch() {
    unawaited(
      api
          .command('chat.unwatch', {'paneKey': paneKey})
          .then((_) {}, onError: (Object _) {}),
    );
  }

  void _onEvent(JsonMap event) {
    if (_paused) return;
    final type = event['t'];
    if (type == 'hello') {
      unawaited(watch());
      return;
    }
    if (type is! String || !type.startsWith('chat.')) return;
    final parsed = ChatEvent.parse(event);
    if (parsed == null || parsed is ChatLastEvent) return;
    final changed = log.apply(parsed);
    if (log.stale) {
      unawaited(watch());
    }
    if (changed) {
      _dropEchoed();
      _notify();
    }
  }

  Future<bool> send(String raw) async {
    final text = raw.trim();
    if (text.isEmpty) return false;
    final seen = {
      for (final item in log.items)
        if (item is ChatMe) item.id,
    };
    final item = OutboxItem(_nextId++, text, _now(), seen);
    outbox.add(item);
    _notify();
    return _deliver(item);
  }

  Future<bool> resend(OutboxItem item) {
    item
      ..state = OutboxState.sending
      ..error = null;
    _notify();
    return _deliver(item);
  }

  void discard(OutboxItem item) {
    _timers.remove(item.id)?.cancel();
    outbox.remove(item);
    _notify();
  }

  Future<bool> _deliver(OutboxItem item) async {
    try {
      await api.command('ws.send', {'paneKey': paneKey, 'text': item.text});
      item.state = OutboxState.sent;
      _timers[item.id] = Timer(sentTtl, () {
        _timers.remove(item.id);
        outbox.remove(item);
        _notify();
      });
      _dropEchoed();
      _notify();
      return true;
    } on RemoteCommandException catch (e) {
      item
        ..state = OutboxState.unsent
        ..error = e.code;
    } on Object {
      item
        ..state = OutboxState.unsent
        ..error = 'internal';
    }
    _notify();
    return false;
  }

  void _dropEchoed() {
    if (outbox.isEmpty) return;
    final mine = [
      for (final item in log.items)
        if (item is ChatMe) item,
    ];
    final used = <String>{};
    outbox.removeWhere((o) {
      if (o.state != OutboxState.sent) return false;
      for (final me in mine) {
        if (used.contains(me.id) || o.seen.contains(me.id)) continue;
        if (me.text.trim() != o.text) continue;
        used.add(me.id);
        _timers.remove(o.id)?.cancel();
        return true;
      }
      return false;
    });
  }

  Future<void> stop(int since) async {
    await api.command('ws.keys', {
      'paneKey': paneKey,
      'keys': 'stop',
      'since': since,
    });
  }

  Future<void> answer(RemoteMenu menu, RemoteMenuItem item) async {
    await api.command('ws.menu', {
      'paneKey': paneKey,
      'sig': menu.sig,
      'index': item.index,
    });
  }

  void _notify() {
    if (!_disposed) notifyListeners();
  }

  @override
  void dispose() {
    _disposed = true;
    for (final timer in _timers.values) {
      timer.cancel();
    }
    _timers.clear();
    unawaited(_sub?.cancel());
    if (!_paused) _unwatch();
    final key = cacheKey;
    if (key != null) chatLogCache.keep(key, log);
    super.dispose();
  }
}
