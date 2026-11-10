use super::axml::{manifest_summary, parse_axml, parse_proto_xml};
use super::zip::{inflate_exact, inflate_head, raw_data, read_entries, ApkError, Entry};
use crate::util::{basename, mtime_ms};
use serde_json::{json, Map, Value};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::fs::File;

pub const TOP: usize = 20;
pub const DIFF_FILES: usize = 100;
const MANIFEST_MAX: u64 = 8 * 1024 * 1024;
const DEX_HEAD_COMPRESSED: u64 = 4096;
const DEX_METHOD_IDS: usize = 0x58;
const GROUP_ORDER: [&str; 8] = ["dex", "res", "arsc", "assets", "lib", "module", "meta", "other"];

pub type SnapshotFile = (String, u64, u64);

#[derive(Clone)]
pub struct Snapshot {
    pub report: Value,
    pub analyzed_at: f64,
    pub files: Vec<SnapshotFile>,
}

pub fn is_apk_name(p: &str) -> bool {
    let l = p.to_ascii_lowercase();
    l.ends_with(".apk") || l.ends_with(".aab")
}

#[derive(Clone)]
pub struct Group {
    pub id: String,
    pub kind: &'static str,
    pub sub: Option<String>,
    pub compressed: u64,
    pub size: u64,
    pub files: u64,
}

impl Group {
    pub fn to_json(&self) -> Value {
        json!({"id": self.id, "kind": self.kind, "sub": self.sub, "compressed": self.compressed, "size": self.size, "files": self.files})
    }
}

fn kind_static(k: &str) -> &'static str {
    GROUP_ORDER.iter().copied().find(|g| *g == k).unwrap_or("other")
}

fn order(kind: &str) -> usize {
    GROUP_ORDER.iter().position(|g| *g == kind).unwrap_or(usize::MAX)
}

pub fn group_of(name: &str, aab: bool) -> (String, &'static str, Option<String>) {
    let g = |kind: &'static str, sub: Option<String>| (sub.as_ref().map(|s| format!("{kind}:{s}")).unwrap_or_else(|| kind.to_string()), kind, sub);
    if name.starts_with("META-INF/") || name.starts_with("BUNDLE-METADATA/") {
        return g("meta", None);
    }
    let mut rest = name;
    if aab {
        let Some(slash) = name.find('/') else { return g("other", None) };
        let module = &name[..slash];
        if module != "base" {
            return g("module", Some(module.to_string()));
        }
        rest = &name[slash + 1..];
        if rest.starts_with("dex/") {
            return g("dex", None);
        }
        if rest == "resources.pb" {
            return g("arsc", None);
        }
    } else {
        if rest.strip_prefix("classes").and_then(|r| r.strip_suffix(".dex")).is_some_and(|d| d.bytes().all(|b| b.is_ascii_digit())) {
            return g("dex", None);
        }
        if rest == "resources.arsc" {
            return g("arsc", None);
        }
    }
    if rest.starts_with("res/") {
        return g("res", None);
    }
    if rest.starts_with("assets/") {
        return g("assets", None);
    }
    if let Some(r) = rest.strip_prefix("lib/") {
        if let Some(i) = r.find('/') {
            if i > 0 {
                return g("lib", Some(r[..i].to_string()));
            }
        }
    }
    g("other", None)
}

pub fn group_files(files: &[SnapshotFile], aab: bool) -> Vec<Group> {
    let mut groups: Vec<Group> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for (name, compressed, size) in files {
        let (id, kind, sub) = group_of(name, aab);
        let i = *index.entry(id.clone()).or_insert_with(|| {
            groups.push(Group { id, kind: kind_static(kind), sub, compressed: 0, size: 0, files: 0 });
            groups.len() - 1
        });
        groups[i].compressed += compressed;
        groups[i].size += size;
        groups[i].files += 1;
    }
    groups.sort_by(|a, b| order(a.kind).cmp(&order(b.kind)).then(b.compressed.cmp(&a.compressed)));
    groups
}

pub fn top_files(files: &[SnapshotFile], n: usize) -> Vec<Value> {
    let mut sorted: Vec<&SnapshotFile> = files.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));
    sorted.into_iter().take(n).map(|(name, c, s)| json!({"name": name, "compressed": c, "size": s})).collect()
}

fn entry_data(f: &mut File, e: &Entry, file_size: u64) -> Result<Option<Vec<u8>>, ApkError> {
    if e.size > MANIFEST_MAX || e.compressed > MANIFEST_MAX || (e.method != 0 && e.method != 8) {
        return Ok(None);
    }
    let Some(raw) = raw_data(f, e, file_size, e.compressed)? else { return Ok(None) };
    Ok(if e.method == 8 { inflate_exact(&raw, e.size as usize) } else { Some(raw) })
}

fn dex_methods(f: &mut File, e: &Entry, file_size: u64) -> Result<Option<u64>, ApkError> {
    if e.method != 0 && e.method != 8 {
        return Ok(None);
    }
    let Some(raw) = raw_data(f, e, file_size, if e.method == 8 { DEX_HEAD_COMPRESSED } else { 0x70 })? else { return Ok(None) };
    let head = if e.method == 8 {
        match inflate_head(&raw) {
            Some(h) => h,
            None => return Ok(None),
        }
    } else {
        raw
    };
    if head.len() < 0x70 || &head[..4] != b"dex\n" {
        return Ok(None);
    }
    Ok(Some(u32::from_le_bytes([head[DEX_METHOD_IDS], head[DEX_METHOD_IDS + 1], head[DEX_METHOD_IDS + 2], head[DEX_METHOD_IDS + 3]]) as u64))
}

pub fn analyze_apk(file: &str, now: f64) -> Result<Snapshot, ApkError> {
    let mut f = File::open(file).map_err(|_| ApkError::ReadFailed)?;
    let meta = f.metadata().map_err(|_| ApkError::ReadFailed)?;
    if !meta.is_file() {
        return Err(ApkError::ReadFailed);
    }
    let file_size = meta.len();
    let entries: Vec<Entry> = read_entries(&mut f, file_size)?.into_iter().filter(|e| !e.name.ends_with('/')).collect();
    let aab = file.to_ascii_lowercase().ends_with(".aab") || entries.iter().any(|e| e.name == "BundleConfig.pb");
    let files: Vec<SnapshotFile> = entries.iter().map(|e| (e.name.clone(), e.compressed, e.size)).collect();
    let manifest_name = if aab { "base/manifest/AndroidManifest.xml" } else { "AndroidManifest.xml" };
    let manifest_bytes = match entries.iter().find(|e| e.name == manifest_name) {
        Some(e) => entry_data(&mut f, e, file_size)?,
        None => None,
    };
    let manifest = match manifest_bytes {
        Some(b) => manifest_summary(if aab { parse_proto_xml(&b) } else { parse_axml(&b) }.as_ref()),
        None => Value::Null,
    };
    let dex: Vec<&Entry> = entries.iter().filter(|e| group_of(&e.name, aab).1 == "dex" && e.name.ends_with(".dex")).collect();
    let mut methods: Option<u64> = None;
    for e in &dex {
        if let Some(n) = dex_methods(&mut f, e, file_size)? {
            methods = Some(methods.unwrap_or(0) + n);
        }
    }
    let report = json!({
        "path": file,
        "name": basename(file),
        "kind": if aab { "aab" } else { "apk" },
        "fileSize": file_size,
        "mtime": mtime_ms(&meta),
        "entries": files.len(),
        "compressed": files.iter().map(|f| f.1).sum::<u64>(),
        "size": files.iter().map(|f| f.2).sum::<u64>(),
        "groups": group_files(&files, aab).iter().map(Group::to_json).collect::<Vec<_>>(),
        "top": top_files(&files, TOP),
        "manifest": manifest,
        "dexMethods": methods,
        "dexFiles": dex.len(),
    });
    Ok(Snapshot { report, analyzed_at: now, files })
}

fn get<'a>(v: &'a Value, k: &str) -> &'a Value {
    v.get(k).unwrap_or(&Value::Null)
}

pub fn side_of(s: &Snapshot) -> Value {
    let r = &s.report;
    let m = get(r, "manifest");
    json!({
        "name": get(r, "name"),
        "fileSize": get(r, "fileSize"),
        "compressed": get(r, "compressed"),
        "size": get(r, "size"),
        "versionName": get(m, "versionName"),
        "versionCode": get(m, "versionCode"),
        "analyzedAt": s.analyzed_at,
    })
}

struct Delta {
    name: String,
    before: Option<(u64, u64)>,
    after: Option<(u64, u64)>,
}

impl Delta {
    fn magnitude(&self) -> u64 {
        let c = self.after.map_or(0, |a| a.0).abs_diff(self.before.map_or(0, |b| b.0));
        if c != 0 {
            return c;
        }
        self.after.map_or(0, |a| a.1).abs_diff(self.before.map_or(0, |b| b.1))
    }

    fn to_json(&self) -> Value {
        let s = |v: Option<(u64, u64)>| v.map(|(c, z)| json!({"compressed": c, "size": z})).unwrap_or(Value::Null);
        json!({"name": self.name, "before": s(self.before), "after": s(self.after)})
    }
}

fn locale_key(c: char) -> (u8, char) {
    if c.is_alphabetic() {
        (2, c.to_lowercase().next().unwrap_or(c))
    } else if c.is_ascii_digit() {
        (1, c)
    } else {
        (0, c)
    }
}

pub fn locale_cmp(a: &str, b: &str) -> Ordering {
    let primary = a.chars().map(locale_key).cmp(b.chars().map(locale_key));
    if primary != Ordering::Equal {
        return primary;
    }
    for (x, y) in a.chars().zip(b.chars()) {
        if x != y {
            return if x.is_lowercase() { Ordering::Less } else { Ordering::Greater };
        }
    }
    Ordering::Equal
}

pub fn groups_of(report: &Value) -> Vec<Group> {
    let n = |v: &Value, k: &str| v.get(k).and_then(Value::as_u64).unwrap_or(0);
    report
        .get("groups")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .map(|g| Group {
                    id: g.get("id").and_then(Value::as_str).unwrap_or("").to_string(),
                    kind: kind_static(g.get("kind").and_then(Value::as_str).unwrap_or("other")),
                    sub: g.get("sub").and_then(Value::as_str).map(str::to_string),
                    compressed: n(g, "compressed"),
                    size: n(g, "size"),
                    files: n(g, "files"),
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn compare_snapshots(before: &Snapshot, after: &Snapshot, limit: usize) -> Value {
    let old: HashMap<&str, &SnapshotFile> = before.files.iter().map(|f| (f.0.as_str(), f)).collect();
    let cur: HashMap<&str, &SnapshotFile> = after.files.iter().map(|f| (f.0.as_str(), f)).collect();
    let (mut added, mut removed, mut changed) = (Vec::new(), Vec::new(), Vec::new());
    let mut seen = std::collections::HashSet::new();
    for f in &after.files {
        if !seen.insert(f.0.as_str()) {
            continue;
        }
        let f = cur[f.0.as_str()];
        match old.get(f.0.as_str()) {
            None => added.push(Delta { name: f.0.clone(), before: None, after: Some((f.1, f.2)) }),
            Some(o) if o.1 != f.1 || o.2 != f.2 => changed.push(Delta { name: f.0.clone(), before: Some((o.1, o.2)), after: Some((f.1, f.2)) }),
            _ => {}
        }
    }
    let mut seen = std::collections::HashSet::new();
    for f in &before.files {
        if !seen.insert(f.0.as_str()) {
            continue;
        }
        if !cur.contains_key(f.0.as_str()) {
            let o = old[f.0.as_str()];
            removed.push(Delta { name: f.0.clone(), before: Some((o.1, o.2)), after: None });
        }
    }
    let by_delta = |a: &Delta, b: &Delta| b.magnitude().cmp(&a.magnitude()).then_with(|| locale_cmp(&a.name, &b.name));
    let (gb, ga) = (groups_of(&before.report), groups_of(&after.report));
    let mut ids: Vec<String> = Vec::new();
    for g in ga.iter().chain(gb.iter()) {
        if !ids.contains(&g.id) {
            ids.push(g.id.clone());
        }
    }
    let mut groups: Vec<(Group, Option<Group>, Option<Group>)> = ids
        .iter()
        .map(|id| {
            let b = gb.iter().find(|g| g.id == *id).cloned();
            let a = ga.iter().find(|g| g.id == *id).cloned();
            (a.clone().or_else(|| b.clone()).expect("group"), b, a)
        })
        .collect();
    groups.sort_by(|x, y| order(x.0.kind).cmp(&order(y.0.kind)).then(y.2.as_ref().map_or(0, |g| g.compressed).cmp(&x.2.as_ref().map_or(0, |g| g.compressed))));
    let side = |g: &Option<Group>| g.as_ref().map(|g| json!({"compressed": g.compressed, "size": g.size})).unwrap_or(Value::Null);
    let groups_json: Vec<Value> = groups.iter().map(|(g, b, a)| json!({"id": g.id, "kind": g.kind, "sub": g.sub, "before": side(b), "after": side(a)})).collect();
    let (ac, rc, cc) = (added.len(), removed.len(), changed.len());
    for list in [&mut added, &mut removed, &mut changed] {
        list.sort_by(by_delta);
        list.truncate(limit);
    }
    let mut out = Map::new();
    out.insert("before".into(), side_of(before));
    out.insert("after".into(), side_of(after));
    out.insert("groups".into(), Value::Array(groups_json));
    out.insert("added".into(), Value::Array(added.iter().map(Delta::to_json).collect()));
    out.insert("removed".into(), Value::Array(removed.iter().map(Delta::to_json).collect()));
    out.insert("changed".into(), Value::Array(changed.iter().map(Delta::to_json).collect()));
    out.insert("addedCount".into(), json!(ac));
    out.insert("removedCount".into(), json!(rc));
    out.insert("changedCount".into(), json!(cc));
    Value::Object(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_and_order() {
        assert_eq!(group_of("classes2.dex", false).1, "dex");
        assert_eq!(group_of("lib/arm64-v8a/x.so", false).0, "lib:arm64-v8a");
        assert_eq!(group_of("camera/dex/classes.dex", true).0, "module:camera");
        assert_eq!(group_of("base/resources.pb", true).1, "arsc");
        assert_eq!(group_of("kotlin/x", false).1, "other");
        let files: Vec<SnapshotFile> = vec![("a.dex".into(), 1, 1), ("classes.dex".into(), 5, 9), ("res/a".into(), 7, 7)];
        let g = group_files(&files, false);
        assert_eq!(g.iter().map(|x| x.id.as_str()).collect::<Vec<_>>(), vec!["dex", "res", "other"]);
        assert_eq!(locale_cmp("a", "B"), Ordering::Less);
        assert_eq!(locale_cmp("a", "A"), Ordering::Less);
    }
}
