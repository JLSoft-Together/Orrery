class ChatLimits {
  static const int proto = 1;
  static const int page = 30;
  static const int pageMax = 50;
  static const int watchMax = 2;
  static const int meMax = 4000;
  static const int aiMax = 8000;
  static const int toolsMax = 30;
  static const int filesMax = 20;
  static const int lastMax = 160;
  static const int idMax = 64;
  static const int menuItemsMax = 12;
}

enum ChatToolCat { edit, run, read, search, web, agent, other }

ChatToolCat chatToolCatOf(Object? value) => switch (value) {
  'edit' => ChatToolCat.edit,
  'run' => ChatToolCat.run,
  'read' => ChatToolCat.read,
  'search' => ChatToolCat.search,
  'web' => ChatToolCat.web,
  'agent' => ChatToolCat.agent,
  _ => ChatToolCat.other,
};

enum ChatSysKind { stop, error, limit, compact }

ChatSysKind? chatSysKindOf(Object? value) => switch (value) {
  'stop' => ChatSysKind.stop,
  'error' => ChatSysKind.error,
  'limit' => ChatSysKind.limit,
  'compact' => ChatSysKind.compact,
  _ => null,
};

String? _str(Object? v) => v is String ? v : null;

int? _int(Object? v) =>
    v is int ? v : (v is double && v.isFinite && v == v.roundToDouble() ? v.toInt() : null);

int? _count(Object? v) {
  final n = _int(v);
  return n != null && n >= 0 ? n : null;
}

bool _flag(Object? v) => v == true;

bool _okId(String? v) => v != null && v.isNotEmpty && v.length <= ChatLimits.idMax;

List<T> _list<T>(Object? raw, T? Function(Object?) parse, int max) {
  if (raw is! List) return const [];
  final out = <T>[];
  for (final e in raw) {
    if (out.length >= max) break;
    final v = parse(e);
    if (v != null) out.add(v);
  }
  return out;
}

class ChatTool {
  const ChatTool({required this.tool, required this.cat, this.target, this.err = false});

  final String tool;
  final ChatToolCat cat;
  final String? target;
  final bool err;

  static ChatTool? parse(Object? raw) {
    if (raw is! Map) return null;
    final tool = _str(raw['tool']);
    if (tool == null || tool.isEmpty) return null;
    return ChatTool(
      tool: tool,
      cat: chatToolCatOf(raw['cat']),
      target: _str(raw['target']),
      err: _flag(raw['err']),
    );
  }
}

class ChatFile {
  const ChatFile({required this.path, this.add, this.del, this.outside = false});

  final String path;
  final int? add;
  final int? del;
  final bool outside;

  static ChatFile? parse(Object? raw) {
    if (raw is! Map) return null;
    final path = _str(raw['path']);
    if (path == null || path.isEmpty) return null;
    return ChatFile(path: path, add: _count(raw['add']), del: _count(raw['del']), outside: _flag(raw['out']));
  }
}

sealed class ChatItem {
  const ChatItem({required this.id, required this.seq, required this.rev, this.at});

  final String id;
  final int seq;
  final int rev;
  final DateTime? at;

  static ChatItem? parse(Object? raw) {
    if (raw is! Map) return null;
    final id = _str(raw['id']);
    final seq = _count(raw['seq']);
    if (!_okId(id) || seq == null) return null;
    final rev = _count(raw['rev']) ?? 0;
    final atMs = _int(raw['at']);
    final at = atMs != null && atMs > 0 ? DateTime.fromMillisecondsSinceEpoch(atMs) : null;
    switch (raw['k']) {
      case 'me':
        final text = _str(raw['text']);
        return text == null ? null : ChatMe(id: id!, seq: seq, rev: rev, at: at, text: text, cut: _flag(raw['cut']));
      case 'ai':
        final text = _str(raw['text']);
        return text == null ? null : ChatAi(id: id!, seq: seq, rev: rev, at: at, text: text, cut: _flag(raw['cut']));
      case 'tools':
        final sum = <ChatToolCat, int>{};
        final rawSum = raw['sum'];
        if (rawSum is Map) {
          rawSum.forEach((k, v) {
            final n = _count(v);
            if (n == null || n == 0) return;
            final cat = chatToolCatOf(k);
            sum[cat] = (sum[cat] ?? 0) + n;
          });
        }
        return ChatTools(
          id: id!,
          seq: seq,
          rev: rev,
          at: at,
          sum: sum,
          items: _list(raw['items'], ChatTool.parse, ChatLimits.toolsMax),
          more: _count(raw['more']) ?? 0,
        );
      case 'files':
        final root = _str(raw['root']);
        if (root == null) return null;
        return ChatFiles(
          id: id!,
          seq: seq,
          rev: rev,
          at: at,
          root: root,
          files: _list(raw['files'], ChatFile.parse, ChatLimits.filesMax),
          more: _count(raw['more']) ?? 0,
        );
      case 'sys':
        final kind = chatSysKindOf(raw['kind']);
        return kind == null ? null : ChatSys(id: id!, seq: seq, rev: rev, at: at, kind: kind, text: _str(raw['text']));
      default:
        return null;
    }
  }
}

class ChatMe extends ChatItem {
  const ChatMe({required super.id, required super.seq, required super.rev, super.at, required this.text, this.cut = false});

  final String text;
  final bool cut;
}

class ChatAi extends ChatItem {
  const ChatAi({required super.id, required super.seq, required super.rev, super.at, required this.text, this.cut = false});

  final String text;
  final bool cut;
}

class ChatTools extends ChatItem {
  const ChatTools({
    required super.id,
    required super.seq,
    required super.rev,
    super.at,
    required this.sum,
    required this.items,
    this.more = 0,
  });

  final Map<ChatToolCat, int> sum;
  final List<ChatTool> items;
  final int more;
}

class ChatFiles extends ChatItem {
  const ChatFiles({
    required super.id,
    required super.seq,
    required super.rev,
    super.at,
    required this.root,
    required this.files,
    this.more = 0,
  });

  final String root;
  final List<ChatFile> files;
  final int more;

  int get added => files.fold(0, (t, f) => t + (f.add ?? 0));
  int get removed => files.fold(0, (t, f) => t + (f.del ?? 0));
}

class ChatSys extends ChatItem {
  const ChatSys({required super.id, required super.seq, required super.rev, super.at, required this.kind, this.text});

  final ChatSysKind kind;
  final String? text;
}

enum ChatAgent { claude, codex }

class ChatPage {
  const ChatPage({required this.paneKey, required this.agent, required this.gen, required this.items, required this.older});

  final String paneKey;
  final ChatAgent agent;
  final String gen;
  final List<ChatItem> items;
  final int? older;

  static ChatPage? parse(Object? raw) {
    if (raw is! Map) return null;
    final paneKey = _str(raw['paneKey']);
    final gen = _str(raw['gen']);
    if (paneKey == null || gen == null) return null;
    return ChatPage(
      paneKey: paneKey,
      agent: raw['agent'] == 'codex' ? ChatAgent.codex : ChatAgent.claude,
      gen: gen,
      items: _list(raw['items'], ChatItem.parse, ChatLimits.pageMax * 4),
      older: _count(raw['older']),
    );
  }
}

class ChatLast {
  const ChatLast({required this.paneKey, required this.text, this.at});

  final String paneKey;
  final String? text;
  final DateTime? at;

  static ChatLast? parse(Object? raw) {
    if (raw is! Map) return null;
    final paneKey = _str(raw['paneKey']);
    if (paneKey == null || paneKey.isEmpty) return null;
    final atMs = _int(raw['at']);
    return ChatLast(
      paneKey: paneKey,
      text: _str(raw['text']),
      at: atMs != null && atMs > 0 ? DateTime.fromMillisecondsSinceEpoch(atMs) : null,
    );
  }
}

sealed class ChatEvent {
  const ChatEvent();

  static ChatEvent? parse(Object? raw) {
    if (raw is! Map) return null;
    switch (raw['t']) {
      case 'chat.items':
        final paneKey = _str(raw['paneKey']);
        final gen = _str(raw['gen']);
        if (paneKey == null || gen == null) return null;
        return ChatItemsEvent(paneKey, gen, _list(raw['items'], ChatItem.parse, 1000));
      case 'chat.reset':
        final paneKey = _str(raw['paneKey']);
        final gen = _str(raw['gen']);
        if (paneKey == null || gen == null) return null;
        return ChatResetEvent(paneKey, gen);
      case 'chat.last':
        return ChatLastEvent(_flag(raw['full']), _list(raw['items'], ChatLast.parse, 1000));
      default:
        return null;
    }
  }
}

class ChatItemsEvent extends ChatEvent {
  const ChatItemsEvent(this.paneKey, this.gen, this.items);

  final String paneKey;
  final String gen;
  final List<ChatItem> items;
}

class ChatResetEvent extends ChatEvent {
  const ChatResetEvent(this.paneKey, this.gen);

  final String paneKey;
  final String gen;
}

class ChatLastEvent extends ChatEvent {
  const ChatLastEvent(this.full, this.items);

  final bool full;
  final List<ChatLast> items;
}

class ChatLog {
  ChatLog(this.paneKey);

  final String paneKey;
  String? gen;
  int? older;
  bool stale = false;
  final Map<String, ChatItem> _byId = {};

  List<ChatItem> get items {
    final out = _byId.values.toList()
      ..sort((a, b) {
        final c = a.seq.compareTo(b.seq);
        return c != 0 ? c : a.id.compareTo(b.id);
      });
    return out;
  }

  List<ChatItem> get view {
    final out = <ChatItem>[];
    final held = <ChatItem>[];
    for (final item in items) {
      if (item is ChatFiles) {
        held.add(item);
        continue;
      }
      if (item is ChatMe) {
        out.addAll(held);
        held.clear();
      }
      out.add(item);
    }
    return out..addAll(held);
  }

  bool get hasOlder => older != null;

  void addPage(ChatPage page) {
    if (page.paneKey != paneKey) return;
    if (gen != page.gen) {
      _byId.clear();
      gen = page.gen;
      older = page.older;
    } else if (older != null) {
      final next = page.older;
      older = next == null || next < older! ? next : older;
    }
    stale = false;
    _upsert(page.items);
  }

  bool apply(ChatEvent event) {
    switch (event) {
      case ChatItemsEvent(:final paneKey, :final gen, :final items):
        if (paneKey != this.paneKey) return false;
        if (gen != this.gen) {
          stale = true;
          return false;
        }
        _upsert(items);
        return true;
      case ChatResetEvent(:final paneKey, :final gen):
        if (paneKey != this.paneKey || gen == this.gen) return false;
        _byId.clear();
        this.gen = gen;
        older = null;
        stale = true;
        return true;
      case ChatLastEvent():
        return false;
    }
  }

  void _upsert(List<ChatItem> list) {
    for (final item in list) {
      final cur = _byId[item.id];
      if (cur == null || item.rev >= cur.rev) _byId[item.id] = item;
    }
  }
}

enum RemoteMenuKind { ask, askMulti, permission, plan, picker, codexApproval, unknown }

RemoteMenuKind remoteMenuKindOf(Object? value) => switch (value) {
  'ask' => RemoteMenuKind.ask,
  'ask-multi' => RemoteMenuKind.askMulti,
  'permission' => RemoteMenuKind.permission,
  'plan' => RemoteMenuKind.plan,
  'picker' => RemoteMenuKind.picker,
  'codex-approval' => RemoteMenuKind.codexApproval,
  _ => RemoteMenuKind.unknown,
};

class RemoteMenuItem {
  const RemoteMenuItem({
    required this.index,
    required this.label,
    this.selected = false,
    this.checked,
    this.other = false,
    this.submit = false,
  });

  final int index;
  final String label;
  final bool selected;
  final bool? checked;
  final bool other;
  final bool submit;

  static RemoteMenuItem? parse(Object? raw) {
    if (raw is! Map) return null;
    final index = _int(raw['index']);
    final label = _str(raw['label']);
    if (index == null || index < 1 || index > ChatLimits.menuItemsMax || label == null) return null;
    final checked = raw['checked'];
    return RemoteMenuItem(
      index: index,
      label: label,
      selected: _flag(raw['selected']),
      checked: checked is bool ? checked : null,
      other: _flag(raw['other']),
      submit: _flag(raw['submit']),
    );
  }
}

class RemoteMenu {
  const RemoteMenu({
    required this.sig,
    required this.kind,
    required this.question,
    required this.items,
    required this.numbered,
    required this.confident,
    this.since,
  });

  final String sig;
  final RemoteMenuKind kind;
  final String question;
  final List<RemoteMenuItem> items;
  final bool numbered;
  final bool confident;
  final DateTime? since;

  List<RemoteMenuItem> get answerable => items.where((i) => !i.other).toList();

  static RemoteMenu? parse(Object? raw) {
    if (raw is! Map) return null;
    final sig = _str(raw['sig']);
    if (sig == null || sig.isEmpty) return null;
    final items = _list(raw['items'], RemoteMenuItem.parse, ChatLimits.menuItemsMax);
    if (items.isEmpty) return null;
    final since = _int(raw['since']);
    return RemoteMenu(
      sig: sig,
      kind: remoteMenuKindOf(raw['kind']),
      question: _str(raw['question']) ?? '',
      items: items,
      numbered: _flag(raw['numbered']),
      confident: _flag(raw['confident']),
      since: since != null && since > 0 ? DateTime.fromMillisecondsSinceEpoch(since) : null,
    );
  }
}
