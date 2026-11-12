import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'secure_kv.dart';

class RelayToken {
  const RelayToken(this.token, this.exp);

  final String token;
  final int exp;

  bool expired(int nowSeconds) => nowSeconds >= exp;

  static RelayToken? parse(String token) {
    try {
      final padded = token.padRight((token.length + 3) ~/ 4 * 4, '=');
      final bytes = base64Url.decode(padded);
      if (bytes.length < 41) return null;
      final view = ByteData.sublistView(
        Uint8List.fromList(bytes.sublist(bytes.length - 40, bytes.length - 32)),
      );
      return RelayToken(token, view.getUint64(0));
    } on FormatException {
      return null;
    }
  }
}

const int maxMachineAlias = 40;

String? cleanAlias(String? raw) {
  if (raw == null) return null;
  final kept = String.fromCharCodes(
    raw.runes.where(
      (r) =>
          r >= 0x20 &&
          !(r >= 0x7f && r <= 0x9f) &&
          r != 0xad &&
          !(r >= 0x200b && r <= 0x200f) &&
          !(r >= 0x2028 && r <= 0x202e) &&
          !(r >= 0x2060 && r <= 0x206f) &&
          r != 0xfeff,
    ),
  );
  final trimmed = kept.trim();
  if (trimmed.isEmpty) return null;
  final cut = trimmed.runes.take(maxMachineAlias).toList();
  final text = String.fromCharCodes(cut).trim();
  return text.isEmpty ? null : text;
}

class Machine {
  const Machine({
    required this.id,
    required this.name,
    required this.mkey,
    required this.pairedAt,
    this.lastHost,
    this.lastPort,
    this.relayUrl,
    this.relayToken,
    this.revoked = false,
    this.alias,
  });

  final String id;
  final String name;
  final String? alias;
  final Uint8List mkey;
  final int pairedAt;
  final String? lastHost;
  final int? lastPort;
  final String? relayUrl;
  final RelayToken? relayToken;
  final bool revoked;

  String get displayName => alias ?? name;

  Machine copyWith({
    String? name,
    String? lastHost,
    int? lastPort,
    String? relayUrl,
    RelayToken? relayToken,
    bool? revoked,
  }) => Machine(
    id: id,
    name: name ?? this.name,
    mkey: mkey,
    pairedAt: pairedAt,
    lastHost: lastHost ?? this.lastHost,
    lastPort: lastPort ?? this.lastPort,
    relayUrl: relayUrl ?? this.relayUrl,
    relayToken: relayToken ?? this.relayToken,
    revoked: revoked ?? this.revoked,
    alias: alias,
  );

  Machine withAlias(String? next) => Machine(
    id: id,
    name: name,
    mkey: mkey,
    pairedAt: pairedAt,
    lastHost: lastHost,
    lastPort: lastPort,
    relayUrl: relayUrl,
    relayToken: relayToken,
    revoked: revoked,
    alias: next,
  );

  Map<String, Object?> toPublicJson() => {
    'id': id,
    'name': name,
    'mkey': base64Url.encode(mkey),
    'pairedAt': pairedAt,
    'lastHost': lastHost,
    'lastPort': lastPort,
    'relayUrl': relayUrl,
    'revoked': revoked,
    if (alias != null) 'alias': alias,
  };

  static Machine fromPublicJson(Map<String, dynamic> json, RelayToken? token) =>
      Machine(
        id: json['id'] as String,
        name: json['name'] as String,
        mkey: Uint8List.fromList(base64Url.decode(json['mkey'] as String)),
        pairedAt: (json['pairedAt'] as num).toInt(),
        lastHost: json['lastHost'] as String?,
        lastPort: (json['lastPort'] as num?)?.toInt(),
        relayUrl: json['relayUrl'] as String?,
        relayToken: token,
        revoked: json['revoked'] == true,
        alias: cleanAlias(json['alias'] as String?),
      );
}

class MachineRepository {
  MachineRepository({this.directory, required this._secure});

  final Directory? directory;
  final SecureKv _secure;
  final Map<String, Machine> _machines = {};
  bool _loaded = false;

  File get _file =>
      File('${directory!.path}${Platform.pathSeparator}machines.json');

  String _tokenKey(String id) => 'remote.relay_token.$id';

  Future<List<Machine>> load() async {
    if (_loaded) return all;
    _machines.clear();
    if (directory != null && await _file.exists()) {
      try {
        final raw = jsonDecode(await _file.readAsString());
        if (raw is List) {
          for (final entry in raw.whereType<Map<String, dynamic>>()) {
            final id = entry['id'];
            if (id is! String) continue;
            final tokenText = await _secure.read(_tokenKey(id));
            final machine = Machine.fromPublicJson(
              entry,
              tokenText == null ? null : RelayToken.parse(tokenText),
            );
            _machines[id] = machine;
          }
        }
      } on FormatException {
        _machines.clear();
      }
    }
    _loaded = true;
    return all;
  }

  List<Machine> get all => List.unmodifiable(_machines.values);

  Machine? get(String id) => _machines[id];

  Future<void> save(Machine machine) async {
    await load();
    final existing = _machines[machine.id];
    _machines[machine.id] = existing == null
        ? machine
        : machine.withAlias(existing.alias);
    final token = machine.relayToken;
    if (token != null) {
      await _secure.write(_tokenKey(machine.id), token.token);
    }
    await _persist();
  }

  Future<void> rename(String id, String? raw) async {
    await load();
    final machine = _machines[id];
    if (machine == null) return;
    _machines[id] = machine.withAlias(cleanAlias(raw));
    await _persist();
  }

  Future<void> remove(String id) async {
    await load();
    _machines.remove(id);
    await _secure.delete(_tokenKey(id));
    await _persist();
  }

  Future<void> _persist() async {
    final dir = directory;
    if (dir == null) return;
    await dir.create(recursive: true);
    final text = jsonEncode(
      _machines.values.map((m) => m.toPublicJson()).toList(),
    );
    final temp = File('${_file.path}.tmp');
    await temp.writeAsString(text, flush: true);
    await temp.rename(_file.path);
  }
}
