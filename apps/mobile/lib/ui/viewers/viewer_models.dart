String? str(Object? v) => v is String && v.isNotEmpty ? v : null;

int intOf(Object? v) => v is num ? v.toInt() : 0;

int? intOrNull(Object? v) => v is num ? v.toInt() : null;

List<Map<String, dynamic>> maps(Object? v) => v is List
    ? [
        for (final e in v)
          if (e is Map) e.cast<String, dynamic>(),
      ]
    : const [];

class DirEntry {
  const DirEntry(this.name, this.path, this.directory, this.outOfRoot);

  final String name;
  final String path;
  final bool directory;
  final bool outOfRoot;

  static List<DirEntry> parseAll(Object? raw) {
    final rows = [
      for (final m in maps(raw))
        if (str(m['name']) != null && str(m['path']) != null)
          DirEntry(
            m['name'] as String,
            m['path'] as String,
            m['directory'] == true,
            m['outOfRoot'] == true,
          ),
    ];
    rows.sort((a, b) {
      if (a.directory != b.directory) return a.directory ? -1 : 1;
      return a.name.toLowerCase().compareTo(b.name.toLowerCase());
    });
    return rows;
  }
}

class FileView {
  const FileView({
    required this.kind,
    required this.path,
    required this.size,
    required this.mtimeMs,
    this.text,
    this.lang,
    this.eol,
    this.bom = false,
    this.invalidUtf8 = false,
  });

  final String kind;
  final String path;
  final int size;
  final num mtimeMs;
  final String? text;
  final String? lang;
  final String? eol;
  final bool bom;
  final bool invalidUtf8;

  bool get editable => kind == 'text' && !invalidUtf8;

  static FileView parse(Map<String, dynamic> m) => FileView(
    kind: str(m['kind']) ?? 'missing',
    path: str(m['path']) ?? '',
    size: intOf(m['size']),
    mtimeMs: m['mtimeMs'] is num ? m['mtimeMs'] as num : 0,
    text: m['text'] is String ? m['text'] as String : null,
    lang: str(m['lang']),
    eol: str(m['eol']),
    bom: m['bom'] == true,
    invalidUtf8: m['invalidUtf8'] == true,
  );
}

class ChangeEntry {
  const ChangeEntry({
    required this.path,
    required this.kind,
    required this.x,
    required this.y,
    this.origPath,
    this.added,
    this.removed,
    this.binary = false,
  });

  final String path;
  final String kind;
  final String x;
  final String y;
  final String? origPath;
  final int? added;
  final int? removed;
  final bool binary;

  bool get staged => x != '.' && x != '?';

  bool get unstaged => y != '.' || x == '?';

  String get mark => switch (kind) {
    'added' => 'A',
    'deleted' => 'D',
    'renamed' => 'R',
    'untracked' => '?',
    'conflict' => '!',
    _ => 'M',
  };

  static ChangeEntry? parse(Map<String, dynamic> m) {
    final path = str(m['path']);
    if (path == null) return null;
    return ChangeEntry(
      path: path,
      kind: str(m['kind']) ?? 'modified',
      x: str(m['x']) ?? '.',
      y: str(m['y']) ?? '.',
      origPath: str(m['origPath']),
      added: intOrNull(m['added']),
      removed: intOrNull(m['removed']),
      binary: m['binary'] == true,
    );
  }
}

class ChangesStatus {
  const ChangesStatus({
    required this.root,
    required this.entries,
    this.branch,
    this.upstream,
    this.ahead = 0,
    this.behind = 0,
    this.truncated = false,
  });

  final String root;
  final List<ChangeEntry> entries;
  final String? branch;
  final String? upstream;
  final int ahead;
  final int behind;
  final bool truncated;

  List<ChangeEntry> get staged => [
    for (final e in entries)
      if (e.staged) e,
  ];

  List<ChangeEntry> get unstaged => [
    for (final e in entries)
      if (e.unstaged) e,
  ];

  static ChangesStatus parse(Map<String, dynamic> m) => ChangesStatus(
    root: str(m['root']) ?? '',
    branch: str(m['branch']),
    upstream: str(m['upstream']),
    ahead: intOf(m['ahead']),
    behind: intOf(m['behind']),
    truncated: m['truncated'] == true,
    entries: [for (final e in maps(m['entries'])) ?ChangeEntry.parse(e)],
  );
}

String joinPath(String dir, String name) {
  final windows = dir.contains('\\') && !dir.contains('/');
  final trimmed = dir.replaceAll(RegExp(r'[\\/]+$'), '');
  return '$trimmed${windows ? '\\' : '/'}${windows ? name.replaceAll('/', '\\') : name}';
}

String sizeText(int bytes) {
  if (bytes < 1024) return '$bytes B';
  if (bytes < 1024 * 1024) return '${(bytes / 1024).toStringAsFixed(1)} KB';
  return '${(bytes / 1048576).toStringAsFixed(1)} MB';
}

const List<String> monoFallback = ['Menlo', 'Consolas', 'Courier New'];
