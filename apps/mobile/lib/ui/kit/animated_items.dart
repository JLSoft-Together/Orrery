import 'package:flutter/material.dart';

import '../theme/orrery_shape.dart';
import '../widgets/common.dart';

class AnimatedItems<T> extends StatefulWidget {
  const AnimatedItems({
    super.key,
    required this.items,
    required this.keyOf,
    required this.builder,
  });

  final List<T> items;
  final Object Function(T item) keyOf;
  final Widget Function(BuildContext context, T item) builder;

  @override
  State<AnimatedItems<T>> createState() => _AnimatedItemsState<T>();
}

class _Slot<T> {
  _Slot(this.key, this.item, {this.entering = true});

  final Object key;
  T item;
  bool entering;
  bool leaving = false;
}

class _AnimatedItemsState<T> extends State<AnimatedItems<T>> {
  late List<_Slot<T>> _slots = [
    for (final item in widget.items)
      _Slot<T>(widget.keyOf(item), item, entering: false),
  ];

  @override
  void didUpdateWidget(AnimatedItems<T> old) {
    super.didUpdateWidget(old);
    final next = <Object, T>{
      for (final item in widget.items) widget.keyOf(item): item,
    };
    final merged = <_Slot<T>>[];
    final seen = <Object>{};
    final existing = {for (final s in _slots) s.key: s};
    var cursor = 0;
    for (final item in widget.items) {
      final key = widget.keyOf(item);
      seen.add(key);
      while (cursor < _slots.length &&
          !next.containsKey(_slots[cursor].key) &&
          !seen.contains(_slots[cursor].key)) {
        final gone = _slots[cursor++];
        if (!merged.contains(gone)) merged.add(gone..leaving = true);
      }
      final slot = existing[key];
      if (slot != null) {
        slot.item = item;
        slot.leaving = false;
        merged.add(slot);
        if (_slots.indexOf(slot) >= cursor) cursor = _slots.indexOf(slot) + 1;
      } else {
        merged.add(_Slot<T>(key, item));
      }
    }
    for (final s in _slots) {
      if (!next.containsKey(s.key) && !merged.contains(s)) {
        merged.add(s..leaving = true);
      }
    }
    _slots = merged;
  }

  void _drop(_Slot<T> slot) {
    if (!mounted) return;
    setState(() => _slots.remove(slot));
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        for (final slot in _slots)
          _ItemTransition(
            key: ValueKey(slot.key),
            leaving: slot.leaving,
            animateIn: slot.entering,
            onGone: () => _drop(slot),
            child: widget.builder(context, slot.item),
          ),
      ],
    );
  }
}

class _ItemTransition extends StatefulWidget {
  const _ItemTransition({
    super.key,
    required this.leaving,
    required this.animateIn,
    required this.onGone,
    required this.child,
  });

  final bool leaving;
  final bool animateIn;
  final VoidCallback onGone;
  final Widget child;

  @override
  State<_ItemTransition> createState() => _ItemTransitionState();
}

class _ItemTransitionState extends State<_ItemTransition>
    with SingleTickerProviderStateMixin {
  late final AnimationController _controller = AnimationController(
    vsync: this,
    value: widget.animateIn ? 0 : 1,
  );
  bool _started = false;

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    if (_started) return;
    _started = true;
    _controller.duration = motion(context, 220);
    _controller.reverseDuration = motion(context, 150);
    if (widget.animateIn) {
      if (reduceMotion(context)) {
        _controller.value = 1;
      } else {
        _controller.forward();
      }
    }
    if (widget.leaving) _leave();
  }

  @override
  void didUpdateWidget(_ItemTransition old) {
    super.didUpdateWidget(old);
    if (widget.leaving && !old.leaving) _leave();
    if (!widget.leaving && old.leaving) _controller.forward();
  }

  void _leave() {
    if (reduceMotion(context)) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (mounted && widget.leaving) widget.onGone();
      });
      return;
    }
    _controller.reverse().whenComplete(() {
      if (mounted && widget.leaving) widget.onGone();
    });
  }

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final tokens = context.tokens;
    final curved = CurvedAnimation(
      parent: _controller,
      curve: tokens.out,
      reverseCurve: tokens.inCurve,
    );
    final enter = Tween<Offset>(
      begin: const Offset(0, -0.25),
      end: Offset.zero,
    ).animate(curved);
    final exit = Tween<Offset>(
      begin: const Offset(0.12, 0),
      end: Offset.zero,
    ).animate(curved);
    return SizeTransition(
      sizeFactor: curved,
      axisAlignment: -1,
      child: FadeTransition(
        opacity: curved,
        child: SlideTransition(
          position: widget.leaving ? exit : enter,
          child: Padding(
            padding: const EdgeInsets.only(bottom: 10),
            child: widget.child,
          ),
        ),
      ),
    );
  }
}

class AnimatedRename extends StatelessWidget {
  const AnimatedRename({
    super.key,
    required this.text,
    this.style,
    this.maxLines = 1,
  });

  final String text;
  final TextStyle? style;
  final int maxLines;

  @override
  Widget build(BuildContext context) {
    return AnimatedSwitcher(
      duration: motion(context, 150),
      layoutBuilder: (current, previous) => Stack(
        alignment: Alignment.centerLeft,
        children: [...previous, ?current],
      ),
      child: Text(
        text,
        key: ValueKey(text),
        maxLines: maxLines,
        overflow: TextOverflow.ellipsis,
        style: style,
      ),
    );
  }
}
