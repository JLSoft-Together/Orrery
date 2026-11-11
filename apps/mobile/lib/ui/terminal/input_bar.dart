import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:xterm/xterm.dart';

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
      height: 44,
      child: ListView.separated(
        scrollDirection: Axis.horizontal,
        padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 4),
        itemCount: keys.length,
        separatorBuilder: (_, _) => const SizedBox(width: 6),
        itemBuilder: (context, index) {
          final (label, id, action) = keys[index];
          return Semantics(
            button: true,
            label: id,
            child: SizedBox(
              height: 36,
              child: FilledButton.tonal(
                key: ValueKey('key-$id'),
                style: FilledButton.styleFrom(
                  minimumSize: const Size(44, 36),
                  padding: const EdgeInsets.symmetric(horizontal: 10),
                  textStyle: Theme.of(context).textTheme.labelLarge
                      ?.copyWith(fontSize: 14, fontWeight: FontWeight.w600),
                ),
                onPressed: enabled ? action : null,
                child: label is IconData
                    ? Icon(label, size: 20)
                    : Text(label as String),
              ),
            ),
          );
        },
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
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.fromLTRB(8, 0, 8, 8),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.end,
        children: [
          IconButton.filledTonal(
            key: const ValueKey('direct-toggle'),
            tooltip: l.typeDirect,
            isSelected: widget.direct,
            onPressed: widget.enabled ? widget.onToggleDirect : null,
            icon: const Icon(Icons.keyboard_alt_outlined),
            selectedIcon: const Icon(Icons.keyboard_alt_rounded),
            style: IconButton.styleFrom(minimumSize: const Size(48, 48)),
          ),
          const SizedBox(width: 6),
          Expanded(
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
              ),
            ),
          ),
          const SizedBox(width: 6),
          IconButton.filled(
            key: const ValueKey('send'),
            tooltip: l.send,
            onPressed: widget.enabled && _text.text.trim().isNotEmpty
                ? _send
                : null,
            icon: const Icon(Icons.arrow_upward_rounded),
            style: IconButton.styleFrom(
              minimumSize: const Size(48, 48),
              backgroundColor: scheme.primary,
              foregroundColor: scheme.onPrimary,
            ),
          ),
        ],
      ),
    );
  }
}
