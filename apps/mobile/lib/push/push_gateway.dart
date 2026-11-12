import 'dart:async';

import 'package:firebase_core/firebase_core.dart';
import 'package:firebase_messaging/firebase_messaging.dart';
import 'package:flutter/services.dart';

import 'push_config.dart';

enum PushPermission { granted, denied }

class PushException implements Exception {
  const PushException(this.code);

  final String code;

  @override
  String toString() => 'PushException($code)';
}

class PushTokenChange {
  const PushTokenChange(this.projectId, this.token);

  final String projectId;
  final String token;
}

abstract class PushGateway {
  String? get activeProject;

  Future<bool> open(PushConfig config);

  Future<PushPermission> permission();

  Future<PushPermission> requestPermission();

  Future<String?> token();

  Future<void> deleteToken();

  Stream<PushTokenChange> get tokenRefresh;

  Stream<Map<String, String>> get opened;

  Future<Map<String, String>?> initialOpen();

  Future<void> dispose();
}

class FirebasePushGateway implements PushGateway {
  FirebasePushGateway();

  final StreamController<PushTokenChange> _refresh =
      StreamController.broadcast();
  final StreamController<Map<String, String>> _opened =
      StreamController.broadcast();
  final List<StreamSubscription<Object?>> _subs = [];
  FirebaseMessaging? _active;
  String? _project;
  Future<void>? _opening;

  static Map<String, String> _data(RemoteMessage message) =>
      message.data.map((key, value) => MapEntry(key, '$value'));

  static PushPermission _status(AuthorizationStatus status) =>
      status == AuthorizationStatus.authorized ||
          status == AuthorizationStatus.provisional
      ? PushPermission.granted
      : PushPermission.denied;

  FirebaseMessaging get _messaging {
    final active = _active;
    if (active == null) throw const PushException('not-open');
    return active;
  }

  Future<T> _guard<T>(Future<T> Function() run) async {
    try {
      return await run();
    } on FirebaseException catch (error) {
      throw PushException(error.code);
    } on PlatformException catch (error) {
      throw PushException(error.code);
    }
  }

  @override
  String? get activeProject => _project;

  @override
  Future<bool> open(PushConfig config) async {
    if (_project == null) {
      final running = _opening;
      if (running != null) {
        await running;
      } else {
        final job = _init(config);
        _opening = job;
        try {
          await job;
        } finally {
          _opening = null;
        }
      }
    }
    return _project == config.projectId;
  }

  Future<void> _init(PushConfig config) => _guard(() async {
    final app = Firebase.apps.isNotEmpty
        ? Firebase.app()
        : await Firebase.initializeApp(
            options: FirebaseOptions(
              apiKey: config.apiKey,
              appId: config.appId,
              messagingSenderId: config.senderId,
              projectId: config.projectId,
            ),
          );
    final messaging = FirebaseMessaging.instance;
    final project = app.options.projectId;
    _subs
      ..add(
        messaging.onTokenRefresh.listen(
          (token) => _refresh.add(PushTokenChange(project, token)),
          onError: (Object _) {},
        ),
      )
      ..add(
        FirebaseMessaging.onMessageOpenedApp.listen(
          (message) => _opened.add(_data(message)),
          onError: (Object _) {},
        ),
      );
    _active = messaging;
    _project = project;
  });

  @override
  Future<PushPermission> permission() => _guard(
    () async => _status(
      (await _messaging.getNotificationSettings()).authorizationStatus,
    ),
  );

  @override
  Future<PushPermission> requestPermission() => _guard(
    () async =>
        _status((await _messaging.requestPermission()).authorizationStatus),
  );

  @override
  Future<String?> token() => _guard(() => _messaging.getToken());

  @override
  Future<void> deleteToken() => _guard(() => _messaging.deleteToken());

  @override
  Stream<PushTokenChange> get tokenRefresh => _refresh.stream;

  @override
  Stream<Map<String, String>> get opened => _opened.stream;

  @override
  Future<Map<String, String>?> initialOpen() => _guard(() async {
    final message = await _messaging.getInitialMessage();
    return message == null ? null : _data(message);
  });

  @override
  Future<void> dispose() async {
    for (final sub in _subs) {
      await sub.cancel();
    }
    _subs.clear();
    await _refresh.close();
    await _opened.close();
  }
}
