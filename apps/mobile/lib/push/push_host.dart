import '../app/pane_link.dart';
import 'push_config.dart';

abstract class PushSession {
  String get machineId;

  Future<void> send(String type, Map<String, Object?> fields);
}

class PushConfigUpdate {
  const PushConfigUpdate(this.machineId, this.config);

  final String machineId;
  final PushConfig? config;
}

abstract class PushHost {
  Stream<PushSession> get readySessions;

  List<PushSession> get readyNow;

  Stream<PushConfigUpdate> get configs;

  Future<void> dispose();
}

enum PushResult { on, off, denied, unavailable }

const String pushEnabledKey = 'push.enabled';
const String pushConfigsKey = 'push.configs';
const String pushPendingKey = 'push.unregister';
const String pushMuteKey = 'push.mute';
const List<String> pushGroups = ['waiting', 'done', 'error'];

Uri? pushLink(Map<String, String> data) {
  final machine = data['m'];
  final pane = data['p'];
  if (machine == null) return null;
  if (!RegExp(r'^[A-Za-z0-9_-]{1,128}$').hasMatch(machine)) return null;
  if (pane == null) {
    return Uri(scheme: appLinkScheme, host: 'm', pathSegments: [machine]);
  }
  if (!RegExp(r'^[A-Za-z0-9_.:-]{1,128}$').hasMatch(pane)) return null;
  return Uri(
    scheme: appLinkScheme,
    host: 'm',
    pathSegments: [machine, 'p', pane],
  );
}
