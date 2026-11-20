import 'package:flutter/material.dart';

import '../../remote/chat_protocol.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';
import 'chat_controller.dart';
import 'chat_markdown_view.dart';
import 'chat_text.dart';

class MeBubble extends StatelessWidget {
  const MeBubble({super.key, required this.text, this.outbox, this.onResend});

  final String text;
  final OutboxItem? outbox;
  final VoidCallback? onResend;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    final unsent = outbox?.state == OutboxState.unsent;
    final sending = outbox?.state == OutboxState.sending;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.end,
      children: [
        ConstrainedBox(
          constraints: BoxConstraints(
            maxWidth: MediaQuery.sizeOf(context).width * 0.82,
          ),
          child: DecoratedBox(
            decoration: BoxDecoration(
              color: unsent ? null : c.raised,
              borderRadius: BorderRadius.circular(22),
              border: unsent ? Border.all(color: c.lineStrong) : null,
            ),
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 10),
              child: Text(
                text,
                style: TextStyle(
                  fontSize: 16,
                  height: 1.5,
                  color: unsent || sending ? c.inkMuted : c.ink,
                ),
              ),
            ),
          ),
        ),
        if (unsent)
          Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(Icons.error_outline_rounded, size: 15, color: c.waiting),
              const SizedBox(width: 6),
              Text(
                l.chatUnsent,
                style: TextStyle(fontSize: 13, color: c.waiting),
              ),
              TextButton(
                key: ValueKey('chat-resend-${outbox!.id}'),
                onPressed: onResend,
                style: TextButton.styleFrom(
                  minimumSize: const Size(44, 44),
                  foregroundColor: c.ink,
                  shape: const StadiumBorder(),
                ),
                child: Text(
                  l.chatResend,
                  style: const TextStyle(
                    fontSize: 13.5,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
            ],
          ),
      ],
    );
  }
}

class AiText extends StatelessWidget {
  const AiText({super.key, required this.item, this.onTerminal});

  final ChatAi item;
  final VoidCallback? onTerminal;

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        ChatMarkdown(text: item.text),
        if (item.cut)
          Align(
            alignment: Alignment.centerLeft,
            child: TextButton.icon(
              onPressed: onTerminal,
              style: TextButton.styleFrom(
                minimumSize: const Size(44, 44),
                foregroundColor: context.orrery.inkMuted,
              ),
              icon: const Icon(Icons.terminal_rounded, size: 16),
              label: Text(l.chatCutMore),
            ),
          ),
      ],
    );
  }
}

class ToolsLine extends StatefulWidget {
  const ToolsLine({super.key, required this.item});

  final ChatTools item;

  @override
  State<ToolsLine> createState() => _ToolsLineState();
}

class _ToolsLineState extends State<ToolsLine> {
  bool _open = false;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    final item = widget.item;
    final errors = item.items.any((t) => t.err);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        InkWell(
          key: ValueKey('chat-tools-${item.id}'),
          borderRadius: BorderRadius.circular(22),
          onTap: () => setState(() => _open = !_open),
          child: ConstrainedBox(
            constraints: const BoxConstraints(minHeight: 44),
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                Icon(
                  errors ? Icons.error_outline_rounded : Icons.build_outlined,
                  size: 16,
                  color: errors ? c.error : c.inkMuted,
                ),
                const SizedBox(width: 8),
                Flexible(
                  child: Text(
                    toolSummary(l, item.sum),
                    style: TextStyle(fontSize: 14, color: c.inkMuted),
                  ),
                ),
                const SizedBox(width: 4),
                AnimatedRotation(
                  turns: _open ? 0.25 : 0,
                  duration: motion(context, 150),
                  child: Icon(
                    Icons.chevron_right_rounded,
                    size: 18,
                    color: c.inkMuted,
                  ),
                ),
                const SizedBox(width: 8),
              ],
            ),
          ),
        ),
        if (_open)
          Container(
            margin: const EdgeInsets.only(left: 7, bottom: 4),
            padding: const EdgeInsets.only(left: 16, top: 2, bottom: 2),
            decoration: BoxDecoration(
              border: Border(left: BorderSide(color: c.lineStrong, width: 1.5)),
            ),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                for (final tool in item.items) _ToolRow(tool: tool),
                if (item.more > 0)
                  Padding(
                    padding: const EdgeInsets.symmetric(vertical: 3),
                    child: Text(
                      l.chatMore(item.more),
                      style: TextStyle(fontSize: 13.5, color: c.inkMuted),
                    ),
                  ),
              ],
            ),
          ),
      ],
    );
  }
}

class _ToolRow extends StatelessWidget {
  const _ToolRow({required this.tool});

  final ChatTool tool;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final verb = toolVerb(context.l10n, tool.cat, tool.tool);
    final target = tool.target;
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 3),
      child: Text.rich(
        TextSpan(
          style: TextStyle(
            fontSize: 13.5,
            color: tool.err ? c.error : c.inkMuted,
          ),
          children: [
            TextSpan(text: verb),
            if (target != null && target.isNotEmpty) ...[
              const TextSpan(text: ' '),
              TextSpan(
                text: target,
                style: TextStyle(
                  fontFamily: chatMono,
                  fontSize: 12.5,
                  color: tool.err ? c.error : c.ink,
                ),
              ),
            ],
          ],
        ),
      ),
    );
  }
}
