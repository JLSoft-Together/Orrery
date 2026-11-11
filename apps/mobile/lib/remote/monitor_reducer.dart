const int activityCap = 500;

void _upsert(Map<String, dynamic> snap, String key, Object? item) {
  if (item is! Map) return;
  final list = (snap[key] as List<dynamic>?) ?? <dynamic>[];
  snap[key] = list;
  final id = item['id'];
  final index = list.indexWhere((e) => e is Map && e['id'] == id);
  final value = item.cast<String, dynamic>();
  if (index >= 0) {
    list[index] = value;
  } else {
    list.add(value);
  }
}

void _merge(Map<String, dynamic> snap, String key, Object? patch) {
  if (patch is! Map) return;
  final list = snap[key];
  if (list is! List) return;
  final id = patch['id'];
  final index = list.indexWhere((e) => e is Map && e['id'] == id);
  if (index < 0) return;
  final current = (list[index] as Map).cast<String, dynamic>();
  list[index] = {...current, ...patch.cast<String, dynamic>()};
}

void _remove(Map<String, dynamic> snap, String key, Object? payload) {
  if (payload is! Map) return;
  final list = snap[key];
  if (list is! List) return;
  list.removeWhere((e) => e is Map && e['id'] == payload['id']);
}

void applyMonitorEvent(Map<String, dynamic> snap, Object? raw) {
  if (raw is! Map) return;
  final type = raw['type'];
  final payload = raw['payload'];
  switch (type) {
    case 'terminal.created':
      _upsert(snap, 'terminals', payload);
    case 'terminal.updated':
      _merge(snap, 'terminals', payload);
    case 'terminal.removed':
      _remove(snap, 'terminals', payload);
    case 'agent.created':
      _upsert(snap, 'agents', payload);
    case 'agent.updated':
      _merge(snap, 'agents', payload);
    case 'agent.removed':
      _remove(snap, 'agents', payload);
    case 'link.created':
      _upsert(snap, 'peerLinks', payload);
    case 'link.updated':
      _merge(snap, 'peerLinks', payload);
    case 'link.removed':
      _remove(snap, 'peerLinks', payload);
    case 'activity':
      if (payload is Map) {
        final list = (snap['activity'] as List<dynamic>?) ?? <dynamic>[];
        snap['activity'] = list;
        list.add(payload.cast<String, dynamic>());
        if (list.length > activityCap) {
          list.removeRange(0, list.length - activityCap);
        }
      }
    case 'sessions.loading':
      if (payload is Map) snap['loading'] = payload['loading'];
    case 'usage.reset':
      snap['usage'] = payload;
    case 'usage.updated':
      final usage = snap['usage'];
      if (usage is Map && payload is Map) usage['buckets'] = payload['buckets'];
    case 'favorites.updated':
      if (payload is Map) snap['favorites'] = payload['favorites'];
    case 'jobs.updated':
      if (payload is Map) snap['jobs'] = payload['jobs'];
    case 'limits.updated':
      snap['limits'] = payload;
    case 'codexLimits.updated':
      snap['codexLimits'] = payload;
    case 'response.updated':
      snap['response'] = payload;
    case 'models.updated':
      if (payload is Map) snap['modelNames'] = payload['modelNames'];
    case 'snooze.updated':
      if (payload is Map) snap['snoozes'] = payload['snoozes'];
    case 'resources':
      snap['resources'] = payload;
    case 'notify.settings':
      snap['notify'] = payload;
    case 'sensitive.settings':
      snap['sensitive'] = payload;
  }
}
