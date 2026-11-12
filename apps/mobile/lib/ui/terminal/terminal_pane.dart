import 'dart:async';
import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:xterm/xterm.dart';

import '../../remote/link_state.dart';
import '../../remote/protocol.dart';
import '../../remote/providers.dart';
import '../../l10n/app_localizations.dart';
import '../commands.dart';
import '../machine_api.dart';
import '../model/workspace_model.dart';
import '../ui_providers.dart';
import '../widgets/common.dart';
import 'input_bar.dart';
import 'pane_header.dart';
import 'permission_bar.dart';
import 'terminal_surface.dart';
import 'wp26_probe.dart';

class TerminalPane extends ConsumerStatefulWidget {
  const TerminalPane({
    super.key,
    required this.machineId,
    required this.tabKey,
    required this.pane,
    this.showInput = true,
    this.compactHeader = false,
    this.onActions,
    this.onSwipe,
  });

  final String machineId;
  final String tabKey;
  final PaneInfo pane;
  final bool showInput;
  final bool compactHeader;
  final VoidCallback? onActions;
  final ValueChanged<int>? onSwipe;

  @override
  ConsumerState<TerminalPane> createState() => _TerminalPaneState();
}

class _TerminalPaneState extends ConsumerState<TerminalPane> {
  final Terminal _terminal = Terminal(maxLines: 5000);
  final TerminalController _controller = TerminalController();
  late final LineProbe _probe = LineProbe(_terminal);
  final StringBuffer _pendingOut = StringBuffer();
  bool _flushScheduled = false;
  PtyHandle? _handle;
  StreamSubscription<String>? _outSub;
  StreamSubscription<JsonMap>? _evSub;
  Timer? _typingTimer;
  bool _attaching = false;
  bool _typing = false;
  bool _direct = false;
  bool _claimed = false;
  bool _writing = false;
  String? _error;
  bool _exited = false;
  int? _exitCode;
  int _deskCols = 80;
  int _deskRows = 24;
  int _fitCols = 80;
  int _fitRows = 24;

  late final MachineApi _api;

  @override
  void initState() {
    super.initState();
    _api = ref.read(machineApiProvider(widget.machineId));
    _terminal.onOutput = _onOutput;
    _terminal.onResize = _onTerminalResize;
  }

  @override
  void dispose() {
    _probe.dispose();
    _typingTimer?.cancel();
    _outSub?.cancel();
    _evSub?.cancel();
    final handle = _handle;
    _handle = null;
    if (handle != null) {
      unawaited(_api.detach(handle).catchError((Object _) {}));
    }
    _controller.dispose();
    super.dispose();
  }

  @override
  void didUpdateWidget(TerminalPane old) {
    super.didUpdateWidget(old);
    if (old.pane.sessionId == widget.pane.sessionId) return;
    final handle = _handle;
    _handle = null;
    _outSub?.cancel();
    _evSub?.cancel();
    if (handle != null) {
      unawaited(_api.detach(handle).catchError((Object _) {}));
    }
    _pendingOut.clear();
    _terminal.write('\x1b[0m\x1b[3J\x1b[2J\x1b[H');
    _exitCode = null;
    _exited = false;
    _error = null;
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) _attach();
    });
  }

  void _write(String data) {
    _pendingOut.write(data);
    if (_flushScheduled) return;
    _flushScheduled = true;
    SchedulerBinding.instance.scheduleFrameCallback((_) => _flushOut());
  }

  void _flushOut() {
    _flushScheduled = false;
    if (!mounted || _pendingOut.isEmpty) return;
    _probe.schedule();
    final data = _pendingOut.toString();
    _pendingOut.clear();
    _writing = true;
    try {
      _terminal.write(data);
    } finally {
      _writing = false;
    }
  }

  void _onOutput(String data) {
    final handle = _handle;
    if (handle == null || _writing) return;
    _api.input(handle, Uint8List.fromList(utf8.encode(data)), claim: true);
  }

  void _onTerminalResize(int cols, int rows, int pw, int ph) {
    if (!_typing) return;
    final handle = _handle;
    if (handle == null) return;
    unawaited(_api.resize(handle, cols, rows).catchError((Object _) {}));
  }

  Future<void> _attach() async {
    final sessionId = widget.pane.sessionId;
    if (_attaching || _handle != null || sessionId == null) return;
    _attaching = true;
    final api = _api;
    try {
      final handle = await api.attach(
        sessionId,
        _fitCols,
        _fitRows,
        paneKey: widget.pane.key,
      );
      if (!mounted || widget.pane.sessionId != sessionId) {
        unawaited(api.detach(handle).catchError((Object _) {}));
        if (mounted) {
          WidgetsBinding.instance.addPostFrameCallback((_) {
            if (mounted) _attach();
          });
        }
        return;
      }
      final tail = handle.tail;
      _handle = handle;
      _outSub = const Utf8Decoder(allowMalformed: true)
          .bind(handle.output)
          .listen(_write);
      _evSub = handle.events.listen(_onEvent);
      if (tail.isNotEmpty) {
        _write(utf8.decode(tail, allowMalformed: true));
      }
      setState(() => _error = null);
    } on RemoteCommandException catch (error) {
      if (mounted) setState(() => _error = error.code);
    } finally {
      _attaching = false;
    }
  }

  void _onEvent(JsonMap event) {
    _probe.event('${event['t']}');
    switch (event['t']) {
      case 'pty.ready':
        _flushOut();
        _terminal.write('\x1b[0m\x1b[3J\x1b[2J\x1b[H');
        _applyDesk(event);
        if (_exited || _error != null) {
          setState(() {
            _exitCode = null;
            _exited = false;
            _error = null;
          });
        }
      case 'pty.resized':
        _applyDesk(event);
        if (_claimed && !_typing) setState(() => _claimed = false);
      case 'pty.exit':
        _flushOut();
        final code = event['code'];
        setState(() {
          _exited = true;
          _exitCode = code is num ? code.toInt() : null;
        });
      case 'pty.error':
        setState(() => _error = (event['error'] as String?) ?? 'internal');
    }
  }

  void _applyDesk(JsonMap event) {
    final cols = event['cols'];
    final rows = event['rows'];
    if (cols is! num || rows is! num) return;
    final changed = cols.toInt() != _deskCols || rows.toInt() != _deskRows;
    _deskCols = cols.toInt();
    _deskRows = rows.toInt();
    if (changed && mounted) setState(() {});
  }

  void _beginTyping() {
    _typingTimer?.cancel();
    if (!_typing || !_claimed) {
      setState(() {
        _typing = true;
        _claimed = true;
      });
    }
  }

  void _endTypingSoon() {
    _typingTimer?.cancel();
    _typingTimer = Timer(const Duration(seconds: 8), () {
      if (mounted && !_direct) setState(() => _typing = false);
    });
  }

  void _onFit(int cols, int rows) {
    _fitCols = cols;
    _fitRows = rows;
    if (_handle == null && !_attaching) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted) _attach();
      });
    }
  }

  Future<void> _sendKeys(String keys) async {
    final facts = ref.read(
      paneFactsProvider((widget.machineId, widget.pane.terminalId)),
    );
    await runCommand(context, ref, widget.machineId, 'ws.keys', {
      'paneKey': widget.pane.key,
      'keys': keys,
      'since': facts.statusSince?.millisecondsSinceEpoch ?? 0,
    });
  }

  @override
  Widget build(BuildContext context) {
    final link = ref.watch(linkProvider(widget.machineId));
    final facts = ref.watch(
      paneFactsProvider((widget.machineId, widget.pane.terminalId)),
    );
    final online = link.status == LinkStatus.ready;
    final pane = widget.pane;
    final l = context.l10n;
    return Column(
      children: [
        PaneHeader(
          pane: pane,
          facts: facts,
          claimed: _claimed && online,
          compact: widget.compactHeader,
          onActions: widget.onActions,
          onSwipe: widget.onSwipe,
        ),
        PermissionBar(pane: pane, enabled: online, onKeys: _sendKeys),
        Expanded(
          child: TerminalSurface(
            terminal: _terminal,
            controller: _controller,
            typing: _typing,
            direct: _direct,
            deskCols: _deskCols,
            deskRows: _deskRows,
            onFit: _onFit,
            overlay: _overlay(l, pane),
          ),
        ),
        if (widget.showInput &&
            (MediaQuery.sizeOf(context).height >= 520 || _typing))
          ExtraKeysRow(
            terminal: _terminal,
            enabled: online && _handle != null,
            onUse: () {
              _beginTyping();
              _endTypingSoon();
            },
          ),
        if (widget.showInput)
          InputBar(
            terminal: _terminal,
            enabled: online && _handle != null && !_exited,
            direct: _direct,
            onToggleDirect: () {
              setState(() => _direct = !_direct);
              if (_direct) {
                _beginTyping();
              } else {
                _endTypingSoon();
              }
            },
            onFocusChange: (focused) {
              if (focused) {
                _beginTyping();
              } else {
                _endTypingSoon();
              }
            },
            onSent: () {
              _beginTyping();
              _endTypingSoon();
            },
          ),
      ],
    );
  }

  Widget? _overlay(AppLocalizations l, PaneInfo pane) {
    if (!pane.canAttach) {
      return _Notice(
        icon: Icons.desktop_windows_outlined,
        text: l.paneNotTerminal,
      );
    }
    if (_error != null) {
      final text = _error == 'limit'
          ? l.attachLimit
          : errorText(l, RemoteCommandException(_error!));
      return _Notice(icon: Icons.link_off_rounded, text: text);
    }
    if (_exited) {
      return _Notice(
        icon: Icons.stop_circle_outlined,
        text: _exitCode == null ? l.sessionLost : l.sessionEnded(_exitCode!),
      );
    }
    return null;
  }
}

class _Notice extends StatelessWidget {
  const _Notice({required this.icon, required this.text});

  final IconData icon;
  final String text;

  @override
  Widget build(BuildContext context) {
    return Align(
      alignment: Alignment.topCenter,
      child: Container(
        margin: const EdgeInsets.all(12),
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
        decoration: BoxDecoration(
          color: Colors.black.withValues(alpha: 0.78),
          borderRadius: BorderRadius.circular(12),
        ),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 18, color: Colors.white),
            const SizedBox(width: 8),
            Flexible(
              child: Text(text, style: const TextStyle(color: Colors.white)),
            ),
          ],
        ),
      ),
    );
  }
}
