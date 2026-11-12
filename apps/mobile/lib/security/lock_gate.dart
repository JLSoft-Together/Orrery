import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

import '../ui/kit/kit.dart';
import '../ui/theme/orrery_shape.dart';
import '../ui/widgets/common.dart';
import 'app_lock.dart';
import 'device_auth.dart';
import 'privacy_screen.dart';

class LockGate extends StatefulWidget {
  const LockGate({
    super.key,
    required this.lock,
    required this.child,
    this.privacy = const PrivacyScreen(),
  });

  final AppLock lock;
  final Widget child;
  final PrivacyScreen privacy;

  @override
  State<LockGate> createState() => _LockGateState();
}

class _LockGateState extends State<LockGate> with WidgetsBindingObserver {
  bool _resumed = true;
  bool? _secure;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    widget.lock.addListener(_syncSecure);
    _syncSecure();
  }

  @override
  void didUpdateWidget(LockGate old) {
    super.didUpdateWidget(old);
    if (old.lock != widget.lock) {
      old.lock.removeListener(_syncSecure);
      widget.lock.addListener(_syncSecure);
      _syncSecure();
    }
  }

  @override
  void dispose() {
    widget.lock.removeListener(_syncSecure);
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  void _syncSecure() {
    final want = widget.lock.enabled;
    if (_secure == want) return;
    _secure = want;
    widget.privacy.setSecure(want);
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    final now =
        state == AppLifecycleState.resumed ||
        (state == AppLifecycleState.inactive &&
            defaultTargetPlatform != TargetPlatform.iOS);
    if (now != _resumed) setState(() => _resumed = now);
  }

  @override
  Widget build(BuildContext context) {
    return ListenableBuilder(
      listenable: widget.lock,
      builder: (context, _) {
        final lock = widget.lock;
        final shut = lock.enabled && (lock.locked || !lock.loaded);
        final cover = lock.enabled && !_resumed && !shut;
        if (shut) {
          WidgetsBinding.instance.addPostFrameCallback(
            (_) => FocusManager.instance.primaryFocus?.unfocus(),
          );
        }
        return Stack(
          fit: StackFit.expand,
          children: [
            ExcludeFocus(
              excluding: shut || cover,
              child: Visibility(
                visible: !shut && !cover,
                maintainState: true,
                child: widget.child,
              ),
            ),
            if (shut)
              LockScreen(lock: lock, privacy: widget.privacy)
            else if (cover)
              const _Cover(),
          ],
        );
      },
    );
  }
}

class _Cover extends StatelessWidget {
  const _Cover();

  @override
  Widget build(BuildContext context) => ColoredBox(
    key: const ValueKey('lock-cover'),
    color: Theme.of(context).colorScheme.surface,
    child: Center(
      child: Icon(
        Icons.lock_outline_rounded,
        size: 48,
        color: Theme.of(context).colorScheme.primary,
      ),
    ),
  );
}

class LockScreen extends StatefulWidget {
  const LockScreen({super.key, required this.lock, required this.privacy});

  final AppLock lock;
  final PrivacyScreen privacy;

  @override
  State<LockScreen> createState() => _LockScreenState();
}

class _LockScreenState extends State<LockScreen> {
  bool _confirming = false;
  bool _wasBusy = false;
  int _shakes = 0;

  @override
  void initState() {
    super.initState();
    widget.lock.addListener(_onLock);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted && widget.lock.loaded) widget.lock.unlock();
    });
  }

  void _onLock() {
    final lock = widget.lock;
    final ended = _wasBusy && !lock.busy;
    _wasBusy = lock.busy;
    if (ended && lock.lastOutcome == AuthOutcome.failed && mounted) {
      setState(() => _shakes++);
    }
  }

  @override
  void dispose() {
    widget.lock.removeListener(_onLock);
    super.dispose();
  }

  Future<void> _askTurnOff() async {
    final insecure = await widget.lock.auth.isDeviceInsecure();
    if (!mounted) return;
    if (insecure) {
      setState(() => _confirming = true);
    } else {
      widget.lock.unlock();
    }
  }

  Future<void> _turnOff() async {
    final done = await widget.lock.turnOffWithoutAuth();
    if (!mounted) return;
    setState(() => _confirming = false);
    if (!done) widget.lock.unlock();
  }

  String _message(BuildContext context) {
    final l = context.l10n;
    return switch (widget.lock.lastOutcome) {
      AuthOutcome.cancelled => l.lockCancelled,
      AuthOutcome.failed => l.lockFailed,
      AuthOutcome.notAvailable => l.lockNoCredentials,
      _ => l.lockBody,
    };
  }

  @override
  Widget build(BuildContext context) {
    final l = context.l10n;
    final c = context.orrery;
    final lock = widget.lock;
    final missing = lock.lastOutcome == AuthOutcome.notAvailable;
    final text = Theme.of(context).textTheme;
    return Scaffold(
      key: const ValueKey('lock-screen'),
      body: SafeArea(
        child: FadeIn(
          child: Center(
            child: Padding(
              padding: const EdgeInsets.all(32),
              child: ConstrainedBox(
                constraints: const BoxConstraints(maxWidth: 420),
                child: SingleChildScrollView(
                  child: Column(
                    mainAxisSize: MainAxisSize.min,
                    children: [
                      ShakeBox(
                        trigger: _shakes,
                        child: const OrbitMark(size: 112),
                      ),
                      const SizedBox(height: 24),
                      Text(
                        l.lockTitle,
                        textAlign: TextAlign.center,
                        style: text.titleLarge?.copyWith(
                          fontWeight: FontWeight.w600,
                        ),
                      ),
                      const SizedBox(height: 10),
                      Text(
                        _confirming ? l.lockTurnOffWarn : _message(context),
                        key: const ValueKey('lock-message'),
                        textAlign: TextAlign.center,
                        style: TextStyle(color: c.inkMuted),
                      ),
                      const SizedBox(height: 32),
                      if (_confirming) ...[
                        OrreryButton(
                          key: const ValueKey('lock-turn-off-confirm'),
                          label: l.lockTurnOff,
                          kind: ButtonKind.danger,
                          onPressed: _turnOff,
                        ),
                        const SizedBox(height: 8),
                        OrreryButton(
                          key: const ValueKey('lock-turn-off-cancel'),
                          label: l.cancel,
                          kind: ButtonKind.text,
                          onPressed: () => setState(() => _confirming = false),
                        ),
                      ] else ...[
                        _FingerprintButton(
                          busy: lock.busy,
                          onTap: lock.busy ? null : lock.unlock,
                        ),
                        const SizedBox(height: 10),
                        Text(l.lockUnlock, style: text.titleSmall),
                        if (missing &&
                            defaultTargetPlatform ==
                                TargetPlatform.android) ...[
                          const SizedBox(height: 16),
                          OrreryButton(
                            key: const ValueKey('lock-open-security'),
                            label: l.lockOpenSecurity,
                            kind: ButtonKind.secondary,
                            onPressed: widget.privacy.openSecuritySettings,
                          ),
                        ],
                        if (missing) ...[
                          const SizedBox(height: 8),
                          OrreryButton(
                            key: const ValueKey('lock-turn-off'),
                            label: l.lockTurnOff,
                            kind: ButtonKind.text,
                            onPressed: _askTurnOff,
                          ),
                        ],
                      ],
                    ],
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}

class _FingerprintButton extends StatelessWidget {
  const _FingerprintButton({required this.busy, required this.onTap});

  final bool busy;
  final VoidCallback? onTap;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    return Semantics(
      button: true,
      child: Material(
        key: const ValueKey('lock-unlock'),
        color: c.selection,
        shape: CircleBorder(
          side: BorderSide(color: c.accent.withValues(alpha: 0.55)),
        ),
        clipBehavior: Clip.antiAlias,
        child: InkWell(
          onTap: onTap,
          child: SizedBox.square(
            dimension: 76,
            child: Center(
              child: AnimatedSwitcher(
                duration: motion(context, 150),
                child: busy
                    ? SizedBox.square(
                        key: const ValueKey('busy'),
                        dimension: 28,
                        child: CircularProgressIndicator(
                          strokeWidth: 2.5,
                          color: c.accent,
                        ),
                      )
                    : Icon(
                        Icons.fingerprint_rounded,
                        key: const ValueKey('icon'),
                        size: 40,
                        color: c.accent,
                      ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
