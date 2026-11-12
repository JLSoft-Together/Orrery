import 'dart:developer' as developer;
import 'dart:io';
import 'dart:ui';

import 'package:path_provider/path_provider.dart';

import '../push/push_gateway.dart';
import '../push/push_service.dart';
import '../push/remote_host_push.dart';
import '../remote/connection.dart';
import '../remote/crypto_rust.dart';
import '../remote/discovery.dart';
import '../remote/keys.dart';
import '../remote/machines.dart';
import '../remote/network_watcher.dart';
import '../remote/remote_host.dart';
import '../remote/secure_kv.dart';
import '../remote/wire.dart';

Future<PushService?> buildPushService(RemoteHost host) async {
  try {
    final service = PushService(
      gateway: FirebasePushGateway(),
      kv: FlutterSecureKv(),
      host: RemoteHostPush(host),
      platform: Platform.isIOS ? 'ios' : 'android',
      localeCode: () => PlatformDispatcher.instance.locale.languageCode,
    );
    await service.init();
    return service;
  } on Object catch (error, stack) {
    developer.log(
      'push disabled',
      name: 'orrery.push',
      error: error,
      stackTrace: stack,
    );
    return null;
  }
}

Future<RemoteHost> buildRemoteHost() async {
  final support = await getApplicationSupportDirectory();
  final cache = await getApplicationCacheDirectory();
  final secure = FlutterSecureKv();
  const crypto = RustRemoteCrypto();
  final platform = Platform.isIOS ? 'ios' : 'android';
  final deps = ConnectionDeps(
    crypto: crypto,
    connector: IoWireSocket.connect,
    keys: KeyStore(secure, crypto),
    machines: MachineRepository(directory: support, secure: secure),
    discovery: const BonsoirLanDiscovery(),
    network: ConnectivityNetworkWatcher(),
    config: ConnectionConfig(platform: platform),
  );
  final host = RemoteHost(
    deps: deps,
    cacheDirectory: Directory(
      '${cache.path}${Platform.pathSeparator}remote_cache',
    ),
    deviceName: Platform.isIOS ? 'iPhone' : 'Android phone',
  );
  await host.init();
  return host;
}
