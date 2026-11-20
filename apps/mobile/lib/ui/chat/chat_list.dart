import 'package:flutter/material.dart';

import '../../remote/chat_protocol.dart';
import '../commands.dart' show errorText;
import '../../remote/protocol.dart' show RemoteCommandException;
import '../model/pane_facts.dart';
import '../model/workspace_model.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';
import 'chat_ask.dart';
import 'chat_ask_parts.dart';
import 'chat_controller.dart';
import 'chat_glyph.dart';
import 'chat_files.dart';
import 'chat_items.dart';
import 'chat_live.dart';
import 'chat_text.dart';

class ChatList extends StatelessWidget {
  const ChatList({
    super.key,
    required this.controller,
    this.scroll,
    required this.pane,
    required this.facts,
    required this.online,
    required this.agentName,
    required this.project,
    required this.onTerminal,
    required this.onOpenFile,
  });

  final ChatController controller;
  final ScrollController? scroll;
  final PaneInfo pane;
  final PaneFacts facts;
  final bool online;
  final String agentName;
  final String project;
  final VoidCallback onTerminal;
  final void Function(ChatFiles card, ChatFile file) onOpenFile;

  bool _nearTop(ScrollNotification n) {
    final m = n.metrics;
    if (m.axis != Axis.vertical) return false;
    if (m.pixels >= m.maxScrollExtent - 240) {
      controller.loadOlder();
    }
    return false;
  }

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    final chat = controller;
    final entries = <Widget>[];
    if (chat.log.hasOlder) {
      entries.add(
        Center(
          key: const ValueKey('chat-older'),
          child: chat.loadingOlder
              ? const SizedBox.square(
                  dimension: 20,
                  child: CircularProgressIndicator(strokeWidth: 2),
                )
              : TextButton(onPressed: chat.loadOlder, child: Text(l.chatOlder)),
        ),
      );
    }
    for (final item in chat.items) {
      entries.add(_item(item));
    }
    for (final out in chat.outbox) {
      entries.add(
        MeBubble(
          key: ValueKey('chat-out-${out.id}'),
          text: out.text,
          outbox: out,
          onResend: () => chat.resend(out),
        ),
      );
    }
    final menu = pane.menu;
    if (menu != null) {
      entries.add(
        AskCard(
          key: ValueKey('chat-ask-${menu.sig}'),
          menu: menu,
          enabled: online,
          command: facts.toolSummary,
          onAnswer: chat.answer,
          onTerminal: onTerminal,
        ),
      );
    } else if (pane.isAsked) {
      entries.add(
        WaitCard(title: waitLabel(l, pane.waitText), onTerminal: onTerminal),
      );
    }
    if (pane.rail == RailState.working || pane.rail == RailState.subagents) {
      final tool = facts.tool;
      final text = tool == null
          ? l.chatWorking
          : [liveVerb(l, catOfTool(tool), tool), ?facts.toolSummary].join(' ');
      entries.add(
        LiveLine(
          key: const ValueKey('chat-live'),
          text: text,
          since: facts.toolSince ?? facts.statusSince,
        ),
      );
    }
    if (chat.loading && entries.isEmpty) {
      return const Center(child: CircularProgressIndicator(strokeWidth: 2));
    }
    if (chat.error != null && chat.items.isEmpty) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              errorText(l, RemoteCommandException(chat.error!)),
              style: TextStyle(color: c.inkMuted),
            ),
            TextButton(
              key: const ValueKey('chat-retry'),
              onPressed: chat.watch,
              child: Text(l.retry),
            ),
          ],
        ),
      );
    }
    if (entries.isEmpty) {
      return _Greeting(
        agent: pane.agent,
        agentName: agentName,
        project: project,
      );
    }
    final reversed = entries.reversed.toList();
    return NotificationListener<ScrollNotification>(
      onNotification: _nearTop,
      child: ListView.separated(
        key: const ValueKey('chat-list'),
        controller: scroll,
        reverse: true,
        padding: const EdgeInsets.fromLTRB(20, 12, 20, 28),
        itemCount: reversed.length,
        separatorBuilder: (_, _) => const SizedBox(height: 20),
        itemBuilder: (context, i) => RepaintBoundary(child: reversed[i]),
      ),
    );
  }

  Widget _item(ChatItem item) => switch (item) {
    ChatMe() => MeBubble(key: ValueKey('chat-${item.id}'), text: item.text),
    ChatAi() => AiText(
      key: ValueKey('chat-${item.id}'),
      item: item,
      onTerminal: onTerminal,
    ),
    ChatTools() => ToolsLine(key: ValueKey('chat-${item.id}'), item: item),
    ChatFiles() => FilesCard(
      key: ValueKey('chat-${item.id}'),
      item: item,
      onOpen: (file) => onOpenFile(item, file),
    ),
    ChatSys() => SysLine(key: ValueKey('chat-${item.id}'), item: item),
  };
}

class _Greeting extends StatelessWidget {
  const _Greeting({
    required this.agent,
    required this.agentName,
    required this.project,
  });

  final String? agent;
  final String agentName;
  final String project;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    return Center(
      key: const ValueKey('chat-empty'),
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(
              l.chatHello,
              textAlign: TextAlign.center,
              style: TextStyle(
                fontSize: 34,
                height: 1.2,
                fontWeight: FontWeight.w600,
                letterSpacing: -0.6,
                color: c.accent,
              ),
            ),
            const SizedBox(height: 10),
            Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                ChatGlyph(agent: agent, size: 18),
                const SizedBox(width: 6),
                Flexible(
                  child: Text(
                    l.chatAgentAt(agentName, project),
                    style: TextStyle(fontSize: 15, color: c.inkMuted),
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}
