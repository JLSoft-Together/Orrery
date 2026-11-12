import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

final Map<String, dynamic> vector = jsonDecode(
  File('test/fixtures/push_seal_v2.json').readAsStringSync(),
) as Map<String, dynamic>;

Uint8List hex(String text) => Uint8List.fromList([
  for (var i = 0; i < text.length; i += 2)
    int.parse(text.substring(i, i + 2), radix: 16),
]);

String toHex(List<int> raw) =>
    raw.map((b) => b.toRadixString(16).padLeft(2, '0')).join();

String get machine => vector['machine'] as String;
String get kind => vector['kind'] as String;
String get sealed => vector['e'] as String;
Uint8List get devicePrivate => hex(vector['devicePrivate'] as String);
Uint8List get desktopPublic => hex(vector['desktopPublic'] as String);

Future<Uint8List?> pairedVector(String id) async =>
    id == machine ? desktopPublic : null;

Map<String, String> sealedData({String? e, String v = '2', String? k}) => {
  'v': v,
  'm': machine,
  'k': k ?? kind,
  'e': e ?? sealed,
};

String flip(String text, int index) {
  final raw = base64Url.decode(base64Url.normalize(text));
  raw[index] ^= 1;
  return base64Url.encode(raw).replaceAll('=', '');
}
