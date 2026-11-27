import 'dart:convert';
import 'dart:typed_data';

import 'package:cryptography/cryptography.dart';
import 'package:cryptography/dart.dart';
import 'package:orrery_remote/push/push_seal.dart';

class SealRig {
  SealRig._(this.devicePrivate, this.devicePublic, this.desktop, this.machine);

  final Uint8List devicePrivate;
  final List<int> devicePublic;
  final SimpleKeyPair desktop;
  final String machine;

  static Future<SealRig> create({String machine = 'M1'}) async {
    final x = const DartX25519();
    final seed = Uint8List.fromList(List.generate(32, (i) => i + 7));
    final device = await x.newKeyPairFromSeed(seed);
    final desk = await x.newKeyPairFromSeed(
      Uint8List.fromList(List.generate(32, (i) => 90 - i)),
    );
    return SealRig._(
      seed,
      (await device.extractPublicKey()).bytes,
      desk,
      machine,
    );
  }

  Future<Uint8List> get desktopPublic async =>
      Uint8List.fromList((await desktop.extractPublicKey()).bytes);

  Future<Uint8List?> machineKey(String id) async =>
      id == machine ? desktopPublic : null;

  Future<Map<String, String>> push(
    String kind,
    Map<String, Object?> plain, {
    Map<String, String> extra = const {},
  }) async {
    final x = const DartX25519();
    final eph = await x.newKeyPairFromSeed(
      Uint8List.fromList(List.generate(32, (i) => 200 - i)),
    );
    final ephPub = (await eph.extractPublicKey()).bytes;
    final peer = SimplePublicKey(devicePublic, type: KeyPairType.x25519);
    final shared = await (await x.sharedSecretKey(
      keyPair: eph,
      remotePublicKey: peer,
    )).extractBytes();
    final fixed = await (await x.sharedSecretKey(
      keyPair: desktop,
      remotePublicKey: peer,
    )).extractBytes();
    final okm =
        await DartHkdf(
          hmac: DartHmac(const DartSha256()),
          outputLength: 44,
        ).deriveKey(
          secretKey: SecretKey([...shared, ...fixed]),
          nonce: [...ephPub, ...devicePublic, ...(await desktopPublic)],
          info: pushSealInfo(machine),
        );
    final bytes = await okm.extractBytes();
    final box = await const DartChacha20.poly1305Aead().encrypt(
      utf8.encode(jsonEncode(plain)),
      secretKey: SecretKey(bytes.sublist(0, 32)),
      nonce: bytes.sublist(32),
      aad: pushSealAad(machine, kind),
    );
    final sealed = base64Url
        .encode([...ephPub, ...box.cipherText, ...box.mac.bytes])
        .replaceAll('=', '');
    return {'v': '2', 'm': machine, 'k': kind, 'e': sealed, ...extra};
  }
}
