import 'dart:convert';
import 'dart:ui';

import 'package:flutter_local_notifications/flutter_local_notifications.dart';

import 'push_apk.dart';
import 'push_menu.dart';
import 'push_notice.dart';

const String attentionChannel = 'attention';
const String runningChannel = 'running';
const String notificationIcon = 'ic_stat_orrery';
const Color notificationAccent = Color(0xFFE8835D);
const Duration runningTimeout = Duration(hours: 6);

abstract class PushNotifier {
  Future<bool> init({void Function(String? payload)? onTap});

  Future<void> show(PushNotice notice, {required String channelName});

  Future<void> clearMachine(String machine);

  Future<String?> launchPayload();
}

String? payloadWithAction(String? payload, String? actionId) {
  final data = pushTapData(payload);
  if (data == null) return payload;
  return jsonEncode(
    tapDataWithInstall(tapDataWithAction(data, actionId), actionId),
  );
}

List<AndroidNotificationAction> noticeActions(
  PushNotice notice,
  String openLabel, [
  String installLabel = '',
]) {
  if (notice.apk != null) {
    return [
      AndroidNotificationAction(
        installActionId,
        installLabel,
        showsUserInterface: true,
        cancelNotification: true,
      ),
    ];
  }
  final menu = notice.menu;
  if (menu != null) {
    return [
      for (final item in menu.items.take(pushMenuMax))
        AndroidNotificationAction(
          menuActionId(item.index),
          item.label,
          showsUserInterface: true,
          cancelNotification: false,
        ),
    ];
  }
  if (notice.kind == 'done' && notice.pane != null) {
    return [
      AndroidNotificationAction(
        openActionId,
        openLabel,
        showsUserInterface: true,
      ),
    ];
  }
  return const [];
}

AndroidNotificationDetails noticeDetails(
  PushNotice notice, {
  required String channelName,
  required String openLabel,
  String installLabel = '',
}) {
  if (notice.ongoing) {
    return AndroidNotificationDetails(
      runningChannel,
      channelName,
      icon: notificationIcon,
      color: notificationAccent,
      importance: Importance.low,
      priority: Priority.low,
      visibility: NotificationVisibility.public,
      category: AndroidNotificationCategory.status,
      ongoing: true,
      autoCancel: false,
      onlyAlertOnce: true,
      playSound: false,
      enableVibration: false,
      showWhen: true,
      when: notice.startedAt?.millisecondsSinceEpoch,
      usesChronometer: notice.startedAt != null,
      timeoutAfter: runningTimeout.inMilliseconds,
      tag: notice.tag,
      styleInformation: BigTextStyleInformation(notice.body),
    );
  }
  return AndroidNotificationDetails(
    attentionChannel,
    channelName,
    icon: notificationIcon,
    color: notificationAccent,
    importance: Importance.high,
    priority: Priority.high,
    visibility: NotificationVisibility.public,
    category: AndroidNotificationCategory.message,
    tag: notice.tag,
    actions: noticeActions(notice, openLabel, installLabel),
    styleInformation: BigTextStyleInformation(notice.body),
  );
}

class LocalPushNotifier implements PushNotifier {
  LocalPushNotifier();

  final FlutterLocalNotificationsPlugin _plugin =
      FlutterLocalNotificationsPlugin();

  @override
  Future<bool> init({void Function(String? payload)? onTap}) async {
    final ready =
        await _plugin.initialize(
          settings: const InitializationSettings(
            android: AndroidInitializationSettings(notificationIcon),
          ),
          onDidReceiveNotificationResponse: onTap == null
              ? null
              : (response) => onTap(
                  payloadWithAction(response.payload, response.actionId),
                ),
        ) ??
        false;
    if (ready) await _ensureRunningChannel();
    return ready;
  }

  Future<void> _ensureRunningChannel() async {
    try {
      final strings = pushStrings(
        PlatformDispatcher.instance.locale.languageCode,
      );
      await _plugin
          .resolvePlatformSpecificImplementation<
            AndroidFlutterLocalNotificationsPlugin
          >()
          ?.createNotificationChannel(
            AndroidNotificationChannel(
              runningChannel,
              strings.pushRunningChannel,
              importance: Importance.low,
              playSound: false,
              enableVibration: false,
              showBadge: false,
            ),
          );
    } on Object {
      return;
    }
  }

  @override
  Future<void> show(PushNotice notice, {required String channelName}) {
    final strings = pushStrings(
      PlatformDispatcher.instance.locale.languageCode,
    );
    return _plugin.show(
      id: 0,
      title: notice.title,
      body: notice.body,
      payload: notice.payload,
      notificationDetails: NotificationDetails(
        android: noticeDetails(
          notice,
          channelName: notice.ongoing
              ? strings.pushRunningChannel
              : channelName,
          openLabel: strings.pushActionOpen,
          installLabel: strings.pushActionInstall,
        ),
      ),
    );
  }

  @override
  Future<void> clearMachine(String machine) async {
    final active = await _plugin.getActiveNotifications();
    for (final item in active) {
      final tag = item.tag;
      if (tag == null || !tag.startsWith('$machine:')) continue;
      if (item.channelId == runningChannel) continue;
      await _plugin.cancel(id: item.id ?? 0, tag: tag);
    }
  }

  @override
  Future<String?> launchPayload() async {
    final details = await _plugin.getNotificationAppLaunchDetails();
    if (details == null || !details.didNotificationLaunchApp) return null;
    final response = details.notificationResponse;
    return payloadWithAction(response?.payload, response?.actionId);
  }
}
