import 'dart:convert';
import 'dart:typed_data';

import 'package:orrery_remote/media/dictation.dart';
import 'package:orrery_remote/media/media_prefs.dart';
import 'package:orrery_remote/remote/protocol.dart';
import 'package:orrery_remote/remote/secure_kv.dart';
import 'package:orrery_remote/ui/viewers/viewer_api.dart';

Uint8List png(int w, int h, {int extra = 0}) {
  final b = BytesBuilder()
    ..add([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13])
    ..add(ascii.encode('IHDR'))
    ..add([w >> 24, w >> 16, w >> 8, w, h >> 24, h >> 16, h >> 8, h])
    ..add(List.filled(extra, 1));
  return b.toBytes();
}

Uint8List jpeg(int w, int h, {int extra = 0}) {
  final b = BytesBuilder()
    ..add([0xFF, 0xD8, 0xFF, 0xE0, 0, 4, 0, 0])
    ..add([0xFF, 0xC4, 0, 4, 0, 0])
    ..add([0xFF, 0xC0, 0, 11, 8, h >> 8, h, w >> 8, w, 1, 1, 0x11, 0])
    ..add(List.filled(extra, 2));
  return b.toBytes();
}

class FakeCall {
  final List<(String, JsonMap?)> calls = [];
  Object? failAt;
  int failOn = -1;
  Map<String, ViewerReply> replies = {};

  Future<ViewerReply> call(
    String type, [
    JsonMap? fields,
    Duration? timeout,
  ]) async {
    calls.add((type, fields));
    if (type == 'fs.put') {
      final seq = fields!['seq'] as int;
      if (seq == failOn && failAt != null) throw failAt!;
      final last = fields['last'] == true;
      return ViewerReply(
        data: last
            ? {
                'ok': true,
                'done': true,
                'path': 'D:\\proj\\.orrery\\inbox\\20260101-000000.jpg',
                'name': '20260101-000000.jpg',
              }
            : {'ok': true, 'done': false, 'next': seq + 1},
      );
    }
    final reply = replies[type];
    if (reply == null) throw const ViewerFailure('denied');
    return reply;
  }

  List<JsonMap> of(String type) => [
    for (final c in calls)
      if (c.$1 == type) c.$2!,
  ];
}

class FakeEngine implements SpeechEngine {
  bool initOk = true;
  bool permission = true;
  bool offline = false;
  bool throwOnListen = false;
  int listens = 0;
  bool lastOnDevice = false;
  void Function(String, bool)? result;
  void Function(String)? status;
  int stops = 0;

  @override
  Future<bool> init({
    required void Function(String error) onError,
    required void Function(String status) onStatus,
  }) async {
    status = onStatus;
    return initOk;
  }

  @override
  Future<bool> hasPermission() async => permission;

  @override
  Future<bool> offlineFor(String localeId) async => offline;

  @override
  Future<void> listen({
    required String localeId,
    required bool onDevice,
    required void Function(String words, bool done) onResult,
  }) async {
    if (throwOnListen) throw StateError('x');
    listens++;
    lastOnDevice = onDevice;
    result = onResult;
  }

  @override
  Future<void> stop() async {
    stops++;
    status?.call('done');
  }

  @override
  Future<void> cancel() async {}
}

MediaPrefs memPrefs() => MediaPrefs(MemorySecureKv());
