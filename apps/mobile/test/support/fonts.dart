import 'dart:io';

import 'package:flutter/services.dart';

Future<void> loadFonts() async {
  final root =
      Platform.environment['FLUTTER_ROOT'] ?? 'D:/Flutter/develop/flutter';
  final dir = '$root/bin/cache/artifacts/material_fonts';
  Future<ByteData> bytes(String path) async =>
      ByteData.sublistView(await File(path).readAsBytes());
  final roboto = FontLoader('Roboto')
    ..addFont(bytes('$dir/Roboto-Regular.ttf'))
    ..addFont(bytes('$dir/Roboto-Medium.ttf'))
    ..addFont(bytes('$dir/Roboto-Bold.ttf'));
  await roboto.load();
  for (final (family, file) in [
    ('OrreryMono', 'CascadiaMono-Terminal.ttf'),
    ('OrreryTerminalSymbols', 'NotoSansSymbols-Terminal.ttf'),
    ('OrreryTerminalSymbols2', 'NotoSansSymbols2-Terminal.ttf'),
  ]) {
    final symbols = FontLoader(family)..addFont(bytes('assets/fonts/$file'));
    await symbols.load();
  }
  final icons = FontLoader('MaterialIcons')
    ..addFont(bytes('$dir/MaterialIcons-Regular.otf'));
  await icons.load();
  for (final family in ['monospace', 'Consolas']) {
    final mono = FontLoader(family)
      ..addFont(bytes('C:/Windows/Fonts/consola.ttf'))
      ..addFont(bytes('C:/Windows/Fonts/consolab.ttf'));
    await mono.load();
  }
}
