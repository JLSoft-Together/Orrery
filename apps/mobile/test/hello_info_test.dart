import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/desktop_caps.dart';
import 'package:orrery_remote/remote/hello_info.dart';
import 'package:orrery_remote/remote/store.dart';

void main() {
  test('hello text drops control characters and refuses over-long values', () {
    expect(cleanHelloText(' 4.0.10\n', maxHelloVersion), '4.0.10');
    expect(cleanHelloText('a\u0000b', 10), 'ab');
    expect(cleanHelloText('   ', 10), isNull);
    expect(cleanHelloText(null, 10), isNull);
    expect(cleanHelloText('x' * 17, maxHelloVersion), isNull);
    expect(cleanHelloText('x' * 16, maxHelloVersion), 'x' * 16);
  });

  test('os label is Android plus the release', () {
    expect(androidOsLabel('14'), 'Android 14');
    expect(androidOsLabel(''), isNull);
    expect(androidOsLabel(null), isNull);
  });

  test('toJson keeps only the fields that are known', () {
    expect(HelloInfo.empty.toJson(), isEmpty);
    expect(const HelloInfo(model: 'Pixel 8').toJson(), {'model': 'Pixel 8'});
  });

  test('desktop caps parse the welcome list and tolerate a missing field', () {
    expect(desktopCaps({'caps': ['devinfo', 'clip', 5, '']}), {
      'devinfo',
      'clip',
    });
    expect(desktopCaps(null), isEmpty);
    expect(desktopCaps({'name': 'Studio'}), isEmpty);
    expect(desktopCaps({'caps': 'devinfo'}), isEmpty);
    expect(hasDesktopCap({'caps': ['devinfo']}, 'devinfo'), isTrue);
    expect(hasDesktopCap({'caps': ['devinfo']}, 'clip'), isFalse);
  });

  test('machine view exposes hasCap from the stored welcome', () {
    expect(const MachineView().hasCap('devinfo'), isFalse);
    expect(
      const MachineView(
        hello: {
          'caps': ['devinfo'],
        },
      ).hasCap('devinfo'),
      isTrue,
    );
  });
}
