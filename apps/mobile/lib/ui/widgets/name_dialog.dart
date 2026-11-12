import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'common.dart';

Future<String?> showNameDialog(
  BuildContext context, {
  required String title,
  required String initial,
  required int maxLength,
  required String hint,
}) => showDialog<String>(
  context: context,
  builder: (dialog) => NameDialog(
    title: title,
    initial: initial,
    maxLength: maxLength,
    hint: hint,
  ),
);

class NameDialog extends StatefulWidget {
  const NameDialog({
    super.key,
    required this.title,
    required this.initial,
    required this.maxLength,
    required this.hint,
  });

  final String title;
  final String initial;
  final int maxLength;
  final String hint;

  @override
  State<NameDialog> createState() => _NameDialogState();
}

class _NameDialogState extends State<NameDialog> {
  late final TextEditingController _controller = TextEditingController(
    text: widget.initial,
  );

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  void _submit() => Navigator.pop(context, _controller.text.trim());

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    return AlertDialog(
      title: Text(widget.title),
      content: TextField(
        key: const ValueKey('rename-field'),
        controller: _controller,
        autofocus: true,
        maxLength: widget.maxLength,
        maxLengthEnforcement: MaxLengthEnforcement.truncateAfterCompositionEnds,
        textInputAction: TextInputAction.done,
        onSubmitted: (_) => _submit(),
        decoration: InputDecoration(hintText: widget.hint),
      ),
      actions: [
        TextButton(
          onPressed: () => Navigator.pop(context),
          child: Text(l.cancel),
        ),
        FilledButton(
          key: const ValueKey('rename-save'),
          onPressed: _submit,
          child: Text(l.save),
        ),
      ],
    );
  }
}
