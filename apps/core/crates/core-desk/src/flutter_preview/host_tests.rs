use super::host::{host_context_with, host_source, read_package, write_host, HostContext, L10n};
use super::project::Entry;
use super::scan::PreviewHit;
use std::collections::HashMap;
use std::path::Path;

fn entry(id: &str, file: &str, line: u32, symbol: &str, label: &str, call: &str, skipped: Option<&'static str>) -> Entry {
    Entry { id: id.into(), file: file.into(), hit: PreviewHit { line, symbol: symbol.into(), label: label.into(), call: call.into(), kind: None, skipped } }
}

fn entries() -> Vec<Entry> {
    vec![
        entry("lib/a.dart#a", "lib/a.dart", 1, "a", "a", "a()", None),
        entry("lib/a.dart#Card.small", "lib/a.dart", 5, "Card.small", "Small", "Card.small()", None),
        entry("lib/w/b$'.dart#B", "lib/w/b$'.dart", 2, "B", "B", "B()", None),
        entry("lib/a.dart#_x", "lib/a.dart", 9, "_x", "_x", "", Some("private")),
    ]
}

#[test]
fn package_needs_valid_name() {
    let p = read_package("name: demo_app\ndependencies:\n  flutter:\n    sdk: flutter\n").unwrap();
    assert_eq!((p.name.as_str(), p.flutter), ("demo_app", true));
    assert!(!read_package("name: pure_dart\n").unwrap().flutter);
    assert!(read_package("name: Bad-Name\n").is_none());
}

#[test]
fn host_source_imports_and_registry() {
    let src = host_source("demo_app", &entries(), &HostContext::default());
    assert!(src.contains("import 'package:demo_app/a.dart' as p0;"));
    assert!(src.contains("import 'package:demo_app/w/b\\$\\'.dart' as p1;"));
    assert_eq!(src.matches("as p").count(), 2);
    assert!(src.contains("Map<String, Widget Function()> get _previews =>"));
    assert!(src.contains("'lib/a.dart#a': () => p0.a(),"));
    assert!(src.contains("'lib/a.dart#Card.small': () => p0.Card.small(),"));
    assert!(src.contains("() => p1.B(),"));
    assert!(!src.contains("_x"));
    assert!(src.contains("registerExtension('ext.ccm.show'"));
    assert!(src.contains("debugShowCheckedModeBanner: false"));
    assert!(src.contains("Select a widget in Orrery"));
    assert!(src.contains("ErrorWidget.builder = (details) => _ErrorCard(details.exceptionAsString());"));
    assert!(src.contains("print('ccm.error ' + jsonEncode("));
    assert!(src.contains("message.substring(0, 400)"));
    assert!(src.contains("void reassemble()"));
    assert!(!src.contains("@@"));
    assert_eq!(host_source("demo_app", &entries(), &HostContext::default()), src);
}

#[test]
fn host_source_empty_registry_layout() {
    let src = host_source("demo_app", &[], &HostContext::default());
    assert!(src.contains("import 'package:flutter/material.dart';\n\nMap<String, Widget Function()> get _previews => <String, Widget Function()>{\n    };\n"));
}

#[test]
fn host_source_wrappers() {
    let ctx = HostContext { riverpod: Some("package:flutter_riverpod/flutter_riverpod.dart".into()), l10n: Some(L10n { uri: "package:demo_app/l10n/app_localizations.dart".into(), class_name: "AppLocalizations".into() }) };
    let src = host_source("demo_app", &entries(), &ctx);
    assert!(src.contains("import 'package:flutter_riverpod/flutter_riverpod.dart' as ccm_riverpod;\nimport 'package:demo_app/l10n/app_localizations.dart' as ccm_l10n;\nimport 'package:demo_app/a.dart' as p0;"));
    assert!(src.contains("return ccm_riverpod.ProviderScope(key: ValueKey<String?>(id), child: build());"));
    assert!(src.contains("debugShowCheckedModeBanner: false,\n      localizationsDelegates: ccm_l10n.AppLocalizations.localizationsDelegates,\n      supportedLocales: ccm_l10n.AppLocalizations.supportedLocales,\n      home:"));
}

const PUB: &str = "name: demo_app\ndependencies:\n  flutter:\n    sdk: flutter\n";

fn ctx(pubspec: &str, files: &[(&str, &str)]) -> HostContext {
    let map: HashMap<String, String> = files.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    let norm = |p: &Path| p.to_string_lossy().replace('\\', "/");
    host_context_with(Path::new("/p"), pubspec, &|f| map.contains_key(&norm(f)), &|f| map.get(&norm(f)).cloned())
}

#[test]
fn riverpod_from_dependencies() {
    assert_eq!(ctx(&format!("{PUB}  flutter_riverpod: ^3.4.3\n"), &[]), HostContext { riverpod: Some("package:flutter_riverpod/flutter_riverpod.dart".into()), l10n: None });
    assert_eq!(ctx(&format!("{PUB}  flutter_riverpod: 2.6.1\n  hooks_riverpod: 2.6.1\n"), &[]).riverpod.as_deref(), Some("package:hooks_riverpod/hooks_riverpod.dart"));
    assert_eq!(ctx(&format!("{PUB}  riverpod: ^2.0.0\ndev_dependencies:\n  flutter_riverpod: any\n"), &[]), HostContext::default());
}

#[test]
fn l10n_variants() {
    let yaml = "arb-dir: lib/languages\noutput-localization-file: app_localizations.dart\noutput-class: Tr\noutput-dir: lib/languages\n#synthetic-package: false\n";
    let hit = ctx(PUB, &[("/p/l10n.yaml", yaml), ("/p/lib/languages/app_localizations.dart", "")]);
    assert_eq!(hit.l10n, Some(L10n { uri: "package:demo_app/languages/app_localizations.dart".into(), class_name: "Tr".into() }));
    assert_eq!(ctx(PUB, &[("/p/l10n.yaml", yaml)]), HostContext::default());
    let generated = ctx(&format!("{PUB}flutter:\n  generate: true\n"), &[("/p/l10n.yaml", "template-arb-file: app_en.arb\n")]);
    assert_eq!(generated.l10n, Some(L10n { uri: "package:demo_app/l10n/app_localizations.dart".into(), class_name: "AppLocalizations".into() }));
    let synthetic = ctx(PUB, &[("/p/l10n.yaml", "synthetic-package: true\n"), ("/p/.dart_tool/flutter_gen/gen_l10n/app_localizations.dart", "")]);
    assert_eq!(synthetic.l10n, Some(L10n { uri: "package:flutter_gen/gen_l10n/app_localizations.dart".into(), class_name: "AppLocalizations".into() }));
    assert_eq!(ctx(PUB, &[("/p/l10n.yaml", "output-dir: gen\n"), ("/p/gen/app_localizations.dart", "")]), HostContext::default());
    assert_eq!(ctx(PUB, &[("/p/l10n.yaml", "output-class: 'Bad Name'\n"), ("/p/lib/l10n/app_localizations.dart", "")]), HostContext::default());
    assert_eq!(ctx(PUB, &[("/p/l10n.yaml", "output-dir: lib/../x\n")]), HostContext::default());
}

#[test]
fn write_host_only_when_changed() {
    let root = std::env::temp_dir().join(format!("fwp-host-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let (file, changed) = write_host(&root, "a").unwrap();
    assert!(changed);
    assert_eq!(file, root.join(".dart_tool").join("ccm_preview").join("main.dart"));
    let mtime = std::fs::metadata(&file).unwrap().modified().unwrap();
    assert!(!write_host(&root, "a").unwrap().1);
    assert_eq!(std::fs::metadata(&file).unwrap().modified().unwrap(), mtime);
    assert!(write_host(&root, "b").unwrap().1);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "b");
    let _ = std::fs::remove_dir_all(&root);
}
