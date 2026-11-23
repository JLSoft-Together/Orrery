use crate::claude::mask::mask_hook_text;
use crate::layouts::project_key;
use crate::{arg, js, util, Desk, DeskEnv};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub const MODULE: &str = "cmdhist";
pub const DIR: &str = "cmdhist";
pub const MAX_PER_PROJECT: usize = 200;
pub const MAX_COMMAND: usize = 2000;

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub cmd: String,
    pub code: i64,
    pub ms: u64,
    pub at: u64,
}

impl Entry {
    fn to_json(&self) -> Value {
        json!({"cmd": self.cmd, "code": self.code, "ms": self.ms, "at": self.at})
    }

    fn from_json(v: &Value) -> Option<Entry> {
        let cmd = v.get("cmd")?.as_str()?.to_owned();
        let code = v.get("code")?.as_i64()?;
        let ms = v.get("ms").and_then(Value::as_u64).unwrap_or(0);
        let at = v.get("at")?.as_u64()?;
        (!cmd.trim().is_empty()).then_some(Entry { cmd, code, ms, at })
    }
}

pub fn platform() -> &'static str {
    if cfg!(windows) {
        "win32"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    }
}

pub fn clean_command(raw: &str) -> Option<String> {
    let trimmed = js::slice(raw.trim(), MAX_COMMAND);
    let masked = mask_hook_text(&trimmed, "", platform());
    let masked = masked.trim();
    (!masked.is_empty()).then(|| masked.to_owned())
}

pub fn file_for(dir: &Path, project: &str) -> PathBuf {
    let digest = Sha256::digest(project_key(project).as_bytes());
    let name: String = digest.iter().take(8).map(|b| format!("{b:02x}")).collect();
    dir.join(DIR).join(format!("{name}.json"))
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct History {
    pub items: Vec<Entry>,
}

impl History {
    pub fn load(file: &Path) -> History {
        let items = std::fs::read(file)
            .ok()
            .and_then(|b| js::parse(&String::from_utf8_lossy(&b)))
            .and_then(|v| v.get("items").and_then(Value::as_array).map(|a| a.iter().filter_map(Entry::from_json).collect::<Vec<_>>()))
            .unwrap_or_default();
        let mut h = History { items };
        h.trim();
        h
    }

    pub fn save(&self, file: &Path) -> std::io::Result<()> {
        let items: Vec<Value> = self.items.iter().map(Entry::to_json).collect();
        util::write_atomic(file, &json!({"items": items}).to_string())
    }

    fn trim(&mut self) {
        let extra = self.items.len().saturating_sub(MAX_PER_PROJECT);
        if extra > 0 {
            self.items.drain(..extra);
        }
    }

    pub fn add(&mut self, cmd: &str, code: i64, ms: u64, at: u64) -> bool {
        let Some(cmd) = clean_command(cmd) else { return false };
        let entry = Entry { cmd, code, ms, at };
        match self.items.last_mut() {
            Some(last) if last.cmd == entry.cmd => *last = entry,
            _ => self.items.push(entry),
        }
        self.trim();
        true
    }

    pub fn remove(&mut self, at: u64, cmd: &str) -> bool {
        let before = self.items.len();
        self.items.retain(|e| !(e.at == at && e.cmd == cmd));
        self.items.len() != before
    }

    pub fn to_json(&self) -> Value {
        Value::Array(self.items.iter().rev().map(Entry::to_json).collect())
    }
}

fn project_of(v: &Value) -> Option<&str> {
    v.as_str().filter(|s| !s.is_empty() && s.len() <= 4096 && crate::claude::is_absolute(s))
}

fn count(v: &Value, k: &str) -> Option<u64> {
    let f = v.get(k)?.as_f64()?;
    (f.is_finite() && f >= 0.0 && f.fract() == 0.0 && f < 9.0e15).then_some(f as u64)
}

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    let dir = env.data_dir.clone();
    let lock = Arc::new(Mutex::new(()));

    let (d, l) = (dir.clone(), lock.clone());
    desk.on(MODULE, "ccm:cmdhist-list", move |_, args| {
        let Some(project) = project_of(arg(args, 0)) else { return Ok(json!([])) };
        let _g = l.lock().unwrap();
        Ok(History::load(&file_for(&d, project)).to_json())
    });

    let (d, l) = (dir.clone(), lock.clone());
    desk.on(MODULE, "ccm:cmdhist-add", move |_, args| {
        let (Some(project), p) = (project_of(arg(args, 0)), arg(args, 1)) else { return Ok(Value::Bool(false)) };
        let (Some(cmd), Some(code)) = (p.get("cmd").and_then(Value::as_str), p.get("code").and_then(Value::as_i64)) else { return Ok(Value::Bool(false)) };
        let ms = count(p, "ms").unwrap_or(0);
        let at = count(p, "at").unwrap_or_else(|| util::now_ms() as u64);
        let _g = l.lock().unwrap();
        let file = file_for(&d, project);
        let mut h = History::load(&file);
        if !h.add(cmd, code, ms, at) {
            return Ok(Value::Bool(false));
        }
        Ok(Value::Bool(h.save(&file).is_ok()))
    });

    desk.on(MODULE, "ccm:cmdhist-remove", move |_, args| {
        let (Some(project), Some(at), Some(cmd)) = (project_of(arg(args, 0)), count(arg(args, 1), "at"), arg(args, 1).get("cmd").and_then(Value::as_str)) else {
            return Ok(Value::Bool(false));
        };
        let _g = lock.lock().unwrap();
        let file = file_for(&dir, project);
        let mut h = History::load(&file);
        if !h.remove(at, cmd) {
            return Ok(Value::Bool(false));
        }
        Ok(Value::Bool(h.save(&file).is_ok()))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ccm-cmdhist-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn keeps_at_most_200_and_drops_the_oldest() {
        let mut h = History::default();
        for i in 0..230 {
            assert!(h.add(&format!("echo {i}"), 0, 5, i));
        }
        assert_eq!(h.items.len(), MAX_PER_PROJECT);
        assert_eq!(h.items[0].cmd, "echo 30");
        assert_eq!(h.items.last().unwrap().cmd, "echo 229");
        assert_eq!(h.to_json()[0]["cmd"], "echo 229");
    }

    #[test]
    fn a_repeat_of_the_last_command_replaces_it_instead_of_stacking() {
        let mut h = History::default();
        h.add("cargo test", 1, 100, 1);
        h.add("cargo test", 0, 200, 2);
        assert_eq!(h.items, vec![Entry { cmd: "cargo test".into(), code: 0, ms: 200, at: 2 }]);
        h.add("ls", 0, 1, 3);
        h.add("cargo test", 0, 1, 4);
        assert_eq!(h.items.len(), 3);
    }

    #[test]
    fn secrets_are_masked_before_they_are_stored() {
        let mut h = History::default();
        h.add("curl -H 'Authorization: Bearer abcdefghijklmnop' https://u:p4ss@h.io/x?token=abc", 0, 1, 1);
        h.add("export API_KEY=sk-live-123456 && run --password hunter2", 0, 1, 2);
        let text = format!("{:?}", h.items);
        for leak in ["abcdefghijklmnop", "p4ss", "token=abc", "sk-live-123456", "hunter2"] {
            assert!(!text.contains(leak), "{leak} leaked: {text}");
        }
        assert!(h.items[1].cmd.contains("API_KEY="));
    }

    #[test]
    fn blank_commands_and_long_commands_are_handled() {
        let mut h = History::default();
        assert!(!h.add("   \n ", 0, 1, 1));
        assert!(h.add(&"go ".repeat(2000), 0, 1, 2));
        assert_eq!(h.items[0].cmd.chars().count(), MAX_COMMAND);
    }

    #[test]
    fn remove_matches_time_and_text() {
        let mut h = History::default();
        h.add("a", 0, 1, 10);
        h.add("b", 2, 1, 11);
        assert!(!h.remove(10, "b"));
        assert!(h.remove(11, "b"));
        assert_eq!(h.items.len(), 1);
    }

    #[test]
    fn saves_and_loads_per_project_files_and_survives_junk() {
        let dir = tmp("roundtrip");
        let a = file_for(&dir, if cfg!(windows) { "C:\\work\\a" } else { "/work/a" });
        let b = file_for(&dir, if cfg!(windows) { "C:\\work\\b" } else { "/work/b" });
        assert_ne!(a, b);
        let mut h = History::default();
        h.add("npm test", 1, 3000, 5);
        h.save(&a).unwrap();
        assert_eq!(History::load(&a), h);
        assert_eq!(History::load(&b), History::default());
        std::fs::write(&b, "{not json").unwrap();
        assert_eq!(History::load(&b), History::default());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn channels_add_list_and_remove() {
        let dir = tmp("channels");
        let env = DeskEnv { data_dir: dir.clone(), ..DeskEnv::default() };
        let mut desk = Desk::new();
        register(&mut desk, &env);
        let project = if cfg!(windows) { "C:\\work\\app" } else { "/work/app" };
        let add = |cmd: &str, code: i64, at: u64| desk.dispatch("ccm:cmdhist-add", json!([project, {"cmd": cmd, "code": code, "ms": 12, "at": at}])).unwrap();
        assert_eq!(add("git status", 0, 1), json!(true));
        assert_eq!(add("npm run build", 2, 2), json!(true));
        assert_eq!(add("  ", 0, 3), json!(false));
        let list = desk.dispatch("ccm:cmdhist-list", json!([project])).unwrap();
        assert_eq!(list[0]["cmd"], "npm run build");
        assert_eq!(list[0]["code"], 2);
        assert_eq!(list[1]["cmd"], "git status");
        assert_eq!(desk.dispatch("ccm:cmdhist-remove", json!([project, {"at": 2, "cmd": "npm run build"}])).unwrap(), json!(true));
        assert_eq!(desk.dispatch("ccm:cmdhist-list", json!([project])).unwrap().as_array().unwrap().len(), 1);
        assert_eq!(desk.dispatch("ccm:cmdhist-list", json!(["relative"])).unwrap(), json!([]));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
