import 'dart:convert';
import 'dart:typed_data';

import '../remote/protocol.dart';
import '../ui/viewers/viewer_api.dart';
import 'image_prep.dart';

const int chunkBytes = 384 * 1024;
const Duration putTimeout = Duration(seconds: 20);

typedef PutCall =
    Future<ViewerReply> Function(
      String type, [
      JsonMap? fields,
      Duration? timeout,
    ]);

enum PutError {
  denied,
  offline,
  expired,
  order,
  busy,
  tooLarge,
  image,
  failed,
}

class PutFailure implements Exception {
  const PutFailure(this.error);

  final PutError error;

  @override
  String toString() => 'PutFailure($error)';
}

class PutResult {
  const PutResult(this.path, this.name);

  final String path;
  final String name;
}

List<Uint8List> splitChunks(Uint8List bytes, {int size = chunkBytes}) {
  if (bytes.isEmpty) return const [];
  return [
    for (var at = 0; at < bytes.length; at += size)
      Uint8List.sublistView(
        bytes,
        at,
        at + size < bytes.length ? at + size : bytes.length,
      ),
  ];
}

int _counter = 0;

String newUploadId() {
  _counter = (_counter + 1) & 0xFFFF;
  final t = DateTime.now().microsecondsSinceEpoch.toRadixString(36);
  return 'u$t${_counter.toRadixString(36)}';
}

PutError _fromCode(String code) => switch (code) {
  'denied' => PutError.denied,
  'offline' || 'timeout' => PutError.offline,
  'expired' => PutError.expired,
  'order' => PutError.order,
  'busy' || 'rate' || 'limit' => PutError.busy,
  'size' => PutError.tooLarge,
  'image' => PutError.image,
  _ => PutError.failed,
};

Future<PutResult> putImage(
  PutCall call,
  String root,
  Uint8List bytes, {
  String? up,
}) async {
  if (bytes.isEmpty) throw const PutFailure(PutError.image);
  if (bytes.length > maxImageBytes) throw const PutFailure(PutError.tooLarge);
  final id = up ?? newUploadId();
  final chunks = splitChunks(bytes);
  try {
    for (var seq = 0; seq < chunks.length; seq++) {
      final last = seq == chunks.length - 1;
      final reply = await call('fs.put', {
        'root': root,
        'up': id,
        'seq': seq,
        'last': last,
        'b64': base64Encode(chunks[seq]),
      }, putTimeout);
      if (reply.pending || reply.cancelled) {
        throw const PutFailure(PutError.failed);
      }
      final data = reply.map;
      if (!last) {
        if (data['done'] == true || data['next'] != seq + 1) {
          throw const PutFailure(PutError.failed);
        }
        continue;
      }
      final path = data['path'];
      final name = data['name'];
      if (data['done'] != true || path is! String || name is! String) {
        throw const PutFailure(PutError.failed);
      }
      return PutResult(path, name);
    }
  } on ViewerFailure catch (error) {
    throw PutFailure(_fromCode(error.code));
  } on RemoteCommandException catch (error) {
    throw PutFailure(_fromCode(error.code));
  }
  throw const PutFailure(PutError.failed);
}
