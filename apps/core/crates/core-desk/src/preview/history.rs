use super::files::{random_hex, rename_retry};
use super::paths::root_key;
use crate::util::mtime_ms;
use regex::Regex;
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

pub const LH_MAX_BYTES: u64 = 2 * 1024 * 1024;
pub const LH_KEEP: usize = 50;
pub const LH_MAX_AGE_MS: f64 = 7.0 * 24.0 * 60.0 * 60.0 * 1000.0;
pub const NOTES_MAX_BYTES: usize = 1024 * 1024;

pub fn sha1_hex(s: &str) -> String {
    Sha1::digest(s.as_bytes()).iter().map(|b| format!("{b:02x}")).collect()
}

fn id_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^([0-9]{13})(?:-([0-9]{1,3}))?$").expect("regex"))
}

pub fn is_version_id(id: &Value) -> bool {
    id.as_str().map(|s| id_re().is_match(s)).unwrap_or(false)
}

fn time_of(id: &str) -> f64 {
    id_re().captures(id).and_then(|c| c[1].parse().ok()).unwrap_or(0.0)
}

fn seq_of(id: &str) -> f64 {
    id_re().captures(id).and_then(|c| c.get(2).map(|m| m.as_str().parse().unwrap_or(0.0))).unwrap_or(0.0)
}

pub struct LocalHistory {
    data_dir: PathBuf,
    lock: Mutex<()>,
    swept: AtomicBool,
}

impl LocalHistory {
    pub fn new(data_dir: PathBuf) -> Self {
        LocalHistory { data_dir, lock: Mutex::new(()), swept: AtomicBool::new(false) }
    }

    pub fn dir_of(&self, file: &str) -> PathBuf {
        self.data_dir.join("history").join(sha1_hex(&root_key(file)))
    }

    pub fn record(&self, file: &str, previous: &str) {
        if previous.contains('\0') || previous.len() as u64 > LH_MAX_BYTES {
            return;
        }
        let _g = self.lock.lock().unwrap();
        let dir = self.dir_of(file);
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        let now = crate::util::now_ms().floor();
        let stamp = format!("{:013}", now as u64);
        let names = crate::fs_tree::read_names(&dir);
        let mut id = stamp.clone();
        let mut n = 1;
        while names.iter().any(|x| *x == format!("{id}.txt")) && n < 1000 {
            id = format!("{stamp}-{n}");
            n += 1;
        }
        let target = dir.join(format!("{id}.txt"));
        let ok = std::fs::OpenOptions::new().write(true).create_new(true).open(&target).and_then(|mut f| std::io::Write::write_all(&mut f, previous.as_bytes()));
        if ok.is_err() {
            return;
        }
        self.prune(&dir, now);
        if !self.swept.swap(true, Ordering::SeqCst) {
            self.sweep(now);
        }
    }

    fn ids(&self, dir: &Path) -> Vec<String> {
        let mut ids: Vec<String> = crate::fs_tree::read_names(dir)
            .into_iter()
            .filter_map(|n| n.strip_suffix(".txt").map(String::from))
            .filter(|id| id_re().is_match(id))
            .collect();
        ids.sort_by(|a, b| (time_of(b), seq_of(b)).partial_cmp(&(time_of(a), seq_of(a))).unwrap_or(std::cmp::Ordering::Equal));
        ids
    }

    fn prune(&self, dir: &Path, now: f64) -> usize {
        let ids = self.ids(dir);
        let mut kept = 0;
        for (i, id) in ids.iter().enumerate() {
            if i >= LH_KEEP || now - time_of(id) > LH_MAX_AGE_MS {
                let _ = std::fs::remove_file(dir.join(format!("{id}.txt")));
            } else {
                kept += 1;
            }
        }
        kept
    }

    fn sweep(&self, now: f64) {
        let base = self.data_dir.join("history");
        let hex = Regex::new(r"^[0-9a-f]{40}$").expect("regex");
        for d in crate::fs_tree::read_names(&base) {
            if !hex.is_match(&d) {
                continue;
            }
            let dir = base.join(&d);
            if self.prune(&dir, now) == 0 {
                let _ = std::fs::remove_dir(&dir);
            }
        }
    }

    pub fn list(&self, file: &str) -> Value {
        let dir = self.dir_of(file);
        let now = crate::util::now_ms();
        let mut out = Vec::new();
        for id in self.ids(&dir) {
            if now - time_of(&id) > LH_MAX_AGE_MS || out.len() >= LH_KEEP {
                continue;
            }
            if let Some(m) = std::fs::metadata(dir.join(format!("{id}.txt"))).ok().filter(|m| m.is_file()) {
                out.push(json!({"id": id, "time": time_of(&id) as i64, "size": m.len()}));
            }
        }
        json!({"ok": true, "versions": out})
    }

    pub fn read(&self, file: &str, id: &Value) -> Value {
        if !is_version_id(id) {
            return json!({"ok": false, "error": "denied"});
        }
        let id = id.as_str().unwrap_or("");
        let target = self.dir_of(file).join(format!("{id}.txt"));
        let Some(m) = std::fs::metadata(&target).ok().filter(|m| m.is_file()) else { return json!({"ok": false, "error": "missing"}) };
        if m.len() > LH_MAX_BYTES {
            return json!({"ok": false, "error": "failed"});
        }
        match std::fs::read(&target) {
            Ok(b) => json!({"ok": true, "text": String::from_utf8_lossy(&b), "time": time_of(id) as i64}),
            Err(_) => json!({"ok": false, "error": "failed"}),
        }
    }
}

pub struct NotesStore {
    data_dir: PathBuf,
    lock: Mutex<()>,
}

impl NotesStore {
    pub fn new(data_dir: PathBuf) -> Self {
        NotesStore { data_dir, lock: Mutex::new(()) }
    }

    pub fn file_of(&self, root: &str) -> PathBuf {
        self.data_dir.join("notes").join(format!("{}.md", sha1_hex(&root_key(root))))
    }

    pub fn read(&self, root: &str) -> Value {
        let file = self.file_of(root);
        let meta = match std::fs::metadata(&file) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return json!({"ok": true, "text": "", "mtimeMs": null}),
            Err(_) => return json!({"ok": false, "error": "failed"}),
            Ok(m) => m,
        };
        if !meta.is_file() {
            return json!({"ok": false, "error": "failed"});
        }
        if meta.len() as usize > NOTES_MAX_BYTES {
            return json!({"ok": false, "error": "too-large"});
        }
        match std::fs::read(&file) {
            Ok(b) => json!({"ok": true, "text": String::from_utf8_lossy(&b), "mtimeMs": mtime_ms(&meta)}),
            Err(_) => json!({"ok": false, "error": "failed"}),
        }
    }

    pub fn write(&self, root: &str, text: &Value) -> Value {
        let Some(text) = text.as_str() else { return json!({"ok": false, "error": "denied"}) };
        if crate::js::len(text) > NOTES_MAX_BYTES || text.len() > NOTES_MAX_BYTES {
            return json!({"ok": false, "error": "too-large"});
        }
        let _g = self.lock.lock().unwrap();
        let file = self.file_of(root);
        let failed = json!({"ok": false, "error": "failed"});
        if text.is_empty() {
            return match std::fs::remove_file(&file) {
                Ok(()) => json!({"ok": true, "mtimeMs": null}),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => json!({"ok": true, "mtimeMs": null}),
                Err(_) => failed,
            };
        }
        if std::fs::create_dir_all(self.data_dir.join("notes")).is_err() {
            return failed;
        }
        let mut tmp = file.as_os_str().to_owned();
        tmp.push(format!(".{}.tmp", random_hex(4)));
        let tmp = PathBuf::from(tmp);
        if std::fs::write(&tmp, text).is_err() {
            return failed;
        }
        if rename_retry(&tmp, &file, 5).is_err() {
            let _ = std::fs::remove_file(&tmp);
            return failed;
        }
        match std::fs::metadata(&file) {
            Ok(m) => json!({"ok": true, "mtimeMs": mtime_ms(&m)}),
            Err(_) => failed,
        }
    }
}
