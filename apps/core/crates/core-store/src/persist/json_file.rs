use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

pub struct JsonFile {
    path: PathBuf,
    delay_ms: i64,
    due: Option<i64>,
}

pub fn read_json(path: &Path) -> Option<Value> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_str(&String::from_utf8_lossy(&bytes)).ok()
}

pub fn write_json(path: &Path, value: &Value, pretty: bool) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    let text = if pretty { serde_json::to_string_pretty(value) } else { serde_json::to_string(value) }.map_err(std::io::Error::other)?;
    fs::write(&tmp, text)?;
    fs::rename(&tmp, path)
}

pub fn file_label(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

impl JsonFile {
    pub fn new(path: impl Into<PathBuf>, delay_ms: i64) -> Self {
        Self { path: path.into(), delay_ms, due: None }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn read(&self) -> Option<Value> {
        read_json(&self.path)
    }

    pub fn schedule(&mut self, now: i64) {
        if self.due.is_none() {
            self.due = Some(now + self.delay_ms);
        }
    }

    pub fn has_pending(&self) -> bool {
        self.due.is_some()
    }

    pub fn next_due(&self) -> Option<i64> {
        self.due
    }

    pub fn poll(&mut self, now: i64, serialize: impl FnOnce() -> Value) -> bool {
        match self.due {
            Some(due) if due <= now => self.flush(serialize),
            _ => false,
        }
    }

    pub fn flush_pending(&mut self, serialize: impl FnOnce() -> Value) -> bool {
        if self.due.is_none() {
            return false;
        }
        self.flush(serialize)
    }

    pub fn flush(&mut self, serialize: impl FnOnce() -> Value) -> bool {
        self.due = None;
        match write_json(&self.path, &serialize(), false) {
            Ok(()) => true,
            Err(err) => {
                eprintln!("[collector] cannot save {}: {err}", file_label(&self.path));
                false
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn temp_dir(name: &str) -> PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let d = std::env::temp_dir().join(format!("ccm-store-{name}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = fs::remove_dir_all(&d);
    d
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn debounces_and_writes_atomically() {
        let d = temp_dir("json-file");
        let file = d.join("nested").join("x.json");
        let mut f = JsonFile::new(&file, 500);
        assert_eq!(f.read(), None);
        f.schedule(1000);
        f.schedule(1200);
        assert_eq!(f.next_due(), Some(1500));
        assert!(!f.poll(1499, || json!({"a": 1})));
        assert!(f.poll(1500, || json!({"a": 1})));
        assert!(!f.has_pending());
        assert_eq!(f.read(), Some(json!({"a": 1})));
        assert_eq!(fs::read_to_string(&file).unwrap(), "{\"a\":1}");
        assert!(!d.join("nested").join("x.json.tmp").exists());
        assert!(!f.flush_pending(|| json!(2)));
        assert!(f.flush(|| json!(2)));
        assert_eq!(f.read(), Some(json!(2)));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn unreadable_files_mean_no_state() {
        let d = temp_dir("json-bad");
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("bad.json"), "{nope").unwrap();
        assert_eq!(JsonFile::new(d.join("bad.json"), 1).read(), None);
        let _ = fs::remove_dir_all(&d);
    }
}
