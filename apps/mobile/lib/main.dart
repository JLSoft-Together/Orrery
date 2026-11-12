import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:orrery_remote/app/bootstrap.dart';
import 'package:orrery_remote/app/deep_link_host.dart';
import 'package:orrery_remote/app/home_page.dart';
import 'package:orrery_remote/app/link_source.dart';
import 'package:orrery_remote/push/push_providers.dart';
import 'package:orrery_remote/l10n/app_localizations.dart';
import 'package:orrery_remote/ui/theme/orrery_theme.dart';
import 'package:orrery_remote/ui/ui_providers.dart';
import 'package:orrery_remote/remote/providers.dart';
import 'package:orrery_remote/remote/remote_host.dart';
import 'package:orrery_remote/src/rust/api/simple.dart';
import 'package:orrery_remote/src/rust/frb_generated.dart';
import 'package:orrery_remote/src/spike/spike_page.dart';
import 'package:xterm/xterm.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  final version = protoVersion();
  final echoed = echo(bytes: utf8.encode('Tiếng Việt ⬛'));
  final host = await buildRemoteHost();
  final push = await buildPushService(host);
  await host.startAll();
  runApp(
    ProviderScope(
      overrides: [
        remoteHostProvider.overrideWithValue(host),
        pushServiceProvider.overrideWithValue(push),
      ],
      child: OrreryRemoteApp(
        rustStatus: 'proto_version = $version, echo = ${echoed.length} bytes',
        host: host,
        linkSource: MergedLinkSource([
          const AppLinksSource(),
          ?push,
        ]),
      ),
    ),
  );
}

class OrreryRemoteApp extends StatefulWidget {
  const OrreryRemoteApp({
    super.key,
    required this.rustStatus,
    this.terminal,
    this.host,
    this.linkSource = const AppLinksSource(),
  });

  final String rustStatus;
  final Terminal? terminal;
  final RemoteHost? host;
  final LinkSource linkSource;

  @override
  State<OrreryRemoteApp> createState() => _OrreryRemoteAppState();
}

class _OrreryRemoteAppState extends State<OrreryRemoteApp>
    with WidgetsBindingObserver {
  final GlobalKey<NavigatorState> _navigator = GlobalKey<NavigatorState>();

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    final host = widget.host;
    if (host == null) return;
    switch (state) {
      case AppLifecycleState.resumed:
        host.setForeground(true);
      case AppLifecycleState.paused:
      case AppLifecycleState.hidden:
      case AppLifecycleState.detached:
        host.setForeground(false);
      case AppLifecycleState.inactive:
    }
  }

  void _openSpike() {
    _navigator.currentState?.push(
      MaterialPageRoute<void>(
        builder: (_) =>
            SpikePage(rustStatus: widget.rustStatus, terminal: widget.terminal),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    if (widget.host == null) {
      return MaterialApp(
        navigatorKey: _navigator,
        title: 'Orrery Remote',
        debugShowCheckedModeBanner: false,
        theme: buildOrreryTheme(RemoteTheme.fallback, Brightness.dark),
        home: SpikePage(
          rustStatus: widget.rustStatus,
          terminal: widget.terminal,
        ),
      );
    }
    return Consumer(
      builder: (context, ref, _) {
        final remote = ref.watch(remoteThemeProvider);
        final locale = ref.watch(remoteLocaleProvider);
        return MaterialApp(
          navigatorKey: _navigator,
          onGenerateTitle: (context) => AppLocalizations.of(context).appTitle,
          debugShowCheckedModeBanner: false,
          theme: buildOrreryTheme(remote, Brightness.light),
          darkTheme: buildOrreryTheme(remote, Brightness.dark),
          themeMode: remote.mode,
          locale: locale,
          localizationsDelegates: AppLocalizations.localizationsDelegates,
          supportedLocales: AppLocalizations.supportedLocales,
          builder: (context, child) => DeepLinkHost(
            navigatorKey: _navigator,
            source: widget.linkSource,
            child: child ?? const SizedBox.shrink(),
          ),
          home: HomePage(onOpenSpike: _openSpike),
        );
      },
    );
  }
}
