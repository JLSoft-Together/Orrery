import 'dart:async';
import 'dart:io';

import 'package:clock/clock.dart';
import 'package:cryptography/dart.dart';
import 'package:flutter/foundation.dart';

import 'apk_io.dart';
import 'apk_offer.dart';

enum ApkPhase {
  idle,
  downloading,
  verifying,
  installing,
  started,
  needPermission,
  failed,
}

const Duration apkIdleTimeout = Duration(seconds: 15);
const Duration apkExpiryGrace = Duration(minutes: 2);
const int apkSpaceMargin = 16 * 1024 * 1024;
const String apkFilePrefix = 'offer-';

class ApkInstallState {
  const ApkInstallState(
    this.phase, {
    this.received = 0,
    this.total = 0,
    this.error,
  });

  final ApkPhase phase;
  final int received;
  final int total;
  final ApkError? error;

  double? get fraction =>
      total <= 0 ? null : (received / total).clamp(0.0, 1.0);

  bool get busy =>
      phase == ApkPhase.downloading ||
      phase == ApkPhase.verifying ||
      phase == ApkPhase.installing;
}

String _hex(List<int> bytes) =>
    bytes.map((b) => b.toRadixString(16).padLeft(2, '0')).join();

Future<void> cleanApkDir(
  Directory dir, {
  Duration? olderThan,
  DateTime? now,
}) async {
  try {
    if (!await dir.exists()) return;
    final at = now ?? clock.now();
    await for (final entity in dir.list()) {
      if (entity is File) {
        try {
          if (olderThan != null &&
              at.difference(await entity.lastModified()) < olderThan) {
            continue;
          }
          await entity.delete();
        } on FileSystemException {
          continue;
        }
      }
    }
  } on FileSystemException {
    return;
  }
}

class ApkInstallFlow extends ChangeNotifier {
  ApkInstallFlow({
    required this.offer,
    required this.source,
    required this.installer,
    required this.dir,
    this.idle = apkIdleTimeout,
    DateTime Function()? now,
  }) : _now = now ?? (() => clock.now());

  final ApkOffer offer;
  final ApkSource source;
  final ApkInstaller installer;
  final Directory dir;
  final Duration idle;
  final DateTime Function() _now;

  ApkInstallState _state = const ApkInstallState(ApkPhase.idle);
  bool _cancelled = false;
  bool _busy = false;
  bool _disposed = false;
  File? _file;

  ApkInstallState get state => _state;

  void _set(ApkInstallState next) {
    _state = next;
    if (!_disposed) notifyListeners();
  }

  void _fail(ApkError error) =>
      _set(ApkInstallState(ApkPhase.failed, error: error));

  Future<void> start() async {
    if (_busy) return;
    _busy = true;
    _cancelled = false;
    try {
      await _run();
    } on ApkFailure catch (e) {
      await _discard();
      if (!_cancelled) _fail(e.error);
    } on FileSystemException catch (e) {
      await _discard();
      if (!_cancelled) {
        _fail(isNoSpace(e) ? ApkError.space : ApkError.interrupted);
      }
    } on Object {
      await _discard();
      if (!_cancelled) _fail(ApkError.interrupted);
    } finally {
      if (_cancelled) await _discard();
      source.close();
      _busy = false;
    }
  }

  Future<void> cancel() async {
    if (!_busy) return;
    _cancelled = true;
    source.close();
    await _discard();
    _set(const ApkInstallState(ApkPhase.idle));
  }

  Future<void> _discard() async {
    final file = _file;
    if (file == null) return;
    try {
      if (await file.exists()) await file.delete();
      _file = null;
    } on FileSystemException {
      return;
    }
  }

  Future<void> _run() async {
    if (offer.expiredAt(_now(), grace: apkExpiryGrace)) {
      throw const ApkFailure(ApkError.expired);
    }
    if (!await installer.canInstall()) {
      _set(const ApkInstallState(ApkPhase.needPermission));
      return;
    }
    await cleanApkDir(dir);
    await dir.create(recursive: true);
    final free = await installer.freeBytes(dir.path);
    if (free != null && free < offer.size + apkSpaceMargin) {
      throw const ApkFailure(ApkError.space);
    }
    _set(ApkInstallState(ApkPhase.downloading, total: offer.size));
    final response = await _connect();
    final length = response.length;
    if (length != null && length != offer.size) {
      throw const ApkFailure(ApkError.size);
    }
    final file = File(
      '${dir.path}${Platform.pathSeparator}$apkFilePrefix${_now().millisecondsSinceEpoch}.apk',
    );
    _file = file;
    final digest = await _save(response, file);
    if (_cancelled) return;
    _set(
      ApkInstallState(
        ApkPhase.verifying,
        received: offer.size,
        total: offer.size,
      ),
    );
    if (digest != offer.sha256) throw const ApkFailure(ApkError.sha);
    _set(
      ApkInstallState(
        ApkPhase.installing,
        received: offer.size,
        total: offer.size,
      ),
    );
    switch (await installer.install(file.path)) {
      case InstallStart.started:
        _file = null;
        _set(const ApkInstallState(ApkPhase.started));
      case InstallStart.notAllowed:
        _file = null;
        _set(const ApkInstallState(ApkPhase.needPermission));
      case InstallStart.failed:
        throw const ApkFailure(ApkError.installFailed);
    }
  }

  Future<ApkResponse> _connect() async {
    var reached = false;
    for (final url in offer.urls) {
      if (_cancelled) throw const ApkFailure(ApkError.interrupted);
      try {
        final response = await source.open(url);
        if (response.status == 200) return response;
        reached = true;
        if (expiredStatuses.contains(response.status)) {
          throw const ApkFailure(ApkError.expired);
        }
      } on ApkFailure {
        rethrow;
      } on Object {
        continue;
      }
    }
    throw ApkFailure(reached ? ApkError.interrupted : ApkError.lan);
  }

  Future<String> _save(ApkResponse response, File file) async {
    final sink = file.openWrite();
    final hash = const DartSha256().newHashSink();
    var got = 0;
    var last = DateTime.fromMillisecondsSinceEpoch(0);
    try {
      await for (final chunk in response.body.timeout(idle)) {
        if (_cancelled) break;
        got += chunk.length;
        if (got > offer.size) throw const ApkFailure(ApkError.size);
        hash.add(chunk);
        sink.add(chunk);
        final at = _now();
        if (at.difference(last).inMilliseconds >= 120) {
          last = at;
          _set(
            ApkInstallState(
              ApkPhase.downloading,
              received: got,
              total: offer.size,
            ),
          );
        }
      }
      await sink.flush();
      await sink.close();
    } on ApkFailure {
      await _quiet(sink);
      rethrow;
    } on FileSystemException {
      await _quiet(sink);
      rethrow;
    } on Object {
      await _quiet(sink);
      throw const ApkFailure(ApkError.interrupted);
    }
    if (_cancelled) return '';
    if (got < offer.size) throw const ApkFailure(ApkError.interrupted);
    hash.close();
    return _hex((await hash.hash()).bytes);
  }

  Future<void> _quiet(IOSink sink) async {
    try {
      await sink.close();
    } on Object {
      return;
    }
  }

  @override
  void dispose() {
    _disposed = true;
    super.dispose();
  }
}
