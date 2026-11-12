import 'package:device_info_plus/device_info_plus.dart';

const int maxDeviceName = 40;

String friendlyDeviceName({
  required String fallback,
  String? manufacturer,
  String? model,
}) {
  final maker = (manufacturer ?? '').trim();
  var name = (model ?? '').trim();
  if (name.isEmpty) return fallback;
  final lower = name.toLowerCase();
  if (maker.isNotEmpty && !lower.startsWith(maker.toLowerCase())) {
    name = '${maker[0].toUpperCase()}${maker.substring(1)} $name';
  }
  name = name.replaceAll(RegExp(r'[\u0000-\u001f\u007f]'), '').trim();
  if (name.runes.length > maxDeviceName) {
    name = String.fromCharCodes(name.runes.take(maxDeviceName)).trim();
  }
  return name.isEmpty ? fallback : name;
}

Future<String> loadDeviceName() async {
  const fallback = 'Android phone';
  try {
    final plugin = DeviceInfoPlugin();
    final info = await plugin.androidInfo;
    return friendlyDeviceName(
      fallback: fallback,
      manufacturer: info.manufacturer,
      model: info.model,
    );
  } catch (_) {
    return fallback;
  }
}
