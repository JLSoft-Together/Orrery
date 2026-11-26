import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../media/media_flow.dart';
import '../../media/media_prefs.dart';
import '../../media/media_providers.dart';
import '../../prompts/prompt_model.dart';
import '../../prompts/prompts_api.dart';
import '../../prompts/prompts_controller.dart';
import '../../remote/chat_protocol.dart';
import '../../remote/desktop_caps.dart';
import '../../remote/link_state.dart';
import '../../remote/providers.dart';
import '../commands.dart';
import '../danger_guard.dart';
import '../model/workspace_model.dart';
import '../run/crash_draft.dart';
import '../terminal/terminal_pane.dart';
import '../theme/orrery_shape.dart';
import '../ui_providers.dart';
import '../viewers/diff_page.dart';
import '../viewers/viewer_api.dart';
import '../widgets/common.dart';
import '../workspace/machine_scope.dart';
import 'chat_composer.dart';
import 'chat_controller.dart';
import 'chat_header.dart';
import 'chat_list.dart';
import 'chat_media.dart';
import 'chat_rows.dart';
import 'chat_sheets.dart';
import 'reconnect_strip.dart';

class ChatScreen extends ConsumerStatefulWidget {
  const ChatScreen({
    super.key,
    required this.machineId,
    required this.paneKey,
    this.terminal = false,
  });

  final String machineId;
  final String paneKey;
  final bool terminal;

  @override
  ConsumerState<ChatScreen> createState() => _ChatScreenState();
}

class _ChatScreenState extends ConsumerState<ChatScreen> {
  late final ChatController _chat;
  final GlobalKey<ChatComposerState> _composer = GlobalKey();
  late bool _termMode = widget.terminal;
  late final AppLifecycleListener _life;
  final ScrollController _scroll = ScrollController();
  MediaFlow? _flow;
  PromptsController? _prompts;

  ComposerMedia _media(String? root) {
    final hasPut = ref.watch(
      machineViewProvider(widget.machineId)
          .select((v) => hasDesktopCap(v.hello, putCap)),
    );
    final dictation = ref.watch(dictationProvider);
    if (!hasPut || root == null || root.isEmpty) {
      return ComposerMedia(dictation: dictation);
    }
    final flow = _flow ??= MediaFlow(
      call: ref.read(viewerApiProvider(widget.machineId)).call,
      root: root,
      prefs: ref.read(mediaPrefsProvider),
    );
    flow.root = root;
    return ComposerMedia(dictation: dictation, photo: flow);
  }

  PromptsController? _promptsFor(bool enabled) {
    if (!enabled) return null;
    return _prompts ??= PromptsController(
      api: PromptsApi(ref.read(viewerApiProvider(widget.machineId))),
      paneKey: widget.paneKey,
    );
  }

  @override
  void initState() {
    super.initState();
    _chat = ChatController(
      api: ref.read(machineApiProvider(widget.machineId)),
      paneKey: widget.paneKey,
      cacheKey: '${widget.machineId}|${widget.paneKey}',
      reauthorize: _reauthorize,
    )..addListener(_onChat);
    _chat.start();
    WidgetsBinding.instance.addPostFrameCallback((_) => _takeCrashDraft());
    _life = AppLifecycleListener(
      onPause: _chat.pause,
      onHide: _chat.pause,
      onShow: _chat.resume,
      onResume: _chat.resume,
    );
  }

  void _takeCrashDraft() {
    if (!mounted) return;
    final composer = _composer.currentState;
    if (composer == null) return;
    final text = ref.read(crashDraftProvider.notifier).take(widget.paneKey);
    if (text != null) composer.fill(text);
  }

  void _onChat() {
    if (!mounted) return;
    if (_chat.unsupported && !_termMode) _termMode = true;
    setState(() {});
  }

  @override
  void dispose() {
    _life.dispose();
    _scroll.dispose();
    _prompts?.dispose();
    _chat
      ..removeListener(_onChat)
      ..dispose();
    super.dispose();
  }

  Future<bool> _send(String text, bool authed) {
    if (_scroll.hasClients && _scroll.offset > 0) {
      _scroll.animateTo(
        0,
        duration: const Duration(milliseconds: 220),
        curve: Curves.easeOutCubic,
      );
    }
    return _chat.send(text, authed: authed);
  }

  Future<bool> _reauthorize(String text) =>
      mounted ? authorizeNow(context) : Future.value(false);

  void _toggle() => setState(() => _termMode = !_termMode);

  void _showTerminal() => setState(() => _termMode = true);

  int _since(PaneInfo pane) =>
      ref
          .read(paneFactsProvider((widget.machineId, pane.terminalId)))
          .statusSince
          ?.millisecondsSinceEpoch ??
      0;

  Future<void> _stop(PaneInfo pane) async {
    final l = context.l10n;
    try {
      await _chat.stop(_since(pane));
      if (mounted) toast(context, l.chatStopped);
    } on Object catch (error) {
      if (mounted) toast(context, errorText(l, error));
    }
  }

  void _openFile(ChatFiles card, ChatFile file) {
    FocusManager.instance.primaryFocus?.unfocus();
    Navigator.of(context).push(
      machineRoute<void>(
        widget.machineId,
        (_) => DiffPage(
          machineId: widget.machineId,
          path: file.path,
          type: 'git.diff',
          fields: {'root': card.root, 'path': file.path},
          root: card.root,
        ),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    final id = widget.machineId;
    final model = ref.watch(workspaceModelProvider(id));
    final tab = model.tabOfPane(widget.paneKey);
    final pane = tab?.pane(widget.paneKey);
    if (tab == null || pane == null) {
      return Scaffold(
        appBar: AppBar(),
        body: Center(
          child: Text(
            l.chatGone,
            key: const ValueKey('chat-gone'),
            style: TextStyle(color: c.inkMuted),
          ),
        ),
      );
    }
    final pct = ref.watch(
      machineViewProvider(id)
          .select((v) => contextPctOf(v.snapshot, pane.terminalId)),
    );
    final facts = ref.watch(paneFactsProvider((id, pane.terminalId)));
    final online = ref.watch(
      linkProvider(id).select((s) => s.status == LinkStatus.ready),
    );
    final project = projectOf(tab, pane);
    final canChat = pane.isAgent && !_chat.unsupported;
    final working =
        pane.rail == RailState.working || pane.rail == RailState.subagents;
    final empty =
        !_chat.loading &&
        _chat.error == null &&
        _chat.items.isEmpty &&
        _chat.outbox.isEmpty;
    final agentName = pane.agent == 'codex' ? 'Codex' : 'Claude';
    final hasPrompts = ref.watch(
      machineViewProvider(id).select((v) => hasDesktopCap(v.hello, promptsCap)),
    );
    return Scaffold(
      backgroundColor: c.surface,
      body: SafeArea(
        bottom: _termMode,
        child: Column(
          children: [
            ChatHeader(
              machineId: id,
              pane: pane,
              project: project,
              termMode: _termMode,
              onToggleTerm: canChat ? _toggle : null,
              contextPct: pane.isAgent ? pct : null,
              onMore: () => showChatSessionSheet(
                context,
                ref,
                machineId: id,
                model: model,
                tab: tab,
                pane: pane,
                termMode: _termMode,
                onToggleTerm: canChat ? _toggle : null,
              ),
            ),
            if (!_termMode) ReconnectStrip(machineId: id, showLatency: false),
            Expanded(
              child: _termMode
                  ? TerminalPane(
                      key: ValueKey('chat-terminal-${pane.key}'),
                      machineId: id,
                      tabKey: tab.key,
                      pane: pane,
                      compactHeader: true,
                    )
                  : ChatList(
                      controller: _chat,
                      scroll: _scroll,
                      pane: pane,
                      facts: facts,
                      online: online,
                      agentName: agentName,
                      project: project,
                      onTerminal: _showTerminal,
                      onOpenFile: _openFile,
                    ),
            ),
            if (!_termMode)
              ChatComposer(
                key: _composer,
                hint: pane.isAsked
                    ? l.chatReplyTo(agentName)
                    : l.chatMessage(agentName),
                enabled: true,
                working: working,
                onSend: _send,
                danger: ref.watch(dangerRulesProvider(id)),
                media: _media(tab.project ?? pane.cwd),
                prompts: _promptsFor(hasPrompts),
                projectPath: tab.project ?? pane.cwd,
                onStop: () => _stop(pane),
                onPlus: () => showChatPlusSheet(
                  context,
                  ref,
                  machineId: id,
                  cwd: tab.project ?? pane.cwd,
                  onTerminal: _showTerminal,
                ),
                chips: empty
                    ? [
                        l.chatChipSummary,
                        l.chatChipTest,
                        l.chatChipGit,
                        l.chatChipTypecheck,
                      ]
                    : const [],
              ),
          ],
        ),
      ),
    );
  }
}
