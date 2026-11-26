import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../remote/link_state.dart';
import '../../remote/providers.dart';
import '../away/away_controller.dart';
import '../away/away_slot.dart';
import '../theme/orrery_shape.dart';
import '../queue/queue_model.dart';
import '../queue/queue_page.dart';
import '../ui_providers.dart';
import '../widgets/common.dart';
import '../workspace/pane_actions.dart';
import 'chat_home_row.dart';
import 'chat_live.dart' show MinuteTicker;
import 'chat_home_sheets.dart';
import 'chat_nav.dart';
import 'chat_rows.dart';
import 'reconnect_strip.dart';

class ChatHomePage extends ConsumerStatefulWidget {
  const ChatHomePage({super.key, required this.machineId});

  final String machineId;

  @override
  ConsumerState<ChatHomePage> createState() => _ChatHomePageState();
}

class _ChatHomePageState extends ConsumerState<ChatHomePage> {
  bool _search = false;
  final TextEditingController _query = TextEditingController();

  @override
  void dispose() {
    _query.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    final id = widget.machineId;
    final model = ref.watch(workspaceModelProvider(id));
    ref.watch(
      machineViewProvider(id).select((v) => terminalTimesKey(v.snapshot)),
    );
    final snapshot = ref.read(machineViewProvider(id)).snapshot;
    final lasts = ref.watch(machineViewProvider(id).select((v) => v.chatLast));
    final online = ref.watch(
      linkProvider(id).select((s) => s.status == LinkStatus.ready),
    );
    final machine = ref
        .watch(machinesProvider)
        .where((m) => m.id == id)
        .firstOrNull;
    final awayIds = ref.watch(awayIdsProvider(id));
    final groups = groupChatRows(
      buildChatRows(model, snapshot, lasts)
          .where((r) => awayIds == null || awayIds.contains(r.pane.terminalId))
          .toList(),
      _query.text,
    );
    Widget rowOf(ChatRow row) => MinuteTicker(
      key: ValueKey('chat-row-tick-${row.pane.key}'),
      builder: (context, now) => Consumer(
        builder: (context, ref, _) => ChatInboxRow(
          row: row,
          facts: ref.watch(paneFactsProvider((id, row.pane.terminalId))),
          now: now,
          onTap: () => openSession(context, ref, id, row.tab, row.pane),
          onLongPress: () =>
              showTabActions(context, ref, id, model, row.tab, row.pane),
        ),
      ),
    );
    final label = TextStyle(
      fontSize: 13,
      fontWeight: FontWeight.w600,
      color: c.inkMuted,
    );
    return Scaffold(
      backgroundColor: c.surface,
      body: SafeArea(
        child: Column(
          children: [
            SizedBox(
              height: 56,
              child: Row(
                children: [
                  const SizedBox(width: 8),
                  Expanded(
                    child: Align(
                      alignment: Alignment.centerLeft,
                      child: _MachinePill(
                        name: machine?.displayName ?? '',
                        online: online,
                      ),
                    ),
                  ),
                  IconButton(
                    key: const ValueKey('home-search'),
                    tooltip: l.chatSearch,
                    isSelected: _search,
                    onPressed: () => setState(() {
                      _search = !_search;
                      if (!_search) _query.clear();
                    }),
                    icon: const Icon(Icons.search_rounded),
                  ),
                  IconButton(
                    key: const ValueKey('home-new-shell'),
                    tooltip: l.chatNewShell,
                    onPressed: online ? () => newShell(context, ref, id) : null,
                    icon: const Icon(Icons.terminal_rounded),
                  ),
                  IconButton(
                    key: const ValueKey('home-more'),
                    tooltip: l.chatMoreTitle,
                    onPressed: () => showHomeMoreSheet(context, ref, id),
                    icon: const Icon(Icons.more_horiz_rounded),
                  ),
                  IconButton(
                    key: const ValueKey('home-new'),
                    tooltip: l.chatNewSession,
                    onPressed: online
                        ? () => showNewSessionSheet(context, ref, id)
                        : null,
                    icon: const Icon(Icons.add_rounded),
                  ),
                  const SizedBox(width: 4),
                ],
              ),
            ),
            ReconnectStrip(machineId: id),
            Expanded(
              child: ListView(
                key: const ValueKey('chat-home-list'),
                padding: const EdgeInsets.fromLTRB(16, 6, 16, 32),
                children: [
                  Padding(
                    padding: const EdgeInsets.fromLTRB(4, 0, 4, 6),
                    child: Text(
                      l.chatInbox,
                      style: TextStyle(
                        fontSize: 30,
                        height: 1.2,
                        fontWeight: FontWeight.w600,
                        letterSpacing: -0.6,
                        color: c.ink,
                      ),
                    ),
                  ),
                  AwaySlot(machineId: id),
                  if (_search)
                    Padding(
                      padding: const EdgeInsets.only(top: 8, bottom: 4),
                      child: TextField(
                        key: const ValueKey('home-search-input'),
                        controller: _query,
                        autofocus: true,
                        onChanged: (_) => setState(() {}),
                        decoration: InputDecoration(
                          hintText: l.chatSearchHint,
                          prefixIcon: const Icon(
                            Icons.search_rounded,
                            size: 20,
                          ),
                          filled: true,
                          fillColor: c.raised,
                          border: OutlineInputBorder(
                            borderRadius: BorderRadius.circular(24),
                            borderSide: BorderSide.none,
                          ),
                        ),
                      ),
                    ),
                  if (groups.waiting.isNotEmpty) ...[
                    Padding(
                      padding: const EdgeInsets.fromLTRB(12, 22, 4, 8),
                      child: Row(
                        children: [
                          Expanded(
                            child: Text(
                              l.chatWaitingGroup(groups.waiting.length),
                              key: const ValueKey('home-group-waiting'),
                              style: label.copyWith(color: c.waiting),
                            ),
                          ),
                          if (queueCount(model) > 0)
                            Flexible(
                              child: TextButton(
                                key: const ValueKey('home-queue'),
                                onPressed: () => openQueue(context, id),
                                child: Text(
                                  l.queueOpen(queueCount(model)),
                                  maxLines: 1,
                                  overflow: TextOverflow.ellipsis,
                                ),
                              ),
                            ),
                        ],
                      ),
                    ),
                    RowCard(
                      accent: c.waiting,
                      children: [for (final r in groups.waiting) rowOf(r)],
                    ),
                  ],
                  if (groups.rest.isNotEmpty) ...[
                    Padding(
                      padding: const EdgeInsets.fromLTRB(12, 22, 12, 8),
                      child: Text(
                        l.chatRecentGroup,
                        key: const ValueKey('home-group-recent'),
                        style: label,
                      ),
                    ),
                    RowCard(children: [for (final r in groups.rest) rowOf(r)]),
                  ],
                  if (groups.isEmpty)
                    Padding(
                      padding: const EdgeInsets.symmetric(vertical: 48),
                      child: Text(
                        _query.text.isEmpty ? l.chatNoSessions : l.chatNoMatch,
                        key: const ValueKey('home-empty'),
                        textAlign: TextAlign.center,
                        style: TextStyle(color: c.inkMuted),
                      ),
                    ),
                ],
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _MachinePill extends StatelessWidget {
  const _MachinePill({required this.name, required this.online});

  final String name;
  final bool online;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return Material(
      color: c.raised,
      shape: const StadiumBorder(),
      clipBehavior: Clip.antiAlias,
      child: InkWell(
        key: const ValueKey('home-machine'),
        onTap: () => Navigator.of(context).maybePop(),
        child: SizedBox(
          height: 44,
          child: Padding(
            padding: const EdgeInsets.fromLTRB(12, 0, 10, 0),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Container(
                  width: 8,
                  height: 8,
                  decoration: BoxDecoration(
                    shape: BoxShape.circle,
                    color: online ? c.done : null,
                    border: online
                        ? null
                        : Border.all(color: c.waiting, width: 1.5),
                  ),
                ),
                const SizedBox(width: 8),
                Flexible(
                  child: Text(
                    name,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                    style: TextStyle(
                      fontSize: 16,
                      fontWeight: FontWeight.w600,
                      color: c.ink,
                    ),
                  ),
                ),
                const SizedBox(width: 4),
                Icon(Icons.expand_more_rounded, size: 18, color: c.inkMuted),
              ],
            ),
          ),
        ),
      ),
    );
  }
}
