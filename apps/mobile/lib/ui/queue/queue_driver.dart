import 'dart:async';

import 'package:flutter/material.dart';

import '../../app/menu_action.dart';
import '../../l10n/app_localizations.dart';
import '../../remote/chat_protocol.dart';
import '../../remote/protocol.dart';
import '../../security/danger_rules.dart';
import '../commands.dart';
import '../danger_guard.dart';
import '../machine_api.dart';
import 'queue_model.dart';

const Duration queueSnoozeUndo = Duration(seconds: 5);
const Duration queueGoneDelay = Duration(milliseconds: 1500);
const Duration queueReplyTimeout = Duration(seconds: 8);

String queueErrorText(AppLocalizations l, Object error) {
  final code = error is RemoteCommandException ? error.code : 'internal';
  return switch (code) {
    'notWaiting' => l.queueErrNotWaiting,
    'useMenu' => l.queueErrUseMenu,
    'stale' => l.pushMenuStale,
    _ => errorText(l, error),
  };
}

String? _codeOf(Object error) =>
    error is RemoteCommandException ? error.code : null;

class QueueDriver {
  QueueDriver({
    required this.flow,
    required this.api,
    required this.paneContext,
    required this.rules,
    required this.notify,
    required this.isMounted,
    required this.leave,
    this.undo = menuActionUndo,
    this.snoozeUndo = queueSnoozeUndo,
    this.goneDelay = queueGoneDelay,
  });

  final QueueFlow flow;
  final MachineApi api;
  final BuildContext? Function() paneContext;
  final DangerRules Function() rules;
  final VoidCallback notify;
  final bool Function() isMounted;
  final Duration Function(int ms) leave;
  final Duration undo;
  final Duration snoozeUndo;
  final Duration goneDelay;
  final Map<String, Timer> _timers = {};
  final Map<String, TextEditingController> drafts = {};
  final List<TextEditingController> _retiredDrafts = [];
  final Set<String> _replying = {};

  TextEditingController draftOf(String key) =>
      drafts.putIfAbsent(key, TextEditingController.new);

  void dispose() {
    for (final timer in _timers.values) {
      timer.cancel();
    }
    _timers.clear();
    for (final draft in drafts.values) {
      draft.dispose();
    }
    drafts.clear();
    for (final draft in _retiredDrafts) {
      draft.dispose();
    }
    _retiredDrafts.clear();
  }

  void _later(String key, Duration wait, VoidCallback run) {
    _timers.remove(key)?.cancel();
    _timers[key] = Timer(wait, () {
      _timers.remove(key);
      if (isMounted()) run();
    });
  }

  void _retire(String key) {
    flow.remove(key);
    final draft = drafts.remove(key);
    if (draft != null) _retiredDrafts.add(draft);
    notify();
  }

  void watchGone() {
    for (final slot in flow.slots) {
      if (slot.phase == QueuePhase.gone &&
          slot.open &&
          !_timers.containsKey('gone-${slot.key}')) {
        _later('gone-${slot.key}', goneDelay, () => _retire(slot.key));
      }
    }
  }

  void _sent(QueueSlot slot) {
    flow.begin(slot.key, QueuePhase.sent);
    notify();
    final wait = leave(450);
    _later('sent-${slot.key}', wait, () => _retire(slot.key));
  }

  void _fail(QueueSlot slot, Object error, {bool uncertain = false}) {
    final context = paneContext();
    final code = _codeOf(error);
    if (code == 'notWaiting' || code == 'notFound') {
      flow.begin(slot.key, QueuePhase.gone);
    } else if (code == 'stale' || code == 'useMenu') {
      flow.begin(slot.key, QueuePhase.stale);
    } else if (uncertain && (code == 'timeout' || code == 'offline')) {
      flow.begin(slot.key, QueuePhase.unsure);
      slot.unsureCode = code;
    } else {
      flow.resume(slot.key);
    }
    if (context != null && context.mounted && code != 'stale') {
      toast(context, queueErrorText(AppLocalizations.of(context), error));
    }
    watchGone();
    notify();
  }

  Future<void> pick(QueueSlot slot, RemoteMenuItem item) async {
    final context = paneContext();
    final menu = slot.pane.menu;
    if (context == null || menu == null || !slot.actionable) return;
    slot.pickedLabel = item.label;
    flow.begin(slot.key, QueuePhase.pendingUndo);
    notify();
    var delivered = false;
    Object? failure;
    await runMenuAction(
      context: context,
      pane: slot.pane,
      sig: menu.sig,
      index: item.index,
      undo: undo,
      proceed: isMounted,
      send: (sig, index) async {
        try {
          await api.command('ws.menu', {
            'paneKey': slot.key,
            'sig': sig,
            'index': index,
          });
          delivered = true;
        } on Object catch (error) {
          failure = error;
          rethrow;
        }
      },
    );
    if (!isMounted()) return;
    if (delivered) {
      _sent(slot);
      return;
    }
    final error = failure;
    if (error == null) {
      flow.resume(slot.key);
      watchGone();
      notify();
      return;
    }
    final code = _codeOf(error);
    if (code == 'stale') {
      flow.begin(slot.key, QueuePhase.stale);
    } else if (code == 'notFound' || code == 'notWaiting') {
      flow.begin(slot.key, QueuePhase.gone);
    } else {
      flow.resume(slot.key);
    }
    watchGone();
    notify();
  }

  Future<void> reply(QueueSlot slot) async {
    final context = paneContext();
    final value = draftOf(slot.key).text.trim();
    if (context == null || value.isEmpty || !slot.canResend) return;
    if (!_replying.add(slot.key)) return;
    try {
      await _reply(slot, context, value);
    } finally {
      _replying.remove(slot.key);
    }
  }

  Future<void> _reply(
    QueueSlot slot,
    BuildContext context,
    String value,
  ) async {
    final verdict = await confirmDanger(context, rules(), value);
    if (verdict == DangerVerdict.cancelled || !isMounted()) return;
    var authed = verdict == DangerVerdict.authed;
    flow.begin(slot.key, QueuePhase.sending);
    notify();
    for (var attempt = 0; attempt < 2; attempt++) {
      try {
        await api.command('ws.reply', {
          'paneKey': slot.key,
          'text': value,
          if (authed) 'authed': true,
        }, queueReplyTimeout);
        if (isMounted()) _sent(slot);
        return;
      } on Object catch (error) {
        if (!isMounted()) return;
        final retry = attempt == 0 && _codeOf(error) == 'needsAuth';
        final ctx = paneContext();
        if (retry && ctx != null && ctx.mounted && await authorizeNow(ctx)) {
          authed = true;
          continue;
        }
        _fail(slot, error, uncertain: true);
        return;
      }
    }
  }

  Future<void> snooze(QueueSlot slot, int minutes) async {
    if (!slot.actionable) return;
    flow.begin(slot.key, QueuePhase.sending);
    notify();
    try {
      final result = await api.command('ws.snooze', {
        'paneKey': slot.key,
        'minutes': minutes,
      });
      if (!isMounted()) return;
      final applied = result['minutes'];
      slot.snoozeMinutes = applied is num ? applied.toInt() : minutes;
      flow.begin(slot.key, QueuePhase.snoozed);
      notify();
      _later('snooze-${slot.key}', snoozeUndo, () => _retire(slot.key));
    } on Object catch (error) {
      if (isMounted()) _fail(slot, error);
    }
  }

  Future<void> undoSnooze(QueueSlot slot) async {
    if (slot.phase != QueuePhase.snoozed) return;
    _timers.remove('snooze-${slot.key}')?.cancel();
    flow.begin(slot.key, QueuePhase.sending);
    notify();
    try {
      await api.command('ws.snooze', {'paneKey': slot.key, 'minutes': null});
      if (!isMounted()) return;
      flow.resume(slot.key, afterUnsnooze: true);
      watchGone();
      notify();
    } on Object catch (error) {
      if (!isMounted()) return;
      final context = paneContext();
      if (context != null && context.mounted) {
        toast(context, queueErrorText(AppLocalizations.of(context), error));
      }
      _retire(slot.key);
    }
  }

  void skip(QueueSlot slot) {
    if (slot.phase != QueuePhase.ready && slot.phase != QueuePhase.stale) {
      return;
    }
    flow.skip(slot.key);
    notify();
  }

  void reload(QueueSlot slot) {
    flow.refresh(slot.key);
    watchGone();
    notify();
  }
}
