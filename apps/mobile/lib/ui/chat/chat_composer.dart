import 'package:flutter/material.dart';

import '../../media/media_flow.dart';
import '../../prompts/prompt_model.dart';
import '../../prompts/prompt_strip.dart';
import '../../prompts/prompts_controller.dart';
import '../../security/danger_rules.dart';
import '../commands.dart';
import '../danger_guard.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';
import 'chat_media.dart';

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
    this.danger = const DangerRules.none(),
    this.media = const ComposerMedia(),
    this.prompts,
    this.projectPath,
  });

  final String hint;
  final bool enabled;
  final bool working;
  final Future<bool> Function(String text, bool authed) onSend;
  final DangerRules danger;
  final VoidCallback onStop;
  final VoidCallback? onPlus;
  final List<String> chips;
  final ComposerMedia media;
  final PromptsController? prompts;
  final String? projectPath;

  @override
  State<ChatComposer> createState() => ChatComposerState();
}

class ChatComposerState extends State<ChatComposer> {
  final TextEditingController _text = TextEditingController();
  final FocusNode _focus = FocusNode();
  bool _sending = false;

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

  String get draft => _text.text;

  void setDraft(String value) {
    _text.value = TextEditingValue(
      text: value,
      selection: TextSelection.collapsed(offset: value.length),
    );
  }

  void addPath(String path) {
    setDraft(insertPath(_text.text, path));
    _focus.requestFocus();
  }

  void _insertPrompt(SavedPrompt prompt) {
    final l = context.l10n;
    final next = insertAtCursor(_text.value, prompt.insertText);
    if (next == null) {
      toast(context, l.promptsTooLong);
      return;
    }
    _text.value = next;
    _focus.requestFocus();
    if (prompt.missing.isNotEmpty) {
      toast(context, l.promptsMissing(prompt.missing.join(', ')));
    }
  }

  Future<void> _send() async {
    if (_sending) return;
    final raw = _text.text;
    final value = raw.trim();
    if (value.isEmpty) return;
    _sending = true;
    try {
      final verdict = await _verdict(value);
      if (verdict == DangerVerdict.cancelled || !mounted) return;
      if (_text.text == raw) _text.clear();
      await widget.onSend(value, verdict == DangerVerdict.authed);
    } finally {
      _sending = false;
    }
  }

  Future<DangerVerdict> _verdict(String value) =>
      confirmDanger(context, widget.danger, value);

  Future<void> _sendChip(String chip) async {
    if (_sending) return;
    _sending = true;
    try {
      final verdict = await _verdict(chip);
      if (verdict == DangerVerdict.cancelled || !mounted) return;
      await widget.onSend(chip, verdict == DangerVerdict.authed);
    } finally {
      _sending = false;
    }
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
          if (widget.prompts != null)
            PromptStrip(
              controller: widget.prompts!,
              enabled: widget.enabled,
              onInsert: _insertPrompt,
              projectPath: widget.projectPath,
            ),
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
                      onPressed: widget.enabled ? () => _sendChip(chip) : null,
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
                if (widget.media.photo != null) ...[
                  const SizedBox(width: 4),
                  PhotoButton(
                    flow: widget.media.photo!,
                    enabled: widget.enabled,
                    onPath: addPath,
                  ),
                ],
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
                if (widget.media.dictation != null) ...[
                  const SizedBox(width: 4),
                  MicButton(
                    dictation: widget.media.dictation!,
                    enabled: widget.enabled,
                    getText: () => _text.text,
                    setText: setDraft,
                  ),
                ],
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
