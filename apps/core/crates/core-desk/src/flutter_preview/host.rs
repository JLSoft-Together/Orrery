use super::project::Entry;
use crate::run::flutter::detect::{parse_pubspec, yaml_keys};
use std::path::Path;

pub const HOST_DIR: &str = ".dart_tool/ccm_preview";
pub const HOST_FILE: &str = "main.dart";
pub const SHOW_EXTENSION: &str = "ext.ccm.show";
pub const ERROR_MARKER: &str = "ccm.error ";
const ERROR_CAP: usize = 400;

#[derive(Debug, Clone, PartialEq)]
pub struct PubspecPackage {
    pub name: String,
    pub flutter: bool,
}

fn package_name_ok(name: &str) -> bool {
    let mut c = name.chars();
    matches!(c.next(), Some(f) if f.is_ascii_lowercase() || f == '_') && c.all(|x| x.is_ascii_lowercase() || x.is_ascii_digit() || x == '_') && !name.ends_with('\n')
}

pub fn read_package(pubspec: &str) -> Option<PubspecPackage> {
    let info = parse_pubspec(pubspec);
    package_name_ok(&info.name).then_some(PubspecPackage { name: info.name, flutter: info.flutter })
}

fn dart_string(s: &str) -> String {
    format!("'{}'", s.replace('\\', "\\\\").replace('\'', "\\'").replace('$', "\\$"))
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct L10n {
    pub uri: String,
    pub class_name: String,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct HostContext {
    pub riverpod: Option<String>,
    pub l10n: Option<L10n>,
}

const RIVERPOD: [(&str, &str); 2] = [("hooks_riverpod", "package:hooks_riverpod/hooks_riverpod.dart"), ("flutter_riverpod", "package:flutter_riverpod/flutter_riverpod.dart")];

fn dart_ident(s: &str) -> bool {
    let mut c = s.chars();
    matches!(c.next(), Some(f) if f.is_ascii_alphabetic() || f == '_' || f == '$') && c.all(|x| x.is_ascii_alphanumeric() || x == '_' || x == '$')
}

fn l10n_file(s: &str) -> bool {
    s.strip_suffix(".dart").is_some_and(|stem| !stem.is_empty() && stem.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'))
}

fn l10n_dir(s: &str) -> bool {
    s.split('/').enumerate().all(|(i, p)| if i == 0 { p == "lib" } else { !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') })
}

fn is_file(p: &Path) -> bool {
    std::fs::metadata(p).map(|m| m.is_file()).unwrap_or(false)
}

pub fn host_context(root: &Path, pubspec: &str) -> HostContext {
    host_context_with(root, pubspec, &is_file, &|f| std::fs::read_to_string(f).ok())
}

pub fn host_context_with(root: &Path, pubspec: &str, exists: &dyn Fn(&Path) -> bool, read_text: &dyn Fn(&Path) -> Option<String>) -> HostContext {
    let keys = yaml_keys(pubspec);
    let mut ctx = HostContext::default();
    if let Some((_, uri)) = RIVERPOD.iter().find(|(dep, _)| keys.contains_key(&format!("dependencies.{dep}"))) {
        ctx.riverpod = Some(uri.to_string());
    }
    let Some(yaml) = read_text(&root.join("l10n.yaml")) else { return ctx };
    let l10n = yaml_keys(&yaml);
    let pick = |k: &str, d: &str| l10n.get(k).filter(|v| !v.is_empty()).cloned().unwrap_or_else(|| d.to_string());
    let file = pick("output-localization-file", "app_localizations.dart");
    let class_name = pick("output-class", "AppLocalizations");
    if !l10n_file(&file) || !dart_ident(&class_name) {
        return ctx;
    }
    let generate = keys.get("flutter.generate").is_some_and(|v| v == "true");
    if l10n.get("synthetic-package").is_some_and(|v| v == "true") {
        if generate || exists(&root.join(".dart_tool").join("flutter_gen").join("gen_l10n").join(&file)) {
            ctx.l10n = Some(L10n { uri: format!("package:flutter_gen/gen_l10n/{file}"), class_name });
        }
        return ctx;
    }
    let raw = l10n.get("output-dir").filter(|v| !v.is_empty()).or_else(|| l10n.get("arb-dir").filter(|v| !v.is_empty())).cloned().unwrap_or_else(|| "lib/l10n".to_string());
    let dir = raw.replace('\\', "/");
    let dir = dir.strip_prefix("./").unwrap_or(&dir).trim_end_matches('/').to_string();
    let Some(pkg) = read_package(pubspec).map(|p| p.name) else { return ctx };
    if !l10n_dir(&dir) {
        return ctx;
    }
    let sub = if dir == "lib" { String::new() } else { format!("{}/", &dir["lib/".len()..]) };
    if generate || exists(&root.join(&dir).join(&file)) {
        ctx.l10n = Some(L10n { uri: format!("package:{pkg}/{sub}{file}"), class_name });
    }
    ctx
}

pub fn host_source(pkg: &str, entries: &[Entry], ctx: &HostContext) -> String {
    let usable: Vec<&Entry> = entries.iter().filter(|e| e.hit.skipped.is_none() && !e.hit.call.is_empty()).collect();
    let mut files: Vec<&str> = Vec::new();
    for e in &usable {
        if !files.contains(&e.file.as_str()) {
            files.push(&e.file);
        }
    }
    let alias = |f: &str| format!("p{}", files.iter().position(|x| *x == f).unwrap_or(0));
    let imports: Vec<String> = files.iter().map(|f| format!("import {} as {};", dart_string(&format!("package:{pkg}/{}", &f["lib/".len()..])), alias(f))).collect();
    let rows: Vec<String> = usable.iter().map(|e| format!("      {}: () => {}.{},", dart_string(&e.id), alias(&e.file), e.hit.call)).collect();
    let mut extra: Vec<String> = Vec::new();
    if let Some(r) = &ctx.riverpod {
        extra.push(format!("import {} as ccm_riverpod;", dart_string(r)));
    }
    if let Some(l) = &ctx.l10n {
        extra.push(format!("import {} as ccm_l10n;", dart_string(&l.uri)));
    }
    let shown = if ctx.riverpod.is_some() { "ccm_riverpod.ProviderScope(key: ValueKey<String?>(id), child: build())" } else { "build()" };
    let l10n = ctx.l10n.as_ref().map_or(String::new(), |l| {
        format!("\n      localizationsDelegates: ccm_l10n.{0}.localizationsDelegates,\n      supportedLocales: ccm_l10n.{0}.supportedLocales,", l.class_name)
    });
    let all_imports: Vec<String> = extra.iter().chain(imports.iter()).cloned().collect();
    let imports_nl = if all_imports.is_empty() { "" } else { "\n" };
    let rows_nl = if rows.is_empty() { "" } else { "\n" };
    HOST_TEMPLATE
        .replace("@@IMPORTS@@", &format!("{}{imports_nl}", all_imports.join("\n")))
        .replace("@@ROWS@@", &format!("{}{rows_nl}", rows.join("\n")))
        .replace("@@ERROR_CAP@@", &ERROR_CAP.to_string())
        .replace("@@MARKER@@", &dart_string(ERROR_MARKER))
        .replace("@@SHOW@@", SHOW_EXTENSION)
        .replace("@@L10N@@", &l10n)
        .replace("@@SHOWN@@", shown)
}

pub fn write_host(root: &Path, source: &str) -> std::io::Result<(std::path::PathBuf, bool)> {
    let dir = root.join(HOST_DIR);
    let file = dir.join(HOST_FILE);
    if std::fs::read_to_string(&file).ok().as_deref() == Some(source) {
        return Ok((file, false));
    }
    std::fs::create_dir_all(&dir)?;
    std::fs::write(&file, source)?;
    Ok((file, true))
}

const HOST_TEMPLATE: &str = include_str!("host_template.dart");
