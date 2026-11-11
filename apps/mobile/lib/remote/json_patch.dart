class PatchException implements Exception {
  const PatchException(this.message);

  final String message;

  @override
  String toString() => 'PatchException($message)';
}

Object? deepCopy(Object? value) {
  if (value is Map) {
    return <String, dynamic>{
      for (final entry in value.entries)
        entry.key as String: deepCopy(entry.value),
    };
  }
  if (value is List) {
    return <dynamic>[for (final item in value) deepCopy(item)];
  }
  return value;
}

List<String> _pointer(String path) {
  if (path.isEmpty) return const [];
  if (!path.startsWith('/')) throw PatchException('pointer $path');
  return path
      .substring(1)
      .split('/')
      .map((part) => part.replaceAll('~1', '/').replaceAll('~0', '~'))
      .toList();
}

int _index(String token, int length, {required bool allowEnd}) {
  if (token == '-' && allowEnd) return length;
  final index = int.tryParse(token);
  if (index == null || index < 0 || index > length) {
    throw PatchException('index $token');
  }
  return index;
}

Object? applyJsonPatch(Object? document, List<dynamic> ops) {
  var root = deepCopy(document);
  for (final raw in ops) {
    if (raw is! Map) throw const PatchException('op shape');
    final op = raw['op'];
    final path = raw['path'];
    if (op is! String || path is! String) {
      throw const PatchException('op fields');
    }
    final tokens = _pointer(path);
    if (tokens.isEmpty) {
      if (op == 'remove') throw const PatchException('remove root');
      root = deepCopy(raw['value']);
      continue;
    }
    Object? parent = root;
    for (final token in tokens.take(tokens.length - 1)) {
      parent = _child(parent, token);
    }
    final last = tokens.last;
    final value = deepCopy(raw['value']);
    switch (op) {
      case 'add':
        if (parent is Map) {
          parent[last] = value;
        } else if (parent is List) {
          parent.insert(_index(last, parent.length, allowEnd: true), value);
        } else {
          throw const PatchException('add target');
        }
      case 'replace':
        if (parent is Map) {
          if (!parent.containsKey(last)) throw PatchException('missing $last');
          parent[last] = value;
        } else if (parent is List) {
          final index = _index(last, parent.length, allowEnd: false);
          if (index >= parent.length) throw PatchException('missing $last');
          parent[index] = value;
        } else {
          throw const PatchException('replace target');
        }
      case 'remove':
        if (parent is Map) {
          if (!parent.containsKey(last)) throw PatchException('missing $last');
          parent.remove(last);
        } else if (parent is List) {
          final index = _index(last, parent.length, allowEnd: false);
          if (index >= parent.length) throw PatchException('missing $last');
          parent.removeAt(index);
        } else {
          throw const PatchException('remove target');
        }
      default:
        throw PatchException('op $op');
    }
  }
  return root;
}

Object? _child(Object? parent, String token) {
  if (parent is Map) {
    if (!parent.containsKey(token)) throw PatchException('missing $token');
    return parent[token];
  }
  if (parent is List) {
    final index = _index(token, parent.length, allowEnd: false);
    if (index >= parent.length) throw PatchException('missing $token');
    return parent[index];
  }
  throw PatchException('path $token');
}
