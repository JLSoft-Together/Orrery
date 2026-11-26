import 'dart:async';
import 'dart:io';

import 'package:flutter/services.dart';

import '../remote/crypto.dart';

const String pairChannelName = 'dev.ccm.orrery.remote/pair';

class PairRequest {
  const PairRequest(this.qr, {required this.fromShell, this.fromLink = false});

  final String qr;
  final bool fromShell;
  final bool fromLink;

  @override
  bool operator ==(Object other) =>
      other is PairRequest &&
      other.qr == qr &&
      other.fromShell == fromShell &&
      other.fromLink == fromLink;

  @override
  int get hashCode => Object.hash(qr, fromShell, fromLink);
}

abstract class PairSource {
  Stream<PairRequest> get requests;
}

class NoPairSource implements PairSource {
  const NoPairSource();

  @override
  Stream<PairRequest> get requests => const Stream.empty();
}

class ChannelPairSource implements PairSource {
  ChannelPairSource({MethodChannel? channel})
    : _channel = channel ?? const MethodChannel(pairChannelName);

  final MethodChannel _channel;
  late final StreamController<PairRequest> _controller =
      StreamController<PairRequest>.broadcast(onListen: _start);
  bool _started = false;

  @override
  Stream<PairRequest> get requests => _controller.stream;

  PairRequest? _decode(Object? raw) {
    if (raw is! Map) return null;
    final qr = raw['qr'];
    if (qr is! String || qr.isEmpty) return null;
    return PairRequest(qr, fromShell: raw['fromShell'] == true);
  }

  Future<void> _start() async {
    if (_started) return;
    _started = true;
    _channel.setMethodCallHandler((call) async {
      if (call.method != 'pair') return null;
      final request = _decode(call.arguments);
      if (request != null) _controller.add(request);
      return null;
    });
    try {
      final pending = _decode(await _channel.invokeMethod<Object?>('ready'));
      if (pending != null) _controller.add(pending);
    } on PlatformException {
      return;
    } on MissingPluginException {
      return;
    }
  }
}

typedef LocalAddresses = Future<List<String>> Function();

Future<List<String>> phoneIpv4Addresses() async {
  try {
    final interfaces = await NetworkInterface.list(
      type: InternetAddressType.IPv4,
      includeLoopback: false,
    );
    return [
      for (final i in interfaces)
        for (final a in i.addresses) a.address,
    ];
  } on SocketException {
    return const [];
  }
}

List<int>? _octets(String ip) {
  final parts = ip.split('.');
  if (parts.length != 4) return null;
  final out = <int>[];
  for (final p in parts) {
    final n = int.tryParse(p);
    if (n == null || n < 0 || n > 255) return null;
    out.add(n);
  }
  return out;
}

bool sameSubnet24(String a, String b) {
  final x = _octets(a);
  final y = _octets(b);
  if (x == null || y == null) return false;
  return x[0] == y[0] && x[1] == y[1] && x[2] == y[2];
}

bool lanMatchesPhone(Iterable<String> lanHosts, Iterable<String> phoneIps) {
  for (final host in lanHosts) {
    for (final ip in phoneIps) {
      if (sameSubnet24(host, ip)) return true;
    }
  }
  return false;
}

List<String> qrLanHosts(QrData qr) => [for (final e in qr.lan) e.host];

const String loopbackHost = '127.0.0.1';

bool qrHasLoopback(QrData qr) => qr.lan.any((e) => e.host == loopbackHost);

String? qrFirstIp(QrData qr, {bool skipLoopback = false}) {
  for (final e in qr.lan) {
    if (!skipLoopback || e.host != loopbackHost) return e.host;
  }
  if (skipLoopback && qr.lan.isNotEmpty) return null;
  final host = qr.host;
  if (host == null || host.isEmpty) return null;
  if (skipLoopback && host == loopbackHost) return null;
  return host;
}
