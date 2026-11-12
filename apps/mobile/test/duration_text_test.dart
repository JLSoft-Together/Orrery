import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/l10n/app_localizations.dart';
import 'package:orrery_remote/ui/widgets/common.dart';

void main() {
  final en = lookupAppLocalizations(const Locale('en'));
  final vi = lookupAppLocalizations(const Locale('vi'));

  test('shortElapsed uses the unit of each language', () {
    expect(shortElapsed(en, const Duration(seconds: 30)), '<1m');
    expect(shortElapsed(en, const Duration(minutes: 5)), '5m');
    expect(shortElapsed(en, const Duration(hours: 3)), '3h');
    expect(shortElapsed(en, const Duration(hours: 50)), '2d');
    expect(shortElapsed(vi, const Duration(seconds: 30)), '<1p');
    expect(shortElapsed(vi, const Duration(minutes: 5)), '5p');
    expect(shortElapsed(vi, const Duration(hours: 3)), '3g');
    expect(shortElapsed(vi, const Duration(hours: 50)), '2n');
  });

  test('elapsedText joins two units', () {
    expect(elapsedText(en, const Duration(minutes: 2, seconds: 14)), '2m 14s');
    expect(elapsedText(vi, const Duration(hours: 2, minutes: 14)), '2g 14p');
    expect(elapsedText(vi, const Duration(seconds: 9)), '9s');
  });
}
