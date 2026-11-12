import 'package:flutter/services.dart';

class PrivacyScreen {
  const PrivacyScreen([MethodChannel? channel])
    : _channel = channel ?? const MethodChannel(privacyChannelName);

  final MethodChannel _channel;

  Future<void> setSecure(bool secure) async {
    try {
      await _channel.invokeMethod<void>('setSecure', {'secure': secure});
    } on Object {
      return;
    }
  }

  Future<bool?> isDeviceSecure() async {
    try {
      return await _channel.invokeMethod<bool>('isDeviceSecure');
    } on Object {
      return null;
    }
  }

  Future<int?> elapsedRealtime() async {
    try {
      return await _channel.invokeMethod<int>('elapsedRealtime');
    } on Object {
      return null;
    }
  }

  Future<bool> openSecuritySettings() async {
    try {
      return await _channel.invokeMethod<bool>('openSecuritySettings') ?? false;
    } on Object {
      return false;
    }
  }
}

const String privacyChannelName = 'dev.ccm.orrery.remote/privacy';
