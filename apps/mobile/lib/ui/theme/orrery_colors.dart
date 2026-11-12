import 'package:flutter/material.dart';

Color _l(Color a, Color b, double t) => Color.lerp(a, b, t)!;

@immutable
class OrreryColors extends ThemeExtension<OrreryColors> {
  const OrreryColors({
    required this.canvas,
    required this.surface,
    required this.raised,
    required this.control,
    required this.line,
    required this.lineStrong,
    required this.ink,
    required this.inkMuted,
    required this.inkFaint,
    required this.accent,
    required this.onAccent,
    required this.accentInk,
    required this.selection,
    required this.error,
    required this.working,
    required this.waiting,
    required this.idle,
    required this.done,
    required this.stale,
    required this.info,
    required this.diffAdd,
    required this.diffDel,
    required this.diffMod,
    required this.agentClaude,
    required this.agentCodex,
    required this.agentShell,
    required this.well,
    required this.wellInk,
    required this.scrim,
  });

  final Color canvas;
  final Color surface;
  final Color raised;
  final Color control;
  final Color line;
  final Color lineStrong;
  final Color ink;
  final Color inkMuted;
  final Color inkFaint;
  final Color accent;
  final Color onAccent;
  final Color accentInk;
  final Color selection;
  final Color error;
  final Color working;
  final Color waiting;
  final Color idle;
  final Color done;
  final Color stale;
  final Color info;
  final Color diffAdd;
  final Color diffDel;
  final Color diffMod;
  final Color agentClaude;
  final Color agentCodex;
  final Color agentShell;
  final Color well;
  final Color wellInk;
  final Color scrim;

  static const OrreryColors light = OrreryColors(
    canvas: Color(0xFFF3F3F6),
    surface: Color(0xFFFFFFFF),
    raised: Color(0xFFECECF1),
    control: Color(0xFFFFFFFF),
    line: Color(0x1717171C),
    lineStrong: Color(0x2E17171C),
    ink: Color(0xFF17171C),
    inkMuted: Color(0xFF55555F),
    inkFaint: Color(0xFF61616C),
    accent: Color(0xFFBD5128),
    onAccent: Color(0xFFFFFFFF),
    accentInk: Color(0xFFB04A24),
    selection: Color(0xFFF6DCCF),
    error: Color(0xFFC4001A),
    working: Color(0xFFB02F00),
    waiting: Color(0xFF7F5500),
    idle: Color(0xFF61616C),
    done: Color(0xFF1B6F30),
    stale: Color(0xFF64646E),
    info: Color(0xFF0B5CAD),
    diffAdd: Color(0xFF1B6F30),
    diffDel: Color(0xFFC4001A),
    diffMod: Color(0xFF0B5CAD),
    agentClaude: Color(0xFFC15F3C),
    agentCodex: Color(0xFF6A4FC9),
    agentShell: Color(0xFF0F7F78),
    well: Color(0xFF18181C),
    wellInk: Color(0xFFE5E5EA),
    scrim: Color(0x73000000),
  );

  static const OrreryColors dark = OrreryColors(
    canvas: Color(0xFF000000),
    surface: Color(0xFF08080A),
    raised: Color(0xFF0D0D10),
    control: Color(0xFF16161A),
    line: Color(0xFF141417),
    lineStrong: Color(0xFF1C1C20),
    ink: Color(0xFFECECF1),
    inkMuted: Color(0xFFA6A6B0),
    inkFaint: Color(0xFF85858F),
    accent: Color(0xFFE8835D),
    onAccent: Color(0xFF1A110D),
    accentInk: Color(0xFFF0946F),
    selection: Color(0xFF35261F),
    error: Color(0xFFFF6B6B),
    working: Color(0xFFF5A524),
    waiting: Color(0xFFF7D046),
    idle: Color(0xFF8E8E9A),
    done: Color(0xFF4CC38A),
    stale: Color(0xFF8E8E9A),
    info: Color(0xFF6EA8FE),
    diffAdd: Color(0xFF4CC38A),
    diffDel: Color(0xFFFF6B6B),
    diffMod: Color(0xFF6EA8FE),
    agentClaude: Color(0xFFD97757),
    agentCodex: Color(0xFF9D8CF8),
    agentShell: Color(0xFF4CC2B8),
    well: Color(0xFF030304),
    wellInk: Color(0xFFE5E5EA),
    scrim: Color(0x99000000),
  );

  @override
  OrreryColors copyWith({Color? accent, Color? error}) => OrreryColors(
    canvas: canvas,
    surface: surface,
    raised: raised,
    control: control,
    line: line,
    lineStrong: lineStrong,
    ink: ink,
    inkMuted: inkMuted,
    inkFaint: inkFaint,
    accent: accent ?? this.accent,
    onAccent: onAccent,
    accentInk: accentInk,
    selection: selection,
    error: error ?? this.error,
    working: working,
    waiting: waiting,
    idle: idle,
    done: done,
    stale: stale,
    info: info,
    diffAdd: diffAdd,
    diffDel: diffDel,
    diffMod: diffMod,
    agentClaude: agentClaude,
    agentCodex: agentCodex,
    agentShell: agentShell,
    well: well,
    wellInk: wellInk,
    scrim: scrim,
  );

  @override
  OrreryColors lerp(OrreryColors? other, double t) {
    if (other == null) return this;
    return OrreryColors(
      canvas: _l(canvas, other.canvas, t),
      surface: _l(surface, other.surface, t),
      raised: _l(raised, other.raised, t),
      control: _l(control, other.control, t),
      line: _l(line, other.line, t),
      lineStrong: _l(lineStrong, other.lineStrong, t),
      ink: _l(ink, other.ink, t),
      inkMuted: _l(inkMuted, other.inkMuted, t),
      inkFaint: _l(inkFaint, other.inkFaint, t),
      accent: _l(accent, other.accent, t),
      onAccent: _l(onAccent, other.onAccent, t),
      accentInk: _l(accentInk, other.accentInk, t),
      selection: _l(selection, other.selection, t),
      error: _l(error, other.error, t),
      working: _l(working, other.working, t),
      waiting: _l(waiting, other.waiting, t),
      idle: _l(idle, other.idle, t),
      done: _l(done, other.done, t),
      stale: _l(stale, other.stale, t),
      info: _l(info, other.info, t),
      diffAdd: _l(diffAdd, other.diffAdd, t),
      diffDel: _l(diffDel, other.diffDel, t),
      diffMod: _l(diffMod, other.diffMod, t),
      agentClaude: _l(agentClaude, other.agentClaude, t),
      agentCodex: _l(agentCodex, other.agentCodex, t),
      agentShell: _l(agentShell, other.agentShell, t),
      well: _l(well, other.well, t),
      wellInk: _l(wellInk, other.wellInk, t),
      scrim: _l(scrim, other.scrim, t),
    );
  }
}
