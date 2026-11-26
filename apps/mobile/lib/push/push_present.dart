import 'dart:typed_data';

import 'package:clock/clock.dart';

import '../l10n/app_localizations.dart';
import '../remote/secure_kv.dart';
import 'push_gateway.dart';
import 'push_notice.dart';
import 'push_seal.dart';
import 'waiting_sink.dart';
import 'waiting_update.dart';

class PushPresenter {
  PushPresenter({
    required this.gateway,
    required this.kv,
    required this.sessionOpen,
    required this.resumed,
    required this.localeCode,
    required this.machineKey,
    this.waiting = const NoWaitingSink(),
    this.paused = _notPaused,
    DateTime Function()? now,
  }) : _now = now ?? (() => clock.now());

  static Future<bool> _notPaused() async => false;

  final WaitingSink waiting;
  final Future<bool> Function() paused;
  final DateTime Function() _now;
  final PushGateway gateway;
  final SecureKv kv;
  final bool Function(String? machineId) sessionOpen;
  final bool Function() resumed;
  final String Function() localeCode;
  final Future<Uint8List?> Function(String machineId) machineKey;

  Future<PushNotice?> present(PushArrival arrival) async {
    final data = arrival.data;
    final strings = pushStrings(localeCode());
    var update = parseWaiting(data);
    PushNotice? notice;
    if (update != null &&
        update.count > 0 &&
        update.labels.isEmpty &&
        data['e'] != null) {
      notice = await _open(data, strings);
      if (notice.labels.isNotEmpty) {
        update = WaitingUpdate(count: update.count, labels: notice.labels);
      }
    }
    if (update != null) await waiting.apply(update, at: _now());
    if (isClearPush(data)) {
      await _clear(data['m']);
      return null;
    }
    final forced = data['k'] == 'open' || data['k'] == 'apk';
    if (!forced && await paused()) return null;
    final show =
        forced ||
        shouldShowPush(
          background: false,
          shown: arrival.shown,
          resumed: resumed(),
          sessionOpen: sessionOpen(data['m']),
        );
    if (!show) return null;
    notice ??= await _open(data, strings);
    try {
      await gateway.show(notice, channelName: strings.pushTitle);
      return notice;
    } on Object {
      return null;
    }
  }

  Future<PushNotice> _open(
    Map<String, String> data,
    AppLocalizations strings,
  ) => pushNotice(
    data,
    key: () => readStaticKey(kv),
    machineKey: machineKey,
    strings: strings,
  );

  Future<void> _clear(String? machine) async {
    if (machine == null || !pushMachinePattern.hasMatch(machine)) return;
    try {
      await gateway.clearMachine(machine);
    } on Object {
      return;
    }
  }
}
