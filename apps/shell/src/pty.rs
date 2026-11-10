use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{mpsc, Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::config::{random_hex, uuid};
use crate::presets::{child_env, is_session_id, preset_command, resolve_cwd, PRESETS};

pub const PROTOCOL_PREFIX: &str = "ccm.pty.";
const START_TIMEOUT: Duration = Duration::from_secs(10);
const STOP_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_LIVE: usize = 16;
const CORE_ENV_KEYS: [&str; 11] = ["PATH", "SYSTEMROOT", "WINDIR", "COMSPEC", "TEMP", "TMP", "TMPDIR", "HOME", "USERPROFILE", "LANG", "LC_ALL"];

#[derive(Clone)]
pub struct Endpoint {
    pub url: String,
    pub protocol: String,
}

struct Host {
    child: Child,
    stdin: Arc<Mutex<ChildStdin>>,
    endpoint: Endpoint,
}

#[derive(Default)]
struct Panes {
    live: HashMap<String, bool>,
}

pub struct PtyHost {
    bin: PathBuf,
    origin: String,
    version: String,
    log: PathBuf,
    host: Mutex<Option<Host>>,
    stopped: Mutex<bool>,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

fn send(stdin: &Mutex<ChildStdin>, msg: &Value) {
    let mut line = msg.to_string();
    line.push('\n');
    let mut s = lock(stdin);
    let _ = s.write_all(line.as_bytes());
    let _ = s.flush();
}

fn home() -> PathBuf {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map_or_else(std::env::temp_dir, PathBuf::from)
}

pub fn resolve(rid: &str, msg: &Value, live: usize, home: &Path) -> (Value, Option<String>) {
    let fail = |m: &str| (json!({ "t": "resolved", "rid": rid, "ok": false, "message": m }), None);
    let preset = msg.get("preset").and_then(Value::as_str).unwrap_or("");
    if !PRESETS.contains(&preset) {
        return fail("preset");
    }
    let Some(cwd) = resolve_cwd(msg.get("cwd").and_then(Value::as_str), home) else { return fail("cwd") };
    if live >= MAX_LIVE {
        return fail("limit");
    }
    let resume = msg.get("resume").and_then(Value::as_str);
    if msg.get("resume").is_some() && (preset == "shell" || !resume.is_some_and(is_session_id)) {
        return fail("preset");
    }
    let session = (preset == "claude" && resume.is_none() && msg.get("open").is_none()).then(|| msg.get("session").and_then(Value::as_str)).flatten().filter(|s| is_session_id(s));
    let get = |k: &str| std::env::var(k).ok();
    let spec = preset_command(preset, cfg!(windows), &get, resume, session);
    let mut env: serde_json::Map<String, Value> = serde_json::Map::new();
    for (k, v) in child_env(std::env::vars()).into_iter().chain(spec.env) {
        env.insert(k, Value::String(v));
    }
    let id = uuid();
    let mut reply = json!({ "t": "resolved", "rid": rid, "ok": true, "id": id, "file": spec.file, "args": spec.args, "env": env, "cwd": cwd.to_string_lossy() });
    if let Some(agent) = resume.or(session) {
        reply["agentSession"] = json!(agent);
    }
    (reply, Some(id))
}

fn answer(msg: &Value, panes: &mut Panes, home: &Path) -> Option<Value> {
    let t = msg.get("t").and_then(Value::as_str)?;
    let rid = msg.get("rid").and_then(Value::as_str).unwrap_or("");
    match t {
        "resolve" => {
            let live = panes.live.values().filter(|alive| **alive).count();
            let (reply, id) = resolve(rid, msg.get("msg").unwrap_or(&Value::Null), live, home);
            if let Some(id) = id {
                panes.live.insert(id, true);
            }
            Some(reply)
        }
        "exited" => {
            if let Some(id) = msg.get("id").and_then(Value::as_str) {
                panes.live.insert(id.to_owned(), false);
            }
            None
        }
        "removed" => {
            if let Some(id) = msg.get("id").and_then(Value::as_str) {
                panes.live.remove(id);
            }
            None
        }
        "cwd" => {
            let raw = msg.get("cwd").and_then(Value::as_str).unwrap_or("");
            let ok = (!raw.is_empty()).then(|| resolve_cwd(Some(raw), home)).flatten().filter(|p| p.parent().is_some());
            Some(json!({ "t": "cwd-ok", "rid": rid, "cwd": ok.map(|p| p.to_string_lossy().into_owned()) }))
        }
        "wt" => Some(json!({ "t": "wt-reply", "rid": rid, "reply": { "t": "wt.error", "code": "git" } })),
        _ => None,
    }
}

impl PtyHost {
    pub fn new(bin: PathBuf, origin: String, version: String, log: PathBuf) -> PtyHost {
        PtyHost { bin, origin, version, log, host: Mutex::new(None), stopped: Mutex::new(false) }
    }

    pub fn endpoint(&self) -> Result<Endpoint, String> {
        if *lock(&self.stopped) {
            return Err("pty host stopped".into());
        }
        let mut host = lock(&self.host);
        if let Some(h) = host.as_mut() {
            if matches!(h.child.try_wait(), Ok(None)) {
                return Ok(h.endpoint.clone());
            }
        }
        let started = self.launch()?;
        let endpoint = started.endpoint.clone();
        *host = Some(started);
        Ok(endpoint)
    }

    fn launch(&self) -> Result<Host, String> {
        if !self.bin.is_file() {
            return Err(format!("orrery-core not found at {}", self.bin.display()));
        }
        let log = std::fs::OpenOptions::new().create(true).append(true).open(&self.log).map_err(|e| e.to_string())?;
        let mut cmd = Command::new(&self.bin);
        cmd.env_clear();
        for (k, v) in std::env::vars() {
            if CORE_ENV_KEYS.contains(&k.to_ascii_uppercase().as_str()) {
                cmd.env(k, v);
            }
        }
        cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::from(log));
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        let mut child = cmd.spawn().map_err(|e| format!("orrery-core pty: {e}"))?;
        let stdin = Arc::new(Mutex::new(child.stdin.take().ok_or("no stdin")?));
        let stdout = child.stdout.take().ok_or("no stdout")?;
        let token = random_hex(32);
        send(&stdin, &json!({ "t": "init", "token": token, "origins": [self.origin], "appVersion": self.version }));
        let (tx, rx) = mpsc::channel::<u16>();
        let writer = stdin.clone();
        let _ = std::thread::Builder::new().name("pty-control".into()).spawn(move || {
            let home = home();
            let mut panes = Panes::default();
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
                if msg.get("t").and_then(Value::as_str) == Some("listening") {
                    if let Some(port) = msg.get("port").and_then(Value::as_u64).and_then(|p| u16::try_from(p).ok()) {
                        let _ = tx.send(port);
                    }
                    continue;
                }
                if let Some(reply) = answer(&msg, &mut panes, &home) {
                    send(&writer, &reply);
                }
            }
        });
        let deadline = Instant::now() + START_TIMEOUT;
        match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
            Ok(port) => Ok(Host { child, stdin, endpoint: Endpoint { url: format!("ws://127.0.0.1:{port}/pty"), protocol: format!("{PROTOCOL_PREFIX}{token}") } }),
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                Err("pty host did not start".into())
            }
        }
    }

    pub fn stop(&self) {
        *lock(&self.stopped) = true;
        let Some(mut host) = lock(&self.host).take() else { return };
        send(&host.stdin, &json!({ "t": "shutdown" }));
        let deadline = Instant::now() + STOP_TIMEOUT;
        while Instant::now() < deadline {
            if !matches!(host.child.try_wait(), Ok(None)) {
                return;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let _ = host.child.kill();
        let _ = host.child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shell_spawn_is_resolved_in_home_and_counted() {
        let home = std::env::temp_dir();
        let mut panes = Panes::default();
        let reply = answer(&json!({ "t": "resolve", "rid": "r1", "msg": { "preset": "shell", "cols": 80, "rows": 24 } }), &mut panes, &home).unwrap();
        assert_eq!(reply["ok"], true);
        assert_eq!(reply["rid"], "r1");
        assert!(reply["file"].as_str().is_some_and(|f| !f.is_empty()));
        assert_eq!(reply["env"]["TERM"], "xterm-256color");
        assert!(reply["env"].get("CCM_APP_KEY").is_none());
        let id = reply["id"].as_str().unwrap().to_owned();
        assert_eq!(panes.live.get(&id), Some(&true));
        assert!(answer(&json!({ "t": "exited", "id": id, "code": 0 }), &mut panes, &home).is_none());
        assert_eq!(panes.live.get(&id), Some(&false));
        answer(&json!({ "t": "removed", "id": id }), &mut panes, &home);
        assert!(panes.live.is_empty());
    }

    #[test]
    fn bad_spawns_are_refused_with_the_core_error_names() {
        let home = std::env::temp_dir();
        let msg = |m: Value| resolve("r", &m, 0, &home).0["message"].clone();
        assert_eq!(msg(json!({ "preset": "deepseek" })), "preset");
        assert_eq!(msg(json!({ "preset": "rm" })), "preset");
        assert_eq!(msg(json!({ "preset": "shell", "cwd": "relative/dir" })), "cwd");
        assert_eq!(msg(json!({ "preset": "shell", "resume": "0f8fad5b-d9cb-469f-a165-70867728950e" })), "preset");
        assert_eq!(msg(json!({ "preset": "claude", "resume": "nope" })), "preset");
        assert_eq!(resolve("r", &json!({ "preset": "shell" }), MAX_LIVE, &home).0["message"], "limit");
        let resumed = resolve("r", &json!({ "preset": "claude", "resume": "0f8fad5b-d9cb-469f-a165-70867728950e" }), 0, &home).0;
        assert_eq!(resumed["agentSession"], "0f8fad5b-d9cb-469f-a165-70867728950e");
    }

    #[test]
    fn cwd_reports_are_validated_and_worktrees_are_not_ported_yet() {
        let home = std::env::temp_dir();
        let mut panes = Panes::default();
        let ok = answer(&json!({ "t": "cwd", "rid": "c", "id": "x", "cwd": home.to_string_lossy() }), &mut panes, &home).unwrap();
        assert!(ok["cwd"].is_string());
        let bad = answer(&json!({ "t": "cwd", "rid": "c", "id": "x", "cwd": "nope" }), &mut panes, &home).unwrap();
        assert!(bad["cwd"].is_null());
        let wt = answer(&json!({ "t": "wt", "rid": "w", "msg": { "t": "wt.list" } }), &mut panes, &home).unwrap();
        assert_eq!(wt["reply"]["t"], "wt.error");
    }
}
