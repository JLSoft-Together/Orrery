import 'dart:convert';
import 'dart:io';

import 'package:path_provider/path_provider.dart';

import 'away_summary.dart';

abstract class AwayStore {
  Future<Map<String, AwayMark>> load();

  Future<void> save(Map<String, AwayMark> marks);
}

class FileAwayStore implements AwayStore {
  FileAwayStore([this._dir]);

  final Directory? _dir;

  Future<File> _file() async {
    final dir = _dir ?? await getApplicationSupportDirectory();
    return File('${dir.path}${Platform.pathSeparator}away_marks.json');
  }

  @override
  Future<Map<String, AwayMark>> load() async {
    try {
      final file = await _file();
      if (!await file.exists()) return {};
      final raw = jsonDecode(await file.readAsString());
      if (raw is! Map) return {};
      final out = <String, AwayMark>{};
      for (final e in raw.entries) {
        final mark = AwayMark.fromJson(e.value);
        if (e.key is String && mark != null) out[e.key as String] = mark;
      }
      return out;
    } on Object {
      return {};
    }
  }

  @override
  Future<void> save(Map<String, AwayMark> marks) async {
    try {
      final file = await _file();
      final tmp = File('${file.path}.tmp');
      await tmp.writeAsString(
        jsonEncode({for (final e in marks.entries) e.key: e.value.toJson()}),
        flush: true,
      );
      await tmp.rename(file.path);
    } on Object {
      return;
    }
  }
}

class MemoryAwayStore implements AwayStore {
  MemoryAwayStore([Map<String, AwayMark>? seed]) : marks = {...?seed};

  Map<String, AwayMark> marks;

  @override
  Future<Map<String, AwayMark>> load() async => {...marks};

  @override
  Future<void> save(Map<String, AwayMark> next) async => marks = {...next};
}
