import 'dart:convert';

const int apkMaxBytes = 1 << 30;
const int apkMaxUrls = 3;
const int apkTextMax = 80;
final RegExp _shaPattern = RegExp(r'^[0-9a-f]{64}$');

enum ApkError {
  invalid,
  expired,
  lan,
  interrupted,
  space,
  size,
  sha,
  permission,
  installFailed,
}

class ApkFailure implements Exception {
  const ApkFailure(this.error);

  final ApkError error;

  @override
  String toString() => 'ApkFailure(${error.name})';
}

bool isPrivateLanHost(String host) {
  final parts = host.split('.');
  if (parts.length != 4) return false;
  final n = <int>[];
  for (final p in parts) {
    final v = int.tryParse(p);
    if (v == null || v < 0 || v > 255 || p != '$v') return false;
    n.add(v);
  }
  return n[0] == 10 ||
      (n[0] == 172 && n[1] >= 16 && n[1] <= 31) ||
      (n[0] == 192 && n[1] == 168);
}

Uri? _lanUrl(Object? raw) {
  if (raw is! String || raw.length > 300) return null;
  final uri = Uri.tryParse(raw);
  if (uri == null ||
      uri.scheme != 'http' ||
      !isPrivateLanHost(uri.host) ||
      !uri.hasPort ||
      uri.port < 1 ||
      uri.port > 65535 ||
      uri.userInfo.isNotEmpty) {
    return null;
  }
  return uri;
}

String _clean(Object? raw) {
  if (raw is! String) return '';
  final text = String.fromCharCodes(
    raw.runes.map((r) => r < 0x20 || r == 0x7f ? 0x20 : r),
  ).trim();
  final runes = text.runes.toList();
  return runes.length <= apkTextMax
      ? text
      : String.fromCharCodes(runes.take(apkTextMax));
}

class ApkOffer {
  const ApkOffer({
    required this.urls,
    required this.size,
    required this.sha256,
    required this.exp,
    this.app = '',
    this.variant = '',
    this.machine,
  });

  final List<Uri> urls;
  final int size;
  final String sha256;
  final int exp;
  final String app;
  final String variant;
  final String? machine;

  bool expiredAt(DateTime now, {Duration grace = Duration.zero}) =>
      now.millisecondsSinceEpoch ~/ 1000 > exp + grace.inSeconds;

  ApkOffer withMachine(String? id) => ApkOffer(
    urls: urls,
    size: size,
    sha256: sha256,
    exp: exp,
    app: app,
    variant: variant,
    machine: id,
  );

  Map<String, Object?> toJson() => {
    'urls': [for (final u in urls) u.toString()],
    'size': size,
    'sha256': sha256,
    'app': app,
    'variant': variant,
    'exp': exp,
  };

  String encode() => jsonEncode(toJson());

  static ApkOffer? parse(Object? raw, {String? machine}) {
    if (raw is String) {
      try {
        raw = jsonDecode(raw);
      } on FormatException {
        return null;
      }
    }
    if (raw is! Map) return null;
    final size = raw['size'];
    final sha = raw['sha256'];
    final exp = raw['exp'];
    if (size is! int || size < 1 || size > apkMaxBytes) return null;
    if (sha is! String || !_shaPattern.hasMatch(sha)) return null;
    if (exp is! int || exp <= 0) return null;
    final seen = <String>{};
    final urls = <Uri>[];
    final list = raw['urls'];
    for (final item in [?raw['url'], if (list is List) ...list]) {
      final uri = _lanUrl(item);
      if (uri == null || !seen.add(uri.toString())) continue;
      urls.add(uri);
      if (urls.length == apkMaxUrls) break;
    }
    if (urls.isEmpty) return null;
    return ApkOffer(
      urls: urls,
      size: size,
      sha256: sha,
      exp: exp,
      app: _clean(raw['app']),
      variant: _clean(raw['variant']),
      machine: machine,
    );
  }
}
