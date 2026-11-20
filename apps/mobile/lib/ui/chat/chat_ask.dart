import 'dart:async';

import 'package:flutter/material.dart';

import '../../remote/chat_protocol.dart';
import '../commands.dart' show errorText, toast;
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';
import 'chat_ask_parts.dart';
import 'chat_markdown_view.dart' show chatMono;

typedef MenuAnswer = Future<void> Function(
  RemoteMenu menu,
  RemoteMenuItem item,
);

class AskCard extends StatefulWidget {
  const AskCard({
    super.key,
    required this.menu,
    required this.enabled,
    required this.onAnswer,
    required this.onTerminal,
    this.command,
    this.undo = const Duration(seconds: 3),
  });

  final RemoteMenu menu;
  final bool enabled;
  final MenuAnswer onAnswer;
  final VoidCallback onTerminal;
  final String? command;
  final Duration undo;

  @override
  State<AskCard> createState() => _AskCardState();
}

class _AskCardState extends State<AskCard> {
  RemoteMenuItem? _pending;
  String? _pendingSig;
  Timer? _timer;
  String? _sentLabel;
  String? _sentSig;
  bool _sending = false;

  @override
  void didUpdateWidget(AskCard old) {
    super.didUpdateWidget(old);
    if (_pendingSig != null && _pendingSig != widget.menu.sig) _cancel();
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  void _pick(RemoteMenuItem item) {
    if (item.other) {
      widget.onTerminal();
      return;
    }
    _timer?.cancel();
    setState(() {
      _pending = item;
      _pendingSig = widget.menu.sig;
    });
    _timer = Timer(widget.undo, _commit);
  }

  void _cancel() {
    _timer?.cancel();
    _timer = null;
    if (!mounted) return;
    setState(() {
      _pending = null;
      _pendingSig = null;
    });
  }

  Future<void> _commit() async {
    final item = _pending;
    final menu = widget.menu;
    if (item == null || _pendingSig != menu.sig) return;
    setState(() => _sending = true);
    final l = context.l10n;
    try {
      await widget.onAnswer(menu, item);
      if (!mounted) return;
      setState(() {
        _sentLabel = item.label;
        _sentSig = menu.sig;
        _pending = null;
        _pendingSig = null;
        _sending = false;
      });
    } on Object catch (error) {
      if (!mounted) return;
      setState(() {
        _pending = null;
        _pendingSig = null;
        _sending = false;
      });
      toast(context, errorText(l, error));
    }
  }

  @override
  Widget build(BuildContext context) {
    final menu = widget.menu;
    if (_sentSig == menu.sig && _sentLabel != null) {
      return SentLine(label: _sentLabel!);
    }
    final pending = _pending;
    if (pending != null) {
      return PendingCard(
        label: pending.label,
        undo: widget.undo,
        busy: _sending,
        onUndo: _sending ? null : _cancel,
      );
    }
    return _MenuCard(
      menu: menu,
      command: widget.command,
      enabled: widget.enabled,
      onPick: _pick,
    );
  }
}

bool _isPermission(RemoteMenu menu) =>
    menu.kind == RemoteMenuKind.permission ||
    menu.kind == RemoteMenuKind.codexApproval;

class _MenuCard extends StatelessWidget {
  const _MenuCard({
    required this.menu,
    required this.enabled,
    required this.onPick,
    this.command,
  });

  final RemoteMenu menu;
  final bool enabled;
  final ValueChanged<RemoteMenuItem> onPick;
  final String? command;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    final permission = _isPermission(menu);
    final title = menu.question.isNotEmpty
        ? menu.question
        : (permission ? l.chatAskPermission : l.chatAskChoose);
    final cmd = command;
    return AskFrame(
      icon: permission ? Icons.lock_outline_rounded : Icons.list_rounded,
      title: title,
      children: [
        if (permission && cmd != null && cmd.isNotEmpty)
          Container(
            margin: const EdgeInsets.only(top: 12),
            padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
            decoration: BoxDecoration(
              color: c.surface,
              borderRadius: BorderRadius.circular(14),
            ),
            child: Text(
              cmd,
              style: TextStyle(
                fontFamily: chatMono,
                fontSize: 14,
                color: c.ink,
              ),
            ),
          ),
        const SizedBox(height: 12),
        for (var k = 0; k < menu.items.length; k++)
          Padding(
            padding: EdgeInsets.only(top: k == 0 ? 0 : 6),
            child: AskButton(
              key: ValueKey('chat-menu-${menu.items[k].index}'),
              item: menu.items[k],
              numbered: !permission,
              primary: permission ? k == 0 : menu.items[k].submit,
              onPressed: enabled ? () => onPick(menu.items[k]) : null,
            ),
          ),
      ],
    );
  }
}

class AskFrame extends StatelessWidget {
  const AskFrame({
    super.key,
    required this.icon,
    required this.title,
    required this.children,
  });

  final IconData icon;
  final String title;
  final List<Widget> children;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return Container(
      padding: const EdgeInsets.all(16),
      decoration: BoxDecoration(
        color: c.raised,
        borderRadius: BorderRadius.circular(24),
        border: Border.all(color: c.line),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: [
              Padding(
                padding: const EdgeInsets.only(top: 2),
                child: Icon(icon, size: 16, color: c.waiting),
              ),
              const SizedBox(width: 8),
              Expanded(
                child: Text(
                  title,
                  style: TextStyle(
                    fontSize: 14,
                    fontWeight: FontWeight.w600,
                    color: c.waiting,
                  ),
                ),
              ),
            ],
          ),
          ...children,
        ],
      ),
    );
  }
}

class AskButton extends StatelessWidget {
  const AskButton({
    super.key,
    required this.item,
    required this.numbered,
    required this.primary,
    required this.onPressed,
  });

  final RemoteMenuItem item;
  final bool numbered;
  final bool primary;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    final fg = primary ? c.surface : c.ink;
    final checked = item.checked;
    return Opacity(
      opacity: onPressed == null ? 0.45 : 1,
      child: Material(
        color: primary ? c.ink : c.surface,
        shape: StadiumBorder(
          side: primary ? BorderSide.none : BorderSide(color: c.lineStrong),
        ),
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          onTap: onPressed,
          child: ConstrainedBox(
            constraints: const BoxConstraints(minHeight: 46),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 14, vertical: 8),
              child: Row(
                children: [
                  if (checked != null) ...[
                    Icon(
                      checked
                          ? Icons.check_box_rounded
                          : Icons.check_box_outline_blank_rounded,
                      size: 20,
                      color: fg,
                    ),
                    const SizedBox(width: 10),
                  ] else if (numbered && !item.other) ...[
                    Container(
                      width: 24,
                      height: 24,
                      alignment: Alignment.center,
                      decoration: BoxDecoration(
                        color: c.raised,
                        shape: BoxShape.circle,
                      ),
                      child: Text(
                        '${item.index}',
                        style: TextStyle(
                          fontSize: 13,
                          fontWeight: FontWeight.w600,
                          color: c.inkMuted,
                        ),
                      ),
                    ),
                    const SizedBox(width: 10),
                  ],
                  if (item.other) ...[
                    Icon(Icons.terminal_rounded, size: 18, color: fg),
                    const SizedBox(width: 10),
                  ],
                  Expanded(
                    child: Text(
                      item.other
                          ? '${item.label} (${l.chatOpenTerminal})'
                          : item.label,
                      textAlign: numbered || checked != null || item.other
                          ? TextAlign.start
                          : TextAlign.center,
                      style: TextStyle(
                        fontSize: 15,
                        fontWeight: FontWeight.w500,
                        color: fg,
                      ),
                    ),
                  ),
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
