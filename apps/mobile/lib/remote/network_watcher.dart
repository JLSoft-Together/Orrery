import 'dart:async';

import 'package:connectivity_plus/connectivity_plus.dart';

import 'connection.dart';

String networkSignature(List<ConnectivityResult> results) {
  final active =
      results
          .where((r) => r != ConnectivityResult.none)
          .map((r) => r.name)
          .toList()
        ..sort();
  return active.isEmpty ? 'none' : active.join('+');
}

class ConnectivityNetworkWatcher implements NetworkWatcher {
  ConnectivityNetworkWatcher([Connectivity? connectivity])
    : _connectivity = connectivity ?? Connectivity();

  final Connectivity _connectivity;

  @override
  Stream<String> get changes async* {
    yield networkSignature(await _connectivity.checkConnectivity());
    yield* _connectivity.onConnectivityChanged.map(networkSignature);
  }
}
