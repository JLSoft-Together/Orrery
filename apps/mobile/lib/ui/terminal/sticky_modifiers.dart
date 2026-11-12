import 'package:flutter/foundation.dart';

enum ModState { off, once, locked }

class StickyModifiers extends ChangeNotifier {
  ModState _ctrl = ModState.off;
  ModState _alt = ModState.off;

  ModState get ctrl => _ctrl;
  ModState get alt => _alt;
  bool get any => _ctrl != ModState.off || _alt != ModState.off;

  static ModState _next(ModState s) => switch (s) {
    ModState.off => ModState.once,
    ModState.once => ModState.locked,
    ModState.locked => ModState.off,
  };

  void toggleCtrl() {
    _ctrl = _next(_ctrl);
    notifyListeners();
  }

  void toggleAlt() {
    _alt = _next(_alt);
    notifyListeners();
  }

  void reset() {
    if (!any) return;
    _ctrl = ModState.off;
    _alt = ModState.off;
    notifyListeners();
  }

  (bool ctrl, bool alt) take() {
    final result = (_ctrl != ModState.off, _alt != ModState.off);
    var changed = false;
    if (_ctrl == ModState.once) {
      _ctrl = ModState.off;
      changed = true;
    }
    if (_alt == ModState.once) {
      _alt = ModState.off;
      changed = true;
    }
    if (changed) notifyListeners();
    return result;
  }

  String apply(String data) {
    if (!any || data.runes.length != 1 || data.runes.first < 0x20) {
      return data;
    }
    final (ctrl, alt) = take();
    var out = ctrl ? controlChar(data) : data;
    if (alt) out = '\x1b$out';
    return out;
  }
}

String controlChar(String ch) {
  if (ch.length != 1) return ch;
  final code = ch.codeUnitAt(0);
  if (code == 0x20 || code == 0x40 || code == 0x32) return '\x00';
  if (code == 0x3f) return '\x7f';
  final upper = code >= 0x61 && code <= 0x7a ? code - 0x20 : code;
  if (upper >= 0x40 && upper <= 0x5f) {
    return String.fromCharCode(upper - 0x40);
  }
  return ch;
}
