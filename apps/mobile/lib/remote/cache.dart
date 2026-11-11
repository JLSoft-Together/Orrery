import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

class CacheEntry {
  const CacheEntry({
    required this.savedAt,
    required this.seq,
    required this.wsRev,
    this.snapshot,
    this.workspace,
    this.hello,
    this.tails = const {},
  });

  final DateTime savedAt;
  final int seq;
  final int wsRev;
  final Map<String, dynamic>? snapshot;
  final Map<String, dynamic>? workspace;
  final Map<String, dynamic>? hello;
  final Map<String, Uint8List> tails;

  Map<String, Object?> toJson() => {
    'savedAt': savedAt.millisecondsSinceEpoch,
    'seq': seq,
    'wsRev': wsRev,
    'snapshot': snapshot,
    'workspace': workspace,
    'hello': hello,
    'tails': {
      for (final entry in tails.entries) entry.key: base64.encode(entry.value),
    },
  };

  static CacheEntry? fromJson(Object? raw) {
    if (raw is! Map) return null;
    final tails = <String, Uint8List>{};
    final rawTails = raw['tails'];
    if (rawTails is Map) {
      for (final entry in rawTails.entries) {
        final value = entry.value;
        if (entry.key is String && value is String) {
          try {
            tails[entry.key as String] = base64.decode(value);
          } on FormatException {
            continue;
          }
        }
      }
    }
    Map<String, dynamic>? map(Object? value) =>
        value is Map ? value.cast<String, dynamic>() : null;
    final savedAt = raw['savedAt'];
    if (savedAt is! num) return null;
    return CacheEntry(
      savedAt: DateTime.fromMillisecondsSinceEpoch(savedAt.toInt()),
      seq: (raw['seq'] as num?)?.toInt() ?? 0,
      wsRev: (raw['wsRev'] as num?)?.toInt() ?? 0,
      snapshot: map(raw['snapshot']),
      workspace: map(raw['workspace']),
      hello: map(raw['hello']),
      tails: tails,
    );
  }
}

class MachineCache {
  MachineCache(this.directory);

  final Directory directory;

  File _file(String machineId) {
    final safe = machineId.replaceAll(RegExp(r'[^A-Za-z0-9_-]'), '_');
    return File('${directory.path}${Platform.pathSeparator}$safe.json');
  }

  final Map<String, Future<void>> _queue = {};

  Future<void> save(String machineId, CacheEntry entry) {
    final previous = _queue[machineId] ?? Future<void>.value();
    final next = previous
        .catchError((Object _) {})
        .then((_) => _write(machineId, entry));
    _queue[machineId] = next;
    return next;
  }

  Future<void> _write(String machineId, CacheEntry entry) async {
    try {
      await directory.create(recursive: true);
      final file = _file(machineId);
      final temp = File('${file.path}.tmp');
      await temp.writeAsString(jsonEncode(entry.toJson()), flush: true);
      await temp.rename(file.path);
    } on Object {
      return;
    }
  }

  Future<CacheEntry?> load(String machineId) async {
    final file = _file(machineId);
    if (!await file.exists()) return null;
    try {
      return CacheEntry.fromJson(jsonDecode(await file.readAsString()));
    } on FormatException {
      return null;
    } on TypeError {
      return null;
    }
  }

  Future<void> delete(String machineId) async {
    await _queue[machineId]?.catchError((Object _) {});
    final file = _file(machineId);
    if (await file.exists()) await file.delete();
  }
}
