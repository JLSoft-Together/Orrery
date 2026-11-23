use super::exec::{exec_file_at, exec_git, ExecOpts};
use super::worktrees::{is_branch_name, main_checkout};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

pub const FILE: &str = ".orrery/worktree.json";
pub const CLEANUP_TIMEOUT_MS: u64 = 120_000;
const MAX_CMDS: usize = 20;
const MAX_CMD_CHARS: usize = 500;
const MAX_COPY: usize = 20;
const MAX_FILE: u64 = 64 * 1024;
const DEFAULT_COPY: [&str; 2] = [".env*", "**/google-services.json"];

static STORE: Mutex<()> = Mutex::new(());

#[derive(Debug, Default, Clone, PartialEq)]
pub struct WtConfig {
    pub copy: Option<Vec<String>>,
    pub setup: Vec<String>,
    pub cleanup: Vec<String>,
    pub errors: Vec<String>,
}

fn strings(map: &Map<String, Value>, key: &str, max: usize, errors: &mut Vec<String>) -> Option<Vec<String>> {
    let v = map.get(key)?;
    let Some(list) = v.as_array() else {
        errors.push(key.to_string());
        return None;
    };
    let mut out = Vec::new();
    for item in list {
        let Some(s) = item.as_str() else {
            errors.push(key.to_string());
            return None;
        };
        let s = s.trim();
        if !s.is_empty() && out.len() < max {
            out.push(s.chars().take(MAX_CMD_CHARS).collect());
        }
    }
    Some(out)
}

pub fn parse(text: &str) -> WtConfig {
    let mut cfg = WtConfig::default();
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let Ok(Value::Object(map)) = serde_json::from_str::<Value>(text) else {
        cfg.errors.push("json".into());
        return cfg;
    };
    let mut errors = Vec::new();
    cfg.copy = strings(&map, "copy", MAX_COPY, &mut errors);
    cfg.setup = strings(&map, "setup", MAX_CMDS, &mut errors).unwrap_or_default();
    cfg.cleanup = strings(&map, "cleanup", MAX_CMDS, &mut errors).unwrap_or_default();
    errors.dedup();
    cfg.errors = errors;
    cfg
}

pub fn config_path(main: &Path) -> PathBuf {
    main.join(".orrery").join("worktree.json")
}

pub fn read(main: &Path) -> (bool, WtConfig) {
    let path = config_path(main);
    let Ok(meta) = fs::metadata(&path) else { return (false, WtConfig::default()) };
    if !meta.is_file() || meta.len() > MAX_FILE {
        return (true, WtConfig { errors: vec!["json".into()], ..Default::default() });
    }
    match fs::read_to_string(&path) {
        Ok(text) => (true, parse(&text)),
        Err(_) => (true, WtConfig { errors: vec!["json".into()], ..Default::default() }),
    }
}

pub fn commands_hash(commands: &[String]) -> String {
    let mut h = Sha256::new();
    for c in commands {
        h.update(c.as_bytes());
        h.update([0u8]);
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn store_path(data_dir: &Path) -> PathBuf {
    data_dir.join("worktree-approved.json")
}

fn load_store(data_dir: &Path) -> Map<String, Value> {
    fs::read_to_string(store_path(data_dir)).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).and_then(|v| v.as_object().cloned()).unwrap_or_default()
}

fn repo_key(repo: &str) -> String {
    let k = repo.replace('\\', "/");
    if cfg!(windows) {
        k.to_lowercase()
    } else {
        k
    }
}

pub fn is_approved(data_dir: &Path, repo: &str, kind: &str, commands: &[String]) -> bool {
    if commands.is_empty() {
        return true;
    }
    let _g = STORE.lock().unwrap_or_else(|e| e.into_inner());
    load_store(data_dir).get(&repo_key(repo)).and_then(|r| r.get(kind)).and_then(Value::as_str) == Some(commands_hash(commands).as_str())
}

pub fn approve(data_dir: &Path, repo: &str, kind: &str, commands: &[String]) -> bool {
    let _g = STORE.lock().unwrap_or_else(|e| e.into_inner());
    let mut store = load_store(data_dir);
    let entry = store.entry(repo_key(repo)).or_insert_with(|| json!({}));
    let Some(obj) = entry.as_object_mut() else { return false };
    obj.insert(kind.to_string(), json!(commands_hash(commands)));
    let path = store_path(data_dir);
    let tmp = path.with_extension("json.tmp");
    if fs::create_dir_all(data_dir).is_err() {
        return false;
    }
    let Ok(text) = serde_json::to_string_pretty(&Value::Object(store)) else { return false };
    fs::write(&tmp, text).is_ok() && fs::rename(&tmp, &path).is_ok()
}

pub fn suggest_setup(main: &Path) -> Vec<String> {
    let has = |f: &str| main.join(f).is_file();
    let mut out = Vec::new();
    if has("pnpm-lock.yaml") {
        out.push("pnpm install".to_string());
    } else if has("yarn.lock") {
        out.push("yarn install".to_string());
    } else if has("bun.lockb") || has("bun.lock") {
        out.push("bun install".to_string());
    } else if has("package-lock.json") {
        out.push("npm ci".to_string());
    } else if has("package.json") {
        out.push("npm install".to_string());
    }
    if has("pubspec.yaml") {
        out.push("flutter pub get".to_string());
    }
    out
}

pub fn suggestion_text(main: &Path, existing: &WtConfig) -> String {
    let copy: Vec<String> = existing.copy.clone().unwrap_or_else(|| DEFAULT_COPY.iter().map(|s| s.to_string()).collect());
    let setup = if existing.setup.is_empty() { suggest_setup(main) } else { existing.setup.clone() };
    let v = json!({ "copy": copy, "setup": setup, "cleanup": existing.cleanup });
    let mut text = serde_json::to_string_pretty(&v).unwrap_or_default();
    text.push('\n');
    text
}

pub fn config_reply(data_dir: &Path, dir: &str) -> Value {
    let Some(main) = main_checkout(dir) else { return json!({"ok": false, "error": "not-repo"}) };
    let (exists, cfg) = read(Path::new(&main.path));
    json!({
        "ok": true, "exists": exists, "path": config_path(Path::new(&main.path)).to_string_lossy(),
        "copy": cfg.copy, "setup": cfg.setup, "cleanup": cfg.cleanup, "errors": cfg.errors,
        "setupApproved": is_approved(data_dir, &main.path, "setup", &cfg.setup),
        "cleanupApproved": is_approved(data_dir, &main.path, "cleanup", &cfg.cleanup),
        "suggest": suggest_setup(Path::new(&main.path)),
    })
}

pub fn pending_commands(data_dir: &Path, dir: &str, kind: &str) -> Result<(String, Vec<String>, bool), &'static str> {
    if kind != "setup" && kind != "cleanup" {
        return Err("unknown");
    }
    let main = main_checkout(dir).ok_or("not-repo")?;
    let (_, cfg) = read(Path::new(&main.path));
    let commands = if kind == "setup" { cfg.setup } else { cfg.cleanup };
    let ok = is_approved(data_dir, &main.path, kind, &commands);
    Ok((main.path, commands, ok))
}

pub fn write_suggestion(dir: &str, confirm: &dyn Fn(&str, &str) -> bool) -> Value {
    let Some(main) = main_checkout(dir) else { return json!({"ok": false, "error": "not-repo"}) };
    let main_path = Path::new(&main.path);
    let (exists, cfg) = read(main_path);
    let text = suggestion_text(main_path, &cfg);
    let path = config_path(main_path);
    if exists && !confirm(&path.to_string_lossy(), &text) {
        return json!({"ok": false, "error": "cancelled"});
    }
    if let Some(parent) = path.parent() {
        if fs::create_dir_all(parent).is_err() {
            return json!({"ok": false, "error": "failed"});
        }
    }
    if fs::write(&path, &text).is_err() {
        return json!({"ok": false, "error": "failed"});
    }
    json!({"ok": true, "path": path.to_string_lossy(), "existed": exists})
}

pub fn worktree_path_of(dir: &str, branch: &str) -> Option<String> {
    let res = exec_git(Path::new(dir), &["worktree", "list", "--porcelain"], &ExecOpts::timeout(15_000));
    if !res.ok() {
        return None;
    }
    let text = res.stdout.replace("\r\n", "\n");
    let want = format!("branch refs/heads/{branch}");
    text.split("\n\n").find_map(|block| {
        let lines: Vec<&str> = block.split('\n').collect();
        lines.iter().any(|l| *l == want).then(|| lines.iter().find_map(|l| l.strip_prefix("worktree ")).map(String::from)).flatten()
    })
}

pub struct Target {
    pub main: String,
    pub path: String,
    pub cleanup: Vec<String>,
}

pub fn cleanup_target(dir: &str, branch: &Value) -> Result<Target, (&'static str, String)> {
    let main = main_checkout(dir).ok_or(("not-repo", String::new()))?;
    let branch = branch.as_str().filter(|b| is_branch_name(b)).ok_or(("unknown", String::new()))?;
    if branch == main.branch {
        return Err(("main", String::new()));
    }
    let path = worktree_path_of(dir, branch).ok_or(("unknown", String::new()))?;
    if super::paths::resolve_str(&path) == main.path {
        return Err(("main", String::new()));
    }
    if !Path::new(&path).is_dir() {
        return Err(("unknown", String::new()));
    }
    let status = exec_git(Path::new(&path), &["status", "--porcelain"], &ExecOpts::timeout(20_000));
    if !status.ok() {
        return Err(("git", status.stderr));
    }
    if !status.stdout.trim().is_empty() {
        return Err(("dirty", String::new()));
    }
    let (_, cfg) = read(Path::new(&main.path));
    Ok(Target { main: main.path, path, cleanup: cfg.cleanup })
}

#[cfg(windows)]
fn shell_command(cmd: &str) -> (PathBuf, Vec<String>) {
    let script = format!("{cmd}\nif ($LASTEXITCODE) {{ exit $LASTEXITCODE }}");
    (PathBuf::from("powershell.exe"), vec!["-NoLogo".into(), "-NoProfile".into(), "-NonInteractive".into(), "-Command".into(), script])
}

#[cfg(not(windows))]
fn shell_command(cmd: &str) -> (PathBuf, Vec<String>) {
    (PathBuf::from("/bin/sh"), vec!["-c".into(), cmd.to_string()])
}

pub fn run_cleanup(dir: &Path, commands: &[String], timeout_ms: u64) -> Value {
    let start = Instant::now();
    for (i, cmd) in commands.iter().enumerate() {
        let spent = start.elapsed().as_millis() as u64;
        if spent >= timeout_ms {
            return json!({"ok": false, "error": "timeout", "detail": cmd, "ran": i});
        }
        let (file, args) = shell_command(cmd);
        let argv: Vec<&str> = args.iter().map(String::as_str).collect();
        let mut o = ExecOpts::timeout(timeout_ms - spent).with_max(64 * 1024);
        o.kill_tree = true;
        o.env = vec![("CI".into(), "1".into())];
        let res = exec_file_at(Some(dir), &file, &argv, &o, false);
        if res.timed_out {
            return json!({"ok": false, "error": "timeout", "detail": cmd, "ran": i});
        }
        if !res.ok() {
            let tail = if res.stderr.trim().is_empty() { &res.stdout } else { &res.stderr };
            let tail: String = tail.trim().lines().last().unwrap_or("").chars().take(200).collect();
            return json!({"ok": false, "error": "cleanup-failed", "detail": format!("{cmd}: {tail}"), "ran": i});
        }
    }
    json!({"ok": true, "ran": commands.len()})
}

#[cfg(test)]
#[path = "wtconfig_tests.rs"]
mod tests;
