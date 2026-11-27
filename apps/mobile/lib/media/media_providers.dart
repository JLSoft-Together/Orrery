import 'package:flutter_riverpod/flutter_riverpod.dart';

import 'dictation.dart';
import 'media_prefs.dart';

const String putCap = 'fsput';

final dictationProvider = Provider<Dictation?>((ref) {
  final dictation = Dictation(
    PlatformSpeechEngine(),
    ref.read(mediaPrefsProvider),
  );
  ref.onDispose(dictation.cancel);
  return dictation;
});
