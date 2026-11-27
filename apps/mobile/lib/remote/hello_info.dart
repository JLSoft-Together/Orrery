import 'package:device_info_plus/device_info_plus.dart';
import 'package:package_info_plus/package_info_plus.dart';

import 'device_name.dart';

const int maxHelloVersion = 16;
const Duration helloInfoTimeout = Duration(seconds: 3);

typedef HelloInfoLoader = Future<HelloInfo> Function();

class HelloInfo {
  const HelloInfo({this.appVersion, this.appBuild, this.model, this.os});

  static const HelloInfo empty = HelloInfo();

  final String? appVersion;
  final String? appBuild;
  final String? model;
  final String? os;

  Map<String, String> toJson() => {
    'appVersion': ?appVersion,
    'appBuild': ?appBuild,
    'model': ?model,
    'os': ?os,
  };
}

String? cleanHelloText(String? raw, int max) {
  final text = (raw ?? '')
      .replaceAll(RegExp(r'[\u0000-\u001f\u007f]'), '')
      .trim();
  if (text.isEmpty || text.runes.length > max) return null;
  return text;
}

String? androidOsLabel(String? release) {
  final version = cleanHelloText(release, maxHelloVersion);
  return version == null ? null : 'Android $version';
}

HelloInfo? _loaded;

Future<HelloInfo> loadHelloInfo() async {
  final kept = _loaded;
  if (kept != null) return kept;
  final info = await _read().timeout(
    helloInfoTimeout,
    onTimeout: () => HelloInfo.empty,
  );
  return _loaded = info;
}

Future<HelloInfo> _read() async {
  String? version;
  String? build;
  String? model;
  String? os;
  try {
    final package = await PackageInfo.fromPlatform();
    version = cleanHelloText(package.version, maxHelloVersion);
    build = cleanHelloText(package.buildNumber, maxHelloVersion);
  } catch (_) {}
  try {
    final android = await DeviceInfoPlugin().androidInfo;
    final name = friendlyDeviceName(
      fallback: '',
      manufacturer: android.manufacturer,
      model: android.model,
    );
    model = cleanHelloText(name, maxDeviceName);
    os = androidOsLabel(android.version.release);
  } catch (_) {}
  return HelloInfo(appVersion: version, appBuild: build, model: model, os: os);
}
