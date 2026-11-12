import 'package:flutter/widgets.dart';

import '../security/app_lock.dart';

class AppLockScope extends InheritedWidget {
  const AppLockScope({super.key, required this.lock, required super.child});

  final AppLock? lock;

  static AppLock? maybeOf(BuildContext context) =>
      context.dependOnInheritedWidgetOfExactType<AppLockScope>()?.lock;

  @override
  bool updateShouldNotify(AppLockScope oldWidget) => oldWidget.lock != lock;
}
