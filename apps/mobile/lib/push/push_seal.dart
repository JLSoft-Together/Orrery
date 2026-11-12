import 'dart:convert';
import 'dart:typed_data';

import 'package:cryptography/cryptography.dart';
import 'package:cryptography/dart.dart';

import '../remote/keys.dart';
import '../remote/machines.dart';
import '../remote/secure_kv.dart';

const String pushSealVersion = '2';
const int pushSealLevel = 2;
const String pushSealDomain = 'orrery-push/2';
const int pushSealMax = 4096;
const int _keyLen = 32;
const int _tagLen = 16;
const int _aeadKeyLen = 32;
final RegExp pushPanePattern = RegExp(r'^[A-Za-z0-9_.:-]{1,128}$');

class SealedNotice {
  const SealedNotice({required this.title, required this.body, this.pane});

  final String title;
  final String body;
  final String? pane;
}

List<int> pushSealInfo(String machine) => [
  ...utf8.encode(pushSealDomain),
  0,
  ...utf8.encode(machine),
];

List<int> pushSealAad(String machine, String kind) => [
  ...pushSealInfo(machine),
  0,
  ...utf8.encode(kind),
];

Future<Uint8List?> readStaticKey(SecureKv kv) async {
  final raw = await kv.read(KeyStore.keyName);
  if (raw == null) return null;
  try {
    final bytes = base64Url.decode(base64Url.normalize(raw));
    return bytes.length == _keyLen ? bytes : null;
  } on FormatException {
    return null;
  }
}

Future<Uint8List?> pairedMachineKey(
  MachineRepository machines,
  String machineId,
) async {
  await machines.load();
  final machine = machines.get(machineId);
  if (machine == null || machine.revoked) return null;
  return machine.mkey.length == _keyLen ? machine.mkey : null;
}

Future<List<int>> _agree(SimpleKeyPair pair, List<int> peer) async =>
    (await const DartX25519().sharedSecretKey(
      keyPair: pair,
      remotePublicKey: SimplePublicKey(peer, type: KeyPairType.x25519),
    )).extractBytes();

Future<List<int>?> pushSealOkm({
  required List<int> privateKey,
  required List<int> ephemeral,
  required List<int> desktop,
  required String machine,
}) async {
  if (desktop.length != _keyLen) return null;
  final pair = await const DartX25519().newKeyPairFromSeed(privateKey);
  final mine = (await pair.extractPublicKey()).bytes;
  final shared = await _agree(pair, ephemeral);
  final fixed = await _agree(pair, desktop);
  if (shared.every((b) => b == 0) || fixed.every((b) => b == 0)) return null;
  final okm =
      await DartHkdf(
        hmac: DartHmac(const DartSha256()),
        outputLength: _aeadKeyLen + 12,
      ).deriveKey(
        secretKey: SecretKey([...shared, ...fixed]),
        nonce: [...ephemeral, ...mine, ...desktop],
        info: pushSealInfo(machine),
      );
  return okm.extractBytes();
}

Future<SealedNotice?> openSeal({
  required List<int> privateKey,
  required List<int> desktop,
  required String machine,
  required String kind,
  required String sealed,
}) async {
  try {
    if (privateKey.length != _keyLen || sealed.length > pushSealMax) {
      return null;
    }
    final raw = base64Url.decode(base64Url.normalize(sealed));
    if (raw.length < _keyLen + _tagLen) return null;
    final ephemeral = raw.sublist(0, _keyLen);
    final okm = await pushSealOkm(
      privateKey: privateKey,
      ephemeral: ephemeral,
      desktop: desktop,
      machine: machine,
    );
    if (okm == null) return null;
    final body = raw.sublist(_keyLen);
    final plain = await const DartChacha20.poly1305Aead().decrypt(
      SecretBox(
        body.sublist(0, body.length - _tagLen),
        nonce: okm.sublist(_aeadKeyLen),
        mac: Mac(body.sublist(body.length - _tagLen)),
      ),
      secretKey: SecretKey(okm.sublist(0, _aeadKeyLen)),
      aad: pushSealAad(machine, kind),
    );
    final value = jsonDecode(utf8.decode(plain));
    if (value is! Map<String, dynamic>) return null;
    final title = value['t'];
    final text = value['b'];
    final pane = value['p'];
    if (title is! String || text is! String || title.isEmpty) return null;
    if (pane != null && (pane is! String || !pushPanePattern.hasMatch(pane))) {
      return null;
    }
    return SealedNotice(title: title, body: text, pane: pane as String?);
  } on Object {
    return null;
  }
}
