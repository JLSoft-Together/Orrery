import 'package:flutter/services.dart';
import 'package:image_picker/image_picker.dart';

import 'image_prep.dart';
import 'image_upload.dart';
import 'inbox_ignore.dart';
import 'media_prefs.dart';

enum PickSource { camera, gallery }

enum MediaError {
  cameraDenied,
  unsupported,
  tooLarge,
  busy,
  offline,
  denied,
  interrupted,
  failed,
}

class MediaFailure implements Exception {
  const MediaFailure(this.error);

  final MediaError error;

  @override
  String toString() => 'MediaFailure($error)';
}

typedef PickImage = Future<Uint8List?> Function(PickSource source);

Future<Uint8List?> pickWithPlugin(PickSource source) async {
  try {
    final file = await ImagePicker().pickImage(
      source: source == PickSource.camera
          ? ImageSource.camera
          : ImageSource.gallery,
      maxWidth: maxImageSide.toDouble(),
      maxHeight: maxImageSide.toDouble(),
      imageQuality: 85,
      requestFullMetadata: false,
    );
    return file == null ? null : await file.readAsBytes();
  } on PlatformException catch (error) {
    if (error.code.contains('denied')) {
      throw const MediaFailure(MediaError.cameraDenied);
    }
    throw const MediaFailure(MediaError.failed);
  }
}

class MediaFlow {
  MediaFlow({
    required this.call,
    required this.root,
    required this.prefs,
    this.pick = pickWithPlugin,
    this.shrink = shrinkToPng,
  });

  final PutCall call;
  String root;
  final MediaPrefs prefs;
  final PickImage pick;
  final Shrink shrink;
  bool _sending = false;

  bool get sending => _sending;

  Future<PutResult?> sendImage(PickSource source) async {
    if (_sending) throw const MediaFailure(MediaError.busy);
    _sending = true;
    try {
      final raw = await pick(source);
      if (raw == null) return null;
      final prepared = await prepareImage(raw, shrink: shrink);
      return await putImage(call, root, prepared.bytes);
    } on MediaFailure {
      rethrow;
    } on PrepFailure catch (error) {
      throw MediaFailure(switch (error.error) {
        PrepError.unsupported => MediaError.unsupported,
        PrepError.tooLarge => MediaError.tooLarge,
        PrepError.failed => MediaError.failed,
      });
    } on PutFailure catch (error) {
      throw MediaFailure(switch (error.error) {
        PutError.denied => MediaError.denied,
        PutError.offline => MediaError.offline,
        PutError.expired || PutError.order => MediaError.interrupted,
        PutError.busy => MediaError.busy,
        PutError.tooLarge => MediaError.tooLarge,
        PutError.image => MediaError.unsupported,
        PutError.failed => MediaError.failed,
      });
    } on Object {
      throw const MediaFailure(MediaError.failed);
    } finally {
      _sending = false;
    }
  }

  Future<bool> shouldOfferIgnore() async => !await prefs.ignoreAsked(root);

  Future<void> declineIgnore() => prefs.markIgnoreAsked(root);

  Future<IgnoreResult> acceptIgnore() async {
    await prefs.markIgnoreAsked(root);
    return addInboxIgnore(call, root);
  }
}

String insertPath(String text, String path) {
  final quoted = path.contains(RegExp(r'\s')) ? '"$path"' : path;
  if (text.isEmpty) return '$quoted ';
  final gap = text.endsWith(' ') || text.endsWith('\n') ? '' : ' ';
  return '$text$gap$quoted ';
}
