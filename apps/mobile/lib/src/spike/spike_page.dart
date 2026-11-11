import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:xterm/xterm.dart';

import 'bench.dart';
import 'extra_keys.dart';

class SpikePage extends StatefulWidget {
  const SpikePage({super.key, required this.rustStatus, this.terminal});

  final String rustStatus;
  final Terminal? terminal;

  @override
  State<SpikePage> createState() => _SpikePageState();
}

class _SpikePageState extends State<SpikePage> {
  late final Terminal _terminal = widget.terminal ?? Terminal(maxLines: 10000);
  final TerminalController _controller = TerminalController();
  final ScrollController _scroll = ScrollController();
  final TextEditingController _input = TextEditingController();
  final FrameStats _stats = FrameStats();
  String _metrics = 'chưa đo';
  String _sent = '';
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    _terminal.onOutput = (data) =>
        debugPrint('SPIKE onOutput ${jsonEncode(data)}');
  }

  @override
  void dispose() {
    _scroll.dispose();
    _input.dispose();
    _controller.dispose();
    super.dispose();
  }

  Future<void> _loadFixture() async {
    if (_busy) return;
    setState(() => _busy = true);
    final text = await rootBundle.loadString('assets/fixture.ansi');
    final bytes = utf8.encode(text).length;
    final rssBefore = rssMb();
    final parse = Stopwatch()..start();
    _terminal.write(text);
    parse.stop();
    final paint = Stopwatch()..start();
    await WidgetsBinding.instance.endOfFrame;
    paint.stop();
    final rssAfter = rssMb();
    final line =
        'nạp ${(bytes / 1024).toStringAsFixed(0)}KiB: write=${parse.elapsedMilliseconds}ms '
        'frame đầu=${paint.elapsedMilliseconds}ms '
        'RSS ${rssBefore.toStringAsFixed(0)}→${rssAfter.toStringAsFixed(0)}MB '
        'lines=${_terminal.buffer.lines.length}';
    debugPrint('SPIKE $line');
    if (!mounted) return;
    setState(() {
      _metrics = line;
      _busy = false;
    });
  }

  Future<void> _scrollBench() async {
    if (_busy || !_scroll.hasClients) return;
    setState(() => _busy = true);
    _stats.start();
    for (var i = 0; i < 4; i++) {
      final max = _scroll.position.maxScrollExtent;
      await _scroll.animateTo(
        0,
        duration: const Duration(milliseconds: 1500),
        curve: Curves.linear,
      );
      await _scroll.animateTo(
        max,
        duration: const Duration(milliseconds: 1500),
        curve: Curves.linear,
      );
    }
    await Future<void>.delayed(const Duration(milliseconds: 300));
    final report = _stats.stop();
    final line = 'cuộn: $report RSS=${rssMb().toStringAsFixed(0)}MB';
    debugPrint('SPIKE $line');
    if (!mounted) return;
    setState(() {
      _metrics = line;
      _busy = false;
    });
  }

  void _send() {
    final text = _input.text;
    if (text.isEmpty) return;
    _terminal.write('$text\r\n');
    final hex = utf8
        .encode(text)
        .map((b) => b.toRadixString(16).padLeft(2, '0'))
        .join(' ');
    debugPrint('SPIKE send utf8=$hex');
    setState(() => _sent = 'UTF-8: $hex');
    _input.clear();
  }

  void _extraKey(String label, String sequence) {
    _terminal.write('${visibleEscape(sequence)} ');
    setState(() => _sent = '$label → ${visibleEscape(sequence)}');
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Orrery Remote — spike')),
      body: SafeArea(
        child: Column(
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(12, 4, 12, 0),
              child: Align(
                alignment: Alignment.centerLeft,
                child: Text(
                  widget.rustStatus,
                  key: const ValueKey('rust-status'),
                ),
              ),
            ),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 8),
              child: Wrap(
                spacing: 8,
                children: [
                  FilledButton(
                    key: const ValueKey('load-fixture'),
                    onPressed: _busy ? null : _loadFixture,
                    child: const Text('Nạp fixture'),
                  ),
                  FilledButton.tonal(
                    key: const ValueKey('scroll-bench'),
                    onPressed: _busy ? null : _scrollBench,
                    child: const Text('Đo cuộn'),
                  ),
                  TextButton(
                    key: const ValueKey('clear'),
                    onPressed: () => _terminal.write('\x1b[2J\x1b[3J\x1b[H'),
                    child: const Text('Xoá'),
                  ),
                ],
              ),
            ),
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12),
              child: Align(
                alignment: Alignment.centerLeft,
                child: Text(
                  _metrics,
                  key: const ValueKey('metrics'),
                  style: const TextStyle(fontSize: 11),
                ),
              ),
            ),
            Expanded(
              child: TerminalView(
                _terminal,
                controller: _controller,
                scrollController: _scroll,
                hardwareKeyboardOnly: true,
                textStyle: const TerminalStyle(fontSize: 12),
              ),
            ),
            ExtraKeysBar(onKey: _extraKey),
            if (_sent.isNotEmpty)
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 12),
                child: Align(
                  alignment: Alignment.centerLeft,
                  child: Text(
                    _sent,
                    key: const ValueKey('sent'),
                    style: const TextStyle(fontSize: 11),
                  ),
                ),
              ),
            Padding(
              padding: const EdgeInsets.fromLTRB(8, 0, 8, 8),
              child: Row(
                children: [
                  Expanded(
                    child: TextField(
                      key: const ValueKey('input'),
                      controller: _input,
                      textInputAction: TextInputAction.send,
                      onSubmitted: (_) => _send(),
                      decoration: const InputDecoration(
                        hintText: 'Gõ lệnh (Telex)…',
                        isDense: true,
                        border: OutlineInputBorder(),
                      ),
                    ),
                  ),
                  const SizedBox(width: 8),
                  FilledButton(
                    key: const ValueKey('send'),
                    onPressed: _send,
                    child: const Text('Gửi'),
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
