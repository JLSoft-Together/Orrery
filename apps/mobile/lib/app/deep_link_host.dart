import 'dart:async';

import 'package:clock/clock.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../l10n/app_localizations.dart';
import '../remote/providers.dart';
import '../ui/commands.dart';
import '../ui/model/workspace_model.dart';
import '../ui/ui_providers.dart';
import '../ui/workspace/machine_scope.dart';
import '../ui/workspace/tab_page.dart';
import '../ui/workspace/workspace_page.dart';
import '../security/app_lock.dart';
import 'link_source.dart';
import 'pane_link.dart';

const Duration linkDedupe = Duration(seconds: 2);
const Duration linkWait = Duration(seconds: 20);

class DeepLinkHost extends ConsumerStatefulWidget {
  const DeepLinkHost({
    super.key,
    required this.navigatorKey,
    required this.source,
    required this.child,
    this.lock,
  });

  final GlobalKey<NavigatorState> navigatorKey;
  final LinkSource source;
  final Widget child;
  final AppLock? lock;

  @override
  ConsumerState<DeepLinkHost> createState() => _DeepLinkHostState();
}

class _DeepLinkHostState extends ConsumerState<DeepLinkHost> {
  StreamSubscription<Uri>? _sub;
  ProviderSubscription<WorkspaceModel>? _watch;
  Timer? _timeout;
  PaneLink? _last;
  DateTime? _lastAt;
  PaneLink? _held;

  @override
  void initState() {
    super.initState();
    _sub = widget.source.links.listen(_onUri, onError: (Object _) {});
    widget.lock?.addListener(_onLockChanged);
  }

  @override
  void dispose() {
    _sub?.cancel();
    widget.lock?.removeListener(_onLockChanged);
    _stopWaiting();
    super.dispose();
  }

  void _stopWaiting() {
    _watch?.close();
    _watch = null;
    _timeout?.cancel();
    _timeout = null;
  }

  bool get _shut {
    final lock = widget.lock;
    return lock != null && lock.enabled && (lock.locked || !lock.loaded);
  }

  void _onLockChanged() {
    final held = _held;
    if (held == null || _shut) return;
    _held = null;
    _open(held);
  }

  void _onUri(Uri uri) {
    final link = parsePaneLink(uri);
    if (link == null) return;
    final now = clock.now();
    final at = _lastAt;
    if (link == _last && at != null && now.difference(at) < linkDedupe) return;
    _last = link;
    _lastAt = now;
    if (widget.lock?.expired ?? false) widget.lock?.lock();
    if (_shut) {
      _held = link;
      return;
    }
    _open(link);
  }

  void _open(PaneLink link, {bool retried = false}) {
    _stopWaiting();
    final nav = widget.navigatorKey.currentState;
    final context = widget.navigatorKey.currentContext;
    if (nav == null || context == null) {
      if (!retried) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (mounted) _open(link, retried: true);
        });
        WidgetsBinding.instance.ensureVisualUpdate();
      }
      return;
    }
    final known = ref.read(machinesProvider).any((m) => m.id == link.machineId);
    if (!known) {
      toast(context, AppLocalizations.of(context).linkUnknownMachine);
      return;
    }
    ref.read(activeMachineProvider.notifier).select(link.machineId);
    nav.popUntil((route) => route.isFirst);
    nav.push(
      MaterialPageRoute<void>(
        builder: (_) => WorkspacePage(machineId: link.machineId),
      ),
    );
    _watch = ref.listenManual(
      workspaceModelProvider(link.machineId),
      (_, model) => _check(link, model),
      fireImmediately: true,
    );
    _timeout = Timer(linkWait, _stopWaiting);
  }

  void _check(PaneLink link, WorkspaceModel model) {
    final nav = widget.navigatorKey.currentState;
    final context = widget.navigatorKey.currentContext;
    if (nav == null || context == null) return;
    final tab = model.tabOfPane(link.paneKey);
    if (tab != null) {
      _stopWaiting();
      nav.push(
        machineRoute<void>(
          link.machineId,
          (_) => TabPage(
            machineId: link.machineId,
            tabKey: tab.key,
            paneKey: link.paneKey,
          ),
        ),
      );
      return;
    }
    final view = ref.read(machineViewProvider(link.machineId));
    if (view.workspace != null && !view.stale) {
      _stopWaiting();
      toast(context, AppLocalizations.of(context).linkPaneGone);
    }
  }

  @override
  Widget build(BuildContext context) => widget.child;
}
