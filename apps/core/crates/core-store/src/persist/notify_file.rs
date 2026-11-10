use std::path::PathBuf;

use indexmap::IndexMap;
use serde_json::{json, Value};

use super::json_file::JsonFile;
use super::{system_clock, Clock};
use crate::extra::QuietHours;
use crate::num::JsNum;

pub const TOAST_KINDS: [&str; 10] = ["waiting", "done", "job", "loop", "stuck", "context", "limit", "idle", "sensitive", "disk"];
pub const DEFAULT_TOAST_KINDS: [(&str, bool); 10] = [
    ("waiting", true),
    ("done", false),
    ("job", true),
    ("loop", true),
    ("stuck", true),
    ("context", true),
    ("limit", true),
    ("idle", true),
    ("sensitive", true),
    ("disk", true),
];
pub const STUCK_CHOICES: [f64; 5] = [0.0, 5.0, 10.0, 15.0, 30.0];
pub const DEFAULT_STUCK_MINUTES: f64 = 10.0;
pub const IDLE_CHOICES: [f64; 6] = [0.0, 15.0, 30.0, 60.0, 120.0, 240.0];
pub const DEFAULT_IDLE_MINUTES: f64 = 60.0;

pub fn default_quiet() -> QuietHours {
    QuietHours { enabled: false, from: "22:00".into(), to: "08:00".into() }
}

pub fn is_hm(value: &Value) -> Option<&str> {
    let s = value.as_str()?;
    let b = s.as_bytes();
    let ok = b.len() == 5
        && b[2] == b':'
        && matches!((b[0], b[1]), (b'0' | b'1', b'0'..=b'9') | (b'2', b'0'..=b'3'))
        && matches!(b[3], b'0'..=b'5')
        && b[4].is_ascii_digit();
    ok.then_some(s)
}

pub fn sanitize_quiet(value: &Value) -> Option<QuietHours> {
    let enabled = value.get("enabled")?.as_bool()?;
    let from = is_hm(value.get("from")?)?;
    let to = is_hm(value.get("to")?)?;
    Some(QuietHours { enabled, from: from.into(), to: to.into() })
}

pub fn sanitize_kinds(value: &Value) -> Option<IndexMap<String, bool>> {
    let Value::Object(o) = value else { return None };
    let mut out = IndexMap::new();
    for kind in TOAST_KINDS {
        if let Some(Value::Bool(v)) = o.get(kind) {
            out.insert(kind.to_string(), *v);
        }
    }
    Some(out)
}

fn choice(value: &Value, choices: &[f64]) -> Option<f64> {
    value.as_f64().filter(|n| choices.contains(n))
}

pub fn sanitize_stuck(value: &Value) -> Option<f64> {
    choice(value, &STUCK_CHOICES)
}

pub fn sanitize_idle(value: &Value) -> Option<f64> {
    choice(value, &IDLE_CHOICES)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLocale {
    Vi,
    En,
}

impl ToastLocale {
    pub fn as_str(self) -> &'static str {
        match self {
            ToastLocale::Vi => "vi",
            ToastLocale::En => "en",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NotifyPrefs {
    pub enabled: bool,
    pub locale: ToastLocale,
    pub kinds: IndexMap<String, bool>,
    pub quiet: QuietHours,
    pub stuck_minutes: f64,
    pub idle_minutes: f64,
    pub key: String,
}

fn is_key(s: &str) -> bool {
    s.len() == 32 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

impl NotifyPrefs {
    pub fn defaults(key: String) -> Self {
        Self {
            enabled: true,
            locale: ToastLocale::Vi,
            kinds: DEFAULT_TOAST_KINDS.iter().map(|(k, v)| ((*k).to_string(), *v)).collect(),
            quiet: default_quiet(),
            stuck_minutes: DEFAULT_STUCK_MINUTES,
            idle_minutes: DEFAULT_IDLE_MINUTES,
            key,
        }
    }

    pub fn to_value(&self) -> Value {
        let kinds: serde_json::Map<String, Value> = self.kinds.iter().map(|(k, v)| (k.clone(), Value::Bool(*v))).collect();
        json!({
            "toast": self.enabled,
            "locale": self.locale.as_str(),
            "kinds": kinds,
            "quiet": { "enabled": self.quiet.enabled, "from": self.quiet.from, "to": self.quiet.to },
            "stuckMinutes": JsNum(self.stuck_minutes),
            "idleMinutes": JsNum(self.idle_minutes),
            "key": self.key,
        })
    }
}

pub struct NotifyFile {
    prefs: NotifyPrefs,
    file: Option<JsonFile>,
    clock: Clock,
}

impl NotifyFile {
    pub fn new(file: Option<PathBuf>, new_key: impl FnOnce() -> String) -> Self {
        Self::with_clock(file, new_key, system_clock())
    }

    pub fn with_clock(file: Option<PathBuf>, new_key: impl FnOnce() -> String, clock: Clock) -> Self {
        Self { prefs: NotifyPrefs::defaults(new_key()), file: file.map(|f| JsonFile::new(f, 500)), clock }
    }

    pub fn prefs(&self) -> &NotifyPrefs {
        &self.prefs
    }

    pub fn prefs_mut(&mut self) -> &mut NotifyPrefs {
        &mut self.prefs
    }

    pub fn load(&mut self) {
        let raw = self.file.as_ref().and_then(JsonFile::read).unwrap_or(Value::Null);
        match raw.get("key").and_then(Value::as_str) {
            Some(k) if is_key(k) => self.prefs.key = k.to_string(),
            _ => self.save(),
        }
        if let Some(Value::Bool(v)) = raw.get("toast") {
            self.prefs.enabled = *v;
        }
        match raw.get("locale").and_then(Value::as_str) {
            Some("en") => self.prefs.locale = ToastLocale::En,
            Some("vi") => self.prefs.locale = ToastLocale::Vi,
            _ => {}
        }
        if let Some(kinds) = raw.get("kinds").and_then(sanitize_kinds) {
            for (k, v) in kinds {
                self.prefs.kinds.insert(k, v);
            }
        }
        if let Some(q) = raw.get("quiet").and_then(sanitize_quiet) {
            self.prefs.quiet = q;
        }
        if let Some(n) = raw.get("stuckMinutes").and_then(sanitize_stuck) {
            self.prefs.stuck_minutes = n;
        }
        if let Some(n) = raw.get("idleMinutes").and_then(sanitize_idle) {
            self.prefs.idle_minutes = n;
        }
    }

    pub fn save(&mut self) {
        let now = (self.clock)();
        if let Some(f) = &mut self.file {
            f.schedule(now);
        }
    }

    pub fn next_flush_at(&self) -> Option<i64> {
        self.file.as_ref().and_then(JsonFile::next_due)
    }

    pub fn poll(&mut self, now: i64) -> bool {
        let value = self.prefs.to_value();
        self.file.as_mut().is_some_and(|f| f.poll(now, || value))
    }

    pub fn flush(&mut self) -> bool {
        let value = self.prefs.to_value();
        self.file.as_mut().is_some_and(|f| f.flush_pending(|| value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persist::json_file::temp_dir;
    use std::fs;

    const KEY: &str = "0123456789abcdef0123456789abcdef";

    #[test]
    fn sanitizers_match_the_shared_helpers() {
        assert_eq!(is_hm(&json!("07:30")), Some("07:30"));
        assert_eq!(is_hm(&json!("24:00")), None);
        assert_eq!(is_hm(&json!("7:30")), None);
        assert_eq!(is_hm(&json!("12:60")), None);
        assert_eq!(sanitize_quiet(&json!({ "enabled": true, "from": "01:00", "to": "02:00", "x": 1 })), Some(QuietHours { enabled: true, from: "01:00".into(), to: "02:00".into() }));
        assert_eq!(sanitize_quiet(&json!({ "enabled": "yes", "from": "01:00", "to": "02:00" })), None);
        assert_eq!(sanitize_kinds(&json!({ "done": true, "nope": true, "disk": 1 })).unwrap().into_iter().collect::<Vec<_>>(), [("done".to_string(), true)]);
        assert_eq!(sanitize_stuck(&json!(15)), Some(15.0));
        assert_eq!(sanitize_stuck(&json!(7)), None);
        assert_eq!(sanitize_idle(&json!(240)), Some(240.0));
        assert_eq!(sanitize_idle(&json!("60")), None);
    }

    #[test]
    fn missing_file_keeps_defaults_and_schedules_a_write_with_the_new_key() {
        let dir = temp_dir("notify");
        let file = dir.join("notify.json");
        let mut n = NotifyFile::with_clock(Some(file.clone()), || KEY.into(), Box::new(|| 1000));
        n.load();
        assert_eq!(n.prefs(), &NotifyPrefs::defaults(KEY.into()));
        assert_eq!(n.next_flush_at(), Some(1500));
        assert!(n.poll(1500));
        let saved: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(
            saved,
            json!({
                "toast": true, "locale": "vi",
                "kinds": { "waiting": true, "done": false, "job": true, "loop": true, "stuck": true, "context": true, "limit": true, "idle": true, "sensitive": true, "disk": true },
                "quiet": { "enabled": false, "from": "22:00", "to": "08:00" },
                "stuckMinutes": 10, "idleMinutes": 60, "key": KEY
            })
        );
        let mut again = NotifyFile::with_clock(Some(file), || "other".into(), Box::new(|| 0));
        again.load();
        assert_eq!(again.prefs().key, KEY);
        assert_eq!(again.next_flush_at(), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_keeps_valid_fields_and_drops_invalid_ones() {
        let dir = temp_dir("notify-odd");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("notify.json");
        fs::write(
            &file,
            json!({ "toast": false, "locale": "fr", "kinds": { "done": true, "loop": false, "bogus": true }, "quiet": { "enabled": true, "from": "9:00", "to": "10:00" }, "stuckMinutes": 5, "idleMinutes": 7, "key": "short" }).to_string(),
        )
        .unwrap();
        let mut n = NotifyFile::with_clock(Some(file), || KEY.into(), Box::new(|| 0));
        n.load();
        let p = n.prefs();
        assert!(!p.enabled && p.locale == ToastLocale::Vi);
        assert_eq!((p.kinds["done"], p.kinds["loop"], p.kinds.len()), (true, false, 10));
        assert_eq!(p.quiet, default_quiet());
        assert_eq!((p.stuck_minutes, p.idle_minutes, p.key.as_str()), (5.0, 60.0, KEY));
        assert!(n.next_flush_at().is_some());
        let _ = fs::remove_dir_all(&dir);
    }
}
