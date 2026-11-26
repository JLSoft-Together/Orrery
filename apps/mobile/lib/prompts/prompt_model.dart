import 'package:flutter/widgets.dart';

import '../remote/protocol.dart';

const String promptsCap = 'prompts';
const int maxPromptTitle = 80;
const int maxPromptBody = 4000;
const int composerLimit = 2000;
const String scopeAll = 'all';

class SavedPrompt {
  const SavedPrompt({
    required this.id,
    required this.title,
    required this.body,
    required this.scope,
    this.text,
    this.missing = const [],
    this.local = false,
  });

  final String id;
  final String title;
  final String body;
  final String scope;
  final String? text;
  final List<String> missing;
  final bool local;

  String get insertText => text ?? body;

  static SavedPrompt? parse(Object? raw) {
    if (raw is! Map) return null;
    final id = raw['id'];
    final title = raw['title'];
    final body = raw['body'];
    if (id is! String || title is! String || body is! String) return null;
    final scope = raw['scope'];
    final text = raw['text'];
    final missing = raw['missing'];
    return SavedPrompt(
      id: id,
      title: title,
      body: body,
      scope: scope is String ? scope : scopeAll,
      text: text is String ? text : null,
      missing: missing is List
          ? missing.whereType<String>().toList()
          : const [],
      local: raw['local'] == true,
    );
  }
}

class PromptDraft {
  const PromptDraft({
    this.id,
    required this.title,
    required this.body,
    this.scope = scopeAll,
  });

  final String? id;
  final String title;
  final String body;
  final String scope;

  JsonMap toFields() => {
    'promptId': ?id,
    'title': title.trim(),
    'body': body,
    'scope': scope,
  };
}

class PromptList {
  const PromptList({this.items = const [], this.hidden = 0, this.more = 0});

  final List<SavedPrompt> items;
  final int hidden;
  final int more;

  static PromptList parse(JsonMap data) {
    final raw = data['items'];
    final hidden = data['hidden'];
    final more = data['more'];
    return PromptList(
      items: [
        if (raw is List)
          for (final entry in raw.take(200)) ?SavedPrompt.parse(entry),
      ],
      hidden: hidden is int ? hidden : 0,
      more: more is int ? more : 0,
    );
  }
}

String? draftError(PromptDraft draft) {
  final title = draft.title.trim();
  if (title.isEmpty || title.length > maxPromptTitle) return 'invalid-title';
  if (draft.body.trim().isEmpty || draft.body.length > maxPromptBody) {
    return 'invalid-body';
  }
  return null;
}

TextEditingValue? insertAtCursor(
  TextEditingValue current,
  String insert, {
  int limit = composerLimit,
}) {
  if (insert.isEmpty) return current;
  final text = current.text;
  final sel = current.selection;
  final start = sel.isValid ? sel.start.clamp(0, text.length) : text.length;
  final end = sel.isValid ? sel.end.clamp(start, text.length) : text.length;
  final before = text.substring(0, start);
  final glue = before.isNotEmpty && !RegExp(r'\s$').hasMatch(before) ? ' ' : '';
  final added = '$glue$insert';
  final next = '$before$added${text.substring(end)}';
  if (next.length > limit) return null;
  return TextEditingValue(
    text: next,
    selection: TextSelection.collapsed(offset: before.length + added.length),
  );
}
