use super::exec::{exec_git, ExecOpts};
use crate::mcp::text::clean_note_text;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub const MAX_ITEMS: usize = 50;
pub const MAX_FILE: u64 = 256 * 1024;
const STATES: [&str; 4] = ["idle", "running", "pass", "fail"];

static STORE: Mutex<()> = Mutex::new(());
static DATA: OnceLock<PathBuf> = OnceLock::new();

pub fn set_data_dir(dir: &Path) {
    let _ = DATA.set(dir.to_path_buf());
}

pub fn data_dir() -> Option<PathBuf> {
    DATA.get().cloned()
}

fn lock() -> std::sync::MutexGuard<'static, ()> {
    STORE.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn normalize(root: &str) -> String {
    let real = fs::canonicalize(root).map(|p| crate::mcp::paths::strip_verbatim(p).to_string_lossy().into_owned()).unwrap_or_else(|_| root.to_string());
    let s = real.replace('\\', "/");
    let s = s.trim_end_matches('/');
    if cfg!(windows) {
        s.to_lowercase()
    } else {
        s.to_string()
    }
}

pub fn key(root: &str) -> String {
    let digest = Sha256::digest(normalize(root).as_bytes());
    digest.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

fn file_of(data: &Path, root: &str) -> PathBuf {
    data.join("checklists").join(format!("{}.json", key(root)))
}

fn empty() -> Value {
    json!({ "items": [], "check": { "script": "", "state": "idle", "at": 0, "head": "" }, "seq": 0 })
}

fn read(data: &Path, root: &str) -> Value {
    let path = file_of(data, root);
    let Ok(meta) = fs::metadata(&path) else { return empty() };
    if !meta.is_file() || meta.len() > MAX_FILE {
        return empty();
    }
    let doc = fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok());
    match doc {
        Some(d) if d["items"].is_array() && d["check"].is_object() => d,
        _ => empty(),
    }
}

fn write(data: &Path, root: &str, doc: &Value) -> bool {
    let path = file_of(data, root);
    let Some(parent) = path.parent() else { return false };
    if fs::create_dir_all(parent).is_err() {
        return false;
    }
    let tmp = path.with_extension("json.tmp");
    let Ok(text) = serde_json::to_string(doc) else { return false };
    fs::write(&tmp, text).and_then(|_| fs::rename(&tmp, &path)).is_ok()
}

pub fn view(doc: &Value) -> Value {
    json!({ "items": doc["items"], "check": doc["check"] })
}

pub fn list(data: &Path, root: &str) -> Value {
    let _g = lock();
    view(&read(data, root))
}

pub fn add(data: &Path, root: &str, by: &str, text: &str, now: f64) -> Result<Value, &'static str> {
    let Some(text) = clean_note_text(text) else { return Err("text") };
    let by = if by == "agent" { "agent" } else { "owner" };
    let _g = lock();
    let mut doc = read(data, root);
    let items = doc["items"].as_array_mut().ok_or("failed")?;
    if items.len() >= MAX_ITEMS {
        return Err("full");
    }
    let seq = doc["seq"].as_u64().unwrap_or(0) + 1;
    let item = json!({ "id": format!("t{seq}"), "text": text, "done": false, "by": by, "at": now });
    doc["items"].as_array_mut().ok_or("failed")?.push(item.clone());
    doc["seq"] = json!(seq);
    if write(data, root, &doc) {
        Ok(item)
    } else {
        Err("failed")
    }
}

pub fn set_done(data: &Path, root: &str, id: &str, done: Option<bool>) -> Result<Value, &'static str> {
    let _g = lock();
    let mut doc = read(data, root);
    let items = doc["items"].as_array_mut().ok_or("failed")?;
    let item = items.iter_mut().find(|i| i["id"] == json!(id)).ok_or("unknown")?;
    let next = done.unwrap_or_else(|| item["done"] != json!(true));
    item["done"] = json!(next);
    let out = item.clone();
    if write(data, root, &doc) {
        Ok(out)
    } else {
        Err("failed")
    }
}

pub fn remove(data: &Path, root: &str, id: &str, by: &str) -> Result<(), &'static str> {
    let _g = lock();
    let mut doc = read(data, root);
    let items = doc["items"].as_array_mut().ok_or("failed")?;
    let at = items.iter().position(|i| i["id"] == json!(id)).ok_or("unknown")?;
    if by == "agent" && items[at]["by"] == json!("owner") {
        return Err("owner");
    }
    items.remove(at);
    if write(data, root, &doc) {
        Ok(())
    } else {
        Err("failed")
    }
}

pub fn set_script(data: &Path, root: &str, script: &str) -> Result<(), &'static str> {
    let script = if script.trim().is_empty() { String::new() } else { clean_note_text(script).ok_or("script")? };
    let _g = lock();
    let mut doc = read(data, root);
    doc["check"]["script"] = json!(script);
    doc["check"]["state"] = json!("idle");
    doc["check"]["head"] = json!("");
    if write(data, root, &doc) {
        Ok(())
    } else {
        Err("failed")
    }
}

fn is_head(head: &str) -> bool {
    (4..=64).contains(&head.len()) && head.bytes().all(|b| b.is_ascii_hexdigit())
}

pub fn set_check(data: &Path, root: &str, state: &str, head: &str, now: f64) -> Result<(), &'static str> {
    if !STATES.contains(&state) {
        return Err("state");
    }
    if !is_head(head) {
        return Err("head");
    }
    let _g = lock();
    let mut doc = read(data, root);
    doc["check"]["state"] = json!(state);
    doc["check"]["head"] = json!(head);
    doc["check"]["at"] = json!(now);
    if write(data, root, &doc) {
        Ok(())
    } else {
        Err("failed")
    }
}

pub fn head_of(dir: &str) -> String {
    let res = exec_git(Path::new(dir), &["rev-parse", "HEAD"], &ExecOpts::timeout(10_000));
    if res.ok() {
        res.stdout.trim().to_string()
    } else {
        String::new()
    }
}

pub fn toplevel(cwd: &str) -> String {
    let res = exec_git(Path::new(cwd), &["rev-parse", "--show-toplevel"], &ExecOpts::timeout(10_000));
    let top = res.stdout.trim();
    if res.ok() && !top.is_empty() {
        top.to_string()
    } else {
        cwd.to_string()
    }
}

pub fn worktree_of(repo: &str, wanted: &str) -> Option<String> {
    let res = exec_git(Path::new(repo), &["worktree", "list", "--porcelain"], &ExecOpts::timeout(15_000));
    if !res.ok() {
        return None;
    }
    let want = normalize(wanted);
    res.stdout.lines().filter_map(|l| l.strip_prefix("worktree ")).find(|p| normalize(p) == want).map(str::to_string)
}

pub fn reply(data: &Path, root: &str, req: &Value) -> Value {
    let op = req.get("op").and_then(Value::as_str).unwrap_or("list");
    let text = |k: &str| req.get(k).and_then(Value::as_str).unwrap_or("").to_string();
    let now = crate::util::now_ms();
    let done = |r: Result<(), &'static str>| match r {
        Ok(()) => ok_list(data, root),
        Err(e) => json!({ "ok": false, "error": e }),
    };
    match op {
        "list" => ok_list(data, root),
        "add" => match add(data, root, "owner", &text("text"), now) {
            Ok(_) => ok_list(data, root),
            Err(e) => json!({ "ok": false, "error": e }),
        },
        "toggle" => match set_done(data, root, &text("id"), req.get("done").and_then(Value::as_bool)) {
            Ok(_) => ok_list(data, root),
            Err(e) => json!({ "ok": false, "error": e }),
        },
        "remove" => done(remove(data, root, &text("id"), "owner")),
        "setScript" => done(set_script(data, root, &text("script"))),
        "setCheck" => done(set_check(data, root, &text("state"), &text("head"), now)),
        _ => json!({ "ok": false, "error": "failed" }),
    }
}

fn ok_list(data: &Path, root: &str) -> Value {
    let mut out = list(data, root);
    out["ok"] = json!(true);
    out["head"] = json!(head_of(root));
    out
}

#[cfg(test)]
#[path = "checklist_tests.rs"]
mod tests;
