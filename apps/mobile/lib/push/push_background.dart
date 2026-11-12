import 'dart:ui';

import 'dart:typed_data';

import 'package:firebase_messaging/firebase_messaging.dart';
import 'package:path_provider/path_provider.dart';

import '../remote/machines.dart';
import '../remote/secure_kv.dart';
import 'local_notifier.dart';
import 'push_notice.dart';
import 'push_seal.dart';

@pragma('vm:entry-point')
Future<void> pushBackgroundMessage(RemoteMessage message) async {
  DartPluginRegistrant.ensureInitialized();
  final kv = FlutterSecureKv();
  await handleBackgroundPush(
    PushArrival(
      message.data.map((key, value) => MapEntry(key, '$value')),
      shown: message.notification != null,
    ),
    kv: kv,
    machineKey: pairedMachineKeys(kv),
    notifier: LocalPushNotifier(),
    languageCode: PlatformDispatcher.instance.locale.languageCode,
  );
}

Future<Uint8List?> Function(String machineId) pairedMachineKeys(SecureKv kv) =>
    (machineId) async => pairedMachineKey(
      MachineRepository(
        directory: await getApplicationSupportDirectory(),
        secure: kv,
      ),
      machineId,
    );

Future<void> handleBackgroundPush(
  PushArrival arrival, {
  required SecureKv kv,
  required Future<Uint8List?> Function(String machineId) machineKey,
  required PushNotifier notifier,
  required String languageCode,
}) async {
  final strings = pushStrings(languageCode);
  PushNotice notice;
  try {
    notice = await pushNotice(
      arrival.data,
      key: () => readStaticKey(kv),
      machineKey: machineKey,
      strings: strings,
    );
  } on Object {
    notice = fixedPushNotice(arrival.data, strings);
  }
  if (arrival.shown) {
    if (notice.sealed && await _ready(notifier)) await _mark(kv, true);
    return;
  }
  for (final candidate in [
    notice,
    if (notice.sealed) fixedPushNotice(arrival.data, strings),
  ]) {
    try {
      await notifier.init();
      await notifier.show(candidate, channelName: strings.pushTitle);
      return;
    } on Object {
      continue;
    }
  }
  await _mark(kv, false);
}

Future<bool> _ready(PushNotifier notifier) async {
  try {
    return await notifier.init();
  } on Object {
    return false;
  }
}

Future<void> _mark(SecureKv kv, bool ok) async {
  try {
    await kv.write(pushSealOkKey, ok ? 'on' : 'off');
  } on Object {
    return;
  }
}
