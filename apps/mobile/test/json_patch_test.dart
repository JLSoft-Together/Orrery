import 'package:flutter_test/flutter_test.dart';
import 'package:orrery_remote/remote/json_patch.dart';

void main() {
  Map<String, dynamic> doc() => {
    'rev': 1,
    'tabs': [
      {
        'key': 'a',
        'name': 'one',
        'panes': [
          {'key': 'p1'},
        ],
      },
      {'key': 'b', 'panes': <dynamic>[]},
    ],
    'a/b': 1,
    'x~y': 2,
  };

  test('replace add and remove on objects and arrays', () {
    final result = applyJsonPatch(doc(), [
      {'op': 'replace', 'path': '/tabs/0/name', 'value': 'two'},
      {
        'op': 'add',
        'path': '/tabs/0/panes/-',
        'value': {'key': 'p2'},
      },
      {
        'op': 'add',
        'path': '/tabs/1/panes/0',
        'value': {'key': 'p3'},
      },
      {'op': 'remove', 'path': '/tabs/0/panes/0'},
      {'op': 'add', 'path': '/closedCount', 'value': 3},
    ]) as Map<String, dynamic>;
    final tabs = result['tabs'] as List;
    expect((tabs[0] as Map)['name'], 'two');
    expect(((tabs[0] as Map)['panes'] as List).map((p) => (p as Map)['key']), [
      'p2',
    ]);
    expect(((tabs[1] as Map)['panes'] as List).single['key'], 'p3');
    expect(result['closedCount'], 3);
  });

  test('pointer escapes ~1 and ~0 are honoured', () {
    final result = applyJsonPatch(doc(), [
      {'op': 'replace', 'path': '/a~1b', 'value': 10},
      {'op': 'replace', 'path': '/x~0y', 'value': 20},
    ]) as Map<String, dynamic>;
    expect(result['a/b'], 10);
    expect(result['x~y'], 20);
  });

  test('the input document is never mutated', () {
    final original = doc();
    applyJsonPatch(original, [
      {'op': 'remove', 'path': '/tabs/0'},
    ]);
    expect((original['tabs'] as List).length, 2);
  });

  test('invalid operations throw PatchException', () {
    for (final op in <Map<String, dynamic>>[
      {'op': 'remove', 'path': '/missing'},
      {'op': 'replace', 'path': '/nope', 'value': 1},
      {'op': 'remove', 'path': '/tabs/9'},
      {'op': 'add', 'path': '/tabs/9', 'value': 1},
      {'op': 'add', 'path': '/tabs/x', 'value': 1},
      {'op': 'move', 'path': '/rev', 'from': '/rev'},
      {'op': 'add', 'path': 'no-slash', 'value': 1},
      {'op': 'replace', 'path': '/rev/deeper/er', 'value': 1},
    ]) {
      expect(
        () => applyJsonPatch(doc(), [op]),
        throwsA(isA<PatchException>()),
        reason: '$op',
      );
    }
  });
}
