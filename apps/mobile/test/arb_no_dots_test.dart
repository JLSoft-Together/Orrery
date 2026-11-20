import 'dart:convert';
import 'dart:io';

import 'package:flutter_test/flutter_test.dart';

void main() {
  for (final name in ['app_en.arb', 'app_vi.arb']) {
    test('$name has no middle dot or em dash', () {
      final map = jsonDecode(File('lib/l10n/$name').readAsStringSync()) as Map<String, dynamic>;
      final bad = [
        for (final e in map.entries)
          if (!e.key.startsWith('@') && e.value is String && RegExp('[·—]').hasMatch(e.value as String)) e.key,
      ];
      expect(bad, isEmpty);
    });
  }
}
