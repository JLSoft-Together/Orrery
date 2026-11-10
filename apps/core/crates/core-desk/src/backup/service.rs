use super::format::{self, BackupFile, FILE_SECTION, MAX_BACKUP_BYTES};
use crate::{js, util};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub const BACKUPS_DIR: &str = "backups";
const PENDING_FILE: &str = "pending-import.json";
const KEEP_AUTO: usize = 3;
const PREVIEW_TTL_MS: f64 = 10.0 * 60_000.0;
const MAX_READ: u64 = 4 * 1024 * 1024;

struct Staged {
    token: String,
    backup: BackupFile,
    until: f64,
}

pub struct BackupService {
    data_dir: PathBuf,
    dir: PathBuf,
    app: String,
    platform: String,
    staged: Mutex<Option<Staged>>,
}

pub fn platform_name() -> &'static str {
    if cfg!(windows) {
        "win32"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    }
}

impl BackupService {
    pub fn new(data_dir: &Path, app: &str, platform: &str) -> Self {
        BackupService {
            data_dir: data_dir.to_path_buf(),
            dir: data_dir.join(BACKUPS_DIR),
            app: app.to_string(),
            platform: platform.to_string(),
            staged: Mutex::new(None),
        }
    }

    pub fn read_files(&self) -> Value {
        let mut out = serde_json::Map::new();
        let mut read = |rel: &str| {
            let file = self.data_dir.join(rel);
            if let Ok(meta) = fs::metadata(&file) {
                if meta.len() <= MAX_READ {
                    if let Ok(bytes) = fs::read(&file) {
                        out.insert(rel.to_string(), Value::String(String::from_utf8_lossy(&bytes).into_owned()));
                    }
                }
            }
        };
        for (rel, _) in FILE_SECTION {
            read(rel);
        }
        for n in crate::fs_tree::read_names(&self.data_dir.join("notes")) {
            let rel = format!("notes/{n}");
            if format::is_note_file(&rel) {
                read(&rel);
            }
        }
        Value::Object(out)
    }

    pub fn snapshot(&self, local: &Value, github: &Value) -> BackupFile {
        format::build_backup(&self.app, &self.platform, util::now_ms(), local, &self.read_files(), github)
    }

    pub fn write_auto(&self, kind: &str, tag: &str, local: &Value, github: &Value) -> Option<PathBuf> {
        let safe: String = tag
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') { c.to_string() } else { "_".repeat(c.len_utf16()) })
            .collect();
        let safe = js::slice(&safe, 60);
        let file = self.dir.join(format!("{kind}-{safe}.json"));
        util::write_atomic(&file, &js::stringify_indent(&self.snapshot(local, github).to_value(), " ")).ok()?;
        let entries: Vec<(String, f64)> = fs::read_dir(&self.dir)
            .ok()?
            .filter_map(|e| e.ok())
            .map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                let mtime = fs::metadata(e.path()).map(|m| util::mtime_ms(&m)).unwrap_or(0.0);
                (name, mtime)
            })
            .collect();
        for name in format::prune_list(&entries, &format!("{kind}-"), KEEP_AUTO) {
            let _ = fs::remove_file(self.dir.join(name));
        }
        Some(file)
    }

    pub fn preview(&self, text: &str, file_name: &str) -> Value {
        if js::len(text) > MAX_BACKUP_BYTES {
            return json!({"ok": false, "error": "too-large"});
        }
        let Some(backup) = format::parse_backup(text) else {
            return json!({"ok": false, "error": "invalid"});
        };
        let token = util::uuid();
        let exists = |p: &str| Path::new(p).exists();
        let out = json!({
            "ok": true,
            "token": token,
            "file": util::basename(file_name),
            "app": backup.app,
            "platform": backup.platform,
            "createdAt": backup.created_at,
            "sections": format::sections_of(&backup, Some(&exists)),
            "github": backup.github.iter().map(|g| g.login.clone()).collect::<Vec<_>>(),
        });
        *self.staged.lock().unwrap() = Some(Staged { token, backup, until: util::now_ms() + PREVIEW_TTL_MS });
        out
    }

    pub fn staged_sections(&self, token: &str) -> Option<BackupFile> {
        let s = self.staged.lock().unwrap();
        s.as_ref().filter(|s| s.token == token && s.until > util::now_ms()).map(|s| s.backup.clone())
    }

    pub fn stage(&self, token: &str, ids: &[String], current_local: &Value, github: &Value) -> Value {
        let Some(backup) = self.staged_sections(token) else {
            return json!({"ok": false, "error": "expired"});
        };
        let picked = format::pick_sections(&backup, ids);
        let tag = util::iso_string(util::now_ms()).replace([':', '.'], "-");
        if self.write_auto("pre-import", &tag, current_local, github).is_none() {
            return json!({"ok": false, "error": "write"});
        }
        if !picked.files.is_empty() {
            let body = js::stringify(&json!({"files": format::pairs_to_value(&picked.files)}));
            if util::write_atomic(&self.dir.join(PENDING_FILE), &body).is_err() {
                return json!({"ok": false, "error": "write"});
            }
        }
        *self.staged.lock().unwrap() = None;
        json!({"ok": true, "local": format::pairs_to_value(&picked.local), "github": picked.github})
    }

    pub fn apply_pending(&self) -> usize {
        let pending = self.dir.join(PENDING_FILE);
        let Ok(text) = fs::read(&pending) else { return 0 };
        let Ok(raw) = serde_json::from_slice::<Value>(&text) else { return 0 };
        let files_raw = match &raw {
            Value::Null => return 0,
            Value::Object(o) => o.get("files").cloned().unwrap_or(Value::Null),
            _ => Value::Null,
        };
        let doc = json!({"format": "orrery-backup", "version": 1, "files": files_raw});
        let files = format::parse_backup(&js::stringify(&doc)).map(|b| b.files).unwrap_or_default();
        let mut n = 0;
        for (rel, text) in files {
            let mut file = self.data_dir.clone();
            for part in rel.split('/') {
                file.push(part);
            }
            if util::write_atomic(&file, &text).is_ok() {
                n += 1;
            }
        }
        let _ = fs::remove_file(&pending);
        n
    }
}
