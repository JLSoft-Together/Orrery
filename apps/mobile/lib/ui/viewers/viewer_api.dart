import 'dart:async';

import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../remote/protocol.dart';
import '../machine_api.dart';
import '../ui_providers.dart';

class ViewerFailure implements Exception {
  const ViewerFailure(this.code, [this.detail]);

  final String code;
  final String? detail;

  @override
  String toString() => 'ViewerFailure($code)';
}

class ViewerReply {
  const ViewerReply({this.data, this.pending = false, this.cancelled = false});

  final Object? data;
  final bool pending;
  final bool cancelled;

  bool get done => !pending && !cancelled;

  JsonMap get map =>
      data is Map ? (data! as Map).cast<String, dynamic>() : const {};

  List<dynamic> get list => data is List ? data! as List : const [];
}

const Duration slowCommand = Duration(seconds: 70);

class ViewerApi {
  ViewerApi(this.api) {
    _sub = api.events.listen((event) {
      if (event['t'] != 'hello') return;
      for (final key in _watches.keys.toList()) {
        _send(key);
      }
    });
  }

  final MachineApi api;
  late final StreamSubscription<JsonMap> _sub;

  void dispose() => _sub.cancel();
  final Map<String, int> _watches = {};
  final Map<String, String> _roots = {};

  Stream<JsonMap> get events => api.events;

  void _send(String key, {bool off = false}) {
    final root = _roots[key];
    if (root == null) return;
    final kind = key.substring(0, key.indexOf('|'));
    api
        .command('$kind.${off ? 'unwatch' : 'watch'}', {'root': root})
        .then((_) {}, onError: (_) {});
  }

  void watch(String kind, String root) {
    final key = '$kind|${normPath(root)}';
    final count = (_watches[key] ?? 0) + 1;
    _watches[key] = count;
    _roots[key] = root;
    if (count == 1) _send(key);
  }

  void unwatch(String kind, String root) {
    final key = '$kind|${normPath(root)}';
    final count = (_watches[key] ?? 0) - 1;
    if (count > 0) {
      _watches[key] = count;
      return;
    }
    if (_watches.remove(key) != null) _send(key, off: true);
  }

  Future<ViewerReply> call(
    String type, [
    JsonMap? fields,
    Duration? timeout,
  ]) async {
    final reply = await api.command(type, fields, timeout);
    if (reply['cancelled'] == true) return const ViewerReply(cancelled: true);
    if (reply['confirm'] != null) return const ViewerReply(pending: true);
    final data = reply['data'];
    if (data == null) throw const ViewerFailure('denied');
    if (data is Map && data['ok'] == false) {
      final error = data['error'];
      final detail = data['detail'];
      throw ViewerFailure(
        error is String ? error : 'failed',
        detail is String ? detail : null,
      );
    }
    return ViewerReply(data: data);
  }

  Future<JsonMap> map(String type, [JsonMap? fields]) async =>
      (await call(type, fields)).map;
}

final viewerApiProvider = Provider.family<ViewerApi, String>(
  (ref, id) {
    final api = ViewerApi(ref.read(machineApiProvider(id)));
    ref.onDispose(api.dispose);
    return api;
  },
);

bool changedUnder(JsonMap event, String type, String root) {
  if (event['t'] != type) return false;
  final eventRoot = event['root'];
  return eventRoot is! String || samePath(eventRoot, root);
}

String normPath(String p) =>
    p.replaceAll('\\', '/').replaceAll(RegExp(r'/+$'), '').toLowerCase();

bool samePath(String a, String b) => normPath(a) == normPath(b);
