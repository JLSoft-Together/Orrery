import 'dart:convert';
import 'dart:math';
import 'dart:typed_data';

import 'crypto.dart';
import 'secure_kv.dart';

class PhoneIdentity {
  const PhoneIdentity({required this.privateKey, required this.deviceId});

  final Uint8List privateKey;
  final String deviceId;
}

class KeyStore {
  KeyStore(this._kv, this._crypto, {Random? random})
    : _random = random ?? Random.secure();

  static const String keyName = 'remote.static_key';
  static const String deviceIdName = 'remote.device_id';

  final SecureKv _kv;
  final RemoteCrypto _crypto;
  final Random _random;
  PhoneIdentity? _cached;

  Future<PhoneIdentity> load() async {
    final cached = _cached;
    if (cached != null) return cached;
    var key = await _kv.read(keyName);
    var id = await _kv.read(deviceIdName);
    if (key == null || id == null) {
      key = base64Url.encode(_crypto.generateKey());
      id = base64Url
          .encode(List<int>.generate(16, (_) => _random.nextInt(256)))
          .replaceAll('=', '');
      await _kv.write(keyName, key);
      await _kv.write(deviceIdName, id);
    }
    final identity = PhoneIdentity(
      privateKey: Uint8List.fromList(base64Url.decode(key)),
      deviceId: id,
    );
    _cached = identity;
    return identity;
  }
}
