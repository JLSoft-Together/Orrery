import 'package:orrery_remote/remote/link_state.dart';
import 'package:orrery_remote/remote/store.dart';

import 'ui_fakes.dart';

Map<String, dynamic> sample() => workspace([
  tab('t1', [
    pane('a', title: 'refactor', rail: 'working'),
    pane('b', title: 'tests', rail: 'asked', waitText: 'Bash: rm -rf build'),
  ], project: 'C:\\work\\alpha'),
  tab('t2', [
    pane('c', title: 'docs', rail: 'done', attention: 'done'),
  ], project: 'C:\\work\\beta'),
], closed: 1);

Harness harness({
  Map<String, dynamic>? ws,
  LinkState link = linkReady,
  Map<String, dynamic>? snapshot,
  bool stale = false,
  List<ConfirmAsk> confirms = const [],
}) => Harness(
  api: FakeMachineApi(),
  viewState: view(
    ws: ws ?? sample(),
    snapshot: snapshot ?? dashboard(),
    stale: stale,
    confirms: confirms,
  ),
  linkState: link,
);
