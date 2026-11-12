import 'dart:async';

import 'package:clock/clock.dart';
import 'package:flutter/foundation.dart';

import '../remote/secure_kv.dart';
import 'device_auth.dart';

enum LockTimeout {
  everyOpen(Duration.zero),
  minutes15(Duration(minutes: 15)),
  hour1(Duration(hours: 1)),
  hours4(Duration(hours: 4));

  const LockTimeout(this.span);

  final Duration span;
}

const String lockEnabledKey = 'applock.enabled';
const String lockTimeoutKey = 'applock.timeout';

class AppLock extends ChangeNotifier {
  AppLock({
    required this.auth,
    required this.kv,
    required this.reason,
    this.elapsedRealtime,
  });

  final DeviceAuth auth;
  final SecureKv kv;
  final String Function() reason;
  final Future<int?> Function()? elapsedRealtime;

  bool _enabled = true;
  LockTimeout _timeout = LockTimeout.hour1;
  bool _locked = true;
  bool _loaded = false;
  AuthOutcome? _last;
  Future<AuthOutcome>? _busy;
  Stopwatch? _watch;
  DateTime? _unlockedAt;
  Timer? _expiry;
  bool _away = false;
  bool _foreground = true;
  bool _pausedWhileBusy = false;
  int? _baseNative;
  Duration _nativeElapsed = Duration.zero;

  bool get enabled => _enabled;

  LockTimeout get timeout => _timeout;

  bool get locked => _locked;

  bool get loaded => _loaded;

  bool get busy => _busy != null;

  AuthOutcome? get lastOutcome => _last;

  Future<void> load() async {
    try {
      final storedEnabled = await kv.read(lockEnabledKey);
      final storedTimeout = await kv.read(lockTimeoutKey);
      _enabled = storedEnabled != '0';
      _timeout = LockTimeout.values.firstWhere(
        (value) => value.name == storedTimeout,
        orElse: () => LockTimeout.hour1,
      );
    } on Object {
      _enabled = true;
      _timeout = LockTimeout.hour1;
    }
    _loaded = true;
    _locked = _enabled;
    _unlockedAt = null;
    _watch = null;
    notifyListeners();
  }

  Future<AuthOutcome> _ask() {
    final running = _busy;
    if (running != null) return running;
    final run = auth.authenticate(reason()).then((outcome) {
      _busy = null;
      _last = outcome;
      final paused = _pausedWhileBusy;
      _pausedWhileBusy = false;
      if (paused && (_timeout == LockTimeout.everyOpen || !_foreground)) {
        lock();
      }
      return outcome;
    });
    _busy = run;
    notifyListeners();
    return run;
  }

  Future<AuthOutcome> unlock() async {
    if (!_enabled) {
      if (_locked) await _open();
      return AuthOutcome.success;
    }
    final outcome = await _ask();
    if (outcome == AuthOutcome.success && _enabled && _locked) {
      await _open();
    } else {
      notifyListeners();
    }
    return outcome;
  }

  Future<int?> _readNative() async {
    final source = elapsedRealtime;
    if (source == null) return null;
    try {
      return await source();
    } on Object {
      return null;
    }
  }

  Future<void> _sample() async {
    final base = _baseNative;
    if (base == null) return;
    final now = await _readNative();
    if (now == null) return;
    final delta = Duration(milliseconds: now - base);
    if (delta > _nativeElapsed) _nativeElapsed = delta;
  }

  Future<void> _stamp() async {
    _baseNative = await _readNative();
    _nativeElapsed = Duration.zero;
    _watch = clock.stopwatch()..start();
    _unlockedAt = clock.now();
  }

  Future<void> _open() async {
    await _stamp();
    _locked = false;
    _arm();
    notifyListeners();
  }

  void lock() {
    if (!_enabled) return;
    _expiry?.cancel();
    _expiry = null;
    _watch = null;
    _unlockedAt = null;
    _baseNative = null;
    _nativeElapsed = Duration.zero;
    if (_locked) return;
    _locked = true;
    notifyListeners();
  }

  Duration _elapsed() {
    final watch = _watch;
    final at = _unlockedAt;
    if (watch == null || at == null) return Duration.zero;
    final wall = clock.now().difference(at);
    var best = watch.elapsed;
    if (wall > best) best = wall;
    if (_nativeElapsed > best) best = _nativeElapsed;
    return best;
  }

  bool get expired {
    if (!_enabled || _locked) return false;
    if (_timeout == LockTimeout.everyOpen) return false;
    return _elapsed() >= _timeout.span;
  }

  void _arm() {
    _expiry?.cancel();
    _expiry = null;
    if (!_enabled || _locked || _timeout == LockTimeout.everyOpen) return;
    final remaining = _timeout.span - _elapsed();
    _expiry = Timer(
      remaining.isNegative ? Duration.zero : remaining,
      _onExpiry,
    );
  }

  Future<void> _onExpiry() async {
    _expiry = null;
    await _sample();
    if (expired) {
      lock();
    } else {
      _arm();
    }
  }

  void appPaused() {
    _foreground = false;
    _away = true;
    if (busy) {
      _pausedWhileBusy = true;
      return;
    }
    if (_timeout == LockTimeout.everyOpen) lock();
  }

  Future<void> appResumed() async {
    _foreground = true;
    if (!_enabled || busy) return;
    final away = _away;
    _away = false;
    await _sample();
    if (!_locked && expired) lock();
    if (_locked) {
      if (away) await unlock();
    } else {
      _arm();
    }
  }

  Future<AuthOutcome> confirm() async {
    final outcome = await _ask();
    notifyListeners();
    return outcome;
  }

  Future<bool> turnOffWithoutAuth() async {
    if (!await auth.isDeviceInsecure()) return false;
    return apply(enabled: false);
  }

  Future<bool> apply({bool? enabled, LockTimeout? timeout}) async {
    final nextEnabled = enabled ?? _enabled;
    final nextTimeout = timeout ?? _timeout;
    try {
      await kv.write(lockEnabledKey, nextEnabled ? '1' : '0');
      await kv.write(lockTimeoutKey, nextTimeout.name);
    } on Object {
      return false;
    }
    _enabled = nextEnabled;
    _timeout = nextTimeout;
    if (!_enabled) {
      _expiry?.cancel();
      _expiry = null;
      _locked = false;
    } else if (!_locked) {
      await _stamp();
      _arm();
    }
    notifyListeners();
    return true;
  }

  @override
  void dispose() {
    _expiry?.cancel();
    super.dispose();
  }
}
