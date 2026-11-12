import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:xterm/xterm.dart';

import '../theme/orrery_shape.dart';
import '../widgets/common.dart';

class ExtraKeysRow extends StatelessWidget {
  const ExtraKeysRow({
    super.key,
    required this.terminal,
    required this.enabled,
    required this.onUse,
  });

  final Terminal terminal;
  final bool enabled;
  final VoidCallback onUse;

  void _tap(TerminalKey key, {bool shift = false, bool ctrl = false}) {
    HapticFeedback.selectionClick();
    terminal.keyInput(key, shift: shift, ctrl: ctrl);
    onUse();
  }

  @override
  Widget build(BuildContext context) {
    final keys = <(Object, String, VoidCallback)>[
      ('Esc', 'esc', () => _tap(TerminalKey.escape)),
      ('Tab', 'tab', () => _tap(TerminalKey.tab)),
      ('Shift+Tab', 'shift-tab', () => _tap(TerminalKey.tab, shift: true)),
      ('^C', 'ctrl-c', () => _tap(TerminalKey.keyC, ctrl: true)),
      (Icons.arrow_upward_rounded, 'up', () => _tap(TerminalKey.arrowUp)),
      (Icons.arrow_downward_rounded, 'down', () => _tap(TerminalKey.arrowDown)),
      (Icons.arrow_back_rounded, 'left', () => _tap(TerminalKey.arrowLeft)),
      (
        Icons.arrow_forward_rounded,
        'right',
        () => _tap(TerminalKey.arrowRight),
      ),
      (Icons.keyboard_return_rounded, 'enter', () => _tap(TerminalKey.enter)),
    ];
    return SizedBox(
      height: 52,
      child: ListView.separated(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 6),
        itemCount: keys.length,
        separatorBuilder: (_, _) => const SizedBox(width: 8),
        itemBuilder: (context, index) {
          final (label, id, action) = keys[index];
          return _KeyButton(
            id: id,
            label: label,
            onTap: enabled ? action : null,
          );
        },
      ),
    );
  }
}

class _KeyButton extends StatefulWidget {
  const _KeyButton({
    required this.id,
    required this.label,
    required this.onTap,
  });

  final String id;
  final Object label;
  final VoidCallback? onTap;

  @override
  State<_KeyButton> createState() => _KeyButtonState();
}

class _KeyButtonState extends State<_KeyButton> {
  bool _down = false;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final enabled = widget.onTap != null;
    final fg = enabled ? c.ink : c.inkFaint;
    final label = widget.label;
    return Semantics(
      button: true,
      label: widget.id,
      child: AnimatedScale(
        scale: _down && !reduceMotion(context) ? 0.94 : 1,
        duration: const Duration(milliseconds: 90),
        child: Material(
          color: _down ? c.accent.withValues(alpha: 0.12) : c.control,
          shape: RoundedRectangleBorder(
            borderRadius: context.shape.controlRadius,
            side: BorderSide(
              color: _down ? c.accent.withValues(alpha: 0.5) : c.lineStrong,
            ),
          ),
          clipBehavior: Clip.antiAlias,
          child: InkWell(
            key: ValueKey('key-${widget.id}'),
            onTap: widget.onTap,
            onHighlightChanged: (v) => setState(() => _down = v),
            child: ConstrainedBox(
              constraints: const BoxConstraints(minWidth: 44, minHeight: 40),
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 12),
                child: Center(
                  child: label is IconData
                      ? Icon(label, size: 20, color: fg)
                      : Text(
                          label as String,
                          style: TextStyle(
                            fontFamily: 'OrreryMono',
                            fontSize: 14,
                            fontWeight: FontWeight.w600,
                            color: fg,
                          ),
                        ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class InputBar extends StatefulWidget {
  const InputBar({
    super.key,
    required this.terminal,
    required this.enabled,
    required this.direct,
    required this.onToggleDirect,
    required this.onFocusChange,
    required this.onSent,
  });

  final Terminal terminal;
  final bool enabled;
  final bool direct;
  final VoidCallback onToggleDirect;
  final ValueChanged<bool> onFocusChange;
  final VoidCallback onSent;

  @override
  State<InputBar> createState() => _InputBarState();
}

class _InputBarState extends State<InputBar> {
  final TextEditingController _text = TextEditingController();
  final FocusNode _focus = FocusNode();

  @override
  void initState() {
    super.initState();
    _focus.addListener(() => widget.onFocusChange(_focus.hasFocus));
    _text.addListener(() => setState(() {}));
  }

  @override
  void dispose() {
    _text.dispose();
    _focus.dispose();
    super.dispose();
  }

  void _send() {
    final value = _text.text;
    if (!widget.enabled || value.trim().isEmpty) return;
    HapticFeedback.lightImpact();
    widget.terminal.paste(value);
    widget.terminal.keyInput(TerminalKey.enter);
    _text.clear();
    widget.onSent();
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final c = context.orrery;
    final ready = widget.enabled && _text.text.trim().isNotEmpty;
    return Padding(
      padding: const EdgeInsets.fromLTRB(12, 0, 12, 10),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.end,
        children: [
          IconButton(
            key: const ValueKey('direct-toggle'),
            tooltip: l.typeDirect,
            isSelected: widget.direct,
            onPressed: widget.enabled ? widget.onToggleDirect : null,
            icon: const Icon(Icons.keyboard_alt_outlined),
            selectedIcon: Icon(Icons.keyboard_alt_rounded, color: c.accentInk),
            style: IconButton.styleFrom(minimumSize: const Size(48, 48)),
          ),
          Expanded(
            child: DecoratedBox(
              decoration: BoxDecoration(
                color: c.control,
                borderRadius: BorderRadius.circular(24),
                border: Border.all(color: c.lineStrong),
              ),
              child: TextField(
                key: const ValueKey('input'),
                controller: _text,
                focusNode: _focus,
                enabled: widget.enabled,
                minLines: 1,
                maxLines: 4,
                keyboardType: TextInputType.multiline,
                textInputAction: TextInputAction.newline,
                decoration: InputDecoration(
                  hintText: widget.enabled ? l.inputHint : l.inputOffline,
                  isDense: true,
                  filled: false,
                  border: InputBorder.none,
                  enabledBorder: InputBorder.none,
                  focusedBorder: InputBorder.none,
                  disabledBorder: InputBorder.none,
                  contentPadding: const EdgeInsets.symmetric(
                    horizontal: 18,
                    vertical: 13,
                  ),
                ),
              ),
            ),
          ),
          const SizedBox(width: 8),
          AnimatedScale(
            scale: ready || reduceMotion(context) ? 1 : 0.92,
            duration: motion(context, 150),
            child: IconButton.filled(
              key: const ValueKey('send'),
              tooltip: l.send,
              onPressed: ready ? _send : null,
              icon: const Icon(Icons.send_rounded, size: 20),
              style: IconButton.styleFrom(
                minimumSize: const Size(48, 48),
                backgroundColor: c.accent,
                foregroundColor: c.onAccent,
                disabledBackgroundColor: c.accent.withValues(alpha: 0.4),
                disabledForegroundColor: c.onAccent.withValues(alpha: 0.8),
              ),
            ),
          ),
        ],
      ),
    );
  }
}
