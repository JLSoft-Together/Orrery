import 'package:flutter/material.dart';

import '../widgets/common.dart';
import 'highlight_lines.dart';
import 'viewer_models.dart' show monoFallback;

const double _lineHeight = 20;
const double _charWidth = 7.9;
const double _gutter = 48;

class CodeView extends StatefulWidget {
  const CodeView({super.key, required this.lines});

  final List<TokenLine> lines;

  @override
  State<CodeView> createState() => _CodeViewState();
}

class _CodeViewState extends State<CodeView> {
  final ScrollController _vertical = ScrollController();
  final TextEditingController _find = TextEditingController();
  List<int> _hits = const [];
  int _current = 0;

  @override
  void dispose() {
    _vertical.dispose();
    _find.dispose();
    super.dispose();
  }

  String _plain(TokenLine line) => line.map((t) => t.$1).join();

  void _search(String query) {
    final needle = query.toLowerCase();
    final hits = <int>[];
    if (needle.isNotEmpty) {
      for (var i = 0; i < widget.lines.length; i++) {
        if (_plain(widget.lines[i]).toLowerCase().contains(needle)) hits.add(i);
      }
    }
    setState(() {
      _hits = hits;
      _current = 0;
    });
    _jump();
  }

  void _step(int delta) {
    if (_hits.isEmpty) return;
    setState(() => _current = (_current + delta) % _hits.length);
    _jump();
  }

  void _jump() {
    if (_hits.isEmpty || !_vertical.hasClients) return;
    final target = (_hits[_current] * _lineHeight - 80).clamp(
      0.0,
      _vertical.position.maxScrollExtent,
    );
    _vertical.animateTo(
      target,
      duration: motion(context, 180),
      curve: Curves.easeOut,
    );
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final scheme = Theme.of(context).colorScheme;
    final colors = CodeColors(Theme.of(context).brightness == Brightness.dark);
    final lines = widget.lines;
    var longest = 0;
    for (final line in lines) {
      var length = 0;
      for (final token in line) {
        length += token.$1.length;
      }
      if (length > longest) longest = length;
    }
    final hitSet = _hits.toSet();
    final activeLine = _hits.isEmpty ? -1 : _hits[_current];
    final mono = TextStyle(
      fontFamily: 'monospace',
      fontFamilyFallback: monoFallback,
      fontSize: 13,
      height: 1.0,
      color: scheme.onSurface,
    );
    return Column(
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(12, 6, 4, 6),
          child: Row(
            children: [
              Expanded(
                child: SizedBox(
                  height: 40,
                  child: TextField(
                    key: const ValueKey('find-field'),
                    controller: _find,
                    onChanged: _search,
                    textInputAction: TextInputAction.search,
                    onSubmitted: (_) => _step(1),
                    decoration: InputDecoration(
                      isDense: true,
                      hintText: l.vFindInFile,
                      prefixIcon: const Icon(Icons.search_rounded, size: 20),
                      border: OutlineInputBorder(
                        borderRadius: BorderRadius.circular(12),
                        borderSide: BorderSide.none,
                      ),
                      filled: true,
                      fillColor: scheme.surfaceContainerHigh,
                    ),
                  ),
                ),
              ),
              if (_find.text.isNotEmpty) ...[
                Padding(
                  padding: const EdgeInsets.symmetric(horizontal: 6),
                  child: Text(
                    _hits.isEmpty
                        ? l.vNoMatches
                        : '${_current + 1}/${_hits.length}',
                    style: TextStyle(
                      fontSize: 12,
                      color: scheme.onSurfaceVariant,
                    ),
                  ),
                ),
                IconButton(
                  tooltip: l.vPrevMatch,
                  icon: const Icon(Icons.keyboard_arrow_up_rounded),
                  onPressed: _hits.isEmpty ? null : () => _step(-1),
                ),
                IconButton(
                  tooltip: l.vNextMatch,
                  icon: const Icon(Icons.keyboard_arrow_down_rounded),
                  onPressed: _hits.isEmpty ? null : () => _step(1),
                ),
              ],
            ],
          ),
        ),
        Expanded(
          child: LayoutBuilder(
            builder: (context, box) {
              final width = (_gutter + longest * _charWidth + 24).clamp(
                box.maxWidth,
                double.infinity,
              );
              return SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                child: SizedBox(
                  width: width,
                  height: box.maxHeight,
                  child: ListView.builder(
                    key: const ValueKey('code-list'),
                    controller: _vertical,
                    itemExtent: _lineHeight,
                    itemCount: lines.length,
                    itemBuilder: (context, index) {
                      final hit = hitSet.contains(index);
                      return Container(
                        color: index == activeLine
                            ? scheme.primary.withValues(alpha: 0.28)
                            : hit
                            ? scheme.primary.withValues(alpha: 0.12)
                            : null,
                        alignment: Alignment.centerLeft,
                        child: Row(
                          children: [
                            SizedBox(
                              width: _gutter,
                              child: Padding(
                                padding: const EdgeInsets.only(right: 8),
                                child: Text(
                                  '${index + 1}',
                                  textAlign: TextAlign.right,
                                  style: mono.copyWith(
                                    fontSize: 11,
                                    color: scheme.onSurfaceVariant,
                                  ),
                                ),
                              ),
                            ),
                            Text.rich(
                              TextSpan(
                                children: [
                                  for (final (text, cls) in lines[index])
                                    TextSpan(
                                      text: text,
                                      style: TextStyle(color: colors.of(cls)),
                                    ),
                                ],
                              ),
                              style: mono,
                              softWrap: false,
                            ),
                          ],
                        ),
                      );
                    },
                  ),
                ),
              );
            },
          ),
        ),
      ],
    );
  }
}
