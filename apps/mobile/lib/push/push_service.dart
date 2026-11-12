import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import '../app/link_source.dart';
import '../remote/secure_kv.dart';
import 'push_config.dart';
import 'push_gateway.dart';
import 'push_host.dart';
import 'push_notice.dart';
import 'push_present.dart';
import 'push_seal.dart';

export 'push_host.dart';

class PushService implements LinkSource {
  PushService({
    required this.gateway,
    required this.kv,
    required this.host,
    required this.platform,
    required this.localeCode,
    this.resumed = _alwaysResumed,
    this.machineKey = _noMachine,
  });

  static bool _alwaysResumed() => true;

  static Future<Uint8List?> _noMachine(String _) async => null;

  final PushGateway gateway;
  final SecureKv kv;
  final PushHost host;
  final String platform;
  final String Function() localeCode;
  final bool Function() resumed;
  final Future<Uint8List?> Function(String machineId) machineKey;
  late final PushPresenter _presenter = PushPresenter(
    gateway: gateway,
    kv: kv,
    sessionOpen: (m) => host.readyNow.any((s) => s.machineId == m),
    resumed: resumed,
    localeCode: localeCode,
    machineKey: machineKey,
  );

  final StreamController<void> _changes = StreamController.broadcast();
  final StreamController<Uri> _taps = StreamController.broadcast();
  final List<StreamSubscription<Object?>> _subs = [];
  final Map<String, PushConfig?> _configs = {};
  final Map<String, PushSession> _sessions = {};
  final Set<String> _pending = {};
  String? _token;
  String? _candidate;
  bool _enabled = false;
  bool _revoked = false;
  String? _error;
  Uri? _initial;
  Future<void> _queue = Future<void>.value();
  Set<String> _mute = {'error'};
  bool _content = true;

  bool get content => _content;

  Future<void> setContent(bool on) async {
    if (_content == on) return;
    _content = on;
    _notify();
    await kv.write(pushContentKey, on ? 'on' : 'off');
    await _syncAll();
  }

  bool wants(String group) => !_mute.contains(group);

  Future<void> setGroup(String group, bool on) async {
    if (!pushGroups.contains(group) || wants(group) == on) return;
    _mute = {..._mute}..remove(group);
    if (!on) _mute.add(group);
    _notify();
    await kv.write(pushMuteKey, jsonEncode(_mute.toList()));
    await _syncAll();
  }

  bool get enabled => _enabled && !_revoked;

  bool get revoked => _revoked;

  String? get error => _error;

  bool get configured => _configs.values.any((c) => c != null);

  bool isForeign(String machineId) {
    final config = _configs[machineId];
    final active = gateway.activeProject;
    return config != null && active != null && config.projectId != active;
  }

  int get foreignCount => _configs.keys.where(isForeign).length;

  bool get usable =>
      _configs.entries.any((e) => e.value != null && !isForeign(e.key));

  Stream<void> get changes => _changes.stream;

  @override
  Stream<Uri> get links => Stream.multi((controller) {
    final first = _initial;
    _initial = null;
    if (first != null) controller.add(first);
    final sub = _taps.stream.listen(controller.add);
    controller.onCancel = sub.cancel;
  });

  Future<void> init() async {
    _enabled = (await kv.read(pushEnabledKey)) == 'on';
    _content = (await kv.read(pushContentKey)) != 'off';
    final mute = await kv.read(pushMuteKey);
    if (mute != null) {
      try {
        final list = jsonDecode(mute);
        if (list is List) _mute = list.whereType<String>().toSet();
      } on FormatException {
        _mute = {'error'};
      }
    }
    await _load();
    _subs
      ..add(gateway.tokenRefresh.listen(_onRefresh, onError: _fail))
      ..add(gateway.opened.listen(_onOpened, onError: _fail))
      ..add(gateway.arrivals.listen(_presenter.present))
      ..add(host.readySessions.listen(_onSession))
      ..add(host.configs.listen(_onConfig));
    for (final session in host.readyNow) {
      _sessions[session.machineId] = session;
    }
    if (!_enabled) return;
    final config = _firstConfig();
    if (config == null) return;
    try {
      if (!await gateway.open(config)) return;
      final initial = await gateway.initialOpen();
      if (initial != null) _initial = pushLink(initial);
      await _checkPermission();
      await _syncAll();
    } on PushException catch (e) {
      _fail(e);
    }
  }

  Future<void> dispose() async {
    for (final sub in _subs) {
      await sub.cancel();
    }
    _subs.clear();
    await gateway.dispose();
    await host.dispose();
    await _changes.close();
    await _taps.close();
  }

  void _fail(Object error) {
    _error = error is PushException ? error.code : 'error';
    _notify();
  }

  Future<void> _load() async {
    final raw = await kv.read(pushConfigsKey);
    if (raw != null) {
      try {
        final map = jsonDecode(raw);
        if (map is Map<String, dynamic>) {
          for (final entry in map.entries) {
            final value = entry.value;
            _configs[entry.key] = value is Map<String, dynamic>
                ? PushConfig.tryParse(value)
                : null;
          }
        }
      } on FormatException {
        _configs.clear();
      }
    }
    final pending = await kv.read(pushPendingKey);
    if (pending != null) {
      try {
        final list = jsonDecode(pending);
        if (list is List<dynamic>) _pending.addAll(list.whereType<String>());
      } on FormatException {
        _pending.clear();
      }
    }
  }

  Future<void> _saveConfigs() => kv.write(
    pushConfigsKey,
    jsonEncode({for (final e in _configs.entries) e.key: e.value?.toJson()}),
  );

  Future<void> _savePending() =>
      kv.write(pushPendingKey, jsonEncode(_pending.toList()));

  PushConfig? _firstConfig() {
    for (final config in _configs.values) {
      if (config != null) return config;
    }
    return null;
  }

  PushConfig? _targetConfig(String? machineId) {
    final active = gateway.activeProject;
    if (active != null) {
      for (final config in _configs.values) {
        if (config?.projectId == active) return config;
      }
      return null;
    }
    return _configs[machineId ?? _candidate];
  }

  void _notify() {
    if (!_changes.isClosed) _changes.add(null);
  }

  Future<void> refreshPermission() async {
    if (!_enabled || gateway.activeProject == null) return;
    try {
      await _checkPermission();
      if (!_revoked) await _syncAll();
    } on PushException catch (e) {
      _fail(e);
    }
  }

  Future<void> _checkPermission() async {
    final revoked = await gateway.permission() != PushPermission.granted;
    if (revoked != _revoked) {
      _revoked = revoked;
      _notify();
    }
  }

  void _onOpened(Map<String, String> data) {
    final uri = pushLink(data);
    if (uri != null && !_taps.isClosed) _taps.add(uri);
  }

  void _onRefresh(PushTokenChange change) {
    _token = change.token;
    if (!enabled) return;
    for (final entry in _configs.entries) {
      if (entry.value?.projectId == change.projectId) {
        unawaited(_register(entry.key));
      }
    }
  }

  void _onSession(PushSession session) {
    _sessions[session.machineId] = session;
    if (_pending.contains(session.machineId) && !_enabled) {
      unawaited(_flushPending(session));
      return;
    }
    unawaited(_register(session.machineId));
  }

  void _onConfig(PushConfigUpdate update) {
    if (update.config != null) _candidate = update.machineId;
    if (_configs.containsKey(update.machineId) &&
        _configs[update.machineId] == update.config) {
      unawaited(_register(update.machineId));
      return;
    }
    _configs[update.machineId] = update.config;
    unawaited(_saveConfigs());
    _notify();
    unawaited(_register(update.machineId));
  }

  Future<bool> _send(
    PushSession session,
    String type,
    Map<String, Object?> fields,
  ) async {
    try {
      await session.send(type, fields);
      return true;
    } on Exception {
      return false;
    }
  }

  Future<void> _flushPending(PushSession session) async {
    if (await _send(session, 'push.unregister', const {})) {
      _pending.remove(session.machineId);
      await _savePending();
    }
  }

  Future<void> _register(String machineId) async {
    final config = _configs[machineId];
    final session = _sessions[machineId];
    if (!enabled || config == null || session == null) return;
    if (isForeign(machineId)) return;
    try {
      final token = _token ??= await gateway.token();
      if (token == null || !enabled) return;
      await _send(session, 'push.register', {
        'token': token,
        'platform': platform,
        'locale': localeCode() == 'vi' ? 'vi' : 'en',
        'mute': [
          for (final g in pushGroups)
            if (_mute.contains(g)) g,
        ],
        'content': _content,
        if (gateway.sealing) ...{
          'seal': pushSealLevel,
          'sealOk': await kv.read(pushSealOkKey) == 'on',
        },
      });
    } on PushException catch (e) {
      _fail(e);
    }
  }

  Future<void> _syncAll() async {
    for (final machineId in _sessions.keys.toList()) {
      await _register(machineId);
    }
  }

  Future<PushResult> setEnabled(bool on, {String? machineId}) {
    final done = Completer<PushResult>();
    _queue = _queue.then((_) async {
      try {
        done.complete(await _toggle(on, machineId));
      } on Object catch (error, stack) {
        done.completeError(error, stack);
      }
    });
    return done.future;
  }

  Future<PushResult> _toggle(bool on, String? machineId) async {
    if (on == _enabled) return on ? PushResult.on : PushResult.off;
    if (!on) return _turnOff();
    final config = _targetConfig(machineId);
    if (config == null) return PushResult.unavailable;
    try {
      if (!await gateway.open(config)) return PushResult.unavailable;
      _error = null;
      if (await gateway.requestPermission() != PushPermission.granted) {
        return PushResult.denied;
      }
      _token = await gateway.token();
      if (_token == null) return PushResult.unavailable;
    } on PushException catch (e) {
      _fail(e);
      return PushResult.unavailable;
    }
    _enabled = true;
    _revoked = false;
    _pending.clear();
    await _savePending();
    await kv.write(pushEnabledKey, 'on');
    _notify();
    await _syncAll();
    return PushResult.on;
  }

  Future<PushResult> _turnOff() async {
    _enabled = false;
    await kv.write(pushEnabledKey, 'off');
    for (final machineId in _configs.keys) {
      final session = _sessions[machineId];
      if (session == null ||
          !await _send(session, 'push.unregister', const {})) {
        _pending.add(machineId);
      }
    }
    await _savePending();
    try {
      if (gateway.activeProject != null) await gateway.deleteToken();
    } on PushException catch (e) {
      _fail(e);
    }
    _token = null;
    _notify();
    return PushResult.off;
  }
}
