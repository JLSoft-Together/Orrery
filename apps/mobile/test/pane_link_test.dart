import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/app/pane_link.dart';

void main() {
  const machine = 'abcdefghijklmnopqrstuv';

  test('custom scheme link carries machine and pane', () {
    final link = parsePaneLink(
      Uri.parse('orrery-remote://m/$machine/p/pane-1'),
    );
    expect(link, const PaneLink(machine, 'pane-1'));
  });

  test('relay https link carries machine and pane', () {
    final link = parsePaneLink(
      Uri.parse('https://orrery-relay.jinvu.workers.dev/o/$machine/p1.a:b'),
    );
    expect(link, const PaneLink(machine, 'p1.a:b'));
  });

  test('encoded segments decode before validation', () {
    final link = parsePaneLink(
      Uri.parse('https://orrery-relay.jinvu.workers.dev/o/$machine/a%3Ab'),
    );
    expect(link, const PaneLink(machine, 'a:b'));
    expect(
      parsePaneLink(
        Uri.parse('https://orrery-relay.jinvu.workers.dev/o/$machine/a%2Fb'),
      ),
      isNull,
    );
  });

  test('foreign hosts, shapes and characters are rejected', () {
    for (final raw in [
      'https://evil.example/o/$machine/p1',
      'http://orrery-relay.jinvu.workers.dev/o/$machine/p1',
      'https://orrery-relay.jinvu.workers.dev/x/$machine/p1',
      'https://orrery-relay.jinvu.workers.dev/o/$machine',
      'https://orrery-relay.jinvu.workers.dev/o/$machine/p1/extra',
      'orrery-remote://x/$machine/p/p1',
      'orrery-remote://m/$machine/q/p1',
      'orrery-remote://m/$machine/p',
      'orrery-remote://m/$machine/p/a%20b',
      'orrery-remote://m/$machine/p/a%3Cb',
      'orrery-remote://m/${'a' * 129}/p/p1',
      'other://m/$machine/p/p1',
    ]) {
      expect(parsePaneLink(Uri.parse(raw)), isNull, reason: raw);
    }
  });
}
