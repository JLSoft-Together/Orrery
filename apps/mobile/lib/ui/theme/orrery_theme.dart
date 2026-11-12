import 'package:flutter/material.dart';

import 'orrery_colors.dart';
import 'orrery_shape.dart';
import 'spring_transition.dart';

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
    working: Color(0xFFB02F00),
    asked: Color(0xFF7F5500),
    failed: Color(0xFFC4001A),
    done: Color(0xFF1B6F30),
    idle: Color(0xFF61616C),
    warning: Color(0xFF7F5500),
  );

  static const StatusColors dark = StatusColors(
    working: Color(0xFFF5A524),
    asked: Color(0xFFF7D046),
    failed: Color(0xFFFF6B6B),
    done: Color(0xFF4CC38A),
    idle: Color(0xFF8E8E9A),
    warning: Color(0xFFF7D046),
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

TextStyle _style(
  String? family,
  Color color,
  double size,
  double line, [
  FontWeight weight = FontWeight.w400,
]) => TextStyle(
  fontFamily: family,
  color: color,
  fontSize: size,
  height: line / size,
  fontWeight: weight,
);

ThemeData buildOrreryTheme(
  RemoteTheme remote,
  Brightness brightness, {
  String? fontFamily,
}) {
  final dark = brightness == Brightness.dark;
  final c = dark ? OrreryColors.dark : OrreryColors.light;
  const shape = OrreryShape();
  final scheme =
      ColorScheme.fromSeed(
        seedColor: c.accent,
        brightness: brightness,
      ).copyWith(
        primary: c.accent,
        onPrimary: c.onAccent,
        primaryContainer: c.selection,
        onPrimaryContainer: c.accentInk,
        surface: c.canvas,
        onSurface: c.ink,
        onSurfaceVariant: c.inkMuted,
        surfaceContainerLowest: c.canvas,
        surfaceContainerLow: c.surface,
        surfaceContainer: c.control,
        surfaceContainerHigh: c.raised,
        surfaceContainerHighest: c.raised,
        outline: c.lineStrong,
        outlineVariant: c.line,
        error: c.error,
        scrim: c.scrim,
      );
  final base = ThemeData(brightness: brightness, useMaterial3: true).textTheme;
  final text = base
      .apply(bodyColor: c.ink, displayColor: c.ink, fontFamily: fontFamily)
      .copyWith(
        headlineMedium: _style(fontFamily, c.ink, 28, 34, FontWeight.w600),
        titleMedium: _style(fontFamily, c.ink, 17, 24, FontWeight.w600),
        titleSmall: _style(fontFamily, c.ink, 15, 20, FontWeight.w600),
        bodyMedium: _style(fontFamily, c.ink, 14, 20),
        bodySmall: _style(fontFamily, c.inkMuted, 12, 16),
        labelMedium: _style(fontFamily, c.inkMuted, 12, 16, FontWeight.w600),
      );
  final control = RoundedRectangleBorder(borderRadius: shape.controlRadius);
  OutlineInputBorder field(Color color, [double width = 1]) =>
      OutlineInputBorder(
        borderRadius: shape.controlRadius,
        borderSide: BorderSide(color: color, width: width),
      );
  return ThemeData(
    useMaterial3: true,
    brightness: brightness,
    fontFamily: fontFamily,
    colorScheme: scheme,
    textTheme: text,
    scaffoldBackgroundColor: c.canvas,
    canvasColor: c.canvas,
    visualDensity: VisualDensity.standard,
    extensions: [
      c,
      shape,
      const OrreryMotion(),
      dark ? StatusColors.dark : StatusColors.light,
    ],
    pageTransitionsTheme: const PageTransitionsTheme(
      builders: {
        TargetPlatform.android: OrrerySpringTransitionsBuilder(),
        TargetPlatform.iOS: OrrerySpringTransitionsBuilder(),
      },
    ),
    appBarTheme: AppBarTheme(
      backgroundColor: c.canvas,
      foregroundColor: c.ink,
      elevation: 0,
      scrolledUnderElevation: 0,
      centerTitle: false,
      titleTextStyle: text.titleMedium,
    ),
    cardTheme: CardThemeData(
      color: c.surface,
      elevation: 0,
      margin: EdgeInsets.zero,
      shape: RoundedRectangleBorder(
        borderRadius: shape.cardRadius,
        side: BorderSide(color: c.line),
      ),
    ),
    dividerTheme: DividerThemeData(color: c.line, space: 1),
    bottomSheetTheme: BottomSheetThemeData(
      backgroundColor: c.surface,
      showDragHandle: true,
      shape: RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(
          top: Radius.circular(shape.card + 4),
        ),
      ),
    ),
    dialogTheme: DialogThemeData(
      backgroundColor: c.surface,
      shape: RoundedRectangleBorder(
        borderRadius: shape.cardRadius,
        side: BorderSide(color: c.line),
      ),
    ),
    snackBarTheme: SnackBarThemeData(
      behavior: SnackBarBehavior.floating,
      shape: control,
    ),
    inputDecorationTheme: InputDecorationTheme(
      filled: true,
      fillColor: c.control,
      border: field(c.lineStrong),
      enabledBorder: field(c.lineStrong),
      focusedBorder: field(c.accent, 1.5),
      contentPadding: const EdgeInsets.symmetric(horizontal: 14, vertical: 10),
    ),
    filledButtonTheme: FilledButtonThemeData(
      style: FilledButton.styleFrom(
        backgroundColor: c.accent,
        foregroundColor: c.onAccent,
        minimumSize: Size(shape.touch, shape.touch),
        shape: control,
      ),
    ),
    outlinedButtonTheme: OutlinedButtonThemeData(
      style: OutlinedButton.styleFrom(
        foregroundColor: c.ink,
        backgroundColor: c.control,
        side: BorderSide(color: c.lineStrong),
        minimumSize: Size(shape.touch, shape.touch),
        shape: control,
      ),
    ),
    textButtonTheme: TextButtonThemeData(
      style: TextButton.styleFrom(
        foregroundColor: c.accentInk,
        minimumSize: Size(shape.touch, shape.touch),
        shape: control,
      ),
    ),
    floatingActionButtonTheme: FloatingActionButtonThemeData(
      backgroundColor: c.accent,
      foregroundColor: c.onAccent,
      elevation: 0,
      highlightElevation: 0,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
    ),
    navigationBarTheme: NavigationBarThemeData(
      backgroundColor: c.surface,
      indicatorColor: c.selection,
      surfaceTintColor: Colors.transparent,
    ),
  );
}
