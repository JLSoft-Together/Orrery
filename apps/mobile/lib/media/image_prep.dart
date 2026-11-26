import 'dart:typed_data';
import 'dart:ui' as ui;

const int maxImageSide = 2048;
const int maxImageBytes = 8 * 1024 * 1024;

enum ImageKind { jpeg, png }

enum PrepError { unsupported, tooLarge, failed }

class PrepFailure implements Exception {
  const PrepFailure(this.error);

  final PrepError error;

  @override
  String toString() => 'PrepFailure($error)';
}

class PreparedImage {
  const PreparedImage(this.bytes, this.kind);

  final Uint8List bytes;
  final ImageKind kind;
}

typedef Shrink = Future<Uint8List> Function(Uint8List bytes, int w, int h);

ImageKind? imageKind(Uint8List b) {
  if (b.length >= 3 && b[0] == 0xFF && b[1] == 0xD8 && b[2] == 0xFF) {
    return ImageKind.jpeg;
  }
  const png = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];
  if (b.length >= 8) {
    for (var i = 0; i < 8; i++) {
      if (b[i] != png[i]) return null;
    }
    return ImageKind.png;
  }
  return null;
}

({int w, int h})? imageSize(Uint8List b) {
  return switch (imageKind(b)) {
    ImageKind.png => _pngSize(b),
    ImageKind.jpeg => _jpegSize(b),
    null => null,
  };
}

int _u16(Uint8List b, int i) => (b[i] << 8) | b[i + 1];

int _u32(Uint8List b, int i) =>
    (b[i] << 24) | (b[i + 1] << 16) | (b[i + 2] << 8) | b[i + 3];

({int w, int h})? _pngSize(Uint8List b) {
  if (b.length < 24) return null;
  final w = _u32(b, 16);
  final h = _u32(b, 20);
  return w > 0 && h > 0 ? (w: w, h: h) : null;
}

({int w, int h})? _jpegSize(Uint8List b) {
  var i = 2;
  while (i + 9 < b.length) {
    if (b[i] != 0xFF) return null;
    final marker = b[i + 1];
    if (marker == 0xFF) {
      i++;
      continue;
    }
    final sof =
        marker >= 0xC0 &&
        marker <= 0xCF &&
        marker != 0xC4 &&
        marker != 0xC8 &&
        marker != 0xCC;
    if (sof) {
      final h = _u16(b, i + 5);
      final w = _u16(b, i + 7);
      return w > 0 && h > 0 ? (w: w, h: h) : null;
    }
    if (marker == 0xD8 || marker == 0x01 || (marker >= 0xD0 && marker <= 0xD7)) {
      i += 2;
      continue;
    }
    i += 2 + _u16(b, i + 2);
  }
  return null;
}

bool exceedsSide(int w, int h) => w > maxImageSide || h > maxImageSide;

Future<Uint8List> shrinkToPng(Uint8List bytes, int w, int h) async {
  final wide = w >= h;
  final codec = await ui.instantiateImageCodec(
    bytes,
    targetWidth: wide ? maxImageSide : null,
    targetHeight: wide ? null : maxImageSide,
  );
  try {
    final frame = await codec.getNextFrame();
    final data = await frame.image.toByteData(format: ui.ImageByteFormat.png);
    frame.image.dispose();
    if (data == null) throw const PrepFailure(PrepError.failed);
    return data.buffer.asUint8List(data.offsetInBytes, data.lengthInBytes);
  } finally {
    codec.dispose();
  }
}

Future<PreparedImage> prepareImage(
  Uint8List raw, {
  Shrink shrink = shrinkToPng,
}) async {
  final kind = imageKind(raw);
  final size = imageSize(raw);
  if (kind == null || size == null) {
    throw const PrepFailure(PrepError.unsupported);
  }
  var bytes = raw;
  var out = kind;
  if (exceedsSide(size.w, size.h)) {
    try {
      bytes = await shrink(raw, size.w, size.h);
    } on PrepFailure {
      rethrow;
    } on Object {
      throw const PrepFailure(PrepError.failed);
    }
    out = ImageKind.png;
  }
  if (bytes.length > maxImageBytes) {
    throw const PrepFailure(PrepError.tooLarge);
  }
  return PreparedImage(bytes, out);
}
