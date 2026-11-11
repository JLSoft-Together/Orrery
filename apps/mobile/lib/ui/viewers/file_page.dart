import 'dart:async';
import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../remote/protocol.dart';
import '../commands.dart';
import '../model/workspace_model.dart' show baseName;
import '../widgets/common.dart';
import 'async_panel.dart';
import 'code_view.dart';
import 'highlight_lines.dart';
import 'viewer_api.dart';
import 'viewer_models.dart';

class FilePage extends ConsumerStatefulWidget {
  const FilePage({
    super.key,
    required this.machineId,
    required this.root,
    required this.path,
  });

  final String machineId;
  final String root;
  final String path;

  @override
  ConsumerState<FilePage> createState() => _FilePageState();
}

class _FilePageState extends ConsumerState<FilePage> {
  FileView? _file;
  Object? _error;
  bool _loading = true;
  List<TokenLine> _lines = const [];
  bool _editing = false;
  bool _saving = false;
  bool _diskChanged = false;
  DateTime _lastSave = DateTime.fromMillisecondsSinceEpoch(0);
  num _base = 0;
  final TextEditingController _edit = TextEditingController();
  StreamSubscription<JsonMap>? _sub;
  Timer? _debounce;
  late final ViewerApi _api = ref.read(viewerApiProvider(widget.machineId));

  @override
  void initState() {
    super.initState();
    _api.watch('fs', widget.root);
    _sub = _api.events.listen(_onEvent);
    _load();
  }

  @override
  void dispose() {
    _debounce?.cancel();
    _sub?.cancel();
    _api.unwatch('fs', widget.root);
    _edit.dispose();
    super.dispose();
  }

  void _onEvent(JsonMap event) {
    if (event['t'] != 'fs.changed') return;
    final paths = event['paths'];
    if (paths is! List || !paths.any((p) => p is String && samePath(p, widget.path))) {
      return;
    }
    final own = _saving || DateTime.now().difference(_lastSave) < const Duration(seconds: 3);
    if (_editing) {
      if (own) return;
      if (mounted) setState(() => _diskChanged = true);
      return;
    }
    _debounce?.cancel();
    _debounce = Timer(const Duration(milliseconds: 350), _load);
  }

  Future<void> _load() async {
    if (mounted) setState(() => _loading = true);
    try {
      final reply = await _api.call('fs.read', {'path': widget.path});
      final file = FileView.parse(reply.map);
      final lines = file.kind == 'text' && file.text != null
          ? await highlightText(file.text!, file.lang)
          : const <TokenLine>[];
      if (!mounted) return;
      setState(() {
        _file = file;
        _lines = lines;
        _base = file.mtimeMs;
        _error = null;
        _loading = false;
        _diskChanged = false;
        if (_editing) {
          _editing = false;
        }
      });
    } on Object catch (error) {
      if (!mounted) return;
      setState(() {
        _error = error;
        _loading = false;
      });
    }
  }

  bool get _dirty => _editing && _edit.text != (_file?.text ?? '');

  void _startEdit() {
    _edit.text = _file?.text ?? '';
    setState(() {
      _editing = true;
      _diskChanged = false;
    });
  }

  Future<void> _save() async {
    final l = context.l10n;
    setState(() => _saving = true);
    try {
      final reply = await _api.call('fs.write', {
        'path': widget.path,
        'text': _edit.text,
        'base': _base,
      });
      if (!mounted) return;
      if (reply.pending) {
        _lastSave = DateTime.now();
        toast(context, l.vWaitConfirm);
        setState(() => _saving = false);
        return;
      }
      _lastSave = DateTime.now();
      final text = _edit.text;
      final mtime = reply.map['mtimeMs'];
      final lines = await highlightText(text, _file?.lang);
      if (!mounted) return;
      setState(() {
        _saving = false;
        _editing = false;
        _base = mtime is num ? mtime : _base;
        _file = FileView(
          kind: 'text',
          path: _file!.path,
          size: utf8.encode(text).length,
          mtimeMs: _base,
          text: text,
          lang: _file!.lang,
          eol: _file!.eol,
          bom: _file!.bom,
        );
        _lines = lines;
      });
      toast(context, l.vSaved);
    } on ViewerFailure catch (error) {
      if (!mounted) return;
      setState(() {
        _saving = false;
        if (error.code == 'conflict') _diskChanged = true;
      });
      toast(context, viewerErrorText(l, error));
    } on Object catch (error) {
      if (!mounted) return;
      setState(() => _saving = false);
      toast(context, viewerErrorText(l, error));
    }
  }

  Future<void> _confirmLeave() async {
    final l = context.l10n;
    final leave = await showDialog<bool>(
      context: context,
      builder: (dialog) => AlertDialog(
        title: Text(l.vDiscardEditsTitle),
        content: Text(l.vDiscardEditsBody),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(dialog, false),
            child: Text(l.vKeepEditing),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(dialog, true),
            child: Text(l.vDiscard),
          ),
        ],
      ),
    );
    if (leave == true && mounted) {
      setState(() => _editing = false);
      Navigator.of(context).pop();
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final file = _file;
    final name = baseName(widget.path);
    return PopScope(
      canPop: !_dirty,
      onPopInvokedWithResult: (didPop, _) {
        if (!didPop) _confirmLeave();
      },
      child: Scaffold(
        appBar: AppBar(
          titleSpacing: 0,
          title: Text(name, maxLines: 1, overflow: TextOverflow.ellipsis),
          actions: [
            if (_editing)
              TextButton.icon(
                key: const ValueKey('file-save'),
                onPressed: _saving || !_dirty ? null : _save,
                icon: _saving
                    ? const SizedBox(
                        width: 16,
                        height: 16,
                        child: CircularProgressIndicator(strokeWidth: 2),
                      )
                    : const Icon(Icons.save_outlined),
                label: Text(l.save),
              )
            else if (file != null && file.editable)
              IconButton(
                key: const ValueKey('file-edit'),
                tooltip: l.vEdit,
                icon: const Icon(Icons.edit_outlined),
                onPressed: _startEdit,
              ),
          ],
        ),
        body: _body(context, file),
      ),
    );
  }

  Widget _body(BuildContext context, FileView? file) {
    final l = context.l10n;
    if (file == null) {
      if (_error != null) {
        final limit = _error is RemoteCommandException &&
            (_error! as RemoteCommandException).code == 'limit';
        return ErrorState(
          message: limit ? l.vFileTooBigPhone : viewerErrorText(l, _error!),
          onRetry: _load,
        );
      }
      return const Center(child: CircularProgressIndicator());
    }
    final notice = switch (file.kind) {
      'binary' => l.vFileBinary,
      'image' => l.vFileImage,
      'too-large' => l.vFileTooLarge(sizeText(file.size)),
      'missing' => l.vErrMissing,
      'text' => file.invalidUtf8 ? l.vFileNotUtf8 : null,
      _ => l.errDenied,
    };
    if (notice != null || file.text == null) {
      return EmptyState(icon: Icons.insert_drive_file_outlined, text: notice ?? '');
    }
    return Column(
      children: [
        if (_loading) const LinearProgressIndicator(minHeight: 2),
        if (_diskChanged) _conflictBanner(context),
        Expanded(
          child: _editing
              ? _editor(context)
              : CodeView(key: ValueKey(_base), lines: _lines),
        ),
      ],
    );
  }

  Widget _conflictBanner(BuildContext context) {
    final l = context.l10n;
    final scheme = Theme.of(context).colorScheme;
    return Material(
      color: scheme.errorContainer,
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 10, 8, 6),
        child: Column(
          key: const ValueKey('conflict-banner'),
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Text(
              l.vFileChangedOnDisk,
              style: TextStyle(
                color: scheme.onErrorContainer,
                fontWeight: FontWeight.w600,
              ),
            ),
            Wrap(
              children: [
                TextButton(
                  onPressed: () => Clipboard.setData(
                    ClipboardData(text: _edit.text),
                  ).then((_) {
                    if (context.mounted) toast(context, l.vCopied);
                  }),
                  child: Text(l.vCopyMine),
                ),
                TextButton(
                  key: const ValueKey('conflict-reload'),
                  onPressed: _load,
                  child: Text(l.vReloadDiscard),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Widget _editor(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    return Padding(
      padding: const EdgeInsets.fromLTRB(8, 4, 8, 0),
      child: TextField(
        key: const ValueKey('file-editor'),
        controller: _edit,
        maxLines: null,
        expands: true,
        textAlignVertical: TextAlignVertical.top,
        keyboardType: TextInputType.multiline,
        autocorrect: false,
        enableSuggestions: false,
        onChanged: (_) => setState(() {}),
        style: TextStyle(
          fontFamily: 'monospace',
          fontFamilyFallback: monoFallback,
          fontSize: 13,
          height: 1.4,
          color: scheme.onSurface,
        ),
        decoration: const InputDecoration(border: InputBorder.none),
      ),
    );
  }
}
