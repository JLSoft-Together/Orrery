import 'package:flutter/material.dart';

class ExtraKeysBar extends StatelessWidget {
  const ExtraKeysBar({super.key, required this.onKey});

  final void Function(String label, String sequence) onKey;

  static const keys = <(String, String)>[
    ('Esc', '\x1b'),
    ('Tab', '\t'),
    ('Ctrl-C', '\x03'),
    ('↑', '\x1b[A'),
    ('↓', '\x1b[B'),
    ('←', '\x1b[D'),
    ('→', '\x1b[C'),
    ('Enter', '\r'),
  ];

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 40,
      child: ListView(
        scrollDirection: Axis.horizontal,
        children: [
          for (final (label, seq) in keys)
            Padding(
              padding: const EdgeInsets.symmetric(horizontal: 3, vertical: 4),
              child: OutlinedButton(
                key: ValueKey('key-$label'),
                style: OutlinedButton.styleFrom(
                  minimumSize: const Size(44, 32),
                  padding: const EdgeInsets.symmetric(horizontal: 10),
                ),
                onPressed: () => onKey(label, seq),
                child: Text(label),
              ),
            ),
        ],
      ),
    );
  }
}

String visibleEscape(String sequence) {
  final b = StringBuffer();
  for (final unit in sequence.runes) {
    if (unit == 0x1b) {
      b.write('^[');
    } else if (unit < 0x20) {
      b.write('^${String.fromCharCode(unit + 0x40)}');
    } else {
      b.writeCharCode(unit);
    }
  }
  return b.toString();
}
