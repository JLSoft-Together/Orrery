import 'dart:convert';
import 'dart:typed_data';
import 'dart:ui';

import '../install/apk_offer.dart';
import '../l10n/app_localizations.dart';
import 'push_menu.dart';
import 'push_seal.dart';

const String pushSealOkKey = 'push.sealOk';
const String pushContentKey = 'push.content';
const Set<String> pushKinds = {
  'done',
  'question',
  'permission',
  'attention',
  'error',
  'security',
  'test',
  'running',
  'open',
  'apk',
};
const Set<String> menuKinds = {'question', 'permission', 'attention'};
final RegExp pushMachinePattern = RegExp(r'^[A-Za-z0-9_-]{1,128}$');

class PushArrival {
  const PushArrival(this.data, {required this.shown});

  final Map<String, String> data;
  final bool shown;
}

class PushNotice {
  const PushNotice({
    required this.machine,
    required this.kind,
    required this.title,
    required this.body,
    required this.sealed,
    this.pane,
    this.menu,
    this.startedAt,
    this.labels = const [],
    this.apk,
  });

  final ApkOffer? apk;
  final String? machine;
  final String kind;
  final String title;
  final String body;
  final bool sealed;
  final String? pane;
  final PushMenu? menu;
  final DateTime? startedAt;
  final List<String> labels;

  bool get ongoing => kind == 'running';

  String get tag => '${machine ?? 'orrery'}:${pane ?? kind}';

  String? get payload => machine == null
      ? null
      : jsonEncode({
          'm': machine,
          'p': ?pane,
          'sig': ?menu?.sig,
          if (apk != null) ...{'k': 'apk', 'apk': apk!.encode()},
        });
}

String pushKind(String? raw) => pushKinds.contains(raw) ? raw! : 'attention';

AppLocalizations pushStrings(String languageCode) =>
    lookupAppLocalizations(Locale(languageCode == 'vi' ? 'vi' : 'en'));

(String, String) fixedPushText(AppLocalizations l, String kind) =>
    switch (kind) {
      'done' => (l.pushFixedDoneTitle, l.pushFixedDoneBody),
      'question' => (l.pushFixedQuestionTitle, l.pushFixedQuestionBody),
      'permission' => (l.pushFixedPermissionTitle, l.pushFixedPermissionBody),
      'error' => (l.pushFixedErrorTitle, l.pushFixedErrorBody),
      'running' => (l.pushFixedRunningTitle, l.pushFixedRunningBody),
      'open' => (l.pushFixedOpenTitle, l.pushFixedOpenBody),
      'apk' => (l.pushFixedApkTitle, l.pushFixedApkBody),
      'security' => (l.pushFixedSecurityTitle, l.pushFixedSecurityBody),
      'test' => (l.pushFixedTestTitle, l.pushFixedTestBody),
      _ => (l.pushFixedAttentionTitle, l.pushFixedAttentionBody),
    };

PushNotice fixedPushNotice(Map<String, String> data, AppLocalizations l) {
  final machine = data['m'];
  final pane = data['p'];
  final kind = pushKind(data['k']);
  final (title, body) = fixedPushText(l, kind);
  return PushNotice(
    machine: machine != null && pushMachinePattern.hasMatch(machine)
        ? machine
        : null,
    kind: kind,
    title: title,
    body: body,
    sealed: false,
    pane: pane != null && pushPanePattern.hasMatch(pane) ? pane : null,
    startedAt: _startedAt(data['t0']),
  );
}

DateTime? _startedAt(String? raw) {
  final ms = raw == null ? null : int.tryParse(raw);
  if (ms == null || ms <= 0) return null;
  return DateTime.fromMillisecondsSinceEpoch(ms);
}

Future<PushNotice> pushNotice(
  Map<String, String> data, {
  required Future<Uint8List?> Function() key,
  required Future<Uint8List?> Function(String machineId) machineKey,
  required AppLocalizations strings,
}) async {
  final fixed = fixedPushNotice(data, strings);
  final machine = fixed.machine;
  final kind = data['k'];
  final sealed = data['e'];
  if (machine == null ||
      kind == null ||
      sealed == null ||
      data['v'] != pushSealVersion) {
    return fixed;
  }
  Uint8List? private;
  Uint8List? desktop;
  try {
    desktop = await machineKey(machine);
    private = desktop == null ? null : await key();
  } on Object {
    private = null;
  }
  if (private == null || desktop == null) return fixed;
  final open = await openSeal(
    privateKey: private,
    desktop: desktop,
    machine: machine,
    kind: kind,
    sealed: sealed,
  );
  if (open == null) return fixed;
  return PushNotice(
    machine: machine,
    kind: fixed.kind,
    title: open.title,
    body: open.body,
    sealed: true,
    pane: open.pane,
    menu: menuKinds.contains(fixed.kind) ? open.menu : null,
    startedAt: fixed.startedAt,
    labels: open.labels,
    apk: fixed.kind == 'apk' ? open.apk?.withMachine(machine) : null,
  );
}

bool shouldShowPush({
  required bool background,
  required bool shown,
  required bool resumed,
  required bool sessionOpen,
}) {
  if (background) return !shown;
  return !(resumed && sessionOpen);
}

Map<String, String>? pushTapData(String? payload) {
  if (payload == null) return null;
  try {
    final value = jsonDecode(payload);
    if (value is! Map<String, dynamic>) return null;
    return {
      for (final e in value.entries)
        if (e.value is String) e.key: e.value as String,
    };
  } on FormatException {
    return null;
  }
}
