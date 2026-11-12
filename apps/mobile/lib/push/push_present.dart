import 'dart:typed_data';

import '../remote/secure_kv.dart';
import 'push_gateway.dart';
import 'push_notice.dart';
import 'push_seal.dart';

class PushPresenter {
  PushPresenter({
    required this.gateway,
    required this.kv,
    required this.sessionOpen,
    required this.resumed,
    required this.localeCode,
    required this.machineKey,
  });

  final PushGateway gateway;
  final SecureKv kv;
  final bool Function(String? machineId) sessionOpen;
  final bool Function() resumed;
  final String Function() localeCode;
  final Future<Uint8List?> Function(String machineId) machineKey;

  Future<PushNotice?> present(PushArrival arrival) async {
    final show = shouldShowPush(
      background: false,
      shown: arrival.shown,
      resumed: resumed(),
      sessionOpen: sessionOpen(arrival.data['m']),
    );
    if (!show) return null;
    final strings = pushStrings(localeCode());
    final notice = await pushNotice(
      arrival.data,
      key: () => readStaticKey(kv),
      machineKey: machineKey,
      strings: strings,
    );
    try {
      await gateway.show(notice, channelName: strings.pushTitle);
      return notice;
    } on Object {
      return null;
    }
  }
}
