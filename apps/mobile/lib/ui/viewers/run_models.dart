import 'viewer_models.dart';

class RunConfigInfo {
  const RunConfigInfo(this.id, this.name, this.type);

  final String id;
  final String name;
  final String type;

  static List<RunConfigInfo> parseAll(Object? raw) => [
    for (final m in maps(raw))
      if (str(m['id']) != null)
        RunConfigInfo(
          m['id'] as String,
          str(m['name']) ?? m['id'] as String,
          str(m['type']) ?? 'custom',
        ),
  ];
}

class RunDeviceInfo {
  const RunDeviceInfo(this.id, this.name, this.ready);

  final String id;
  final String name;
  final bool ready;

  static List<RunDeviceInfo> parseAll(Object? raw) => [
    for (final m in maps(raw))
      if (str(m['id']) != null)
        RunDeviceInfo(
          m['id'] as String,
          str(m['name']) ?? m['id'] as String,
          m['state'] == null || m['state'] == 'ready',
        ),
  ];
}

const List<String> liveStates = [
  'before-launch',
  'building',
  'installing',
  'launching',
  'starting',
  'running',
  'reloading',
  'stopping',
];

class RunSessionInfo {
  const RunSessionInfo({
    required this.id,
    required this.configName,
    required this.state,
    required this.cwd,
    this.deviceName,
    this.progress,
    this.failure,
  });

  final String id;
  final String configName;
  final String state;
  final String cwd;
  final String? deviceName;
  final String? progress;
  final String? failure;

  bool get live => liveStates.contains(state);

  static RunSessionInfo? parse(Object? raw) {
    if (raw is! Map) return null;
    final m = raw.cast<String, dynamic>();
    if (str(m['id']) == null) return null;
    final failure = m['failure'];
    return RunSessionInfo(
      id: m['id'] as String,
      configName: str(m['configName']) ?? '',
      state: str(m['state']) ?? 'stopped',
      cwd: str(m['cwd']) ?? '',
      deviceName: str(m['deviceName']),
      progress: str(m['progress']),
      failure: failure is Map ? str(failure['code']) : null,
    );
  }

  static List<RunSessionInfo> parseAll(Object? raw) => [
    if (raw is List)
      for (final e in raw) ?parse(e),
  ];
}

class RunLine {
  const RunLine(this.text, this.level, this.stream);

  final String text;
  final String? level;
  final String stream;
}
