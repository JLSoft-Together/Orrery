import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';

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

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted && widget.lock.loaded) widget.lock.unlock();
    });
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
    final scheme = Theme.of(context).colorScheme;
    final lock = widget.lock;
    final missing = lock.lastOutcome == AuthOutcome.notAvailable;
    return Scaffold(
      key: const ValueKey('lock-screen'),
      body: SafeArea(
        child: Center(
          child: Padding(
            padding: const EdgeInsets.all(32),
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 420),
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  Icon(Icons.lock_rounded, size: 64, color: scheme.primary),
                  const SizedBox(height: 20),
                  Text(
                    l.lockTitle,
                    textAlign: TextAlign.center,
                    style: Theme.of(context).textTheme.titleLarge,
                  ),
                  const SizedBox(height: 10),
                  Text(
                    _confirming ? l.lockTurnOffWarn : _message(context),
                    key: const ValueKey('lock-message'),
                    textAlign: TextAlign.center,
                    style: TextStyle(color: scheme.onSurfaceVariant),
                  ),
                  const SizedBox(height: 24),
                  if (_confirming) ...[
                    FilledButton(
                      key: const ValueKey('lock-turn-off-confirm'),
                      onPressed: _turnOff,
                      child: Text(l.lockTurnOff),
                    ),
                    const SizedBox(height: 8),
                    TextButton(
                      key: const ValueKey('lock-turn-off-cancel'),
                      onPressed: () => setState(() => _confirming = false),
                      child: Text(l.cancel),
                    ),
                  ] else ...[
                    FilledButton.icon(
                      key: const ValueKey('lock-unlock'),
                      onPressed: lock.busy ? null : lock.unlock,
                      icon: lock.busy
                          ? const SizedBox(
                              width: 16,
                              height: 16,
                              child: CircularProgressIndicator(strokeWidth: 2),
                            )
                          : const Icon(Icons.fingerprint_rounded),
                      label: Text(l.lockUnlock),
                    ),
                    if (missing &&
                        defaultTargetPlatform == TargetPlatform.android) ...[
                      const SizedBox(height: 8),
                      OutlinedButton(
                        key: const ValueKey('lock-open-security'),
                        onPressed: widget.privacy.openSecuritySettings,
                        child: Text(l.lockOpenSecurity),
                      ),
                    ],
                    if (missing) ...[
                      const SizedBox(height: 8),
                      TextButton(
                        key: const ValueKey('lock-turn-off'),
                        onPressed: _askTurnOff,
                        child: Text(l.lockTurnOff),
                      ),
                    ],
                  ],
                ],
              ),
            ),
          ),
        ),
      ),
    );
  }
}
