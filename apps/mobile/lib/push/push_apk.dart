import 'dart:typed_data';

import '../install/apk_offer.dart';
import '../l10n/app_localizations.dart';
import 'push_notice.dart';

const String installActionId = 'install';

class ApkTap {
  const ApkTap(this.offer, {required this.auto});

  final ApkOffer offer;
  final bool auto;
}

Future<ApkTap?> apkTapOf(
  Map<String, String> data, {
  required Future<Uint8List?> Function() key,
  required Future<Uint8List?> Function(String machineId) machineKey,
  required AppLocalizations strings,
}) async {
  if (data['k'] != 'apk') return null;
  final machine = data['m'];
  final auto = data['a'] == installActionId;
  final inline = ApkOffer.parse(data['apk'], machine: machine);
  if (inline != null) return ApkTap(inline, auto: auto);
  if (data['e'] == null) return null;
  try {
    final notice = await pushNotice(
      data,
      key: key,
      machineKey: machineKey,
      strings: strings,
    );
    final offer = notice.apk;
    return offer == null ? null : ApkTap(offer, auto: auto);
  } on Object {
    return null;
  }
}

Map<String, String> tapDataWithInstall(
  Map<String, String> data,
  String? actionId,
) => actionId == installActionId ? {...data, 'a': installActionId} : data;
