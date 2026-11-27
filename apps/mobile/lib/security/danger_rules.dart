import 'device_auth.dart';

const int maxDangerRules = 32;
const int maxDangerRuleLength = 200;

enum DangerVerdict { safe, authed, cancelled }

class DangerRules {
  const DangerRules._(this.patterns, this._compiled);

  const DangerRules.none() : patterns = const [], _compiled = const [];

  factory DangerRules.parse(Object? raw) {
    if (raw is! List) return const DangerRules.none();
    final patterns = <String>[];
    final compiled = <RegExp>[];
    for (final item in raw) {
      if (patterns.length >= maxDangerRules) break;
      if (item is! String || item.isEmpty) continue;
      if (item.length > maxDangerRuleLength || patterns.contains(item)) {
        continue;
      }
      try {
        compiled.add(RegExp(item, caseSensitive: false));
      } on FormatException {
        continue;
      }
      patterns.add(item);
    }
    return DangerRules._(patterns, compiled);
  }

  factory DangerRules.fromHello(Map<String, dynamic>? hello) =>
      DangerRules.parse(hello?['dangerRules']);

  final List<String> patterns;
  final List<RegExp> _compiled;

  bool get isEmpty => _compiled.isEmpty;

  String? match(String text) {
    if (_compiled.isEmpty) return null;
    final lines = [text.trim(), ...text.split('\n').map((l) => l.trim())];
    for (var i = 0; i < _compiled.length; i++) {
      for (final line in lines) {
        if (line.isNotEmpty && _compiled[i].hasMatch(line)) return patterns[i];
      }
    }
    return null;
  }
}

bool authPassed(AuthOutcome outcome) =>
    outcome == AuthOutcome.success || outcome == AuthOutcome.notAvailable;

Future<DangerVerdict> guardDanger({
  required DangerRules rules,
  required String text,
  required Future<AuthOutcome> Function()? authenticate,
}) async {
  if (rules.match(text) == null) return DangerVerdict.safe;
  if (authenticate == null) return DangerVerdict.authed;
  try {
    return authPassed(await authenticate())
        ? DangerVerdict.authed
        : DangerVerdict.cancelled;
  } on Object {
    return DangerVerdict.cancelled;
  }
}
