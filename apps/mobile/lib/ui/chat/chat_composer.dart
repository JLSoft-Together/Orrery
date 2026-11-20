import 'package:flutter/material.dart';

import '../theme/orrery_shape.dart';
import '../widgets/common.dart';

class ChatComposer extends StatefulWidget {
  const ChatComposer({
    super.key,
    required this.hint,
    required this.enabled,
    required this.working,
    required this.onSend,
    required this.onStop,
    this.onPlus,
    this.chips = const [],
  });

  final String hint;
  final bool enabled;
  final bool working;
  final Future<bool> Function(String text) onSend;
  final VoidCallback onStop;
  final VoidCallback? onPlus;
  final List<String> chips;

  @override
  State<ChatComposer> createState() => ChatComposerState();
}

class ChatComposerState extends State<ChatComposer> {
  final TextEditingController _text = TextEditingController();
  final FocusNode _focus = FocusNode();

  @override
  void initState() {
    super.initState();
    _text.addListener(() => setState(() {}));
  }

  @override
  void dispose() {
    _text.dispose();
    _focus.dispose();
    super.dispose();
  }

  void fill(String value) {
    _text.text = value;
    _text.selection = TextSelection.collapsed(offset: value.length);
    _focus.requestFocus();
  }

  Future<void> _send() async {
    final value = _text.text.trim();
    if (value.isEmpty) return;
    _text.clear();
    await widget.onSend(value);
  }

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    final hasText = _text.text.trim().isNotEmpty;
    final stop = widget.working && !hasText;
    final canSend = widget.enabled && hasText;
    return Container(
      color: c.surface,
      padding: EdgeInsets.fromLTRB(
        12,
        4,
        12,
        10 + MediaQuery.paddingOf(context).bottom,
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          if (widget.chips.isNotEmpty)
            Padding(
              padding: const EdgeInsets.only(bottom: 10, left: 2, right: 2),
              child: Wrap(
                spacing: 8,
                runSpacing: 8,
                children: [
                  for (final chip in widget.chips)
                    ActionChip(
                      key: ValueKey('chat-chip-$chip'),
                      label: Text(chip),
                      onPressed: widget.enabled
                          ? () => widget.onSend(chip)
                          : null,
                      shape: const StadiumBorder(),
                      side: BorderSide(color: c.line),
                      backgroundColor: c.raised,
                      labelStyle: TextStyle(
                        fontSize: 14,
                        fontWeight: FontWeight.w500,
                        color: c.ink,
                      ),
                      materialTapTargetSize: MaterialTapTargetSize.padded,
                    ),
                ],
              ),
            ),
          Container(
            padding: const EdgeInsets.all(6),
            decoration: BoxDecoration(
              color: c.control,
              borderRadius: BorderRadius.circular(30),
              border: Border.all(
                color: _focus.hasFocus
                    ? c.accent.withValues(alpha: 0.55)
                    : c.lineStrong,
              ),
              boxShadow: [
                BoxShadow(
                  color: Colors.black.withValues(alpha: 0.08),
                  blurRadius: 24,
                  offset: const Offset(0, 6),
                  spreadRadius: -12,
                ),
              ],
            ),
            child: Row(
              crossAxisAlignment: CrossAxisAlignment.end,
              children: [
                if (widget.onPlus != null)
                  _RoundButton(
                    key: const ValueKey('chat-plus'),
                    icon: Icons.add_rounded,
                    tooltip: l.chatPlus,
                    background: c.raised,
                    foreground: c.inkMuted,
                    onPressed: widget.onPlus,
                  ),
                const SizedBox(width: 4),
                Expanded(
                  child: Focus(
                    onFocusChange: (_) => setState(() {}),
                    child: TextField(
                      key: const ValueKey('chat-input'),
                      controller: _text,
                      focusNode: _focus,
                      enabled: widget.enabled,
                      minLines: 1,
                      maxLines: 5,
                      maxLength: 2000,
                      textInputAction: TextInputAction.newline,
                      style: TextStyle(fontSize: 16, height: 1.5, color: c.ink),
                      decoration: InputDecoration(
                        hintText: widget.hint,
                        hintStyle: TextStyle(color: c.inkFaint),
                        border: InputBorder.none,
                        enabledBorder: InputBorder.none,
                        focusedBorder: InputBorder.none,
                        disabledBorder: InputBorder.none,
                        filled: false,
                        counterText: '',
                        isDense: true,
                        contentPadding: const EdgeInsets.symmetric(
                          horizontal: 4,
                          vertical: 10,
                        ),
                      ),
                    ),
                  ),
                ),
                const SizedBox(width: 4),
                if (stop)
                  _RoundButton(
                    key: const ValueKey('chat-stop'),
                    icon: Icons.stop_rounded,
                    tooltip: l.chatStop,
                    background: c.ink,
                    foreground: c.surface,
                    onPressed: widget.enabled ? widget.onStop : null,
                  )
                else
                  _RoundButton(
                    key: const ValueKey('chat-send'),
                    icon: Icons.arrow_upward_rounded,
                    tooltip: l.chatSend,
                    background: canSend ? c.accent : c.raised,
                    foreground: canSend ? c.onAccent : c.inkFaint,
                    onPressed: canSend ? _send : null,
                  ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

class _RoundButton extends StatelessWidget {
  const _RoundButton({
    super.key,
    required this.icon,
    required this.tooltip,
    required this.background,
    required this.foreground,
    required this.onPressed,
  });

  final IconData icon;
  final String tooltip;
  final Color background;
  final Color foreground;
  final VoidCallback? onPressed;

  @override
  Widget build(BuildContext context) {
    return Tooltip(
      message: tooltip,
      child: Material(
        color: background,
        shape: const CircleBorder(),
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          onTap: onPressed,
          child: SizedBox.square(
            dimension: 44,
            child: Icon(icon, size: 21, color: foreground),
          ),
        ),
      ),
    );
  }
}
