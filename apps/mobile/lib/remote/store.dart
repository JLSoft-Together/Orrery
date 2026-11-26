import 'dart:async';
import 'dart:typed_data';

import 'cache.dart';
import 'chat_last.dart';
import 'chat_protocol.dart';
import 'desktop_caps.dart';
import 'json_patch.dart';
import 'link_state.dart';
import 'monitor_reducer.dart';
import 'protocol.dart';
import 'size_notes.dart';

typedef Commander = Future<JsonMap> Function(String type, [JsonMap? fields]);

class ConfirmAsk {
  const ConfirmAsk({
    required this.cid,
    required this.title,
    required this.ok,
    required this.cancel,
    required this.receivedAt,
    this.detail,
  });

  final String cid;
  final String title;
  final String? detail;
  final String ok;
  final String cancel;
  final DateTime receivedAt;
}

const Object _keep = Object();

class MachineView {
  const MachineView({
    this.snapshot,
    this.seq = 0,
    this.workspace,
    this.wsRev = 0,
    this.confirms = const [],
    this.hello,
    this.trimmed,
    this.limit,
    this.stale = true,
    this.fromCache = false,
    this.updatedAt,
    this.revision = 0,
    this.chatLast = const {},
  });

  final Map<String, dynamic>? snapshot;
  final int seq;
  final Map<String, dynamic>? workspace;
  final int wsRev;
  final List<ConfirmAsk> confirms;
  final Map<String, dynamic>? hello;
  final SnapshotTrim? trimmed;
  final SizeLimit? limit;
  final bool stale;
  final bool fromCache;
  final DateTime? updatedAt;
  final int revision;
  final Map<String, ChatLast> chatLast;

  List<dynamic> get tabs {
    final value = workspace?['tabs'];
    return value is List ? value : const [];
  }

  bool hasCap(String name) => hasDesktopCap(hello, name);

  MachineView copyWith({
    Map<String, dynamic>? snapshot,
    int? seq,
    Map<String, dynamic>? workspace,
    int? wsRev,
    List<ConfirmAsk>? confirms,
    Map<String, dynamic>? hello,
    Object? trimmed = _keep,
    Object? limit = _keep,
    bool? stale,
    bool? fromCache,
    DateTime? updatedAt,
    Map<String, ChatLast>? chatLast,
  }) => MachineView(
    snapshot: snapshot ?? this.snapshot,
    seq: seq ?? this.seq,
    workspace: workspace ?? this.workspace,
    wsRev: wsRev ?? this.wsRev,
    confirms: confirms ?? this.confirms,
    hello: hello ?? this.hello,
    trimmed: identical(trimmed, _keep)
        ? this.trimmed
        : trimmed as SnapshotTrim?,
    limit: identical(limit, _keep) ? this.limit : limit as SizeLimit?,
    stale: stale ?? this.stale,
    fromCache: fromCache ?? this.fromCache,
    updatedAt: updatedAt ?? this.updatedAt,
    revision: revision + 1,
    chatLast: chatLast ?? this.chatLast,
  );
}

class MachineStore {
  MachineStore({
    required this.machineId,
    required this.commander,
    this.cache,
    this.tailsSource,
    this.confirmTtl = const Duration(seconds: 60),
    this.saveDelay = const Duration(seconds: 2),
    DateTime Function()? now,
  }) : _now = now ?? DateTime.now;

  final String machineId;
  final Commander commander;
  final MachineCache? cache;
  final Map<String, Uint8List> Function()? tailsSource;
  final Duration confirmTtl;
  final Duration saveDelay;
  final DateTime Function() _now;

  final StreamController<MachineView> _changes = StreamController.broadcast();
  final Map<String, Timer> _confirmTimers = {};
  MachineView _view = const MachineView();
  Timer? _saveTimer;
  bool _resyncPending = false;
  bool _wsGetPending = false;
  Map<String, Uint8List> cachedTails = const {};

  MachineView get view => _view;

  Stream<MachineView> get changes => _changes.stream;

  void _set(MachineView next) {
    _view = next;
    if (!_changes.isClosed) _changes.add(next);
  }

  Future<void> restoreFromCache() async {
    final entry = await cache?.load(machineId);
    if (entry == null) return;
    cachedTails = entry.tails;
    if (_view.snapshot != null || _view.workspace != null) return;
    _set(
      MachineView(
        snapshot: entry.snapshot,
        seq: entry.seq,
        workspace: entry.workspace,
        wsRev: entry.wsRev,
        hello: entry.hello,
        stale: true,
        fromCache: true,
        updatedAt: entry.savedAt,
        revision: _view.revision + 1,
      ),
    );
  }

  void onLink(LinkState link) {
    if (link.status == LinkStatus.ready || _view.stale) return;
    unawaited(flush());
    _set(_view.copyWith(stale: true));
  }

  void handleEvent(JsonMap event) {
    switch (event['t']) {
      case 'hello':
        final copy = {...event};
        copy.remove('re');
        copy.remove('ok');
        _set(_view.copyWith(hello: copy));
      case 'snapshot':
        _onSnapshot(event);
      case 'events':
        _onEvents(event);
      case 'limit':
        _onLimit(event);
        return;
      case 'chat.last':
        _onChatLast(event);
        return;
      case 'ws.snapshot':
        _onWsSnapshot(event);
      case 'ws.patch':
        _onWsPatch(event);
      case 'confirm.ask':
        _onConfirm(event);
      default:
        return;
    }
    _scheduleSave();
  }

  void _onSnapshot(JsonMap event) {
    final data = event['data'];
    final seq = event['seq'];
    if (data is! Map || seq is! num) return;
    _resyncPending = false;
    _set(
      _view.copyWith(
        snapshot: deepCopy(data) as Map<String, dynamic>,
        seq: seq.toInt(),
        trimmed: SnapshotTrim.parse(event['trimmed']),
        limit: null,
        stale: false,
        fromCache: false,
        updatedAt: _now(),
      ),
    );
  }

  void _onEvents(JsonMap event) {
    final seq = event['seq'];
    final list = event['events'];
    final current = _view.snapshot;
    if (seq is! num || list is! List) return;
    final direct = event['direct'] == true;
    if (current == null || _view.fromCache) {
      _requestResync();
      return;
    }
    if (!direct) {
      final value = seq.toInt();
      if (value <= _view.seq) return;
      if (value != _view.seq + 1) {
        _requestResync();
        return;
      }
    }
    final snap = Map<String, dynamic>.of(current);
    for (final item in list) {
      applyMonitorEvent(snap, item);
    }
    _set(
      _view.copyWith(
        snapshot: snap,
        seq: direct ? _view.seq : seq.toInt(),
        stale: false,
        updatedAt: _now(),
      ),
    );
  }

  void _onLimit(JsonMap event) {
    final limit = SizeLimit.parse(event);
    if (limit == null) return;
    _set(_view.copyWith(limit: limit));
  }

  void _onChatLast(JsonMap event) {
    final next = nextChatLast(_view.chatLast, event);
    if (next != null) _set(_view.copyWith(chatLast: next));
  }

  void _requestResync() {
    if (_resyncPending) return;
    _resyncPending = true;
    unawaited(resync());
  }

  Future<void> resync() async {
    try {
      await commander('resync');
    } on RemoteCommandException catch (error) {
      _resyncPending = false;
      if (error.code == 'limit') {
        _set(_view.copyWith(limit: const SizeLimit(what: 'snapshot')));
      }
    } catch (_) {
      _resyncPending = false;
    }
  }

  void _onWsSnapshot(JsonMap event) {
    final rev = event['rev'];
    if (rev is! num) return;
    _wsGetPending = false;
    final workspace = <String, dynamic>{
      for (final entry in event.entries)
        if (entry.key != 't') entry.key: deepCopy(entry.value),
    };
    _set(
      _view.copyWith(
        workspace: workspace,
        wsRev: rev.toInt(),
        updatedAt: _now(),
      ),
    );
  }

  void _onWsPatch(JsonMap event) {
    final rev = event['rev'];
    final ops = event['ops'];
    final workspace = _view.workspace;
    if (rev is! num || ops is! List) return;
    final value = rev.toInt();
    final base = event['base'];
    if (workspace == null) {
      _requestWsGet();
      return;
    }
    if (value <= _view.wsRev) return;
    final lost = base is num
        ? base.toInt() != _view.wsRev
        : value > _view.wsRev + 1;
    if (lost) {
      _requestWsGet();
      return;
    }
    try {
      final next = applyJsonPatch(workspace, ops);
      if (next is! Map) throw const PatchException('root');
      final map = next.cast<String, dynamic>();
      map['rev'] = value;
      _set(_view.copyWith(workspace: map, wsRev: value, updatedAt: _now()));
    } on PatchException {
      _requestWsGet();
    }
  }

  void _requestWsGet() {
    if (_wsGetPending) return;
    _wsGetPending = true;
    unawaited(
      commander('ws.get').then(
        (_) {},
        onError: (Object _) {
          _wsGetPending = false;
        },
      ),
    );
  }

  void _onConfirm(JsonMap event) {
    final cid = event['cid'];
    final title = event['title'];
    if (cid is! String || title is! String) return;
    final ask = ConfirmAsk(
      cid: cid,
      title: title,
      detail: event['detail'] as String?,
      ok: (event['ok'] as String?) ?? 'OK',
      cancel: (event['cancel'] as String?) ?? 'Cancel',
      receivedAt: _now(),
    );
    final others = _view.confirms.where((c) => c.cid != cid).toList();
    _set(_view.copyWith(confirms: [...others, ask]));
    _confirmTimers[cid]?.cancel();
    _confirmTimers[cid] = Timer(confirmTtl, () => _dropConfirm(cid));
  }

  void _dropConfirm(String cid) {
    _confirmTimers.remove(cid)?.cancel();
    final remaining = _view.confirms.where((c) => c.cid != cid).toList();
    if (remaining.length != _view.confirms.length) {
      _set(_view.copyWith(confirms: remaining));
    }
  }

  Future<void> answerConfirm(String cid, bool ok) async {
    _dropConfirm(cid);
    await commander('confirm.answer', {'cid': cid, 'ok': ok});
  }

  void _scheduleSave() {
    if (cache == null || _saveTimer?.isActive == true) return;
    _saveTimer = Timer(saveDelay, () => unawaited(flush()));
  }

  Future<void> flush() async {
    _saveTimer?.cancel();
    final target = cache;
    final view = _view;
    if (target == null || (view.snapshot == null && view.workspace == null)) {
      return;
    }
    final tails = {...cachedTails, ...?tailsSource?.call()};
    await target.save(
      machineId,
      CacheEntry(
        savedAt: view.updatedAt ?? _now(),
        seq: view.seq,
        wsRev: view.wsRev,
        snapshot: view.snapshot,
        workspace: view.workspace,
        hello: view.hello,
        tails: tails,
      ),
    );
  }

  Future<void> dispose() async {
    _saveTimer?.cancel();
    for (final timer in _confirmTimers.values) {
      timer.cancel();
    }
    await flush();
    await _changes.close();
  }
}
