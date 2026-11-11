import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:flutter_riverpod/misc.dart' show Override;
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/l10n/app_localizations.dart';
import 'package:orrery_remote/remote/link_state.dart';
import 'package:orrery_remote/remote/machines.dart';
import 'package:orrery_remote/remote/protocol.dart';
import 'package:orrery_remote/remote/providers.dart';
import 'package:orrery_remote/remote/store.dart';
import 'package:orrery_remote/ui/machine_api.dart';
import 'package:orrery_remote/ui/theme/orrery_theme.dart';
import 'package:orrery_remote/ui/ui_providers.dart';

class FakeHandle implements PtyHandle {
  final StreamController<Uint8List> out = StreamController.broadcast();
  final StreamController<JsonMap> ev = StreamController.broadcast();
  Uint8List initial = Uint8List(0);

  @override
  Stream<Uint8List> get output => out.stream;

  @override
  Stream<JsonMap> get events => ev.stream;

  @override
  Uint8List get tail => initial;
}

class FakeMachineApi implements MachineApi {
  final List<(String, JsonMap?)> commands = [];
  final List<Uint8List> inputs = [];
  final List<bool> claims = [];
  final List<(int, int)> resizes = [];
  final List<FakeHandle> handles = [];
  final List<String> attached = [];
  final List<String> detached = [];
  final List<(String, bool)> answers = [];
  Object? commandError;
  Object? attachError;
  JsonMap commandReply = {'paneKey': 'new'};
  int retries = 0;
  bool cleared = false;
  Uint8List? tail;

  @override
  Future<JsonMap> command(String type, [JsonMap? fields]) async {
    commands.add((type, fields));
    final error = commandError;
    if (error != null) throw error;
    return commandReply;
  }

  @override
  Future<PtyHandle> attach(
    String sessionId,
    int cols,
    int rows, {
    required String paneKey,
  }) async {
    final error = attachError;
    if (error != null) throw error;
    attached.add(paneKey);
    final handle = FakeHandle();
    if (tail != null) handle.initial = tail!;
    handles.add(handle);
    return handle;
  }

  @override
  Future<void> detach(PtyHandle handle) async {
    detached.add('${handles.indexOf(handle as FakeHandle)}');
  }

  @override
  void input(PtyHandle handle, Uint8List bytes, {bool claim = false}) {
    inputs.add(bytes);
    claims.add(claim);
  }

  @override
  Future<void> resize(PtyHandle handle, int cols, int rows) async =>
      resizes.add((cols, rows));

  @override
  Uint8List? cachedTail(String paneKey) => tail;

  @override
  Future<void> answerConfirm(String cid, bool ok) async =>
      answers.add((cid, ok));

  @override
  Future<void> retry() async => retries++;

  @override
  Future<void> clearCache() async => cleared = true;
}

class FakeViewNotifier extends MachineViewNotifier {
  FakeViewNotifier(super.machineId, this.initial);

  final MachineView initial;

  @override
  MachineView build() => initial;

  void emit(MachineView view) => state = view;
}

class FakeLinkNotifier extends LinkNotifier {
  FakeLinkNotifier(super.machineId, this.initial);

  final LinkState initial;

  @override
  LinkState build() => initial;

  void emit(LinkState link) => state = link;
}

class FakeMachines extends MachinesNotifier {
  FakeMachines(this.list);

  final List<Machine> list;
  final List<String> forgotten = [];

  @override
  List<Machine> build() => list;

  @override
  Future<void> forget(String machineId) async {
    forgotten.add(machineId);
    state = state.where((m) => m.id != machineId).toList();
  }
}

Machine machine(String id, String name) =>
    Machine(id: id, name: name, mkey: Uint8List(32), pairedAt: 0);

Map<String, dynamic> pane(
  String key, {
  String title = 'claude',
  String rail = 'idle',
  String kind = 'term',
  String? agent = 'claude',
  String? attention,
  String? waitText,
  String? terminalId,
  int subagents = 0,
}) => {
  'key': key,
  'kind': kind,
  'title': title,
  'state': 'running',
  'rail': rail,
  'subagents': subagents,
  'dirty': false,
  'busy': false,
  'agent': ?agent,
  'sessionId': kind == 'term' ? 'sess-$key' : null,
  'terminalId': terminalId ?? 't-$key',
  'attention': ?attention,
  'waitText': ?waitText,
};

Map<String, dynamic> tab(
  String key,
  List<Map<String, dynamic>> panes, {
  String? project,
  String? name,
  Object? tree,
}) => {
  'key': key,
  'project': ?project,
  'name': ?name,
  'tree': tree ?? _chain(panes),
  'panes': panes,
};

Object _chain(List<Map<String, dynamic>> panes) {
  Object node = {'pane': panes.first['key']};
  for (final p in panes.skip(1)) {
    node = {
      'dir': 'row',
      'ratio': 0.5,
      'a': node,
      'b': {'pane': p['key']},
    };
  }
  return node;
}

Map<String, dynamic> workspace(
  List<Map<String, dynamic>> tabs, {
  int closed = 0,
}) => {'rev': 1, 'tabs': tabs, 'closedCount': closed};

Map<String, dynamic> dashboard({
  List<Map<String, dynamic>> agents = const [],
  List<Map<String, dynamic>> terminals = const [],
  List<Map<String, dynamic>> favorites = const [],
}) => {
  'protocolVersion': 1,
  'terminals': terminals,
  'agents': agents,
  'activity': [],
  'favorites': favorites,
};

MachineView view({
  Map<String, dynamic>? ws,
  Map<String, dynamic>? snapshot,
  bool stale = false,
  DateTime? updatedAt,
  List<ConfirmAsk> confirms = const [],
  Map<String, dynamic>? hello,
}) => MachineView(
  workspace: ws,
  snapshot: snapshot,
  stale: stale,
  updatedAt: updatedAt ?? DateTime(2026, 11, 12, 9, 30),
  confirms: confirms,
  hello: hello,
);

class Harness {
  Harness({
    required this.api,
    required this.viewState,
    required this.linkState,
    this.id = 'm1',
    this.machines,
  });

  final FakeMachineApi api;
  final MachineView viewState;
  final LinkState linkState;
  final String id;
  final List<Machine>? machines;
  late ProviderContainer container;

  FakeViewNotifier get viewNotifier =>
      container.read(machineViewProvider(id).notifier) as FakeViewNotifier;

  FakeLinkNotifier get linkNotifier =>
      container.read(linkProvider(id).notifier) as FakeLinkNotifier;

  List<Override> get overrides => [
    machineApiProvider(id).overrideWithValue(api),
    machineViewProvider(id).overrideWith(() => FakeViewNotifier(id, viewState)),
    linkProvider(id).overrideWith(() => FakeLinkNotifier(id, linkState)),
    machinesProvider.overrideWith(
      () => FakeMachines(machines ?? [machine(id, 'Dev PC')]),
    ),
  ];

  Future<void> pump(
    WidgetTester tester,
    Widget home, {
    Size size = const Size(390, 844),
    Brightness brightness = Brightness.light,
    Locale locale = const Locale('en'),
    GlobalKey? boundary,
    RemoteTheme remote = RemoteTheme.fallback,
    String? fontFamily,
  }) async {
    tester.view.physicalSize = size;
    tester.view.devicePixelRatio = 1;
    addTearDown(() async {
      await tester.pumpWidget(const SizedBox());
      tester.view.reset();
    });
    await tester.pumpWidget(
      UncontrolledProviderScope(
        container: container = ProviderContainer(overrides: overrides),
        child: RepaintBoundary(
          key: boundary,
          child: MaterialApp(
            debugShowCheckedModeBanner: false,
            theme: buildOrreryTheme(remote, brightness, fontFamily: fontFamily),
            localizationsDelegates: AppLocalizations.localizationsDelegates,
            supportedLocales: AppLocalizations.supportedLocales,
            locale: locale,
            home: home,
          ),
        ),
      ),
    );
    await tester.pump();
  }
}

const LinkState linkReady = LinkState(
  status: LinkStatus.ready,
  route: LinkRoute.lan,
);

const LinkState linkOffline = LinkState(status: LinkStatus.offline);
