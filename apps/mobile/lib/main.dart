import 'dart:convert';

import 'package:flutter/material.dart';
import 'package:orrery_remote/src/rust/api/simple.dart';
import 'package:orrery_remote/src/rust/frb_generated.dart';
import 'package:orrery_remote/src/spike/spike_page.dart';
import 'package:xterm/xterm.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  await RustLib.init();
  final version = protoVersion();
  final echoed = echo(bytes: utf8.encode('Tiếng Việt ⬛'));
  runApp(OrreryRemoteApp(rustStatus: 'proto_version = $version, echo = ${echoed.length} bytes'));
}

class OrreryRemoteApp extends StatelessWidget {
  const OrreryRemoteApp({super.key, required this.rustStatus, this.terminal});

  final String rustStatus;
  final Terminal? terminal;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Orrery Remote',
      debugShowCheckedModeBanner: false,
      theme: ThemeData(colorSchemeSeed: Colors.indigo, brightness: Brightness.dark),
      home: SpikePage(rustStatus: rustStatus, terminal: terminal),
    );
  }
}
