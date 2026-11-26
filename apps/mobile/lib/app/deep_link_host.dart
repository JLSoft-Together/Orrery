import 'dart:async';

import 'package:clock/clock.dart';
import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';

import '../l10n/app_localizations.dart';
import '../remote/chat_last.dart';
import '../remote/providers.dart';
import '../ui/chat/chat_screen.dart';
import '../ui/commands.dart';
import '../ui/model/workspace_model.dart';
import '../ui/queue/queue_nudge.dart';
import '../ui/queue/queue_page.dart';
import '../ui/ui_providers.dart';
import '../ui/workspace/machine_scope.dart';
import '../ui/workspace/tab_page.dart';
import '../ui/workspace/workspace_page.dart';
import '../security/app_lock.dart';
import 'app_lock_scope.dart';
import '../remote/crypto.dart';
import 'link_source.dart';
import 'menu_action.dart';
import 'pair_confirm_sheet.dart';
import 'pair_request.dart';
import 'pane_link.dart';

typedef QrDecoder = QrData Function(String text);

const Duration linkDedupe = Duration(seconds: 2);
const Duration linkWait = Duration(seconds: 20);

class DeepLinkHost extends ConsumerStatefulWidget {
  const DeepLinkHost({
    super.key,
    required this.navigatorKey,
    required this.source,
    required this.child,
    this.lock,
    this.pairSource,
    this.decodeQr,
    this.localAddresses = phoneIpv4Addresses,
  });

  final GlobalKey<NavigatorState> navigatorKey;
  final LinkSource source;
  final Widget child;
  final AppLock? lock;
  final PairSource? pairSource;
  final QrDecoder? decodeQr;
  final LocalAddresses localAddresses;

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
  StreamSubscription<PairRequest>? _pairSub;
  late final PairSource _pairSource = widget.pairSource ?? ChannelPairSource();
  PairRequest? _lastPair;
  DateTime? _lastPairAt;
  PairRequest? _heldPair;
  int _sheetToken = 0;
  bool _sheetOpen = false;

  @override
  void initState() {
    super.initState();
    _sub = widget.source.links.listen(_onUri, onError: (Object _) {});
    _pairSub = _pairSource.requests.listen(_onPair, onError: (Object _) {});
    widget.lock?.addListener(_onLockChanged);
  }

  @override
  void dispose() {
    _sub?.cancel();
    _pairSub?.cancel();
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
    if (_shut) return;
    final heldPair = _heldPair;
    if (heldPair != null) {
      _heldPair = null;
      _confirmPair(heldPair);
    }
    final held = _held;
    if (held == null) return;
    _held = null;
    _open(held);
  }

  void _onPair(PairRequest request) {
    final now = clock.now();
    final at = _lastPairAt;
    if (request == _lastPair && at != null && now.difference(at) < linkDedupe) {
      return;
    }
    _lastPair = request;
    _lastPairAt = now;
    if (widget.lock?.expired ?? false) widget.lock?.lock();
    if (_shut) {
      _heldPair = request;
      return;
    }
    _confirmPair(request);
  }

  Future<void> _confirmPair(PairRequest request, {bool retried = false}) async {
    final nav = widget.navigatorKey.currentState;
    final context = nav?.overlay?.context;
    if (nav == null || context == null) {
      if (!retried) {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (mounted) _confirmPair(request, retried: true);
        });
        WidgetsBinding.instance.ensureVisualUpdate();
      }
      return;
    }
    final QrData info;
    try {
      final decode =
          widget.decodeQr ?? ref.read(remoteHostProvider).deps.crypto.decodeQr;
      info = decode(request.qr);
    } on Object {
      toast(context, AppLocalizations.of(context).pairInvalidCode);
      return;
    }
    var blocked = false;
    if (request.fromLink || !qrHasLoopback(info)) {
      final phone = await widget.localAddresses();
      blocked = !lanMatchesPhone(qrLanHosts(info), phone);
    }
    if (!mounted) return;
    await _showSheet(request, info, blocked);
  }

  Future<void> _showSheet(
    PairRequest request,
    QrData info,
    bool blocked,
  ) async {
    final nav = widget.navigatorKey.currentState;
    final sheetContext = nav?.overlay?.context;
    if (nav == null || sheetContext == null) return;
    if (_sheetOpen) nav.pop();
    final token = ++_sheetToken;
    _sheetOpen = true;
    await showPairConfirm(
      sheetContext,
      request: request,
      info: info,
      networkBlocked: blocked,
    );
    if (token == _sheetToken) _sheetOpen = false;
  }

  void _onUri(Uri uri) {
    final qr = parsePairLink(uri);
    if (qr != null) {
      _onPair(PairRequest(qr, fromShell: false, fromLink: true));
      return;
    }
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
    final pane = link.paneKey;
    if (pane == null) {
      if (link.queue) {
        nav.push(
          machineRoute<void>(
            link.machineId,
            (_) => QueuePage(machineId: link.machineId),
          ),
        );
      }
      return;
    }
    _watch = ref.listenManual(
      workspaceModelProvider(link.machineId),
      (_, model) => _check(link, pane, model),
      fireImmediately: true,
    );
    _timeout = Timer(linkWait, _stopWaiting);
  }

  void _check(PaneLink link, String paneKey, WorkspaceModel model) {
    final machineId = link.machineId;
    final nav = widget.navigatorKey.currentState;
    final context = widget.navigatorKey.currentContext;
    if (nav == null || context == null) return;
    final tab = model.tabOfPane(paneKey);
    if (tab != null) {
      _stopWaiting();
      final pane = tab.pane(paneKey);
      final chat =
          pane != null &&
          pane.isAgent &&
          ref.read(machineViewProvider(machineId)).chatEnabled;
      nav.push(
        machineRoute<void>(
          machineId,
          (_) => QueueNudge(
            machineId: machineId,
            paneKey: paneKey,
            child: chat
                ? ChatScreen(machineId: machineId, paneKey: paneKey)
                : TabPage(
                    machineId: machineId,
                    tabKey: tab.key,
                    paneKey: paneKey,
                  ),
          ),
        ),
      );
      _runMenu(link, context, pane);
      return;
    }
    final view = ref.read(machineViewProvider(machineId));
    if (view.workspace != null && !view.stale) {
      _stopWaiting();
      toast(context, AppLocalizations.of(context).linkPaneGone);
    }
  }

  void _runMenu(PaneLink link, BuildContext context, PaneInfo? pane) {
    final sig = link.menuSig;
    final index = link.menuIndex;
    if (sig == null || index == null) return;
    final api = ref.read(machineApiProvider(link.machineId));
    final paneKey = link.paneKey;
    unawaited(
      runMenuAction(
        context: context,
        pane: pane,
        sig: sig,
        index: index,
        send: (sig, index) async {
          await api.command('ws.menu', {
            'paneKey': paneKey,
            'sig': sig,
            'index': index,
          });
        },
      ),
    );
  }

  @override
  Widget build(BuildContext context) =>
      AppLockScope(lock: widget.lock, child: widget.child);
}
