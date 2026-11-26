import 'package:flutter/material.dart';

import '../../l10n/app_localizations.dart';
import '../../media/dictation.dart';
import '../../media/inbox_ignore.dart';
import '../../media/media_flow.dart';
import '../commands.dart';
import '../theme/orrery_shape.dart';
import '../widgets/common.dart';

class ComposerMedia {
  const ComposerMedia({this.dictation, this.photo});

  final Dictation? dictation;
  final MediaFlow? photo;
}

String mediaErrorText(AppLocalizations l, MediaError error) => switch (error) {
  MediaError.cameraDenied => l.mediaCameraDenied,
  MediaError.unsupported => l.mediaErrUnsupported,
  MediaError.tooLarge => l.mediaErrTooLarge,
  MediaError.busy => l.mediaErrBusy,
  MediaError.offline => l.mediaErrOffline,
  MediaError.denied => l.mediaErrDenied,
  MediaError.interrupted => l.mediaErrInterrupted,
  MediaError.failed => l.mediaErrFailed,
};

class MediaRoundButton extends StatelessWidget {
  const MediaRoundButton({
    super.key,
    required this.icon,
    required this.tooltip,
    required this.color,
    this.background,
    this.busy = false,
  });

  final IconData icon;
  final String tooltip;
  final Color color;
  final Color? background;
  final bool busy;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      button: true,
      label: tooltip,
      excludeSemantics: true,
      child: Container(
        width: 44,
        height: 44,
        decoration: BoxDecoration(color: background, shape: BoxShape.circle),
        alignment: Alignment.center,
        child: busy
            ? SizedBox.square(
                dimension: 20,
                child: CircularProgressIndicator(strokeWidth: 2, color: color),
              )
            : Icon(icon, size: 21, color: color),
      ),
    );
  }
}

class PhotoButton extends StatefulWidget {
  const PhotoButton({
    super.key,
    required this.flow,
    required this.enabled,
    required this.onPath,
  });

  final MediaFlow flow;
  final bool enabled;
  final void Function(String path) onPath;

  @override
  State<PhotoButton> createState() => _PhotoButtonState();
}

class _PhotoButtonState extends State<PhotoButton> {
  bool _busy = false;

  Future<PickSource?> _ask() => showModalBottomSheet<PickSource>(
    context: context,
    showDragHandle: true,
    builder: (sheet) {
      final l = sheet.l10n;
      return SafeArea(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            ListTile(
              key: const ValueKey('photo-camera'),
              leading: const Icon(Icons.photo_camera_outlined),
              title: Text(l.mediaTakePhoto),
              onTap: () => Navigator.of(sheet).pop(PickSource.camera),
            ),
            ListTile(
              key: const ValueKey('photo-gallery'),
              leading: const Icon(Icons.photo_library_outlined),
              title: Text(l.mediaChoosePhoto),
              onTap: () => Navigator.of(sheet).pop(PickSource.gallery),
            ),
          ],
        ),
      );
    },
  );

  Future<void> _tap() async {
    if (_busy || !widget.enabled) return;
    final source = await _ask();
    if (source == null || !mounted) return;
    final l = context.l10n;
    setState(() => _busy = true);
    var sent = false;
    try {
      final result = await widget.flow.sendImage(source);
      if (!mounted || result == null) return;
      widget.onPath(result.path);
      toast(context, l.mediaSent);
      sent = true;
    } on MediaFailure catch (error) {
      if (mounted) toast(context, mediaErrorText(l, error.error));
    } finally {
      if (mounted) setState(() => _busy = false);
    }
    if (sent) await _offerIgnore();
  }

  Future<void> _offerIgnore() async {
    if (!mounted || !await widget.flow.shouldOfferIgnore() || !mounted) return;
    final l = context.l10n;
    final add = await showDialog<bool>(
      context: context,
      builder: (dialog) => AlertDialog(
        title: Text(l.mediaIgnoreTitle),
        content: Text(l.mediaIgnoreBody),
        actions: [
          TextButton(
            key: const ValueKey('ignore-skip'),
            onPressed: () => Navigator.of(dialog).pop(false),
            child: Text(l.mediaIgnoreSkip),
          ),
          FilledButton(
            key: const ValueKey('ignore-add'),
            onPressed: () => Navigator.of(dialog).pop(true),
            child: Text(l.mediaIgnoreAdd),
          ),
        ],
      ),
    );
    if (add != true) {
      await widget.flow.declineIgnore();
      return;
    }
    try {
      final result = await widget.flow.acceptIgnore();
      if (!mounted) return;
      toast(
        context,
        switch (result) {
          IgnoreResult.added => l.mediaIgnoreDone,
          IgnoreResult.already => l.mediaIgnoreAlready,
          IgnoreResult.waiting => l.vWaitConfirm,
        },
      );
    } on Object catch (error) {
      if (mounted) toast(context, errorText(l, error));
    }
  }

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    return InkResponse(
      key: const ValueKey('chat-photo'),
      onTap: widget.enabled && !_busy ? _tap : null,
      radius: 24,
      child: MediaRoundButton(
        icon: Icons.add_a_photo_outlined,
        tooltip: _busy ? l.mediaSending : l.mediaPhoto,
        color: widget.enabled ? c.inkMuted : c.inkFaint,
        background: c.raised,
        busy: _busy,
      ),
    );
  }
}

class MicButton extends StatefulWidget {
  const MicButton({
    super.key,
    required this.dictation,
    required this.enabled,
    required this.getText,
    required this.setText,
  });

  final Dictation dictation;
  final bool enabled;
  final String Function() getText;
  final void Function(String text) setText;

  @override
  State<MicButton> createState() => _MicButtonState();
}

class _MicButtonState extends State<MicButton> {
  bool _held = false;
  bool _live = false;

  @override
  void dispose() {
    widget.dictation.cancel();
    super.dispose();
  }

  Future<bool> _askOnline() async {
    final l = context.l10n;
    final ok = await showDialog<bool>(
      context: context,
      builder: (dialog) => AlertDialog(
        title: Text(l.mediaOnlineTitle),
        content: Text(l.mediaOnlineBody),
        actions: [
          TextButton(
            onPressed: () => Navigator.of(dialog).pop(false),
            child: Text(l.cancel),
          ),
          FilledButton(
            key: const ValueKey('mic-online-allow'),
            onPressed: () => Navigator.of(dialog).pop(true),
            child: Text(l.mediaOnlineAllow),
          ),
        ],
      ),
    );
    return ok == true;
  }

  Future<void> _start() async {
    if (!widget.enabled) return;
    _held = true;
    final l = context.l10n;
    final base = widget.getText();
    final locale = speechLocaleFor(Localizations.localeOf(context).languageCode);
    setState(() => _live = true);
    final fail = await widget.dictation.start(
      localeId: locale,
      askOnline: _askOnline,
      onText: (words, done) {
        if (mounted) widget.setText(joinDictated(base, words));
      },
      onEnd: () {
        if (mounted) setState(() => _live = false);
      },
    );
    if (!mounted) return;
    if (fail != null) {
      setState(() => _live = false);
      toast(context, switch (fail) {
        DictationFail.denied => l.mediaMicDenied,
        DictationFail.unavailable => l.mediaMicUnavailable,
        DictationFail.onlineRefused => l.mediaMicRefused,
        DictationFail.failed => l.mediaMicUnavailable,
      });
      return;
    }
    if (!_held) await widget.dictation.stop();
  }

  Future<void> _end() async {
    _held = false;
    await widget.dictation.stop();
    if (mounted) setState(() => _live = false);
  }

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final l = context.l10n;
    return GestureDetector(
      key: const ValueKey('chat-mic'),
      behavior: HitTestBehavior.opaque,
      onTap: () => toast(context, l.mediaMicHint),
      onLongPressStart: (_) => _start(),
      onLongPressEnd: (_) => _end(),
      onLongPressCancel: _end,
      child: MediaRoundButton(
        icon: _live ? Icons.mic_rounded : Icons.mic_none_rounded,
        tooltip: _live ? l.mediaMicListening : l.mediaMicHold,
        color: _live
            ? c.onAccent
            : widget.enabled
            ? c.inkMuted
            : c.inkFaint,
        background: _live ? c.accent : c.raised,
      ),
    );
  }
}
