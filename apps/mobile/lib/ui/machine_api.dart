import 'dart:typed_data';

import '../remote/protocol.dart';
import '../remote/pty_stream.dart';
import '../remote/remote_host.dart';

abstract class PtyHandle {
  Stream<Uint8List> get output;
  Stream<JsonMap> get events;
  Uint8List get tail;
}

abstract class MachineApi {
  Stream<JsonMap> get events;
  Future<JsonMap> command(String type, [JsonMap? fields, Duration? timeout]);
  Future<PtyHandle> attach(
    String sessionId,
    int cols,
    int rows, {
    required String paneKey,
  });
  Future<void> detach(PtyHandle handle);
  void input(PtyHandle handle, Uint8List bytes, {bool claim = false});
  Future<void> resize(PtyHandle handle, int cols, int rows);
  Uint8List? cachedTail(String paneKey);
  Future<void> answerConfirm(String cid, bool ok);
  Future<void> retry();
  Future<void> clearCache();
}

class _RealHandle implements PtyHandle {
  _RealHandle(this.attachment);

  final PtyAttachment attachment;

  @override
  Stream<Uint8List> get output => attachment.output;

  @override
  Stream<JsonMap> get events => attachment.events;

  @override
  Uint8List get tail => attachment.tail;
}

class BundleMachineApi implements MachineApi {
  BundleMachineApi(this.host, this.machineId);

  final RemoteHost host;
  final String machineId;

  MachineBundle get _bundle => host.bundle(machineId);

  @override
  Stream<JsonMap> get events => _bundle.connection.events;

  @override
  Future<JsonMap> command(
    String type, [
    JsonMap? fields,
    Duration? timeout,
  ]) {
    final session = _bundle.connection.session;
    if (session == null) {
      return Future.error(const RemoteCommandException('offline'));
    }
    return session.request(type, fields, timeout);
  }

  @override
  Future<PtyHandle> attach(
    String sessionId,
    int cols,
    int rows, {
    required String paneKey,
  }) async {
    final attachment = await _bundle.pty.attach(
      sessionId,
      cols,
      rows,
      paneKey: paneKey,
      initialTail: _bundle.store.cachedTails[paneKey],
    );
    return _RealHandle(attachment);
  }

  @override
  Future<void> detach(PtyHandle handle) {
    if (handle is! _RealHandle) return Future.value();
    return _bundle.pty.detach(handle.attachment);
  }

  @override
  void input(PtyHandle handle, Uint8List bytes, {bool claim = false}) {
    if (handle is _RealHandle) {
      _bundle.pty.input(handle.attachment, bytes, claim: claim);
    }
  }

  @override
  Future<void> resize(PtyHandle handle, int cols, int rows) {
    if (handle is! _RealHandle) return Future.value();
    return _bundle.pty.resize(handle.attachment, cols, rows);
  }

  @override
  Uint8List? cachedTail(String paneKey) => _bundle.store.cachedTails[paneKey];

  @override
  Future<void> answerConfirm(String cid, bool ok) =>
      _bundle.store.answerConfirm(cid, ok);

  @override
  Future<void> retry() => _bundle.connection.goForeground();

  @override
  Future<void> clearCache() => host.cache.delete(machineId);
}
