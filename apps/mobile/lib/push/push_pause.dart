import 'dart:async';

import 'package:clock/clock.dart';

import '../remote/secure_kv.dart';
import 'waiting_sink.dart';

const String pushPausedKey = 'push.pausedUntil';
const Duration pushPauseLength = Duration(hours: 1);

Future<DateTime?> readPausedUntil(SecureKv kv, DateTime now) async {
  try {
    final raw = await kv.read(pushPausedKey);
    final ms = raw == null ? null : int.tryParse(raw);
    if (ms == null) return null;
    final until = DateTime.fromMillisecondsSinceEpoch(ms);
    return until.isAfter(now) ? until : null;
  } on Object {
    return null;
  }
}

class PushPause {
  PushPause({
    required this.kv,
    required this.sink,
    required this.onChange,
    DateTime Function()? now,
  }) : _now = now ?? (() => clock.now());

  final SecureKv kv;
  final WaitingSink sink;
  final void Function() onChange;
  final DateTime Function() _now;
  DateTime? _until;
  Timer? _timer;

  DateTime? get until => _until;

  bool get active => _until != null;

  Future<void> load() async {
    _until = await readPausedUntil(kv, _now());
    if (_until == null) {
      await kv.delete(pushPausedKey);
    } else {
      _schedule();
    }
    await sink.setPausedUntil(_until);
  }

  Future<bool> check() async {
    final until = _until;
    if (until != null && !until.isAfter(_now())) {
      await _end();
      onChange();
    }
    return _until != null;
  }

  Future<void> start() async {
    final until = _now().add(pushPauseLength);
    _until = until;
    await kv.write(pushPausedKey, '${until.millisecondsSinceEpoch}');
    _schedule();
    await sink.setPausedUntil(until);
    onChange();
  }

  Future<void> stop() async {
    if (_until == null) return;
    await _end();
    onChange();
  }

  Future<void> _end() async {
    _until = null;
    _timer?.cancel();
    _timer = null;
    await kv.delete(pushPausedKey);
    await sink.setPausedUntil(null);
  }

  void _schedule() {
    _timer?.cancel();
    final until = _until;
    if (until == null) return;
    _timer = Timer(until.difference(_now()), () => unawaited(check()));
  }

  void dispose() {
    _timer?.cancel();
    _timer = null;
  }
}
