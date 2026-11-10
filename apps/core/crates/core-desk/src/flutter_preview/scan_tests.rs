use super::classes::{class_hit, is_web_only, scan_dart_classes, widget_names, FileFlags};
use super::mask::mask_dart;
use super::project::scan_project;
use super::scan::{has_required_params, scan_dart_source, PreviewHit};
use std::collections::HashMap;
use std::path::Path;

const SRC: &str = r#"import 'package:flutter/widget_previews.dart';
import 'package:flutter/widget_previews.dart' as wp;

// @Preview() in a comment
const note = '@Preview() in a string ${'@Preview()'}';

@Preview(name: 'Primary button')
Widget primaryButton() => const Text('hi');

@Preview(
  name: "Card",
  group: 'cards',
)
@Deprecated('x')
Widget card() {
  return const Card();
}

@wp.Preview()
List<Widget> prefixed() => [];

@Preview()
Widget _hidden() => const SizedBox();

@Preview()
Widget needsArg(String title) => Text(title);

@Preview()
Widget namedOptional({String title = 'a'}) => Text(title);

@Preview()
Widget namedRequired({required String title}) => Text(title);

class Gallery {
  @Preview(name: 'Static one')
  static Widget one() => const Text('1');

  @Preview()
  Widget instance() => const Text('2');
}

class MyCard extends StatelessWidget {
  @Preview()
  const MyCard({super.key, this.title = '{'});

  @Preview(name: 'Small')
  const MyCard.small({super.key}) : title = 's';

  @Preview()
  MyCard.titled(this.title, {super.key});

  final String title;

  @override
  Widget build(BuildContext context) => Text(title);
}

class _Private extends StatelessWidget {
  @Preview()
  const _Private();
  @override
  Widget build(BuildContext context) => const SizedBox();
}

@Preview()
class NotAFunction {}
"#;

fn by_symbol(hits: Vec<PreviewHit>) -> HashMap<String, PreviewHit> {
    hits.into_iter().map(|h| (h.symbol.clone(), h)).collect()
}

fn tmp(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("fwp-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn put(root: &Path, rel: &str, text: &str) {
    let f = root.join(rel);
    std::fs::create_dir_all(f.parent().unwrap()).unwrap();
    std::fs::write(f, text).unwrap();
}

#[test]
fn masks_nested_interpolation() {
    let m = mask_dart("x = '${m['k']} }'; y {}");
    assert!(m.contains("y {}"));
    assert_eq!(m.matches(['{', '}']).count(), 2);
}

#[test]
fn required_params() {
    for (p, want) in [("", false), ("{super.key}", false), ("{String a = \"x\", int? b}", false), ("[int a = 1]", false), ("String a", true), ("this.title, {super.key}", true), ("{required this.x}", true), ("{@Deprecated(\"\") required int x}", true)] {
        assert_eq!(has_required_params(p), want, "{p}");
    }
}

#[test]
fn finds_functions_members_and_reasons() {
    let hits = scan_dart_source(SRC);
    assert_eq!(hits[0].symbol, "primaryButton");
    let by = by_symbol(hits.clone());
    assert_eq!((by["primaryButton"].label.as_str(), by["primaryButton"].call.as_str(), by["primaryButton"].line), ("Primary button", "primaryButton()", 7));
    assert_eq!((by["card"].label.as_str(), by["card"].call.as_str()), ("Card", "card()"));
    assert_eq!((by["prefixed"].label.as_str(), by["prefixed"].call.as_str(), by["prefixed"].skipped), ("prefixed", "", Some("type")));
    assert_eq!(by["namedOptional"].skipped, None);
    assert_eq!((by["Gallery.one"].call.as_str(), by["Gallery.one"].label.as_str()), ("Gallery.one()", "Static one"));
    assert_eq!(by["MyCard"].call, "MyCard()");
    assert_eq!((by["MyCard.small"].call.as_str(), by["MyCard.small"].label.as_str()), ("MyCard.small()", "Small"));
    assert_eq!(by["_hidden"].skipped, Some("private"));
    assert_eq!(by["_Private"].skipped, Some("private"));
    assert_eq!(by["needsArg"].skipped, Some("params"));
    assert_eq!(by["namedRequired"].skipped, Some("params"));
    assert_eq!(by["MyCard.titled"].skipped, Some("params"));
    assert_eq!(by["Gallery.instance"].skipped, Some("unknown"));
    assert_eq!(hits.iter().filter(|h| h.skipped == Some("unknown") && h.symbol.starts_with('?')).count(), 1);
}

#[test]
fn return_types_and_names() {
    let src = r#"@Preview()
Widget Function() fnType() => () => const SizedBox();

@Preview()
WidgetBuilder builder() => (c) => const SizedBox();

@Preview()
Widget? nullable() => null;

@Preview()
external Widget ext();

@Preview()
StatelessWidget stateless() => const Text('a');

@Preview()
m.MyCard prefixedClass() => const m.MyCard();

@Preview()
omitted() => const Text('a');

@Preview(wrapper: wrap(name: 'inner'))
Widget nestedName() => const Text('a');

@Preview(theme: t(), name: 'Outer')
Widget outerName() => const Text('a');

class G {
  @Preview()
  static List<Widget> many() => [];

  @Preview()
  static Builder builderWidget() => Builder(builder: (_) => const SizedBox());
}
"#;
    let by = by_symbol(scan_dart_source(src));
    assert!(!by.contains_key("Function"));
    for s in ["fnType", "builder", "nullable", "G.many"] {
        assert_eq!(by[s].skipped, Some("type"), "{s}");
    }
    for s in ["ext", "stateless", "prefixedClass", "omitted", "G.builderWidget"] {
        assert_eq!(by[s].skipped, None, "{s}");
    }
    assert_eq!(by["nestedName"].label, "nestedName");
    assert_eq!(by["outerName"].label, "Outer");
}

#[test]
fn stacked_annotations_and_part_files() {
    let hits = scan_dart_source("@Preview(name: 'A')\n@Preview(name: 'B')\nWidget twice() => const Text('a');\n");
    assert_eq!(hits.len(), 1);
    assert_eq!((hits[0].symbol.as_str(), hits[0].label.as_str()), ("twice", "A"));
    let part = scan_dart_source("// lib\npart of 'main.dart';\n\n@Preview()\nWidget a() => const Text('a');\n");
    assert_eq!((part.len(), part[0].symbol.as_str(), part[0].call.as_str(), part[0].skipped), (1, "a", "", Some("part")));
    assert_eq!(scan_dart_source("part 'a.g.dart';\n@Preview()\nWidget a() => const Text('a');\n")[0].skipped, None);
}

#[test]
fn scan_project_walks_lib_only_sorted() {
    let root = tmp("scan");
    let one = "@Preview()\nWidget a() => const Text('a');\n";
    put(&root, "lib/w/b.dart", one);
    put(&root, "lib/a.dart", &format!("{one}@Preview()\nWidget b() => const Text('b');\n"));
    put(&root, "lib/a.g.dart", one);
    put(&root, "lib/.hidden/c.dart", one);
    put(&root, "test/d.dart", one);
    let entries = scan_project(&root);
    assert_eq!(entries.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), ["lib/a.dart#a", "lib/a.dart#b", "lib/w/b.dart#a"]);
    assert_eq!((entries[1].file.as_str(), entries[1].hit.line, entries[1].hit.call.as_str()), ("lib/a.dart", 3, "b()"));
    let _ = std::fs::remove_dir_all(&root);
}

const CLASSES: &str = r#"import 'package:flutter/material.dart' as m;

class Plain extends StatelessWidget {
  const Plain({super.key});
  static Plain make() => const Plain();
  @override
  Widget build(BuildContext context) => const Text('{');
}

class NoCtor extends m.StatelessWidget {
  @override
  Widget build(BuildContext context) => const SizedBox();
}

class Titled extends StatefulWidget {
  const Titled(this.title, {super.key});
  const Titled.empty({super.key}) : title = '';
  final String title;
  @override
  State<Titled> createState() => _TitledState();
}

class _TitledState extends State<Titled> {
  @override
  Widget build(BuildContext context) => Text(widget.title);
}

class Needs extends StatelessWidget {
  const Needs({super.key, required this.count});
  factory Needs.one() => const Needs(count: 1);
  final int count;
  @override
  Widget build(BuildContext context) => Text('$count');
}

class OnlyRequired extends StatelessWidget {
  const OnlyRequired({super.key, required this.count});
  final int count;
  @override
  Widget build(BuildContext context) => const SizedBox();
}

class Hidden extends StatelessWidget {
  const Hidden._();
  @override
  Widget build(BuildContext context) => const SizedBox();
}

abstract class BasePage extends StatelessWidget {
  const BasePage({super.key});
}

class Home extends BasePage {
  const Home({super.key});
  @override
  Widget build(BuildContext context) => const SizedBox();
}

class Box<T extends List<int>> extends StatelessWidget {
  const Box({super.key});
  @override
  Widget build(BuildContext context) => const SizedBox();
}

class _Row extends StatelessWidget {
  const _Row();
  @override
  Widget build(BuildContext context) => const SizedBox();
}

mixin class Mixed {}

class Model {
  Model(this.a);
  final int a;
}
"#;

#[test]
fn classes_and_hits() {
    let masked = mask_dart(CLASSES);
    let decls = scan_dart_classes(CLASSES, &masked);
    let by: HashMap<&str, &super::classes::ClassDecl> = decls.iter().map(|d| (d.name.as_str(), d)).collect();
    assert_eq!((by["Plain"].line, by["Plain"].super_name.as_deref(), by["Plain"].abstract_, by["Plain"].ctors.len()), (3, Some("StatelessWidget"), false, 1));
    assert_eq!((by["NoCtor"].super_name.as_deref(), by["NoCtor"].ctors.len()), (Some("StatelessWidget"), 0));
    let titled: Vec<(&str, bool)> = by["Titled"].ctors.iter().map(|c| (c.name.as_str(), c.required)).collect();
    assert_eq!(titled, [("Titled", true), ("Titled.empty", false)]);
    assert_eq!(by["Needs"].ctors.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(), ["Needs", "Needs.one"]);
    assert_eq!(by["Box"].super_name.as_deref(), Some("StatelessWidget"));
    assert!(by["BasePage"].abstract_);
    assert_eq!(by["Mixed"].super_name, None);
    let widgets = widget_names(decls.iter());
    for n in ["Plain", "NoCtor", "Titled", "Home", "BasePage", "Box", "_Row"] {
        assert!(widgets.contains(n), "{n}");
    }
    for n in ["_TitledState", "Model", "Mixed"] {
        assert!(!widgets.contains(n), "{n}");
    }
    let flags = FileFlags { part: false, web: false };
    let hit = |n: &str| class_hit(by[n], &flags, &[]);
    let p = hit("Plain").unwrap();
    assert_eq!((p.symbol.as_str(), p.label.as_str(), p.call.as_str(), p.kind, p.line), ("Plain", "Plain", "Plain()", Some("class"), 3));
    for (n, call) in [("NoCtor", "NoCtor()"), ("Titled", "Titled.empty()"), ("Needs", "Needs.one()"), ("Box", "Box()"), ("Home", "Home()")] {
        assert_eq!(hit(n).unwrap().call, call, "{n}");
    }
    for (n, why) in [("OnlyRequired", "params"), ("Hidden", "ctor"), ("_Row", "private")] {
        let h = hit(n).unwrap();
        assert_eq!((h.call.as_str(), h.skipped), ("", Some(why)), "{n}");
    }
    assert!(hit("BasePage").is_none());
    assert_eq!(class_hit(by["Plain"], &FileFlags { part: true, web: false }, &[]).unwrap().skipped, Some("part"));
    assert_eq!(class_hit(by["Plain"], &FileFlags { part: false, web: true }, &[]).unwrap().skipped, Some("web"));
    let listed = PreviewHit { line: 4, symbol: "Plain.small".into(), label: "x".into(), call: "Plain.small()".into(), kind: None, skipped: None };
    assert!(class_hit(by["Plain"], &flags, &[listed]).is_none());
}

#[test]
fn web_only_imports() {
    assert!(is_web_only("import 'dart:html';"));
    assert!(is_web_only("import \"package:web/web.dart\" as web;"));
    assert!(!is_web_only("import 'stub.dart' if (dart.library.html) 'web.dart';"));
    assert!(!is_web_only("// import 'dart:html';\nimport 'dart:io';"));
}

#[test]
fn project_widget_classes_across_files_and_rescan() {
    let root = tmp("classes");
    put(&root, "lib/base/page.dart", "abstract class AppPage extends StatelessWidget {\n  const AppPage({super.key});\n}\n");
    let home = [
        "import 'base/page.dart';",
        "class Home extends AppPage {",
        "  const Home({super.key});",
        "  @override",
        "  Widget build(BuildContext context) => const SizedBox();",
        "}",
        "@Preview(name: 'Home dark')",
        "Widget homeDark() => const Home();",
        "class Card2 extends StatelessWidget {",
        "  @Preview()",
        "  const Card2({super.key});",
        "  @override",
        "  Widget build(BuildContext context) => const SizedBox();",
        "}",
        "",
    ]
    .join("\n");
    put(&root, "lib/home.dart", &home);
    let entries = scan_project(&root);
    assert_eq!(entries.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), ["lib/home.dart#Home", "lib/home.dart#homeDark", "lib/home.dart#Card2"]);
    assert_eq!((entries[0].hit.kind, entries[0].hit.call.as_str(), entries[0].hit.line), (Some("class"), "Home()", 2));
    assert_eq!(entries[2].hit.kind, None);
    put(&root, "lib/home.dart", "class Other extends StatelessWidget {\n  const Other({super.key});\n}\n");
    assert_eq!(scan_project(&root).iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), ["lib/home.dart#Other"]);
    let _ = std::fs::remove_dir_all(&root);
}
