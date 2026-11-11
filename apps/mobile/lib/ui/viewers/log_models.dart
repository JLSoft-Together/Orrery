import 'viewer_models.dart';

class LogCommit {
  const LogCommit({
    required this.hash,
    required this.parents,
    required this.author,
    required this.time,
    required this.refs,
    required this.subject,
  });

  final String hash;
  final List<String> parents;
  final String author;
  final int time;
  final String refs;
  final String subject;

  String get shortHash => hash.length > 7 ? hash.substring(0, 7) : hash;

  List<String> get decorations => [
    for (final r in refs.split(','))
      if (r.trim().isNotEmpty) r.trim(),
  ];

  static LogCommit? parse(Map<String, dynamic> m) {
    final hash = str(m['hash']);
    if (hash == null) return null;
    final parents = m['parents'];
    return LogCommit(
      hash: hash,
      parents: [
        if (parents is List)
          for (final p in parents)
            if (p is String) p,
      ],
      author: str(m['author']) ?? '',
      time: intOf(m['time']),
      refs: str(m['refs']) ?? '',
      subject: str(m['subject']) ?? '',
    );
  }
}

class LogPageData {
  const LogPageData(this.commits, this.more);

  final List<LogCommit> commits;
  final bool more;

  static LogPageData parse(Map<String, dynamic> m) => LogPageData([
    for (final c in maps(m['commits'])) ?LogCommit.parse(c),
  ], m['more'] == true);
}

class LocalRef {
  const LocalRef({
    required this.ref,
    required this.name,
    this.upstream,
    this.ahead = 0,
    this.behind = 0,
    this.gone = false,
  });

  final String ref;
  final String name;
  final String? upstream;
  final int ahead;
  final int behind;
  final bool gone;
}

class RefsInfo {
  const RefsInfo({
    required this.head,
    required this.local,
    required this.remotes,
    required this.tags,
  });

  final String? head;
  final List<LocalRef> local;
  final List<(String, String)> remotes;
  final List<(String, String)> tags;

  static RefsInfo parse(Map<String, dynamic> m) {
    final head = m['head'];
    return RefsInfo(
      head: head is Map ? str(head['branch']) : null,
      local: [
        for (final r in maps(m['local']))
          if (str(r['ref']) != null)
            LocalRef(
              ref: r['ref'] as String,
              name: str(r['name']) ?? r['ref'] as String,
              upstream: str(r['upstream']),
              ahead: intOf(r['ahead']),
              behind: intOf(r['behind']),
              gone: r['gone'] == true,
            ),
      ],
      remotes: [
        for (final g in maps(m['remotes']))
          for (final r in maps(g['refs']))
            if (str(r['ref']) != null)
              (r['ref'] as String, str(r['name']) ?? r['ref'] as String),
      ],
      tags: [
        for (final r in maps(m['tags']))
          if (str(r['ref']) != null)
            (r['ref'] as String, str(r['name']) ?? r['ref'] as String),
      ],
    );
  }
}

class CommitFile {
  const CommitFile(this.path, this.kind, this.added, this.removed, this.binary);

  final String path;
  final String kind;
  final int? added;
  final int? removed;
  final bool binary;
}

class CommitDetail {
  const CommitDetail({
    required this.hash,
    required this.author,
    required this.message,
    required this.time,
    required this.files,
  });

  final String hash;
  final String author;
  final String message;
  final int time;
  final List<CommitFile> files;

  static CommitDetail parse(Map<String, dynamic> m) => CommitDetail(
    hash: str(m['hash']) ?? '',
    author: str(m['author']) ?? '',
    message: str(m['message']) ?? '',
    time: intOf(m['authorTime']),
    files: [
      for (final f in maps(m['files']))
        if (str(f['path']) != null)
          CommitFile(
            f['path'] as String,
            str(f['kind']) ?? 'modified',
            intOrNull(f['added']),
            intOrNull(f['removed']),
            f['binary'] == true,
          ),
    ],
  );
}

class PushPreview {
  const PushPreview({
    required this.local,
    required this.remote,
    required this.branch,
    required this.exists,
    required this.hasUpstream,
    required this.commits,
    required this.more,
  });

  final String local;
  final String remote;
  final String branch;
  final bool exists;
  final bool hasUpstream;
  final List<LogCommit> commits;
  final bool more;

  String get target => '$remote/$branch';

  static PushPreview parse(Map<String, dynamic> m) => PushPreview(
    local: str(m['local']) ?? '',
    remote: str(m['remote']) ?? 'origin',
    branch: str(m['branch']) ?? '',
    exists: m['exists'] == true,
    hasUpstream: m['hasUpstream'] == true,
    commits: [for (final c in maps(m['commits'])) ?LogCommit.parse(c)],
    more: m['more'] == true,
  );
}
