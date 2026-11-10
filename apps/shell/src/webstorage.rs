use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use rusty_leveldb::{LdbIterator, Options, DB};
use serde_json::{json, Value};

pub const EXPORT_FILE: &str = "webstorage-export.json";
pub const MARKER_FILE: &str = "webstorage-imported";
pub const PAGE_MARKER: &str = "ccm-shell.webstorage-imported";
const KEY_PREFIX: &str = "ccm.";
const MAX_ENTRIES: usize = 5000;
const MAX_BYTES: usize = 32 * 1024 * 1024;

pub type Entries = BTreeMap<String, String>;

fn bounded(map: Entries) -> Entries {
    let mut total = 0usize;
    map.into_iter()
        .take(MAX_ENTRIES)
        .filter(|(k, v)| {
            total += k.len() + v.len();
            total <= MAX_BYTES && !k.is_empty()
        })
        .collect()
}

pub fn read_export(text: &str) -> Option<Entries> {
    let v: Value = serde_json::from_str(text).ok()?;
    let local = v.get("local")?.as_object()?;
    let map = local.iter().filter_map(|(k, v)| v.as_str().filter(|_| k.starts_with(KEY_PREFIX)).map(|s| (k.clone(), s.to_owned()))).collect();
    Some(bounded(map))
}

fn decode_text(raw: &[u8]) -> Option<String> {
    let (kind, body) = raw.split_first()?;
    match kind {
        0 if body.len() % 2 == 0 => String::from_utf16(&body.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect::<Vec<_>>()).ok(),
        1 => Some(body.iter().map(|&b| char::from(b)).collect()),
        _ => None,
    }
}

pub fn chromium_entry(origin: &str, key: &[u8], value: &[u8]) -> Option<(String, String)> {
    let rest = key.strip_prefix(b"_")?.strip_prefix(origin.as_bytes())?.strip_prefix(b"\0")?;
    let name = decode_text(rest)?;
    name.starts_with(KEY_PREFIX).then(|| decode_text(value).map(|v| (name, v)))?
}

fn copy_db(src: &Path, dest: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        if name == "LOCK" || !entry.file_type()?.is_file() {
            continue;
        }
        std::fs::copy(entry.path(), dest.join(name))?;
    }
    Ok(())
}

fn clear_scratch(scratch: &Path) -> bool {
    match std::fs::symlink_metadata(scratch) {
        Err(_) => true,
        Ok(m) if m.is_dir() => std::fs::remove_dir_all(scratch).is_ok(),
        Ok(_) => std::fs::remove_file(scratch).is_ok(),
    }
}

#[cfg(unix)]
fn private_dir(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new().recursive(true).mode(0o700).create(path)
}

#[cfg(not(unix))]
fn private_dir(path: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)
}

pub fn read_leveldb(dir: &Path, origin: &str, scratch: &Path) -> Option<Entries> {
    if !dir.join("CURRENT").is_file() {
        return None;
    }
    if !clear_scratch(scratch) {
        return None;
    }
    if let Some(parent) = scratch.parent() {
        if std::fs::symlink_metadata(parent).is_ok_and(|m| m.file_type().is_symlink()) {
            return None;
        }
        private_dir(parent).ok()?;
    }
    copy_db(dir, scratch).ok()?;
    let found = (|| {
        let opts = Options { create_if_missing: false, ..Options::default() };
        let mut db = DB::open(scratch, opts).ok()?;
        let mut it = db.new_iter().ok()?;
        let mut map = Entries::new();
        while let Some((k, v)) = it.next() {
            if let Some((key, value)) = chromium_entry(origin, &k, &v) {
                map.insert(key, value);
            }
        }
        Some(bounded(map))
    })();
    clear_scratch(scratch);
    found
}

pub fn find(data_dir: &Path, user_data: Option<&Path>, origin: &str) -> Option<Entries> {
    if data_dir.join(MARKER_FILE).exists() {
        return None;
    }
    if let Some(map) = std::fs::read_to_string(data_dir.join(EXPORT_FILE)).ok().and_then(|t| read_export(&t)) {
        return Some(map).filter(|m| !m.is_empty());
    }
    let dir: PathBuf = user_data?.join("Local Storage").join("leveldb");
    let scratch = data_dir.join("webstorage-scratch").join(format!("{}", std::process::id()));
    read_leveldb(&dir, origin, &scratch).filter(|m| !m.is_empty())
}

pub fn prelude(origin: &str, entries: &Entries) -> String {
    let data = json!(entries);
    let o = json!(origin);
    let m = json!(PAGE_MARKER);
    format!("(function(){{try{{if(location.origin!=={o})return;const M={m};if(localStorage.getItem(M))return;const d={data};for(const k of Object.keys(d)){{if(localStorage.getItem(k)===null)localStorage.setItem(k,d[k])}}localStorage.setItem(M,'1')}}catch(e){{}}}})();\n")
}

pub fn with_import(data_dir: &Path, origin: &str, script: String) -> String {
    match find(data_dir, crate::legacy::electron_user_data().as_deref(), origin) {
        Some(entries) => format!("{}{script}", prelude(origin, &entries)),
        None => script,
    }
}

pub fn confirm_script() -> String {
    format!("localStorage.getItem({})==='1'", json!(PAGE_MARKER))
}

pub fn loaded(webview: &tauri::WebviewWindow, data_dir: &Path) {
    if data_dir.join(MARKER_FILE).exists() {
        return;
    }
    let webview = webview.clone();
    let data_dir = data_dir.to_path_buf();
    std::thread::spawn(move || {
        let ok = crate::hosts::eval_value(webview.as_ref(), &confirm_script(), std::time::Duration::from_secs(5));
        if ok == Some(Value::Bool(true)) {
            mark_done(&data_dir);
        }
    });
}

pub fn mark_done(data_dir: &Path) {
    let _ = std::fs::write(data_dir.join(MARKER_FILE), b"1");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("orrery-ws-test-{name}-{}-{}", std::process::id(), crate::config::random_hex(4)));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn utf16(s: &str) -> Vec<u8> {
        let mut v = vec![0u8];
        for u in s.encode_utf16() {
            v.extend_from_slice(&u.to_le_bytes());
        }
        v
    }

    fn latin1(s: &str) -> Vec<u8> {
        let mut v = vec![1u8];
        v.extend(s.bytes());
        v
    }

    fn key(origin: &str, k: Vec<u8>) -> Vec<u8> {
        let mut v = format!("_{origin}\0").into_bytes();
        v.extend(k);
        v
    }

    #[test]
    fn chromium_keys_decode_latin1_and_utf16_for_the_origin_only() {
        let o = "http://127.0.0.1:4317";
        assert_eq!(chromium_entry(o, &key(o, latin1("ccm.theme")), &latin1("dark")), Some(("ccm.theme".into(), "dark".into())));
        assert_eq!(chromium_entry(o, &key(o, utf16("ccm.tên")), &utf16("Việt")), Some(("ccm.tên".into(), "Việt".into())));
        assert!(chromium_entry(o, &key("http://127.0.0.1:5173", latin1("ccm.x")), &latin1("1")).is_none());
        assert!(chromium_entry(o, b"META:http://127.0.0.1:4317", &latin1("x")).is_none());
        assert!(chromium_entry(o, &key(o, vec![0, 1]), &latin1("x")).is_none());
        assert!(chromium_entry(o, &key(o, latin1("k")), &[9, 1]).is_none());
    }

    #[test]
    fn a_chromium_style_leveldb_is_read_from_a_copy() {
        let o = "http://127.0.0.1:4317";
        let root = tmp("ldb");
        let dir = root.join("leveldb");
        {
            let mut db = DB::open(&dir, Options::default()).unwrap();
            db.put(b"VERSION", b"1").unwrap();
            db.put(format!("META:{o}").as_bytes(), b"x").unwrap();
            db.put(&key(o, latin1("ccm.locale")), &latin1("vi")).unwrap();
            db.put(&key(o, utf16("ccm.layout")), &utf16("{\"a\":\"ồ\"}")).unwrap();
            db.put(&key("http://127.0.0.1:9", latin1("ccm.other")), &latin1("no")).unwrap();
            db.flush().unwrap();
        }
        let got = read_leveldb(&dir, o, &root.join("scratch")).unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got["ccm.locale"], "vi");
        assert_eq!(got["ccm.layout"], "{\"a\":\"ồ\"}");
        assert!(!root.join("scratch").exists());
        assert!(read_leveldb(&root.join("missing"), o, &root.join("s2")).is_none());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn only_ccm_keys_are_imported() {
        let o = "http://127.0.0.1:4317";
        assert!(chromium_entry(o, &key(o, latin1("other.theme")), &latin1("x")).is_none());
        assert!(chromium_entry(o, &key(o, latin1("ccm.ok")), &latin1("x")).is_some());
        let got = read_export(r#"{"local":{"ccm.a":"1","token":"2","ccmx":"3"}}"#).unwrap();
        assert_eq!(got.keys().collect::<Vec<_>>(), ["ccm.a"]);
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_scratch_is_not_followed() {
        let root = tmp("link");
        let victim = root.join("victim");
        std::fs::create_dir_all(&victim).unwrap();
        std::fs::write(victim.join("keep"), b"1").unwrap();
        let scratch = root.join("scratch");
        std::os::unix::fs::symlink(&victim, &scratch).unwrap();
        assert!(clear_scratch(&scratch));
        assert!(victim.join("keep").exists());
        assert!(!scratch.exists());
        let parent_link = root.join("plink");
        std::os::unix::fs::symlink(&victim, &parent_link).unwrap();
        let src = root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("CURRENT"), b"x").unwrap();
        assert!(read_leveldb(&src, "http://127.0.0.1:1", &parent_link.join("s")).is_none());
        assert!(!victim.join("s").exists());
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn the_confirmation_reads_the_page_marker() {
        assert!(confirm_script().contains(PAGE_MARKER));
    }

    #[test]
    fn the_export_file_wins_and_the_marker_stops_a_second_import() {
        let data = tmp("export");
        std::fs::write(data.join(EXPORT_FILE), r#"{"origin":"http://127.0.0.1:4317","local":{"ccm.a":"1","ccm.b":2}}"#).unwrap();
        let got = find(&data, None, "http://127.0.0.1:4317").unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got["ccm.a"], "1");
        mark_done(&data);
        assert!(find(&data, None, "http://127.0.0.1:4317").is_none());
        let _ = std::fs::remove_dir_all(data);
    }

    #[test]
    fn nothing_to_import_without_a_source() {
        let data = tmp("none");
        assert!(find(&data, None, "http://127.0.0.1:4317").is_none());
        assert!(find(&data, Some(&data.join("nope")), "http://127.0.0.1:4317").is_none());
        assert!(read_export("[]").is_none());
        let _ = std::fs::remove_dir_all(data);
    }

    #[test]
    fn the_prelude_is_guarded_by_origin_and_a_page_marker() {
        let mut m = Entries::new();
        m.insert("ccm.x".into(), "</script>\"'".into());
        let js = prelude("http://127.0.0.1:4317", &m);
        assert!(js.contains("location.origin!==\"http://127.0.0.1:4317\""));
        assert!(js.contains(PAGE_MARKER));
        assert!(js.contains("localStorage.getItem(k)===null"));
        assert!(js.trim_end().ends_with("})();"));
    }

    #[test]
    fn imports_stay_bounded() {
        let mut m = Entries::new();
        for i in 0..(MAX_ENTRIES + 10) {
            m.insert(format!("k{i:05}"), "v".into());
        }
        m.insert(String::new(), "x".into());
        assert_eq!(bounded(m).len(), MAX_ENTRIES - 1);
    }
}
