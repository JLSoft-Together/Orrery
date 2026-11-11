import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../../remote/link_state.dart';
import '../../remote/providers.dart';
import '../commands.dart';
import '../model/workspace_model.dart';
import '../ui_providers.dart';
import '../widgets/common.dart';

class ProjectChoice {
  const ProjectChoice(this.dir, this.label, this.favorite);

  final String dir;
  final String label;
  final bool favorite;
}

List<ProjectChoice> projectChoices(
  Map<String, dynamic>? snapshot,
  WorkspaceModel model,
) {
  final seen = <String>{};
  final out = <ProjectChoice>[];
  final favorites = snapshot?['favorites'];
  if (favorites is List) {
    for (final item in favorites) {
      if (item is! Map) continue;
      final dir = item['dir'];
      if (dir is! String || !seen.add(dir)) continue;
      final label = item['label'];
      out.add(
        ProjectChoice(
          dir,
          label is String && label.isNotEmpty ? label : baseName(dir),
          true,
        ),
      );
    }
  }
  for (final tab in model.tabs) {
    final dir = tab.project;
    if (dir != null && seen.add(dir)) {
      out.add(ProjectChoice(dir, baseName(dir), false));
    }
  }
  return out;
}

class OpenPanePage extends ConsumerStatefulWidget {
  const OpenPanePage({super.key, required this.machineId});

  final String machineId;

  @override
  ConsumerState<OpenPanePage> createState() => _OpenPanePageState();
}

class _OpenPanePageState extends ConsumerState<OpenPanePage> {
  final TextEditingController _prompt = TextEditingController();
  String? _dir;
  String _preset = 'claude';
  bool _submit = false;
  bool _busy = false;

  @override
  void dispose() {
    _prompt.dispose();
    super.dispose();
  }

  Future<void> _open() async {
    final dir = _dir;
    if (dir == null || _busy) return;
    setState(() => _busy = true);
    final navigator = Navigator.of(context);
    final l = context.l10n;
    final prompt = _prompt.text.trim();
    final reply = await runCommand(context, ref, widget.machineId, 'ws.open', {
      'cwd': dir,
      'preset': _preset,
      if (_preset != 'shell' && prompt.isNotEmpty) 'prompt': prompt,
      if (_preset != 'shell' && prompt.isNotEmpty && _submit) 'submit': true,
    }, l.paneOpened);
    if (!mounted) return;
    if (reply != null) {
      navigator.pop();
    } else {
      setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final scheme = Theme.of(context).colorScheme;
    final id = widget.machineId;
    final snapshot = ref.watch(
      machineViewProvider(id).select((v) => v.snapshot),
    );
    final model = ref.watch(workspaceModelProvider(id));
    final online = ref.watch(
      linkProvider(id).select((s) => s.status == LinkStatus.ready),
    );
    final choices = projectChoices(snapshot, model);
    final agentic = _preset != 'shell';
    return Scaffold(
      appBar: AppBar(title: Text(l.openPane)),
      body: ListView(
        padding: const EdgeInsets.fromLTRB(16, 8, 16, 32),
        children: [
          _Section(l.pickProject),
          if (choices.isEmpty)
            Padding(
              padding: const EdgeInsets.symmetric(vertical: 12),
              child: Text(
                l.noProjects,
                style: TextStyle(color: scheme.onSurfaceVariant),
              ),
            ),
          RadioGroup<String>(
            groupValue: _dir,
            onChanged: (v) => setState(() => _dir = v),
            child: Column(
              children: [
                for (final choice in choices)
                  RadioListTile<String>(
                    key: ValueKey('project-${choice.dir}'),
                    value: choice.dir,
                    title: Text(choice.label),
                    subtitle: Text(
                      choice.dir,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                    ),
                    secondary: Icon(
                      choice.favorite
                          ? Icons.star_rounded
                          : Icons.folder_outlined,
                      color: choice.favorite ? scheme.primary : null,
                    ),
                    contentPadding: EdgeInsets.zero,
                  ),
              ],
            ),
          ),
          const SizedBox(height: 16),
          _Section(l.pickAgent),
          SegmentedButton<String>(
            showSelectedIcon: false,
            segments: [
              ButtonSegment(
                value: 'claude',
                label: const Text('Claude'),
                icon: const Icon(Icons.auto_awesome_rounded),
              ),
              ButtonSegment(
                value: 'codex',
                label: const Text('Codex'),
                icon: const Icon(Icons.code_rounded),
              ),
              ButtonSegment(
                value: 'shell',
                label: Text(l.shell),
                icon: const Icon(Icons.terminal_rounded),
              ),
            ],
            selected: {_preset},
            onSelectionChanged: (v) => setState(() => _preset = v.first),
          ),
          AnimatedSize(
            duration: motion(context),
            curve: Curves.easeOutCubic,
            alignment: Alignment.topCenter,
            child: !agentic
                ? const SizedBox(width: double.infinity)
                : Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      const SizedBox(height: 16),
                      _Section(l.promptOptional),
                      TextField(
                        key: const ValueKey('prompt'),
                        controller: _prompt,
                        minLines: 3,
                        maxLines: 6,
                        maxLength: 2000,
                        onChanged: (_) => setState(() {}),
                        decoration: InputDecoration(hintText: l.promptHint),
                      ),
                      SwitchListTile(
                        key: const ValueKey('submit'),
                        contentPadding: EdgeInsets.zero,
                        value: _submit,
                        onChanged: _prompt.text.trim().isEmpty
                            ? null
                            : (v) => setState(() => _submit = v),
                        title: Text(l.submitNow),
                        subtitle: Text(l.submitNowHint),
                      ),
                    ],
                  ),
          ),
          const SizedBox(height: 20),
          FilledButton.icon(
            key: const ValueKey('open'),
            onPressed: _dir != null && online && !_busy ? _open : null,
            icon: _busy
                ? const SizedBox(
                    width: 18,
                    height: 18,
                    child: CircularProgressIndicator(strokeWidth: 2),
                  )
                : const Icon(Icons.play_arrow_rounded),
            label: Text(online ? l.openPaneAction : l.errOffline),
          ),
        ],
      ),
    );
  }
}

class _Section extends StatelessWidget {
  const _Section(this.text);

  final String text;

  @override
  Widget build(BuildContext context) => Padding(
    padding: const EdgeInsets.only(top: 8, bottom: 8),
    child: Text(
      text,
      style: TextStyle(
        fontSize: 13,
        fontWeight: FontWeight.w600,
        color: Theme.of(context).colorScheme.onSurfaceVariant,
      ),
    ),
  );
}
