import 'dart:async';

import 'package:flutter/services.dart';

import 'waiting_update.dart';

const String waitingChannelName = 'dev.ccm.orrery.remote/waiting';

abstract class WaitingSink {
  Future<void> start();

  Future<void> apply(WaitingUpdate update, {required DateTime at});

  Future<void> setPausedUntil(DateTime? until);

  Stream<void> get pauseRequests;
}

class NoWaitingSink implements WaitingSink {
  const NoWaitingSink();

  @override
  Future<void> start() async {}

  @override
  Future<void> apply(WaitingUpdate update, {required DateTime at}) async {}

  @override
  Future<void> setPausedUntil(DateTime? until) async {}

  @override
  Stream<void> get pauseRequests => const Stream<void>.empty();
}

class ChannelWaitingSink implements WaitingSink {
  ChannelWaitingSink({MethodChannel? channel})
    : _channel = channel ?? const MethodChannel(waitingChannelName) {
    _channel.setMethodCallHandler((call) async {
      if (call.method == 'pause') _emit();
      return null;
    });
  }

  final MethodChannel _channel;
  bool _held = false;
  late final StreamController<void> _requests = StreamController<void>.broadcast(
    onListen: () {
      if (!_held) return;
      _held = false;
      scheduleMicrotask(() {
        if (_requests.hasListener) _requests.add(null);
      });
    },
  );

  void _emit() {
    if (_requests.hasListener) {
      _requests.add(null);
    } else {
      _held = true;
    }
  }

  @override
  Stream<void> get pauseRequests => _requests.stream;

  @override
  Future<void> start() async {
    try {
      final pending = await _channel.invokeMethod<bool>('ready');
      if (pending == true) _emit();
    } on MissingPluginException {
      return;
    } on PlatformException {
      return;
    }
  }

  @override
  Future<void> apply(WaitingUpdate update, {required DateTime at}) =>
      _call('apply', {
        'count': update.count,
        'labels': update.labels,
        'at': at.millisecondsSinceEpoch,
      });

  @override
  Future<void> setPausedUntil(DateTime? until) =>
      _call('setPausedUntil', {'until': until?.millisecondsSinceEpoch ?? 0});

  Future<void> _call(String method, Map<String, Object?> args) async {
    try {
      await _channel.invokeMethod<void>(method, args);
    } on MissingPluginException {
      return;
    } on PlatformException {
      return;
    }
  }
}
