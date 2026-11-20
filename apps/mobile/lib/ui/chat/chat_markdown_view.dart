import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import '../commands.dart' show toast;
import '../theme/orrery_shape.dart';
import '../viewers/highlight_lines.dart';
import '../widgets/common.dart';
import 'chat_markdown.dart';

const String chatMono = 'OrreryMono';

class ChatMarkdown extends StatelessWidget {
  const ChatMarkdown({super.key, required this.text});

  final String text;

  @override
  Widget build(BuildContext context) {
    final blocks = cachedMarkdown(text);
    final children = <Widget>[];
    for (final block in blocks) {
      if (children.isNotEmpty) children.add(const SizedBox(height: 14));
      children.add(_block(context, block));
    }
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: children,
    );
  }

  Widget _block(BuildContext context, MdBlock block) {
    final base = chatTextStyle(context);
    switch (block) {
      case MdPara(:final text):
        return MdText(text, style: base);
      case MdHeading(:final level, :final text):
        final size = switch (level) {
          1 => 21.0,
          2 => 19.0,
          _ => 17.0,
        };
        return MdText(
          text,
          style: base.copyWith(
            fontSize: size,
            fontWeight: FontWeight.w600,
            height: 1.35,
          ),
        );
      case MdList(:final ordered, :final items, :final start):
        final muted = context.orrery.inkMuted;
        return Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            for (var k = 0; k < items.length; k++)
              Padding(
                padding: EdgeInsets.only(top: k == 0 ? 0 : 6),
                child: Row(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    SizedBox(
                      width: 24,
                      child: Text(
                        ordered ? '${start + k}.' : '•',
                        style: base.copyWith(color: muted),
                      ),
                    ),
                    Expanded(child: MdText(items[k], style: base)),
                  ],
                ),
              ),
          ],
        );
      case MdQuote(:final text):
        final c = context.orrery;
        return DecoratedBox(
          decoration: BoxDecoration(
            border: Border(left: BorderSide(color: c.lineStrong, width: 3)),
          ),
          child: Padding(
            padding: const EdgeInsets.only(left: 12),
            child: MdText(text, style: base.copyWith(color: c.inkMuted)),
          ),
        );
      case MdCode(:final code, :final lang):
        return ChatCodeBlock(code: code, lang: lang);
      case MdRule():
        return Divider(height: 1, color: context.orrery.line);
    }
  }
}

final Map<String, List<MdBlock>> _parsed = {};

List<MdBlock> cachedMarkdown(String text) {
  final hit = _parsed.remove(text);
  final blocks = hit ?? parseMarkdown(text);
  _parsed[text] = blocks;
  if (_parsed.length > 200) _parsed.remove(_parsed.keys.first);
  return blocks;
}

TextStyle chatTextStyle(BuildContext context) =>
    TextStyle(fontSize: 16.5, height: 1.6, color: context.orrery.ink);

class MdText extends StatelessWidget {
  const MdText(this.text, {super.key, required this.style});

  final String text;
  final TextStyle style;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final spans = <InlineSpan>[];
    for (final run in parseInline(text)) {
      var s = style;
      if (run.styles.contains(MdStyle.bold)) {
        s = s.copyWith(fontWeight: FontWeight.w600);
      }
      if (run.styles.contains(MdStyle.italic)) {
        s = s.copyWith(fontStyle: FontStyle.italic);
      }
      if (run.styles.contains(MdStyle.link)) {
        s = s.copyWith(
          color: c.accentInk,
          decoration: TextDecoration.underline,
          decorationColor: c.accentInk.withValues(alpha: 0.5),
        );
      }
      if (run.styles.contains(MdStyle.code)) {
        s = s.copyWith(
          fontFamily: chatMono,
          fontSize: (style.fontSize ?? 16) * 0.82,
          backgroundColor: c.raised,
        );
        spans.add(TextSpan(text: ' ${run.text} ', style: s));
        continue;
      }
      spans.add(TextSpan(text: run.text, style: s));
    }
    return Text.rich(TextSpan(children: spans));
  }
}

class ChatCodeBlock extends StatelessWidget {
  const ChatCodeBlock({super.key, required this.code, this.lang});

  final String code;
  final String? lang;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    final dark = Theme.of(context).brightness == Brightness.dark;
    final colors = CodeColors(dark);
    final lines = code.length > highlightMaxChars || code.length > isolateAbove
        ? plainLines(code)
        : highlightSync((code, lang));
    final style = TextStyle(
      fontFamily: chatMono,
      fontSize: 13,
      height: 1.6,
      color: c.ink,
    );
    final spans = <InlineSpan>[];
    for (var i = 0; i < lines.length; i++) {
      if (i > 0) spans.add(const TextSpan(text: '\n'));
      for (final (text, cls) in lines[i]) {
        spans.add(
          TextSpan(
            text: text,
            style: TextStyle(color: colors.of(cls)),
          ),
        );
      }
    }
    return DecoratedBox(
      decoration: BoxDecoration(
        color: c.raised,
        borderRadius: BorderRadius.circular(18),
        border: Border.all(color: c.line),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          Row(
            children: [
              const SizedBox(width: 16),
              Expanded(
                child: Text(
                  lang ?? '',
                  style: TextStyle(fontSize: 12.5, color: c.inkMuted),
                ),
              ),
              TextButton(
                key: const ValueKey('chat-code-copy'),
                style: TextButton.styleFrom(
                  minimumSize: const Size(44, 44),
                  foregroundColor: c.inkMuted,
                  shape: const StadiumBorder(),
                ),
                onPressed: () async {
                  await Clipboard.setData(ClipboardData(text: code));
                  if (context.mounted) toast(context, l.chatCopied);
                },
                child: Text(l.chatCopy, style: const TextStyle(fontSize: 12.5)),
              ),
              const SizedBox(width: 4),
            ],
          ),
          SingleChildScrollView(
            scrollDirection: Axis.horizontal,
            dragStartBehavior: DragStartBehavior.down,
            padding: const EdgeInsets.fromLTRB(16, 0, 16, 14),
            child: Text.rich(TextSpan(style: style, children: spans)),
          ),
        ],
      ),
    );
  }
}
