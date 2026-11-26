import 'package:flutter/widgets.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../app/app_lock_scope.dart';
import '../remote/providers.dart';
import '../security/danger_rules.dart';

final dangerRulesProvider = Provider.family<DangerRules, String>((ref, id) {
  final hello = ref.watch(machineViewProvider(id).select((v) => v.hello));
  return DangerRules.fromHello(hello);
});

Future<bool> authorizeNow(BuildContext context) async {
  final lock = AppLockScope.maybeOf(context);
  if (lock == null) return true;
  try {
    return authPassed(await lock.confirm());
  } on Object {
    return false;
  }
}

Future<DangerVerdict> confirmDanger(
  BuildContext context,
  DangerRules rules,
  String text,
) {
  final lock = AppLockScope.maybeOf(context);
  return guardDanger(
    rules: rules,
    text: text,
    authenticate: lock == null ? null : () => lock.confirm(),
  );
}
