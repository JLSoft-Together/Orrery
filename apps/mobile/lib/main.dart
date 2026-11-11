import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:orrery_remote/app/bootstrap.dart';
import 'package:orrery_remote/app/home_page.dart';
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
  await host.startAll();
  runApp(
    ProviderScope(
      overrides: [remoteHostProvider.overrideWithValue(host)],
      child: OrreryRemoteApp(
        rustStatus: 'proto_version = $version, echo = ${echoed.length} bytes',
        host: host,
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
  });

  final String rustStatus;
  final Terminal? terminal;
  final RemoteHost? host;

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
    return MaterialApp(
      navigatorKey: _navigator,
      title: 'Orrery Remote',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorSchemeSeed: Colors.indigo,
        brightness: Brightness.dark,
      ),
      home: widget.host == null
          ? SpikePage(rustStatus: widget.rustStatus, terminal: widget.terminal)
          : HomePage(onOpenSpike: _openSpike),
    );
  }
}
