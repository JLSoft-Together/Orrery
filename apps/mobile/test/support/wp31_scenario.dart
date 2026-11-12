import 'package:orrery_remote/remote/link_state.dart';

import 'ui_fakes.dart';

String _ago(Duration d) => DateTime.now().subtract(d).toIso8601String();

Map<String, dynamic> scenarioSnapshot() => dashboard(
  terminals: [
    {
      'id': 't-a',
      'title': 'Dựng mockup UI mobile',
      'status': 'working',
      'provider': 'claude',
      'gitBranch': 'feat/v97-mobile',
      'cwd': 'C:\\work\\claude-code-monitor',
      'statusSince': _ago(const Duration(minutes: 2, seconds: 14)),
    },
    {
      'id': 't-b',
      'title': 'Sửa lỗi pty.overflow',
      'status': 'waiting',
      'provider': 'codex',
      'gitBranch': 'fix/pty-overflow',
      'cwd': 'C:\\work\\claude-code-monitor',
      'waitingFor': 'Xin chạy: npm test -w @ccm/collector',
      'statusSince': _ago(const Duration(minutes: 6)),
    },
    {
      'id': 't-c',
      'title': 'Refactor AdsGate',
      'status': 'working',
      'provider': 'claude',
      'gitBranch': 'main',
      'cwd': 'C:\\work\\ads',
      'statusSince': _ago(const Duration(minutes: 11)),
    },
    {
      'id': 't-d',
      'title': 'npm run dev',
      'status': 'idle',
      'provider': 'shell',
      'gitBranch': 'feat/v97-mobile',
      'cwd': 'C:\\work\\claude-code-monitor',
      'statusSince': _ago(const Duration(minutes: 41)),
    },
    {
      'id': 't-e',
      'title': 'Review PR #482',
      'status': 'idle',
      'provider': 'codex',
      'gitBranch': 'review/pr-482',
      'cwd': 'C:\\work\\claude-code-monitor',
      'statusSince': _ago(const Duration(hours: 1)),
    },
  ],
  agents: [
    {
      'id': 'a1',
      'terminalId': 't-a',
      'role': 'main',
      'status': 'working',
      'model': 'opus',
      'currentTool': 'Edit',
      'currentToolSummary': 'docs/design/index.html',
      'contextPct': 54,
      'totalTokens': 182000,
    },
    {
      'id': 'a2',
      'terminalId': 't-a',
      'role': 'subagent',
      'parentId': 'a1',
      'status': 'working',
      'name': 'Dựng khung điện thoại',
    },
    {
      'id': 'a3',
      'terminalId': 't-a',
      'role': 'subagent',
      'parentId': 'a1',
      'status': 'done',
      'name': 'Đọc token themes.css',
    },
    {'id': 'b1', 'terminalId': 't-b', 'role': 'main', 'status': 'waiting'},
    {'id': 'c1', 'terminalId': 't-c', 'role': 'main', 'status': 'working'},
    for (var i = 0; i < 3; i++)
      {
        'id': 'c-s$i',
        'terminalId': 't-c',
        'role': 'subagent',
        'parentId': 'c1',
        'status': 'working',
        'name': 'sub $i',
      },
  ],
);

Map<String, dynamic> _pane(
  String key,
  String title,
  String rail,
  String agent,
  String branch, {
  String? waitText,
}) => {
  ...pane(
    key,
    title: title,
    rail: rail,
    agent: agent == 'shell' ? null : agent,
    terminalId: 't-$key',
    waitText: waitText,
  ),
  'branch': branch,
};

Map<String, dynamic> scenarioWorkspace() => workspace([
  tab(
    'tab1',
    [
      _pane(
        'a',
        'Dựng mockup UI mobile',
        'working',
        'claude',
        'feat/v97-mobile',
      ),
      _pane(
        'b',
        'Sửa lỗi pty.overflow',
        'asked',
        'codex',
        'fix/pty-overflow',
        waitText: 'Xin chạy: npm test [y/n]',
      ),
      _pane('d', 'npm run dev', 'idle', 'shell', 'feat/v97-mobile'),
    ],
    project: 'C:\\work\\claude-code-monitor',
    name: 'Mobile UI',
  ),
  tab(
    'tab2',
    [
      _pane('c', 'Refactor AdsGate', 'working', 'claude', 'main'),
      _pane('e', 'Review PR #482', 'idle', 'codex', 'review/pr-482'),
      _pane('f', 'flutter test', 'failed', 'shell', 'main'),
    ],
    project: 'C:\\work\\ads',
    name: 'Collector',
  ),
  tab(
    'tab3',
    [_pane('g', 'lint', 'idle', 'shell', 'main')],
    project: 'C:\\work\\ads2',
    name: 'Ads',
  ),
]);

Harness scenarioHarness({LinkState link = linkReady}) => Harness(
  api: FakeMachineApi(),
  viewState: view(ws: scenarioWorkspace(), snapshot: scenarioSnapshot()),
  linkState: link,
  machines: [machine('m1', 'DESKTOP-DAUPH')],
);

const termDemo =
    '\x1b[90m\$ claude --resume 4f2a\x1b[0m\r\n'
    '\x1b[31m●\x1b[0m \x1b[1mĐọc\x1b[0m \x1b[36mapps/mobile/lib/ui/terminal/key_bar.dart\x1b[0m\r\n'
    '\x1b[31m●\x1b[0m \x1b[1mSửa\x1b[0m \x1b[36mdocs/design/v97-mobile-ui/index.html\x1b[0m\r\n'
    '  \x1b[32m+ 214\x1b[0m \x1b[31m- 0\x1b[0m\r\n'
    '\x1b[31m●\x1b[0m \x1b[1mBash\x1b[0m(npm run typecheck)\r\n'
    '  ⎿ 0 lỗi, 2 cảnh báo\r\n\r\n'
    '\x1b[33m✻\x1b[0m Đang dựng khung… (2m 14s · esc)\r\n';
