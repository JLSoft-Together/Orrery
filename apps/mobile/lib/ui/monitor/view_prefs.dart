import 'dart:convert';
import 'dart:io';

import 'package:flutter_riverpod/flutter_riverpod.dart';
import 'package:path_provider/path_provider.dart';

enum ViewMode { primary, alternate }

class ViewPrefs {
  const ViewPrefs({
    this.monitor = ViewMode.primary,
    this.workspace = ViewMode.primary,
    this.language = 'auto',
    this.theme = 'auto',
  });

  final ViewMode monitor;
  final ViewMode workspace;
  final String language;
  final String theme;

  ViewPrefs copyWith({
    ViewMode? monitor,
    ViewMode? workspace,
    String? language,
    String? theme,
  }) => ViewPrefs(
    monitor: monitor ?? this.monitor,
    workspace: workspace ?? this.workspace,
    language: language ?? this.language,
    theme: theme ?? this.theme,
  );

  Map<String, Object?> toJson() => {
    'monitor': monitor.name,
    'workspace': workspace.name,
    'language': language,
    'theme': theme,
  };

  static const languages = ['auto', 'vi', 'en'];
  static const themes = ['auto', 'system', 'light', 'dark'];

  static String _pick(Object? raw, List<String> allowed) =>
      raw is String && allowed.contains(raw) ? raw : 'auto';

  static ViewMode _mode(Object? raw) =>
      raw == 'alternate' ? ViewMode.alternate : ViewMode.primary;

  static ViewPrefs fromJson(Object? raw) => raw is Map
      ? ViewPrefs(
          monitor: _mode(raw['monitor']),
          workspace: _mode(raw['workspace']),
          language: _pick(raw['language'], languages),
          theme: _pick(raw['theme'], themes),
        )
      : const ViewPrefs();
}

abstract class ViewPrefsStore {
  Future<ViewPrefs> load();

  Future<void> save(ViewPrefs prefs);
}

class FileViewPrefsStore implements ViewPrefsStore {
  FileViewPrefsStore([this._dir]);

  final Directory? _dir;

  Future<File> _file() async {
    final dir = _dir ?? await getApplicationSupportDirectory();
    return File('${dir.path}${Platform.pathSeparator}view_prefs.json');
  }

  @override
  Future<ViewPrefs> load() async {
    try {
      final file = await _file();
      if (!await file.exists()) return const ViewPrefs();
      return ViewPrefs.fromJson(jsonDecode(await file.readAsString()));
    } on Object {
      return const ViewPrefs();
    }
  }

  @override
  Future<void> save(ViewPrefs prefs) async {
    try {
      final file = await _file();
      await file.writeAsString(jsonEncode(prefs.toJson()));
    } on Object {
      return;
    }
  }
}

final viewPrefsStoreProvider = Provider<ViewPrefsStore>(
  (ref) => FileViewPrefsStore(),
);

class ViewPrefsNotifier extends Notifier<ViewPrefs> {
  bool _touched = false;

  @override
  ViewPrefs build() {
    final store = ref.read(viewPrefsStoreProvider);
    store.load().then((loaded) {
      if (!_touched && ref.mounted) state = loaded;
    });
    return const ViewPrefs();
  }

  ViewMode _flip(ViewMode mode) =>
      mode == ViewMode.primary ? ViewMode.alternate : ViewMode.primary;

  void toggleMonitor() => _set(state.copyWith(monitor: _flip(state.monitor)));

  void toggleWorkspace() =>
      _set(state.copyWith(workspace: _flip(state.workspace)));

  void setLanguage(String value) => _set(state.copyWith(language: value));

  void setTheme(String value) => _set(state.copyWith(theme: value));

  void _set(ViewPrefs next) {
    _touched = true;
    state = next;
    ref.read(viewPrefsStoreProvider).save(next);
  }
}

final viewPrefsProvider = NotifierProvider<ViewPrefsNotifier, ViewPrefs>(
  ViewPrefsNotifier.new,
);
