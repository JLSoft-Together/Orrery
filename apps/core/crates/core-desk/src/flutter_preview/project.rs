use super::classes::{class_hit, is_web_only, scan_dart_classes, widget_names, ClassDecl, FileFlags};
use super::mask::mask_dart;
use super::scan::{part_of, re, scan_dart_masked, PreviewHit};
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

pub const MAX_FILES: usize = 3000;
pub const MAX_FILE_BYTES: u64 = 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub id: String,
    pub file: String,
    pub hit: PreviewHit,
}

impl Entry {
    pub fn to_value(&self) -> Value {
        let h = &self.hit;
        let mut v = json!({"id": self.id, "file": self.file, "line": h.line, "symbol": h.symbol, "label": h.label, "call": h.call});
        if let Some(k) = h.kind {
            v["kind"] = json!(k);
        }
        if let Some(s) = h.skipped {
            v["skipped"] = json!(s);
        }
        v
    }
}

pub fn safe_rel(rel: &str) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, r"^lib/[A-Za-z0-9_\-./]+\.dart$").is_match(rel)
}

fn skip_dir(name: &str) -> bool {
    name.starts_with('.') || name == "build"
}

fn skip_file(name: &str) -> bool {
    !name.ends_with(".dart") || name.ends_with(".g.dart") || name.ends_with(".freezed.dart")
}

fn list_dart(root: &Path) -> Vec<String> {
    fn walk(root: &Path, rel: &str, out: &mut Vec<String>) {
        if out.len() >= MAX_FILES {
            return;
        }
        let Ok(rd) = std::fs::read_dir(root.join(rel)) else { return };
        let mut entries: Vec<(String, bool, bool)> = rd
            .flatten()
            .filter_map(|e| {
                let t = e.file_type().ok()?;
                Some((e.file_name().to_string_lossy().to_string(), t.is_dir(), t.is_file()))
            })
            .collect();
        entries.sort_by(|a, b| a.0.cmp(&b.0));
        for (name, is_dir, is_file) in entries {
            if out.len() >= MAX_FILES {
                return;
            }
            let child = format!("{rel}/{name}");
            if is_dir && !skip_dir(&name) {
                walk(root, &child, out);
            } else if is_file && !skip_file(&name) && safe_rel(&child) {
                out.push(child);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, "lib", &mut out);
    out
}

#[derive(Clone)]
struct FileScan {
    mtime: Option<SystemTime>,
    size: u64,
    hits: Vec<PreviewHit>,
    classes: Vec<ClassDecl>,
    part: bool,
    web: bool,
}

fn cache() -> &'static Mutex<HashMap<PathBuf, FileScan>> {
    static C: OnceLock<Mutex<HashMap<PathBuf, FileScan>>> = OnceLock::new();
    C.get_or_init(Default::default)
}

fn scan_file(abs: &Path) -> Option<FileScan> {
    let meta = std::fs::metadata(abs).ok()?;
    if meta.len() > MAX_FILE_BYTES {
        return None;
    }
    let mtime = meta.modified().ok();
    if let Some(old) = cache().lock().unwrap().get(abs) {
        if old.mtime == mtime && old.size == meta.len() {
            return Some(old.clone());
        }
    }
    let text = String::from_utf8_lossy(&std::fs::read(abs).ok()?).to_string();
    let preview = text.contains("Preview");
    let classes = text.contains("class");
    let masked = if preview || classes { mask_dart(&text) } else { String::new() };
    let scan = FileScan {
        mtime,
        size: meta.len(),
        hits: if preview { scan_dart_masked(&text, &masked) } else { Vec::new() },
        classes: if classes { scan_dart_classes(&text, &masked) } else { Vec::new() },
        part: part_of(&masked),
        web: is_web_only(&text),
    };
    let mut c = cache().lock().unwrap();
    if c.len() >= MAX_FILES * 2 {
        c.clear();
    }
    c.insert(abs.to_path_buf(), scan.clone());
    Some(scan)
}

pub fn scan_project(root: &Path) -> Vec<Entry> {
    let files: Vec<(String, FileScan)> = list_dart(root).into_iter().filter_map(|f| scan_file(&root.join(&f)).map(|s| (f, s))).collect();
    let widgets = widget_names(files.iter().flat_map(|(_, s)| s.classes.iter()));
    let mut entries = Vec::new();
    for (file, scan) in &files {
        let mut hits = scan.hits.clone();
        let flags = FileFlags { part: scan.part, web: scan.web };
        for decl in &scan.classes {
            if widgets.contains(&decl.name) {
                if let Some(h) = class_hit(decl, &flags, &scan.hits) {
                    hits.push(h);
                }
            }
        }
        hits.sort_by_key(|h| h.line);
        for hit in hits {
            entries.push(Entry { id: format!("{file}#{}", hit.symbol), file: file.clone(), hit });
        }
    }
    entries
}
