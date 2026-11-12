import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/device_name.dart';

void main() {
  test('android model gets the manufacturer in front', () {
    expect(
      friendlyDeviceName(
        fallback: 'Android phone',
        manufacturer: 'samsung',
        model: 'SM-A217F',
      ),
      'Samsung SM-A217F',
    );
  });

  test('model that already starts with the manufacturer is kept', () {
    expect(
      friendlyDeviceName(
        fallback: 'x',
        manufacturer: 'Google',
        model: 'Google Pixel 8',
      ),
      'Google Pixel 8',
    );
  });

  test('ios machine id maps to a friendly name', () {
    expect(
      friendlyDeviceName(
        fallback: 'iPhone',
        manufacturer: 'Apple',
        model: 'iPhone14,5',
      ),
      'iPhone 13',
    );
  });

  test('name is cut to 40 characters and empty falls back', () {
    final long = friendlyDeviceName(
      fallback: 'x',
      manufacturer: 'Acme',
      model: 'M' * 80,
    );
    expect(long.runes.length, lessThanOrEqualTo(maxDeviceName));
    expect(
      friendlyDeviceName(fallback: 'Android phone', model: ' '),
      'Android phone',
    );
  });
}
