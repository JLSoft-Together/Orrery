use serde_json::{json, Value};

use crate::data::FILE_TEXT;

struct File {
    path: String,
    kind: &'static str,
    staged: bool,
    counts: Option<(u32, u32)>,
}

pub enum Reply {
    Data(Value, Vec<Value>),
    Denied,
    Confirm(Vec<Value>),
}

pub struct Vw {
    text: String,
    mtime: u64,
    files: Vec<File>,
    commits: Vec<Value>,
    sessions: Vec<Value>,
    pending: Option<String>,
    next: u32,
}

fn file(path: &str, kind: &'static str, staged: bool, counts: Option<(u32, u32)>) -> File {
    File { path: path.to_owned(), kind, staged, counts }
}

fn session_event(session: &Value) -> Value {
    json!({"t": "run.event", "event": {"type": "session", "session": session}})
}

fn log_event(session: &str, text: &str) -> Value {
    json!({"t": "run.event", "event": {"type": "log", "sessionId": session, "lines": [{"text": text, "level": "info"}]}})
}

fn changed(root: &str) -> Value {
    json!({"t": "git.changed", "root": root})
}

impl Vw {
    pub fn new() -> Vw {
        Vw {
            text: FILE_TEXT.to_owned(),
            mtime: 1,
            files: vec![
                file("src/app.ts", "modified", false, Some((3, 1))),
                file("notes.txt", "untracked", false, None),
                file("lib/util.ts", "modified", true, Some((1, 0))),
            ],
            commits: vec![
                json!({"hash": "a1b2c3d4e5f6", "parents": [], "author": "Cham", "time": 1767225600, "refs": "HEAD, feat/api", "subject": "Thêm thanh phím phụ"}),
                json!({"hash": "0f9e8d7c6b5a", "parents": [], "author": "Cham", "time": 1767139200, "refs": "", "subject": "Khởi tạo"}),
            ],
            sessions: vec![json!({"id": "r1", "configName": "flutter run", "state": "running", "cwd": "D:/Code/api", "progress": "Hot reload 1.2s"})],
            pending: None,
            next: 2,
        }
    }

    pub fn disk_change(&mut self) {
        self.mtime += 10;
        self.text.push_str("// edited on disk\n");
    }

    pub fn answer(&mut self, ok: bool) -> Vec<Value> {
        match self.pending.take() {
            Some(path) if ok => {
                self.files.retain(|f| f.path != path);
                vec![changed("D:/Code/api")]
            }
            _ => Vec::new(),
        }
    }

    fn entries(&self) -> Vec<Value> {
        self.files
            .iter()
            .map(|f| {
                let (x, y) = if f.staged {
                    (if f.kind == "added" { "A" } else { "M" }, ".")
                } else if f.kind == "untracked" {
                    ("?", "?")
                } else {
                    (".", "M")
                };
                let mut entry = json!({"path": f.path, "kind": f.kind, "x": x, "y": y});
                if let Some((added, removed)) = f.counts {
                    entry["added"] = json!(added);
                    entry["removed"] = json!(removed);
                }
                entry
            })
            .collect()
    }

    fn paths(value: &Value) -> Vec<String> {
        value.get("paths").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect()).unwrap_or_default()
    }

    pub fn handle(&mut self, view_only: bool, kind: &str, value: &Value) -> Option<Reply> {
        let root = value.get("root").and_then(Value::as_str).unwrap_or("D:/Code/api").to_owned();
        let mutating = matches!(kind, "fs.write" | "git.stage" | "git.unstage" | "git.commit" | "git.discard" | "run.start" | "run.stop" | "run.reload");
        if mutating && view_only {
            return Some(Reply::Denied);
        }
        match kind {
            "fs.read" => {
                let path = value.get("path").and_then(Value::as_str).unwrap_or("");
                Some(Reply::Data(json!({"kind": "text", "path": path, "size": self.text.len(), "mtimeMs": self.mtime, "text": self.text}), Vec::new()))
            }
            "fs.write" => {
                let base = value.get("base").and_then(Value::as_u64);
                let text = value.get("text").and_then(Value::as_str).unwrap_or("").to_owned();
                println!("{}", json!({"ev": "write", "text": text, "base": base}));
                if base != Some(self.mtime) {
                    return Some(Reply::Data(json!({"ok": false, "error": "conflict"}), Vec::new()));
                }
                self.text = text;
                self.mtime += 1;
                Some(Reply::Data(json!({"ok": true, "mtimeMs": self.mtime}), Vec::new()))
            }
            "git.status" => Some(Reply::Data(json!({"root": root, "branch": "feat/api", "entries": self.entries()}), Vec::new())),
            "git.stage" | "git.unstage" => {
                let stage = kind == "git.stage";
                let paths = Self::paths(value);
                println!("{}", json!({"ev": kind, "paths": paths}));
                for f in self.files.iter_mut().filter(|f| paths.contains(&f.path)) {
                    f.staged = stage;
                    if stage && f.kind == "untracked" {
                        f.kind = "added";
                    } else if !stage && f.kind == "added" {
                        f.kind = "untracked";
                    }
                }
                Some(Reply::Data(json!({"ok": true}), vec![changed(&root)]))
            }
            "git.commit" => {
                let message = value.get("message").and_then(Value::as_str).unwrap_or("").to_owned();
                println!("{}", json!({"ev": "commit", "message": message}));
                if !self.files.iter().any(|f| f.staged) {
                    return Some(Reply::Data(json!({"ok": false, "error": "failed"}), Vec::new()));
                }
                self.files.retain(|f| !f.staged);
                self.next += 1;
                self.commits.insert(0, json!({"hash": format!("c{:011}", self.next), "parents": [], "author": "Cham", "time": 1767312000 + self.next, "refs": "HEAD, feat/api", "subject": message}));
                Some(Reply::Data(json!({"ok": true}), vec![changed(&root)]))
            }
            "git.discard" => {
                let path = value.get("path").and_then(Value::as_str).unwrap_or("").to_owned();
                println!("{}", json!({"ev": "discard", "path": path}));
                self.pending = Some(path.clone());
                let ask = json!({"t": "confirm.ask", "cid": "cid1", "re": "git.discard", "title": format!("Bỏ thay đổi trong {path}?"), "detail": format!("git checkout -- {path}"), "ok": "Bỏ thay đổi", "cancel": "Giữ lại"});
                Some(Reply::Confirm(vec![ask]))
            }
            "log.page" => Some(Reply::Data(json!({"commits": self.commits, "more": false}), Vec::new())),
            "run.configs" => Some(Reply::Data(json!([{"id": "c1", "name": "flutter run", "type": "flutter"}, {"id": "c2", "name": "npm dev", "type": "node"}]), Vec::new())),
            "run.devices" => Some(Reply::Data(json!([{"id": "d1", "name": "Pixel 8", "state": "ready"}, {"id": "d2", "name": "Galaxy A33", "state": "offline"}]), Vec::new())),
            "run.sessions" => Some(Reply::Data(Value::Array(self.sessions.clone()), Vec::new())),
            "run.start" => {
                let config = value.get("configId").and_then(Value::as_str).unwrap_or("c1");
                println!("{}", json!({"ev": "run.start", "configId": config, "deviceId": value.get("deviceId")}));
                let name = if config == "c2" { "npm dev" } else { "flutter run" };
                let id = format!("r{}", self.sessions.len() + 1);
                let session = json!({"id": id, "configName": name, "state": "running", "cwd": root, "progress": null});
                self.sessions.push(session.clone());
                Some(Reply::Data(json!({"ok": true, "sessionId": id}), vec![session_event(&session), log_event(&id, "Launching lib/main.dart")]))
            }
            "run.stop" => {
                let id = value.get("sessionId").and_then(Value::as_str).unwrap_or("").to_owned();
                println!("{}", json!({"ev": "run.stop", "sessionId": id}));
                let mut events = Vec::new();
                for s in self.sessions.iter_mut().filter(|s| s["id"] == id) {
                    s["state"] = json!("stopped");
                    events.push(session_event(s));
                }
                Some(Reply::Data(json!({"ok": true}), events))
            }
            "run.reload" => {
                let id = value.get("sessionId").and_then(Value::as_str).unwrap_or("").to_owned();
                let hot = value.get("hot").and_then(Value::as_bool).unwrap_or(false);
                println!("{}", json!({"ev": "run.reload", "sessionId": id, "hot": hot}));
                Some(Reply::Data(json!({"ok": true}), vec![log_event(&id, if hot { "Reloaded 1 of 10 libraries" } else { "Restarted application" })]))
            }
            _ => None,
        }
    }
}
