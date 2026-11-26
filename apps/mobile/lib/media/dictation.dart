import 'dart:async';

import 'package:device_info_plus/device_info_plus.dart';
import 'package:speech_to_text/speech_to_text.dart';

import 'media_prefs.dart';

const Duration speechLocaleProbe = Duration(seconds: 2);
const Duration speechMax = Duration(seconds: 60);
const Duration speechPause = Duration(seconds: 6);
const int onDeviceSdk = 33;

abstract class SpeechEngine {
  Future<bool> init({
    required void Function(String error) onError,
    required void Function(String status) onStatus,
  });

  Future<bool> hasPermission();

  Future<bool> offlineFor(String localeId);

  Future<void> listen({
    required String localeId,
    required bool onDevice,
    required void Function(String words, bool done) onResult,
  });

  Future<void> stop();

  Future<void> cancel();
}

enum DictationFail { denied, unavailable, onlineRefused, failed }

String speechLanguage(String localeId) =>
    localeId.split(RegExp(r'[-_]')).first.toLowerCase();

class Dictation {
  Dictation(this.engine, this.prefs);

  final SpeechEngine engine;
  final MediaPrefs prefs;
  bool _listening = false;
  bool _starting = false;
  bool _cancelRequested = false;
  void Function()? _onEnd;

  bool get listening => _listening;

  Future<DictationFail?> start({
    required String localeId,
    required Future<bool> Function() askOnline,
    required void Function(String words, bool done) onText,
    void Function()? onEnd,
  }) async {
    if (_listening || _starting) return null;
    _starting = true;
    _cancelRequested = false;
    try {
      final ok = await engine.init(
        onError: (_) => _finish(),
        onStatus: (s) {
          if (s == 'done' || s == 'notListening') _finish();
        },
      );
      if (_cancelRequested) return null;
      if (!ok) {
        return await engine.hasPermission()
            ? DictationFail.unavailable
            : DictationFail.denied;
      }
      final offline = await engine.offlineFor(localeId);
      if (_cancelRequested) return null;
      if (!offline && !await prefs.onlineSpeechAllowed()) {
        if (_cancelRequested) return null;
        if (!await askOnline()) return DictationFail.onlineRefused;
        if (_cancelRequested) return null;
        await prefs.allowOnlineSpeech();
      }
      if (_cancelRequested) return null;
      _onEnd = onEnd;
      _listening = true;
      await engine.listen(
        localeId: localeId,
        onDevice: offline,
        onResult: onText,
      );
      return null;
    } on Object {
      _listening = false;
      _onEnd = null;
      return DictationFail.failed;
    } finally {
      _starting = false;
    }
  }

  Future<void> stop() async {
    if (!_listening) return;
    try {
      await engine.stop();
    } on Object {
      await _cancelQuietly();
    }
    _finish();
  }

  Future<void> cancel() async {
    if (!_listening && !_starting) return;
    if (_starting && !_listening) {
      _cancelRequested = true;
      return;
    }
    await _cancelQuietly();
    _finish();
  }

  Future<void> _cancelQuietly() async {
    try {
      await engine.cancel();
    } on Object {
      return;
    }
  }

  void _finish() {
    if (!_listening) return;
    _listening = false;
    final end = _onEnd;
    _onEnd = null;
    end?.call();
  }
}

class PlatformSpeechEngine implements SpeechEngine {
  PlatformSpeechEngine({SpeechToText? speech, Future<int> Function()? sdk})
    : _speech = speech ?? SpeechToText(),
      _sdk = sdk ?? _androidSdk;

  final SpeechToText _speech;
  final Future<int> Function() _sdk;

  static Future<int> _androidSdk() async =>
      (await DeviceInfoPlugin().androidInfo).version.sdkInt;

  @override
  Future<bool> init({
    required void Function(String error) onError,
    required void Function(String status) onStatus,
  }) => _speech.initialize(
    onError: (e) => onError(e.errorMsg),
    onStatus: onStatus,
  );

  @override
  Future<bool> hasPermission() => _speech.hasPermission;

  @override
  Future<bool> offlineFor(String localeId) async {
    try {
      if (await _sdk() < onDeviceSdk) return false;
      final locales = await _speech.locales().timeout(
        speechLocaleProbe,
        onTimeout: () => const <LocaleName>[],
      );
      final lang = speechLanguage(localeId);
      return locales.any((l) => speechLanguage(l.localeId) == lang);
    } on Object {
      return false;
    }
  }

  @override
  Future<void> listen({
    required String localeId,
    required bool onDevice,
    required void Function(String words, bool done) onResult,
  }) => _speech.listen(
    onResult: (r) => onResult(r.recognizedWords, r.finalResult),
    listenFor: speechMax,
    pauseFor: speechPause,
    localeId: localeId,
    listenOptions: SpeechListenOptions(
      onDevice: onDevice,
      partialResults: true,
      cancelOnError: true,
      listenMode: ListenMode.dictation,
    ),
  );

  @override
  Future<void> stop() => _speech.stop();

  @override
  Future<void> cancel() => _speech.cancel();
}

String speechLocaleFor(String languageCode) => switch (languageCode) {
  'vi' => 'vi_VN',
  'en' => 'en_US',
  final other => other,
};

String joinDictated(String base, String words) {
  final spoken = words.trim();
  if (spoken.isEmpty) return base;
  if (base.isEmpty) return spoken;
  final gap = base.endsWith(' ') || base.endsWith('\n') ? '' : ' ';
  return '$base$gap$spoken';
}
