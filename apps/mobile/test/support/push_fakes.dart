import 'dart:async';
import 'dart:typed_data';

import 'package:orrery_remote/push/push_config.dart';
import 'package:orrery_remote/push/push_gateway.dart';
import 'package:orrery_remote/push/push_notice.dart';
import 'package:orrery_remote/push/push_service.dart';
import 'package:orrery_remote/remote/protocol.dart';
import 'package:orrery_remote/remote/secure_kv.dart';

const pushConfigA = PushConfig(
  apiKey: 'key-a',
  appId: '1:1:android:a',
  senderId: '111',
  projectId: 'proj-a',
);
const pushConfigB = PushConfig(
  apiKey: 'key-b',
  appId: '1:2:android:b',
  senderId: '222',
  projectId: 'proj-b',
);

class FakeGateway implements PushGateway {
  PushPermission current = PushPermission.granted;
  PushPermission asked = PushPermission.granted;
  final Map<String, String?> tokens = {
    'proj-a': 'token-a-1',
    'proj-b': 'token-b-1',
  };
  final List<String> deleted = [];
  final List<String> opened_ = [];
  int permissionAsks = 0;
  bool openFails = false;
  bool disposed = false;
  String? project;
  Map<String, String>? initial;
  final StreamController<PushTokenChange> refresh =
      StreamController.broadcast();
  final StreamController<Map<String, String>> open_ =
      StreamController.broadcast();
  final StreamController<PushArrival> arrive = StreamController.broadcast();
  final List<PushNotice> shown = [];
  @override
  bool sealing = true;
  bool showFails = false;

  @override
  String? get activeProject => project;

  @override
  Future<bool> open(PushConfig config) async {
    if (openFails) throw const PushException('boom');
    opened_.add(config.projectId);
    project ??= config.projectId;
    return project == config.projectId;
  }

  @override
  Future<PushPermission> permission() async => current;

  @override
  Future<PushPermission> requestPermission() async {
    permissionAsks++;
    return asked;
  }

  @override
  Future<String?> token() async => tokens[project];

  @override
  Future<void> deleteToken() async {
    deleted.add(project!);
  }

  @override
  Stream<PushTokenChange> get tokenRefresh => refresh.stream;

  @override
  Stream<Map<String, String>> get opened => open_.stream;

  @override
  Future<Map<String, String>?> initialOpen() async => initial;

  @override
  Stream<PushArrival> get arrivals => arrive.stream;

  @override
  Future<void> show(PushNotice notice, {required String channelName}) async {
    if (showFails) throw const PushException('show');
    shown.add(notice);
  }

  @override
  Future<void> dispose() async => disposed = true;
}

class FakeSession implements PushSession {
  FakeSession(this.machineId);

  @override
  final String machineId;
  final List<(String, Map<String, Object?>)> sent = [];
  bool failing = false;

  @override
  Future<void> send(String type, Map<String, Object?> fields) async {
    if (failing) throw const RemoteCommandException('offline');
    sent.add((type, fields));
  }
}

class FakeHost implements PushHost {
  final StreamController<PushSession> ready = StreamController.broadcast();
  final StreamController<PushConfigUpdate> config =
      StreamController.broadcast();
  final List<PushSession> now = [];

  @override
  Stream<PushSession> get readySessions => ready.stream;

  @override
  List<PushSession> get readyNow => now;

  @override
  Stream<PushConfigUpdate> get configs => config.stream;

  bool disposed = false;

  @override
  Future<void> dispose() async => disposed = true;

  void connect(FakeSession s) {
    now.add(s);
    ready.add(s);
  }
}

Future<void> settle() => Future<void>.delayed(Duration.zero);

class Rig {
  Rig({String locale = 'vi'}) {
    service = PushService(
      gateway: gateway,
      kv: kv,
      host: host,
      platform: 'android',
      localeCode: () => locale,
      resumed: () => resumed,
      machineKey: (id) async => machineKeys[id],
    );
  }

  final FakeGateway gateway = FakeGateway();
  final FakeHost host = FakeHost();
  final MemorySecureKv kv = MemorySecureKv();
  late final PushService service;
  bool resumed = true;
  final Map<String, Uint8List> machineKeys = {};
}
