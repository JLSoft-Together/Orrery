import 'package:flutter/painting.dart';
import 'package:flutter_local_notifications/flutter_local_notifications.dart';

import 'push_notice.dart';

const String attentionChannel = 'attention';
const String notificationIcon = 'ic_stat_orrery';
const Color notificationAccent = Color(0xFFE8835D);

abstract class PushNotifier {
  Future<bool> init({void Function(String? payload)? onTap});

  Future<void> show(PushNotice notice, {required String channelName});

  Future<String?> launchPayload();
}

class LocalPushNotifier implements PushNotifier {
  LocalPushNotifier();

  final FlutterLocalNotificationsPlugin _plugin =
      FlutterLocalNotificationsPlugin();

  @override
  Future<bool> init({void Function(String? payload)? onTap}) async =>
      await _plugin.initialize(
        settings: const InitializationSettings(
          android: AndroidInitializationSettings(notificationIcon),
        ),
        onDidReceiveNotificationResponse: onTap == null
            ? null
            : (response) => onTap(response.payload),
      ) ??
      false;

  @override
  Future<void> show(PushNotice notice, {required String channelName}) =>
      _plugin.show(
        id: 0,
        title: notice.title,
        body: notice.body,
        payload: notice.payload,
        notificationDetails: NotificationDetails(
          android: AndroidNotificationDetails(
            attentionChannel,
            channelName,
            icon: notificationIcon,
            color: notificationAccent,
            importance: Importance.high,
            priority: Priority.high,
            visibility: NotificationVisibility.public,
            category: AndroidNotificationCategory.message,
            tag: notice.tag,
            styleInformation: BigTextStyleInformation(notice.body),
          ),
        ),
      );

  @override
  Future<String?> launchPayload() async {
    final details = await _plugin.getNotificationAppLaunchDetails();
    if (details == null || !details.didNotificationLaunchApp) return null;
    return details.notificationResponse?.payload;
  }
}
