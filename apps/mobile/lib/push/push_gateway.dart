import 'dart:async';

import 'package:firebase_core/firebase_core.dart';
import 'package:firebase_messaging/firebase_messaging.dart';

import 'push_config.dart';

enum PushPermission { granted, denied }

class PushTokenChange {
  const PushTokenChange(this.projectId, this.token);

  final String projectId;
  final String token;
}

abstract class PushGateway {
  Future<PushPermission> requestPermission(PushConfig config);

  Future<String?> token(PushConfig config);

  Future<void> deleteToken(PushConfig config);

  Stream<PushTokenChange> get tokenRefresh;

  Stream<Map<String, String>> get opened;

  Future<Map<String, String>?> initialOpen(Iterable<PushConfig> configs);
}

class FirebasePushGateway implements PushGateway {
  FirebasePushGateway();

  final StreamController<PushTokenChange> _refresh =
      StreamController.broadcast();
  FirebaseMessaging? _active;
  String? _project;
  Future<void>? _opening;

  static Map<String, String> _data(RemoteMessage message) =>
      message.data.map((key, value) => MapEntry(key, '$value'));

  Future<FirebaseMessaging?> _for(PushConfig config) async {
    if (_project == null) {
      final running = _opening;
      if (running != null) {
        await running;
      } else {
        final job = _open(config);
        _opening = job;
        try {
          await job;
        } finally {
          _opening = null;
        }
      }
    }
    return _project == config.projectId ? _active : null;
  }

  Future<void> _open(PushConfig config) async {
    try {
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
      if (app.options.projectId != config.projectId) return;
      final messaging = FirebaseMessaging.instance;
      final dynamic stream = messaging;
      (stream.onTokenRefresh as Stream<String>).listen(
        (token) => _refresh.add(PushTokenChange(config.projectId, token)),
        onError: (Object _) {},
      );
      _active = messaging;
      _project = config.projectId;
    } on Object {
      return;
    }
  }

  @override
  Future<PushPermission> requestPermission(PushConfig config) async {
    final messaging = await _for(config);
    if (messaging == null) return PushPermission.denied;
    try {
      final status = (await messaging.requestPermission()).authorizationStatus;
      return status == AuthorizationStatus.authorized ||
              status == AuthorizationStatus.provisional
          ? PushPermission.granted
          : PushPermission.denied;
    } on Object {
      return PushPermission.denied;
    }
  }

  @override
  Future<String?> token(PushConfig config) async {
    final messaging = await _for(config);
    if (messaging == null) return null;
    try {
      final dynamic dynamicMessaging = messaging;
      return await dynamicMessaging.getToken() as String?;
    } on Object {
      return null;
    }
  }

  @override
  Future<void> deleteToken(PushConfig config) async {
    final messaging = await _for(config);
    if (messaging == null) return;
    try {
      final dynamic dynamicMessaging = messaging;
      await dynamicMessaging.deleteToken();
    } on Object {
      return;
    }
  }

  @override
  Stream<PushTokenChange> get tokenRefresh => _refresh.stream;

  @override
  Stream<Map<String, String>> get opened =>
      FirebaseMessaging.onMessageOpenedApp.map(_data);

  @override
  Future<Map<String, String>?> initialOpen(Iterable<PushConfig> configs) async {
    for (final config in configs) {
      final messaging = await _for(config);
      if (messaging == null) continue;
      try {
        final message = await messaging.getInitialMessage();
        if (message != null) return _data(message);
      } on Object {
        continue;
      }
    }
    return null;
  }
}
