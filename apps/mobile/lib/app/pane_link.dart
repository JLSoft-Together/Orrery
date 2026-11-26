import 'dart:convert';

const String relayLinkHost = 'orrery-relay.jinvu.workers.dev';
const String appLinkScheme = 'orrery-remote';

final RegExp _machinePattern = RegExp(r'^[A-Za-z0-9_-]{1,128}$');
final RegExp _panePattern = RegExp(r'^[A-Za-z0-9_.:-]{1,128}$');

class PaneLink {
  const PaneLink(this.machineId, this.paneKey);

  final String machineId;
  final String? paneKey;

  @override
  bool operator ==(Object other) =>
      other is PaneLink &&
      other.machineId == machineId &&
      other.paneKey == paneKey;

  @override
  int get hashCode => Object.hash(machineId, paneKey);

  @override
  String toString() => 'PaneLink($machineId, $paneKey)';
}

PaneLink? parsePaneLink(Uri uri) {
  final List<String> parts;
  if (uri.scheme == appLinkScheme && uri.host == 'm') {
    parts = uri.pathSegments;
    if (parts.length == 1) return _build(parts[0], null);
    if (parts.length != 3 || parts[1] != 'p') return null;
    return _build(parts[0], parts[2]);
  }
  if (uri.scheme == 'https' && uri.host == relayLinkHost) {
    parts = uri.pathSegments;
    if (parts.length != 3 || parts[0] != 'o') return null;
    return _build(parts[1], parts[2]);
  }
  return null;
}

const int pairLinkMaxBytes = 640;

String? parsePairLink(Uri uri) {
  if (uri.scheme != appLinkScheme || uri.host != 'pair') return null;
  final q = uri.queryParameters['q'];
  if (q == null || q.isEmpty) return null;
  if (utf8.encode(q).length > pairLinkMaxBytes) return null;
  if (q.runes.any((r) => r < 0x20 || r == 0x7f)) return null;
  return q;
}

PaneLink? _build(String machine, String? pane) {
  if (!_machinePattern.hasMatch(machine) ||
      (pane != null && !_panePattern.hasMatch(pane))) {
    return null;
  }
  return PaneLink(machine, pane);
}
