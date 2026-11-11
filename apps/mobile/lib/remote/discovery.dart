import 'dart:async';

import 'package:bonsoir/bonsoir.dart';

class LanEndpoint {
  const LanEndpoint(this.host, this.port);

  final String host;
  final int port;

  @override
  bool operator ==(Object other) =>
      other is LanEndpoint && other.host == host && other.port == port;

  @override
  int get hashCode => Object.hash(host, port);

  @override
  String toString() => '$host:$port';
}

abstract class LanDiscovery {
  Future<LanEndpoint?> find(String machineId, Duration timeout);
}

class NoDiscovery implements LanDiscovery {
  const NoDiscovery();

  @override
  Future<LanEndpoint?> find(String machineId, Duration timeout) async => null;
}

class BonsoirLanDiscovery implements LanDiscovery {
  const BonsoirLanDiscovery();

  static const String serviceType = '_orrery._tcp';
  static const List<String> machineKeys = ['machine', 'machineId', 'id'];

  static String? machineOf(Map<String, String> attributes) {
    for (final key in machineKeys) {
      final value = attributes[key];
      if (value != null && value.isNotEmpty) return value;
    }
    return null;
  }

  static String? pickHost(List<String> addresses) {
    for (final address in addresses) {
      if (!address.contains(':')) return address;
    }
    return null;
  }

  @override
  Future<LanEndpoint?> find(String machineId, Duration timeout) async {
    final discovery = BonsoirDiscovery(type: serviceType);
    final result = Completer<LanEndpoint?>();
    StreamSubscription<BonsoirDiscoveryEvent>? subscription;
    try {
      await discovery.initialize();
      subscription = discovery.eventStream?.listen((event) {
        switch (event) {
          case BonsoirDiscoveryServiceFoundEvent():
            event.service.resolve(discovery.serviceResolver);
          case BonsoirDiscoveryServiceResolvedEvent():
            final service = event.service;
            if (machineOf(service.attributes) != machineId) return;
            final host = pickHost(service.hostAddresses);
            if (host != null && !result.isCompleted) {
              result.complete(LanEndpoint(host, service.port));
            }
          default:
        }
      });
      await discovery.start();
      return await result.future.timeout(timeout, onTimeout: () => null);
    } catch (_) {
      return null;
    } finally {
      await subscription?.cancel();
      try {
        await discovery.stop();
      } catch (_) {}
    }
  }
}
