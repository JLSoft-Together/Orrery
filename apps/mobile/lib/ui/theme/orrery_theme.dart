import 'package:flutter/material.dart';

class RemoteTheme {
  const RemoteTheme(this.mode, this.palette);

  static const RemoteTheme fallback = RemoteTheme(ThemeMode.system, {});

  final ThemeMode mode;
  final Map<String, String> palette;

  static RemoteTheme fromHello(Object? raw) {
    if (raw is! Map) return fallback;
    final mode = switch (raw['mode']) {
      'dark' => ThemeMode.dark,
      'light' => ThemeMode.light,
      _ => ThemeMode.system,
    };
    final colors = <String, String>{};
    final palette = raw['palette'];
    if (palette is Map) {
      for (final entry in palette.entries) {
        final value = entry.value;
        if (entry.key is String && value is String) {
          colors[(entry.key as String).replaceFirst(
                RegExp(r'^-+(ccm-)?'),
                '',
              )] =
              value;
        }
      }
    }
    return RemoteTheme(mode, colors);
  }

  Color? pick(List<String> names) {
    for (final name in names) {
      final color = parseColor(palette[name]);
      if (color != null) return color;
    }
    return null;
  }

  @override
  bool operator ==(Object other) =>
      other is RemoteTheme &&
      other.mode == mode &&
      other.palette.length == palette.length &&
      other.palette.entries.every((e) => palette[e.key] == e.value);

  @override
  int get hashCode => Object.hash(mode, palette.length);
}

Color? parseColor(String? text) {
  if (text == null) return null;
  var hex = text.trim();
  if (!hex.startsWith('#')) return null;
  hex = hex.substring(1);
  if (hex.length == 3) hex = hex.split('').map((c) => '$c$c').join();
  if (hex.length == 6) hex = 'ff$hex';
  if (hex.length != 8) return null;
  final value = int.tryParse(hex, radix: 16);
  return value == null ? null : Color(value);
}

@immutable
class StatusColors extends ThemeExtension<StatusColors> {
  const StatusColors({
    required this.working,
    required this.asked,
    required this.failed,
    required this.done,
    required this.idle,
    required this.warning,
  });

  final Color working;
  final Color asked;
  final Color failed;
  final Color done;
  final Color idle;
  final Color warning;

  static const StatusColors light = StatusColors(
    working: Color(0xFF0060DF),
    asked: Color(0xFFB25000),
    failed: Color(0xFFD70015),
    done: Color(0xFF248A3D),
    idle: Color(0xFF6C6C70),
    warning: Color(0xFFB25000),
  );

  static const StatusColors dark = StatusColors(
    working: Color(0xFF64A8FF),
    asked: Color(0xFFFFB340),
    failed: Color(0xFFFF6961),
    done: Color(0xFF32D74B),
    idle: Color(0xFF9A9AA0),
    warning: Color(0xFFFFB340),
  );

  @override
  StatusColors copyWith({
    Color? working,
    Color? asked,
    Color? failed,
    Color? done,
    Color? idle,
    Color? warning,
  }) => StatusColors(
    working: working ?? this.working,
    asked: asked ?? this.asked,
    failed: failed ?? this.failed,
    done: done ?? this.done,
    idle: idle ?? this.idle,
    warning: warning ?? this.warning,
  );

  @override
  StatusColors lerp(StatusColors? other, double t) {
    if (other == null) return this;
    return StatusColors(
      working: Color.lerp(working, other.working, t)!,
      asked: Color.lerp(asked, other.asked, t)!,
      failed: Color.lerp(failed, other.failed, t)!,
      done: Color.lerp(done, other.done, t)!,
      idle: Color.lerp(idle, other.idle, t)!,
      warning: Color.lerp(warning, other.warning, t)!,
    );
  }
}

extension StatusColorsX on BuildContext {
  StatusColors get statusColors =>
      Theme.of(this).extension<StatusColors>() ?? StatusColors.light;
}

ThemeData buildOrreryTheme(
  RemoteTheme remote,
  Brightness brightness, {
  String? fontFamily,
}) {
  final dark = brightness == Brightness.dark;
  final accent =
      remote.pick(const ['accent', 'accent-fill', 'primary']) ??
      (dark ? const Color(0xFF0A84FF) : const Color(0xFF0060DF));
  final background =
      remote.pick(const ['bg', 'background', 'surface-0']) ??
      (dark ? const Color(0xFF000000) : const Color(0xFFF2F2F7));
  final surface =
      remote.pick(const ['surface', 'card', 'surface-1']) ??
      (dark ? const Color(0xFF1C1C1E) : const Color(0xFFFFFFFF));
  final ink =
      remote.pick(const ['ink', 'text', 'fg', 'foreground']) ??
      (dark ? const Color(0xFFF2F2F7) : const Color(0xFF1C1C1E));
  final base = ColorScheme.fromSeed(seedColor: accent, brightness: brightness);
  final scheme = base.copyWith(
    primary: accent,
    onPrimary: accent.computeLuminance() > 0.5
        ? const Color(0xFF111111)
        : Colors.white,
    surface: surface,
    onSurface: ink,
    onSurfaceVariant: ink.withValues(alpha: 0.72),
    surfaceContainerLowest: background,
    surfaceContainerLow: surface,
    surfaceContainer: surface,
    surfaceContainerHigh: dark
        ? const Color(0xFF2C2C2E)
        : const Color(0xFFEAEAF0),
    surfaceContainerHighest: dark
        ? const Color(0xFF3A3A3C)
        : const Color(0xFFE0E0E8),
    outlineVariant: dark ? const Color(0xFF3A3A3C) : const Color(0xFFD8D8DE),
    error: dark ? const Color(0xFFFF6961) : const Color(0xFFD70015),
  );
  final radius = BorderRadius.circular(14);
  return ThemeData(
    useMaterial3: true,
    fontFamily: fontFamily,
    colorScheme: scheme,
    scaffoldBackgroundColor: background,
    visualDensity: VisualDensity.standard,
    extensions: [dark ? StatusColors.dark : StatusColors.light],
    appBarTheme: AppBarTheme(
      backgroundColor: background,
      foregroundColor: ink,
      elevation: 0,
      scrolledUnderElevation: 0,
      centerTitle: false,
      titleTextStyle: TextStyle(
        fontFamily: fontFamily,
        color: ink,
        fontSize: 20,
        fontWeight: FontWeight.w600,
      ),
    ),
    cardTheme: CardThemeData(
      color: surface,
      elevation: 0,
      margin: EdgeInsets.zero,
      shape: RoundedRectangleBorder(
        borderRadius: radius,
        side: BorderSide(color: scheme.outlineVariant),
      ),
    ),
    dividerTheme: DividerThemeData(color: scheme.outlineVariant, space: 1),
    bottomSheetTheme: BottomSheetThemeData(
      backgroundColor: surface,
      showDragHandle: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(20)),
      ),
    ),
    snackBarTheme: SnackBarThemeData(
      behavior: SnackBarBehavior.floating,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: scheme.surfaceContainerHigh,
      border: OutlineInputBorder(
        borderRadius: BorderRadius.circular(12),
        borderSide: BorderSide.none,
      ),
      contentPadding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        minimumSize: const Size(48, 48),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        minimumSize: const Size(48, 48),
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(minimumSize: const Size(48, 48)),
    ),
  );
}
