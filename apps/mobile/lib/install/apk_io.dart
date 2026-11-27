import 'dart:async';
import 'dart:io';

import 'package:flutter/services.dart';

import 'apk_offer.dart';

const String installChannelName = 'dev.ccm.orrery.remote/install';
const Duration apkConnectTimeout = Duration(seconds: 5);

class ApkResponse {
  const ApkResponse({required this.status, required this.body, this.length});

  final int status;
  final int? length;
  final Stream<List<int>> body;
}

abstract class ApkSource {
  Future<ApkResponse> open(Uri url);

  void close();
}

class HttpApkSource implements ApkSource {
  HttpClient? _client;

  @override
  Future<ApkResponse> open(Uri url) async {
    final client = _client ??= HttpClient()
      ..connectionTimeout = apkConnectTimeout
      ..autoUncompress = false
      ..findProxy = ((_) => 'DIRECT');
    final request = await client.getUrl(url).timeout(apkConnectTimeout)
      ..followRedirects = false;
    final response = await request.close().timeout(apkConnectTimeout);
    final length = response.contentLength;
    return ApkResponse(
      status: response.statusCode,
      length: length < 0 ? null : length,
      body: response,
    );
  }

  @override
  void close() {
    _client?.close(force: true);
    _client = null;
  }
}

enum InstallStart { started, notAllowed, failed }

abstract class ApkInstaller {
  Future<bool> canInstall();

  Future<void> openSettings();

  Future<int?> freeBytes(String path);

  Future<InstallStart> install(String path);
}

class ChannelApkInstaller implements ApkInstaller {
  const ChannelApkInstaller([
    this.channel = const MethodChannel(installChannelName),
  ]);

  final MethodChannel channel;

  @override
  Future<bool> canInstall() async {
    try {
      return await channel.invokeMethod<bool>('canInstall') ?? false;
    } on PlatformException {
      return false;
    } on MissingPluginException {
      return false;
    }
  }

  @override
  Future<void> openSettings() async {
    try {
      await channel.invokeMethod<bool>('openSettings');
    } on PlatformException {
      return;
    } on MissingPluginException {
      return;
    }
  }

  @override
  Future<int?> freeBytes(String path) async {
    try {
      return await channel.invokeMethod<int>('freeBytes', {'path': path});
    } on PlatformException {
      return null;
    } on MissingPluginException {
      return null;
    }
  }

  @override
  Future<InstallStart> install(String path) async {
    try {
      final result = await channel.invokeMethod<String>('install', {
        'path': path,
      });
      return switch (result) {
        'started' => InstallStart.started,
        'notAllowed' => InstallStart.notAllowed,
        _ => InstallStart.failed,
      };
    } on PlatformException {
      return InstallStart.failed;
    } on MissingPluginException {
      return InstallStart.failed;
    }
  }
}

bool isNoSpace(FileSystemException error) {
  final code = error.osError?.errorCode;
  return code == 28 || error.message.toLowerCase().contains('no space');
}

const Set<int> expiredStatuses = {401, 403, 404, 410};

ApkFailure failureOfStatus(int status) => ApkFailure(
  expiredStatuses.contains(status) ? ApkError.expired : ApkError.interrupted,
);
