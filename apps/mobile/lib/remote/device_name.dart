import 'dart:io';

import 'package:device_info_plus/device_info_plus.dart';

const int maxDeviceName = 40;

const Map<String, String> _iosModels = {
  'iPhone12,1': 'iPhone 11',
  'iPhone12,3': 'iPhone 11 Pro',
  'iPhone12,5': 'iPhone 11 Pro Max',
  'iPhone13,1': 'iPhone 12 mini',
  'iPhone13,2': 'iPhone 12',
  'iPhone13,3': 'iPhone 12 Pro',
  'iPhone13,4': 'iPhone 12 Pro Max',
  'iPhone14,4': 'iPhone 13 mini',
  'iPhone14,5': 'iPhone 13',
  'iPhone14,2': 'iPhone 13 Pro',
  'iPhone14,3': 'iPhone 13 Pro Max',
  'iPhone14,7': 'iPhone 14',
  'iPhone14,8': 'iPhone 14 Plus',
  'iPhone15,2': 'iPhone 14 Pro',
  'iPhone15,3': 'iPhone 14 Pro Max',
  'iPhone15,4': 'iPhone 15',
  'iPhone15,5': 'iPhone 15 Plus',
  'iPhone16,1': 'iPhone 15 Pro',
  'iPhone16,2': 'iPhone 15 Pro Max',
};

String friendlyDeviceName({
  required String fallback,
  String? manufacturer,
  String? model,
}) {
  final maker = (manufacturer ?? '').trim();
  var name = (model ?? '').trim();
  if (name.isEmpty) return fallback;
  final mapped = _iosModels[name];
  if (mapped != null) name = mapped;
  final lower = name.toLowerCase();
  if (mapped == null &&
      maker.isNotEmpty &&
      !lower.startsWith(maker.toLowerCase())) {
    name = '${maker[0].toUpperCase()}${maker.substring(1)} $name';
  }
  name = name.replaceAll(RegExp(r'[\u0000-\u001f\u007f]'), '').trim();
  if (name.runes.length > maxDeviceName) {
    name = String.fromCharCodes(name.runes.take(maxDeviceName)).trim();
  }
  return name.isEmpty ? fallback : name;
}

Future<String> loadDeviceName() async {
  final fallback = Platform.isIOS ? 'iPhone' : 'Android phone';
  try {
    final plugin = DeviceInfoPlugin();
    if (Platform.isIOS) {
      final info = await plugin.iosInfo;
      return friendlyDeviceName(
        fallback: fallback,
        manufacturer: 'Apple',
        model: info.utsname.machine,
      );
    }
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
