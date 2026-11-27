import 'package:flutter/material.dart';

import '../ui/commands.dart';
import '../ui/kit/buttons.dart';
import '../ui/kit/overlays.dart';
import '../ui/theme/orrery_shape.dart';
import '../ui/widgets/common.dart';
import 'prompt_errors.dart';
import 'prompt_model.dart';
import 'prompts_controller.dart';

Future<void> showPromptEditor(
  BuildContext context,
  PromptsController controller, {
  SavedPrompt? prompt,
  String? projectPath,
}) async {
  await showOrrerySheet<void>(
    context,
    builder: (sheet) => _PromptEditor(
      controller: controller,
      prompt: prompt,
      projectPath: projectPath,
      host: context,
    ),
  );
}

Future<void> showPromptActions(
  BuildContext context,
  PromptsController controller,
  SavedPrompt prompt,
  String? projectPath,
) async {
  final l = context.l10n;
  final choice = await showOrrerySheet<String>(
    context,
    builder: (sheet) => SafeArea(
      child: Padding(
        padding: const EdgeInsets.fromLTRB(16, 4, 16, 12),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Padding(
              padding: const EdgeInsets.fromLTRB(6, 4, 6, 10),
              child: Text(
                prompt.title,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
                style: TextStyle(
                  fontSize: 17,
                  fontWeight: FontWeight.w600,
                  color: sheet.orrery.ink,
                ),
              ),
            ),
            OrreryButton(
              key: const ValueKey('prompt-action-edit'),
              label: l.promptsEdit,
              icon: Icons.edit_outlined,
              kind: ButtonKind.secondary,
              onPressed: () => Navigator.pop(sheet, 'edit'),
            ),
            const SizedBox(height: 8),
            OrreryButton(
              key: const ValueKey('prompt-action-delete'),
              label: l.promptsDelete,
              icon: Icons.delete_outline_rounded,
              kind: ButtonKind.danger,
              onPressed: () => Navigator.pop(sheet, 'delete'),
            ),
          ],
        ),
      ),
    ),
  );
  if (choice == null || !context.mounted) return;
  if (choice == 'edit') {
    await showPromptEditor(
      context,
      controller,
      prompt: prompt,
      projectPath: projectPath,
    );
  } else {
    await _confirmDelete(context, controller, prompt);
  }
}

Future<void> _confirmDelete(
  BuildContext context,
  PromptsController controller,
  SavedPrompt prompt,
) async {
  final l = context.l10n;
  final yes = await showOrreryDialog<bool>(
    context,
    builder: (dialog) => OrreryDialog(
      title: l.promptsDeleteTitle,
      body: l.promptsDeleteBody,
      actions: [
        OrreryButton(
          label: l.cancel,
          kind: ButtonKind.text,
          expand: false,
          onPressed: () => Navigator.pop(dialog, false),
        ),
        OrreryButton(
          key: const ValueKey('prompt-delete-yes'),
          label: l.promptsDelete,
          kind: ButtonKind.danger,
          expand: false,
          onPressed: () => Navigator.pop(dialog, true),
        ),
      ],
    ),
  );
  if (yes != true || !context.mounted) return;
  try {
    await controller.remove(prompt.id);
    if (context.mounted) toast(context, l.promptsDeleted);
  } on Object catch (error) {
    if (context.mounted) toast(context, promptErrorText(l, error));
  }
}

class _PromptEditor extends StatefulWidget {
  const _PromptEditor({
    required this.controller,
    required this.prompt,
    required this.projectPath,
    required this.host,
  });

  final PromptsController controller;
  final SavedPrompt? prompt;
  final String? projectPath;
  final BuildContext host;

  @override
  State<_PromptEditor> createState() => _PromptEditorState();
}

class _PromptEditorState extends State<_PromptEditor> {
  late final TextEditingController _title = TextEditingController(
    text: widget.prompt?.title ?? '',
  );
  late final TextEditingController _body = TextEditingController(
    text: widget.prompt?.body ?? '',
  );
  late String _scope = widget.prompt?.scope ?? scopeAll;
  String? _error;
  bool _busy = false;

  @override
  void dispose() {
    _title.dispose();
    _body.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    final l = context.l10n;
    final draft = PromptDraft(
      id: widget.prompt?.id,
      title: _title.text,
      body: _body.text,
      scope: _scope,
    );
    final invalid = draftError(draft);
    if (invalid != null) {
      setState(() {
        _error = invalid == 'invalid-title'
            ? l.promptsErrTitle
            : l.promptsErrBody;
      });
      return;
    }
    setState(() {
      _busy = true;
      _error = null;
    });
    try {
      await widget.controller.save(draft);
      if (!mounted) return;
      Navigator.pop(context);
      if (widget.host.mounted) toast(widget.host, l.promptsSaved);
    } on Object catch (error) {
      if (!mounted) return;
      setState(() {
        _busy = false;
        _error = promptErrorText(l, error);
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    final project = widget.projectPath;
    final target = _scope != scopeAll ? _scope : project;
    return Padding(
      padding: EdgeInsets.only(bottom: MediaQuery.viewInsetsOf(context).bottom),
      child: SafeArea(
        child: SingleChildScrollView(
          padding: const EdgeInsets.fromLTRB(16, 4, 16, 12),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Padding(
                padding: const EdgeInsets.fromLTRB(6, 4, 6, 10),
                child: Text(
                  widget.prompt == null ? l.promptsAdd : l.promptsEditTitle,
                  style: TextStyle(
                    fontSize: 17,
                    fontWeight: FontWeight.w600,
                    color: c.ink,
                  ),
                ),
              ),
              TextField(
                key: const ValueKey('prompt-title'),
                controller: _title,
                maxLength: maxPromptTitle,
                textInputAction: TextInputAction.next,
                decoration: InputDecoration(labelText: l.promptsFieldTitle),
              ),
              const SizedBox(height: 8),
              TextField(
                key: const ValueKey('prompt-body'),
                controller: _body,
                maxLength: maxPromptBody,
                minLines: 4,
                maxLines: 8,
                keyboardType: TextInputType.multiline,
                decoration: InputDecoration(
                  labelText: l.promptsFieldBody,
                  alignLabelWithHint: true,
                ),
              ),
              if (target != null) ...[
                const SizedBox(height: 8),
                Wrap(
                  spacing: 8,
                  children: [
                    ChoiceChip(
                      key: const ValueKey('prompt-scope-all'),
                      label: Text(l.promptsScopeAll),
                      selected: _scope == scopeAll,
                      onSelected: (_) => setState(() => _scope = scopeAll),
                    ),
                    ChoiceChip(
                      key: const ValueKey('prompt-scope-here'),
                      label: Text(l.promptsScopeHere),
                      selected: _scope != scopeAll,
                      onSelected: (_) => setState(() => _scope = target),
                    ),
                  ],
                ),
              ],
              if (_error != null)
                Padding(
                  padding: const EdgeInsets.only(top: 8),
                  child: Text(
                    _error!,
                    key: const ValueKey('prompt-error'),
                    style: TextStyle(fontSize: 13, color: c.error),
                  ),
                ),
              const SizedBox(height: 12),
              OrreryButton(
                key: const ValueKey('prompt-save'),
                label: l.promptsSave,
                busy: _busy,
                onPressed: _busy ? null : _save,
              ),
            ],
          ),
        ),
      ),
    );
  }
}
