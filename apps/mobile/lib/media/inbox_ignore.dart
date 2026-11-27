import '../ui/viewers/viewer_api.dart';
import 'image_upload.dart';

const String inboxIgnoreLine = '.orrery/inbox/';

const Set<String> _covers = {
  '.orrery',
  '.orrery/',
  '/.orrery',
  '/.orrery/',
  '.orrery/inbox',
  '.orrery/inbox/',
  '/.orrery/inbox',
  '/.orrery/inbox/',
  '.orrery/*',
  '.orrery/**',
};

bool ignoresInbox(String text) =>
    text.split(RegExp(r'\r?\n')).any((l) => _covers.contains(l.trim()));

String withInboxIgnore(String text, {String eol = '\n'}) {
  if (text.isEmpty) return '$inboxIgnoreLine$eol';
  final tail = text.endsWith('\n') ? '' : eol;
  return '$text$tail$inboxIgnoreLine$eol';
}

String joinRoot(String root, String name) {
  final back = root.contains('\\') && !root.contains('/');
  final sep = back ? '\\' : '/';
  final trimmed = root.replaceAll(RegExp(r'[\\/]+$'), '');
  return '$trimmed$sep$name';
}

enum IgnoreResult { added, already, waiting }

Future<IgnoreResult> addInboxIgnore(PutCall call, String root) async {
  final path = joinRoot(root, '.gitignore');
  var text = '';
  var eol = '\n';
  num? base;
  final read = (await call('fs.read', {'path': path})).map;
  final kind = read['kind'];
  if (kind == 'text') {
    text = read['text'] is String ? read['text'] as String : '';
    eol = read['eol'] == 'crlf' || read['eol'] == '\r\n' ? '\r\n' : '\n';
    base = read['mtimeMs'] is num ? read['mtimeMs'] as num : null;
  } else if (kind != 'missing') {
    throw const ViewerFailure('failed');
  }
  if (ignoresInbox(text)) return IgnoreResult.already;
  final fields = <String, dynamic>{
    'path': path,
    'text': withInboxIgnore(text, eol: eol),
  };
  if (base != null) fields['base'] = base;
  final reply = await call('fs.write', fields);
  return reply.pending ? IgnoreResult.waiting : IgnoreResult.added;
}
