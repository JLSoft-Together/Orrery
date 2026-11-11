import 'dart:convert';
import 'dart:io';

import 'package:flutter_rust_bridge/flutter_rust_bridge_for_generated.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/connection.dart';
import 'package:orrery_remote/remote/crypto_rust.dart';
import 'package:orrery_remote/remote/keys.dart';
import 'package:orrery_remote/remote/link_state.dart';
import 'package:orrery_remote/remote/machines.dart';
import 'package:orrery_remote/remote/wire.dart';
import 'package:orrery_remote/src/rust/frb_generated.dart';

String libraryPath() {
  final name = Platform.isWindows
      ? 'rust_lib_orrery_remote.dll'
      : Platform.isMacOS
      ? 'librust_lib_orrery_remote.dylib'
      : 'librust_lib_orrery_remote.so';
  return 'rust/target/debug/$name';
}

String gatewayPath() {
  final exe = Platform.isWindows ? '.exe' : '';
  return 'rust/target/debug/examples/fake_gateway$exe';
}

class Gateway {
  Gateway(this.process, this.qr, this.port, this.machine);

  final Process process;
  final String qr;
  final int port;
  final String machine;

  static Future<Gateway> start({int expIn = 300}) async {
    final process = await Process.start(File(gatewayPath()).absolute.path, [
      '0',
      '$expIn',
    ]);
    final line = await process.stdout
        .transform(utf8.decoder)
        .transform(const LineSplitter())
        .first
        .timeout(const Duration(seconds: 20));
    final info = jsonDecode(line) as Map<String, dynamic>;
    return Gateway(
      process,
      info['qr'] as String,
      info['port'] as int,
      info['machine'] as String,
    );
  }

  void stop() => process.kill();
}

ConnectionDeps realDeps(MachineRepository repo, KeyStore keys) =>
    ConnectionDeps(
      crypto: const RustRemoteCrypto(),
      connector: IoWireSocket.connect,
      keys: keys,
      machines: repo,
      config: const ConnectionConfig(
        handshakeTimeout: Duration(seconds: 5),
        backoff: [Duration(milliseconds: 100), Duration(milliseconds: 200)],
      ),
    );

Future<LinkState> waitFor(
  MachineConnection conn,
  bool Function(LinkState) test, {
  Duration timeout = const Duration(seconds: 10),
}) async {
  if (test(conn.state)) return conn.state;
  return conn.states.firstWhere(test).timeout(timeout);
}

Future<void> buildGateway() async {
  final build = await Process.run('cargo', [
    'build',
    '-j',
    '3',
    '--examples',
  ], workingDirectory: 'rust');
  expect(build.exitCode, 0, reason: '${build.stderr}');
  await RustLib.init(externalLibrary: ExternalLibrary.open(libraryPath()));
}
