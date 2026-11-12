import 'package:flutter/material.dart';

import '../theme/orrery_shape.dart';
import '../widgets/common.dart';

class OrrerySegmented extends StatelessWidget {
  const OrrerySegmented({
    super.key,
    required this.labels,
    required this.index,
    required this.onChanged,
    this.keyPrefix = 'seg',
  });

  final List<String> labels;
  final int index;
  final ValueChanged<int> onChanged;
  final String keyPrefix;

  @override
  Widget build(BuildContext context) {
    final c = context.orrery;
    final shape = context.shape;
    final n = labels.length;
    return DecoratedBox(
      decoration: BoxDecoration(
        color: c.raised,
        borderRadius: shape.controlRadius,
        border: Border.all(color: c.line),
      ),
      child: Padding(
        padding: const EdgeInsets.all(3),
        child: SizedBox(
          height: shape.touch - 4,
          child: Stack(
            children: [
              AnimatedAlign(
                duration: motion(context, 220),
                curve: context.tokens.out,
                alignment: Alignment(n == 1 ? 0 : -1 + 2 * index / (n - 1), 0),
                child: FractionallySizedBox(
                  widthFactor: 1 / n,
                  heightFactor: 1,
                  child: DecoratedBox(
                    decoration: BoxDecoration(
                      color: c.control,
                      borderRadius: BorderRadius.circular(shape.control - 2),
                      border: Border.all(color: c.lineStrong),
                    ),
                  ),
                ),
              ),
              Row(
                children: [
                  for (var i = 0; i < n; i++)
                    Expanded(
                      child: Semantics(
                        button: true,
                        selected: i == index,
                        child: GestureDetector(
                          key: ValueKey('$keyPrefix-$i'),
                          behavior: HitTestBehavior.opaque,
                          onTap: () => onChanged(i),
                          child: Center(
                            child: AnimatedDefaultTextStyle(
                              duration: motion(context, 220),
                              style:
                                  (Theme.of(context).textTheme.bodyMedium ??
                                          const TextStyle())
                                      .copyWith(
                                        fontWeight: FontWeight.w600,
                                        color: i == index ? c.ink : c.inkMuted,
                                      ),
                              child: Text(labels[i]),
                            ),
                          ),
                        ),
                      ),
                    ),
                ],
              ),
            ],
          ),
        ),
      ),
    );
  }
}
