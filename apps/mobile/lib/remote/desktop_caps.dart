const int maxDesktopCaps = 32;

Set<String> desktopCaps(Map<String, dynamic>? hello) {
  final value = hello?['caps'];
  if (value is! List) return const {};
  return {
    for (final cap in value.take(maxDesktopCaps))
      if (cap is String && cap.isNotEmpty) cap,
  };
}

bool hasDesktopCap(Map<String, dynamic>? hello, String name) =>
    desktopCaps(hello).contains(name);
