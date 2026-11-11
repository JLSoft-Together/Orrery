import 'keys.dart';
import 'crypto.dart';
import 'discovery.dart';
import 'machines.dart';
import 'wire.dart';

abstract class NetworkWatcher {
  Stream<String> get changes;
}

class NoNetworkWatcher implements NetworkWatcher {
  const NoNetworkWatcher();

  @override
  Stream<String> get changes => const Stream.empty();
}

class ConnectionConfig {
  const ConnectionConfig({
    this.lanBudget = const Duration(milliseconds: 1500),
    this.relayConnectTimeout = const Duration(seconds: 8),
    this.handshakeTimeout = const Duration(seconds: 5),
    this.backgroundGrace = const Duration(seconds: 30),
    this.keepalive = const Duration(seconds: 30),
    this.deadAfter = const Duration(seconds: 75),
    this.backoff = const [
      Duration(seconds: 1),
      Duration(seconds: 2),
      Duration(seconds: 4),
      Duration(seconds: 8),
      Duration(seconds: 16),
      Duration(seconds: 30),
    ],
    this.lanPort = 4320,
    this.proto = 1,
    this.appName = 'orrery-remote',
    this.platform = 'android',
  });

  final Duration lanBudget;
  final Duration relayConnectTimeout;
  final Duration handshakeTimeout;
  final Duration backgroundGrace;
  final Duration keepalive;
  final Duration deadAfter;
  final List<Duration> backoff;
  final int lanPort;
  final int proto;
  final String appName;
  final String platform;

  Duration delayFor(int failures) {
    final index = failures <= 0 ? 0 : failures - 1;
    return backoff[index >= backoff.length ? backoff.length - 1 : index];
  }
}

class ConnectionDeps {
  const ConnectionDeps({
    required this.crypto,
    required this.connector,
    required this.keys,
    required this.machines,
    this.discovery = const NoDiscovery(),
    this.network = const NoNetworkWatcher(),
    this.config = const ConnectionConfig(),
  });

  final RemoteCrypto crypto;
  final WireConnector connector;
  final KeyStore keys;
  final MachineRepository machines;
  final LanDiscovery discovery;
  final NetworkWatcher network;
  final ConnectionConfig config;
}

String relayPhoneUrl(String relay, String machineId, String token) {
  var base = relay.trim();
  if (base.startsWith('https://')) {
    base = 'wss://${base.substring(8)}';
  } else if (base.startsWith('http://')) {
    base = 'ws://${base.substring(7)}';
  }
  while (base.endsWith('/')) {
    base = base.substring(0, base.length - 1);
  }
  final id = Uri.encodeComponent(machineId);
  final query = Uri.encodeQueryComponent(token);
  return '$base/p/$id?t=$query';
}
