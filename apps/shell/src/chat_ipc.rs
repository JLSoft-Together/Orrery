use std::collections::HashMap;
use std::path::Path;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use core_transcript::chat::edits::{self, Sources, Store, STRONG};
use core_transcript::chat::export::{self, Form, Meta};
use core_transcript::chat::page::{self as chat_page, Live, Poll};
use core_transcript::chat::{Agent, PAGE, PAGE_MAX};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

use crate::remote::chat_src::{page_of, Ask, Hub, Roots, ShellClean, Source, Target};
use crate::state::lock;

const CHANNEL: &str = "ccm:chat";
const WATCH_MAX: usize = 4;
const KEY_MAX: usize = 200;
const GEN_MAX: usize = 64;
const ID_MAX: usize = 100;
const CWD_MAX: usize = 1024;
const LABEL_MAX: usize = 200;
const COMMITS_MAX: usize = 100;
const COMMIT_FILES_MAX: usize = 3000;
const COMMIT_PATH_MAX: usize = 1024;
const GIT_TIMEOUT: Duration = Duration::from_secs(10);

static INGEST: Mutex<()> = Mutex::new(());

pub fn handles(channel: &str) -> bool {
    channel == CHANNEL
}

struct Watch {
    key: String,
    gen: String,
    source: Option<Source>,
    live: Option<Live>,
}

#[derive(Default)]
pub struct DeskChat {
    hub: Hub,
    watches: Mutex<Vec<Watch>>,
}

static CHAT: LazyLock<DeskChat> = LazyLock::new(DeskChat::default);

pub fn dispatch(app: &AppHandle, args: &[Value]) -> Result<Value, String> {
    let v = args.first().unwrap_or(&Value::Null);
    if v["op"] == "commits" {
        let dir = app.state::<crate::state::Shell>().data_dir.join("edits");
        return Ok(commits(&dir, &Roots::from_env(), v, &git_files));
    }
    let reply = CHAT.op(&Roots::from_env(), v);
    if v["op"] != "export" || reply["ok"] != true {
        return Ok(reply);
    }
    Ok(match v["dest"].as_str().unwrap_or("text") {
        "save" => save(app, &reply),
        "telegram" => send(app, v, &reply),
        _ => reply,
    })
}

fn save(app: &AppHandle, reply: &Value) -> Value {
    let mut d = app.dialog().file().set_file_name(reply["name"].as_str().unwrap_or("session.md")).add_filter("Markdown", &["md"]);
    if let Some(w) = app.get_webview_window(crate::state::MAIN) {
        d = d.set_parent(&w);
    }
    if let Ok(dir) = app.path().download_dir() {
        d = d.set_directory(dir);
    }
    let Some(file) = d.blocking_save_file().and_then(|p| p.into_path().ok()) else { return json!({ "ok": true, "saved": false }) };
    if std::fs::write(&file, reply["text"].as_str().unwrap_or("")).is_err() {
        return fail("write");
    }
    json!({ "ok": true, "saved": true, "path": file.to_string_lossy(), "cut": reply["cut"] })
}

fn send(app: &AppHandle, v: &Value, reply: &Value) -> Value {
    let caption = format!("{} session: {}", reply["label"].as_str().unwrap_or(""), reply["project"].as_str().unwrap_or(""));
    let args = json!({ "cwd": v["cwd"], "from": v["key"], "name": reply["name"], "text": reply["text"], "caption": caption });
    match crate::desk::call_inside(app, "desk:tg-doc", args) {
        Ok(res) if res["ok"] == true => json!({ "ok": true, "sent": true, "cut": reply["cut"] }),
        Ok(res) => fail(res["error"].as_str().unwrap_or("failed")),
        Err(_) => fail("failed"),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Req {
    pub op: String,
    pub key: String,
    pub target: Target,
    pub session: String,
    pub gen: Option<String>,
    pub before: Option<u64>,
    pub limit: usize,
    pub form: Form,
    pub branch: String,
    pub project: String,
    pub turn: Option<u64>,
}

#[derive(Debug, PartialEq)]
pub struct CommitAsk {
    pub hash: String,
    pub time: i64,
    pub files: Option<Vec<String>>,
}

fn hash_ok(h: &str) -> bool {
    (7..=64).contains(&h.len()) && h.bytes().all(|b| b.is_ascii_hexdigit())
}

pub fn parse_commits(v: &Value) -> Result<(String, Vec<CommitAsk>), &'static str> {
    let root = v["root"].as_str().filter(|r| !r.is_empty() && r.len() <= CWD_MAX && Path::new(r).is_absolute()).ok_or("invalid")?;
    let list = v["commits"].as_array().filter(|a| a.len() <= COMMITS_MAX).ok_or("invalid")?;
    let mut out = Vec::with_capacity(list.len());
    for c in list {
        let hash = c["hash"].as_str().filter(|h| hash_ok(h)).ok_or("invalid")?.to_ascii_lowercase();
        let time = c["time"].as_i64().filter(|t| *t > 0).ok_or("invalid")?;
        let files = match &c["files"] {
            Value::Null => None,
            Value::Array(a) if a.len() <= COMMIT_FILES_MAX => Some(a.iter().map(|f| f.as_str().filter(|p| !p.is_empty() && p.len() <= COMMIT_PATH_MAX).map(str::to_owned).ok_or("invalid")).collect::<Result<Vec<_>, _>>()?),
            _ => return Err("invalid"),
        };
        out.push(CommitAsk { hash, time, files });
    }
    Ok((root.to_owned(), out))
}

pub fn git_files(root: &str, hashes: &[String]) -> HashMap<String, Vec<String>> {
    let mut args: Vec<&str> = vec!["-c", "core.quotepath=off", "show", "--no-walk=unsorted", "--name-only", "--format=%x01%H"];
    args.extend(hashes.iter().map(String::as_str));
    let r = core_desk::git::run_git(Path::new(root), &args, GIT_TIMEOUT);
    if !r.ok {
        return HashMap::new();
    }
    parse_show(&r.stdout)
}

pub fn parse_show(out: &str) -> HashMap<String, Vec<String>> {
    let mut map = HashMap::new();
    for chunk in out.split('\u{1}').filter(|c| !c.trim().is_empty()) {
        let mut lines = chunk.lines();
        let Some(hash) = lines.next().map(str::trim).filter(|h| hash_ok(h)) else { continue };
        let files: Vec<String> = lines.map(str::trim).filter(|l| !l.is_empty()).map(str::to_owned).collect();
        map.insert(hash.to_ascii_lowercase(), files);
    }
    map
}

pub type FilesOf = dyn Fn(&str, &[String]) -> HashMap<String, Vec<String>>;

pub fn commits(dir: &Path, roots: &Roots, v: &Value, files_of: &FilesOf) -> Value {
    let (root, asks) = match parse_commits(v) {
        Ok(p) => p,
        Err(code) => return fail(code),
    };
    if !Path::new(&root).is_dir() {
        return fail("missing");
    }
    let need: Vec<String> = asks.iter().filter(|a| a.files.is_none()).map(|a| a.hash.clone()).collect();
    let fetched = if need.is_empty() { HashMap::new() } else { files_of(&root, &need) };
    let recs = {
        let _one = lock(&INGEST);
        let src = Sources { projects: roots.projects.clone(), sessions: roots.sessions.clone() };
        let since = asks.iter().map(|a| a.time * 1000).min().unwrap_or(0) - edits::BEFORE_MS;
        match Store::new(dir, &root).ingest_since(&src, core_transcript::time::now_ms(), since, &ShellClean) {
            Ok(r) => r,
            Err(_) => return fail("internal"),
        }
    };
    let fold = edits::fold(&root);
    let mut found = serde_json::Map::new();
    for a in &asks {
        let files = a.files.as_ref().or_else(|| fetched.get(&a.hash)).map(Vec::as_slice).unwrap_or(&[]);
        let (hits, more) = edits::match_commit(&recs, a.time * 1000, files, fold);
        if hits.is_empty() {
            continue;
        }
        let chips: Vec<Value> = hits
            .iter()
            .map(|h| json!({ "agent": h.agent, "session": h.session_id, "label": h.label, "cwd": h.cwd, "turn": h.turn, "score": (h.score * 100.0).round() / 100.0, "strong": h.score >= STRONG, "prompt": h.prompt, "at": h.end }))
            .collect();
        found.insert(a.hash.clone(), json!({ "chips": chips, "more": more }));
    }
    json!({ "ok": true, "commits": found })
}

fn fail(code: &str) -> Value {
    json!({ "ok": false, "code": code })
}

pub fn parse(v: &Value) -> Result<Req, &'static str> {
    let op = v["op"].as_str().filter(|o| matches!(*o, "page" | "poll" | "unwatch" | "export" | "seek")).ok_or("invalid")?;
    let key = v["key"].as_str().filter(|k| !k.is_empty() && k.len() <= KEY_MAX).ok_or("invalid")?;
    let agent = if op == "unwatch" { Some(Agent::Claude) } else { v["agent"].as_str().and_then(Agent::parse) }.ok_or("invalid")?;
    let session = v["session"].as_str().unwrap_or("");
    if session.len() > ID_MAX {
        return Err("invalid");
    }
    let cwd = v["cwd"].as_str().unwrap_or("");
    if cwd.len() > CWD_MAX {
        return Err("invalid");
    }
    let limit = match &v["limit"] {
        Value::Null => PAGE,
        l => l.as_u64().filter(|n| (1..=PAGE_MAX as u64).contains(n)).ok_or("invalid")? as usize,
    };
    let before = match &v["before"] {
        Value::Null => None,
        b => Some(b.as_u64().ok_or("invalid")?),
    };
    let gen = match &v["gen"] {
        Value::Null => None,
        g => Some(g.as_str().filter(|g| g.len() <= GEN_MAX).ok_or("invalid")?.to_owned()),
    };
    let form = match &v["form"] {
        Value::Null => Form::Full,
        f => f.as_str().and_then(Form::parse).ok_or("invalid")?,
    };
    let label = |k: &str| match &v[k] {
        Value::Null => Ok(String::new()),
        Value::String(s) if s.len() <= LABEL_MAX => Ok(s.trim().to_owned()),
        _ => Err("invalid"),
    };
    let (branch, project) = (label("branch")?, label("project")?);
    let turn = match &v["turn"] {
        Value::Null if op == "seek" => return Err("invalid"),
        Value::Null => None,
        t => Some(t.as_u64().ok_or("invalid")?),
    };
    Ok(Req { op: op.to_owned(), key: key.to_owned(), target: Target { agent, terminal: None, cwd: cwd.to_owned() }, session: session.to_owned(), gen, before, limit, form, branch, project, turn })
}

fn project_of(root: &str) -> String {
    root.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().unwrap_or("").to_owned()
}

fn branch_of(root: &str) -> String {
    let git = std::path::Path::new(root).join(".git");
    let dir = match std::fs::read_to_string(&git) {
        Ok(link) => link.trim().strip_prefix("gitdir:").map(|d| std::path::PathBuf::from(d.trim())).unwrap_or(git),
        Err(_) => git,
    };
    let head = std::fs::read_to_string(dir.join("HEAD")).unwrap_or_default();
    head.trim().strip_prefix("ref: refs/heads/").unwrap_or("").to_owned()
}

fn terminal_of(r: &Req) -> Value {
    let field = match r.target.agent {
        Agent::Claude => "claudeSessionId",
        Agent::Codex => "codexSessionId",
    };
    json!({ "cwd": r.target.cwd, field: r.session })
}

impl DeskChat {
    fn source(&self, r: &Req, roots: &Roots) -> Option<Source> {
        if r.session.is_empty() {
            return None;
        }
        self.hub.source_of(&r.target, Some(&terminal_of(r)), roots)
    }

    pub fn op(&self, roots: &Roots, v: &Value) -> Value {
        let r = match parse(v) {
            Ok(r) => r,
            Err(code) => return fail(code),
        };
        match r.op.as_str() {
            "unwatch" => {
                self.unwatch(&r.key);
                json!({ "ok": true })
            }
            "poll" => self.poll(&r, roots),
            "export" => self.export(&r, roots),
            "seek" => self.seek(&r, roots),
            _ => self.page(&r, roots),
        }
    }

    fn seek(&self, r: &Req, roots: &Roots) -> Value {
        let turn = r.turn.unwrap_or(0);
        let Some(src) = self.source(r, roots) else { return fail("empty") };
        let latest = |missing: bool| {
            let live = Req { before: None, gen: None, ..r.clone() };
            let mut v = self.page(&live, roots);
            if v["ok"] == true {
                v["newer"] = Value::Null;
                v["turn"] = Value::from(turn);
                if missing {
                    v["missing"] = Value::Bool(true);
                }
            }
            v
        };
        match chat_page::turn_bounds(&src.path, r.target.agent, turn) {
            Ok(Some(Some(next))) => {
                let ask = Ask { pane: &r.key, target: &r.target, gen: None, before: Some(next), limit: r.limit, live: false };
                match page_of(&self.hub, &ask, Some(src)) {
                    Ok(p) => {
                        let mut v = p.reply;
                        v["newer"] = Value::from(next);
                        v["turn"] = Value::from(turn);
                        v
                    }
                    Err(code) => fail(code),
                }
            }
            Ok(Some(None)) => latest(false),
            Ok(None) => latest(true),
            Err(_) => fail("internal"),
        }
    }

    fn export(&self, r: &Req, roots: &Roots) -> Value {
        let Some(src) = self.source(r, roots) else { return fail("empty") };
        let home = std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).unwrap_or_default();
        let project = if r.project.is_empty() { project_of(&src.root) } else { r.project.clone() };
        let branch = if r.branch.is_empty() { branch_of(&src.root) } else { r.branch.clone() };
        let meta = Meta { agent: r.target.agent, project: &project, branch: &branch, home: &home, platform: crate::files_ipc::platform() };
        match export::export(&src.path, &src.root, &meta, r.form, &ShellClean) {
            Ok(out) => json!({ "ok": true, "text": out.text, "cut": out.cut, "name": export::file_name(&meta, core_transcript::time::now_ms()), "label": export::label(r.target.agent), "project": project }),
            Err(_) => fail("internal"),
        }
    }

    fn unwatch(&self, key: &str) {
        lock(&self.watches).retain(|w| w.key != key);
    }

    #[cfg(test)]
    pub fn watching(&self) -> usize {
        lock(&self.watches).len()
    }

    fn page(&self, r: &Req, roots: &Roots) -> Value {
        let source = self.source(r, roots);
        let live = r.before.is_none();
        let ask = Ask { pane: &r.key, target: &r.target, gen: r.gen.as_deref(), before: r.before, limit: r.limit, live };
        let paged = match page_of(&self.hub, &ask, source) {
            Ok(p) => p,
            Err(code) => return fail(code),
        };
        if live {
            let mut watches = lock(&self.watches);
            watches.retain(|w| w.key != r.key);
            if watches.len() >= WATCH_MAX {
                watches.remove(0);
            }
            watches.push(Watch { key: r.key.clone(), gen: paged.gen.clone(), source: paged.source.clone(), live: paged.live });
        }
        paged.reply
    }

    fn poll(&self, r: &Req, roots: &Roots) -> Value {
        let now = self.source(r, roots);
        let mut watches = lock(&self.watches);
        let Some(at) = watches.iter().position(|w| w.key == r.key) else { return json!({ "ok": true, "reset": true }) };
        let w = &mut watches[at];
        let moved = now.as_ref().map(|s| &s.path) != w.source.as_ref().map(|s| &s.path);
        if moved || r.gen.as_deref().is_some_and(|g| g != w.gen) {
            watches.remove(at);
            return json!({ "ok": true, "reset": true });
        }
        let gen = w.gen.clone();
        let Some(live) = w.live.as_mut() else { return json!({ "ok": true, "gen": gen, "items": [] }) };
        match live.poll() {
            Ok(Poll::Items(items)) => {
                let items: Vec<Value> = items.iter().map(|i| i.to_json(&ShellClean)).collect();
                json!({ "ok": true, "gen": gen, "items": items })
            }
            Ok(Poll::Reset) => {
                if let Some(src) = &w.source {
                    self.hub.bump(src);
                }
                watches.remove(at);
                json!({ "ok": true, "reset": true })
            }
            Err(_) => json!({ "ok": true, "gen": gen, "items": [] }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::PathBuf;

    const TS: &str = "2026-11-20T01:00:00.000Z";
    const SID: &str = "0f0ce96d-chat-ipc";

    fn line(v: Value) -> String {
        format!("{v}\n")
    }

    fn prompt(uuid: &str, text: &str) -> String {
        line(json!({ "type": "user", "uuid": uuid, "timestamp": TS, "isSidechain": false, "message": { "role": "user", "content": text } }))
    }

    fn say(id: &str, text: &str) -> String {
        line(json!({ "type": "assistant", "timestamp": TS, "message": { "id": id, "model": "m", "content": [{ "type": "text", "text": text }], "stop_reason": "end_turn" } }))
    }

    fn roots(name: &str) -> (Roots, PathBuf, String) {
        let base = std::env::temp_dir().join(format!("ccm-chat-ipc-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let cwd = base.join("work").to_string_lossy().into_owned();
        let dir = base.join("projects").join("p");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(format!("{SID}.jsonl"));
        (Roots { projects: base.join("projects"), sessions: base.join("sessions") }, file, cwd)
    }

    fn ask(op: &str, cwd: &str) -> Value {
        json!({ "op": op, "key": "p1", "agent": "claude", "session": SID, "cwd": cwd })
    }

    #[test]
    fn parse_checks_every_field() {
        assert_eq!(parse(&json!({ "op": "x", "key": "a", "agent": "claude" })), Err("invalid"));
        assert_eq!(parse(&json!({ "op": "page", "key": "", "agent": "claude" })), Err("invalid"));
        assert_eq!(parse(&json!({ "op": "page", "key": "a", "agent": "deepseek" })), Err("invalid"));
        assert_eq!(parse(&json!({ "op": "page", "key": "a", "agent": "codex", "limit": 0 })), Err("invalid"));
        assert_eq!(parse(&json!({ "op": "page", "key": "a", "agent": "codex", "limit": PAGE_MAX + 1 })), Err("invalid"));
        assert_eq!(parse(&json!({ "op": "page", "key": "a", "agent": "codex", "before": "x" })), Err("invalid"));
        assert_eq!(parse(&json!({ "op": "page", "key": "a", "agent": "codex", "session": "s".repeat(ID_MAX + 1) })), Err("invalid"));
        let r = parse(&json!({ "op": "page", "key": "a", "agent": "codex", "session": "abc", "cwd": "C:\\w", "before": 9, "limit": 5 })).unwrap();
        assert_eq!((r.target.agent, r.before, r.limit, r.session.as_str()), (Agent::Codex, Some(9), 5, "abc"));
        assert!(parse(&json!({ "op": "unwatch", "key": "a" })).is_ok());
    }

    #[test]
    fn page_then_poll_streams_new_items_and_masks_secrets() {
        let (roots, file, cwd) = roots("stream");
        std::fs::write(&file, prompt("u1", "Fix login with token sk-ant-api03-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA") + &say("a1", "Done, retry is 500ms.")).unwrap();
        let chat = DeskChat::default();
        let page = chat.op(&roots, &ask("page", &cwd));
        assert_eq!(page["ok"], true);
        let items = page["items"].as_array().unwrap();
        assert!(items.iter().any(|i| i["k"] == "me" && i["text"].as_str().unwrap().contains("Fix login")));
        assert!(items.iter().all(|i| !i.to_string().contains("AAAAAAAAAAAAAAAAAAAA")));
        assert!(items.iter().any(|i| i["k"] == "ai"));
        assert_eq!(chat.watching(), 1);
        let gen = page["gen"].as_str().unwrap().to_owned();
        let mut f = std::fs::OpenOptions::new().append(true).open(&file).unwrap();
        f.write_all((prompt("u2", "Add a test") + &say("a2", "Test added.")).as_bytes()).unwrap();
        let mut q = ask("poll", &cwd);
        q["gen"] = Value::from(gen.clone());
        let polled = chat.op(&roots, &q);
        assert_eq!(polled["gen"], gen.as_str());
        let fresh = polled["items"].as_array().unwrap();
        assert!(fresh.iter().any(|i| i["text"] == "Add a test"));
        assert!(fresh.iter().any(|i| i["text"] == "Test added."));
        assert_eq!(chat.op(&roots, &q)["items"].as_array().map(Vec::len), Some(0));
        chat.op(&roots, &json!({ "op": "unwatch", "key": "p1" }));
        assert_eq!(chat.watching(), 0);
        assert_eq!(chat.op(&roots, &q)["reset"], true);
    }

    #[test]
    fn a_shrunk_file_resets_and_a_missing_session_is_empty() {
        let (roots, file, cwd) = roots("reset");
        std::fs::write(&file, prompt("u1", "first question here") + &say("a1", "first answer")).unwrap();
        let chat = DeskChat::default();
        let page = chat.op(&roots, &ask("page", &cwd));
        std::fs::write(&file, prompt("u9", "x")).unwrap();
        let mut q = ask("poll", &cwd);
        q["gen"] = page["gen"].clone();
        assert_eq!(chat.op(&roots, &q)["reset"], true);
        let mut none = ask("page", &cwd);
        none["session"] = Value::from("");
        let empty = chat.op(&roots, &none);
        assert_eq!((empty["ok"].clone(), empty["items"].as_array().map(Vec::len)), (Value::Bool(true), Some(0)));
    }

    #[test]
    fn export_returns_markdown_without_watching_and_checks_its_fields() {
        let (roots, file, cwd) = roots("export");
        std::fs::write(&file, prompt("u1", "Fix login with token sk-ant-api03-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA") + &say("a1", "Done, retry is 500ms.")).unwrap();
        let chat = DeskChat::default();
        let mut q = ask("export", &cwd);
        q["branch"] = Value::from("feat/x");
        let full = chat.op(&roots, &q);
        assert_eq!(full["ok"], true);
        let text = full["text"].as_str().unwrap();
        assert!(text.contains("### Bạn") && text.contains("### Claude") && text.contains("- Nhánh: feat/x"), "{text}");
        assert!(!text.contains("AAAAAAAAAAAA") && !text.contains("sk-ant-"), "{text}");
        assert_eq!((full["label"].as_str(), full["cut"].clone()), (Some("Claude"), Value::Bool(false)));
        assert!(full["name"].as_str().unwrap().ends_with(".md"));
        std::fs::create_dir_all(std::path::Path::new(&cwd).join(".git")).unwrap();
        std::fs::write(std::path::Path::new(&cwd).join(".git").join("HEAD"), "ref: refs/heads/main\n").unwrap();
        q["branch"] = Value::Null;
        assert!(chat.op(&roots, &q)["text"].as_str().unwrap().contains("- Nhánh: main"));
        q["form"] = Value::from("short");
        let short = chat.op(&roots, &q);
        assert!(short["text"].as_str().unwrap().starts_with("## Yêu cầu"));
        assert_eq!(chat.watching(), 0);
        q["form"] = Value::from("tiny");
        assert_eq!(chat.op(&roots, &q), fail("invalid"));
        q["form"] = Value::Null;
        q["session"] = Value::from("");
        assert_eq!(chat.op(&roots, &q), fail("empty"));
    }

    #[test]
    fn watches_are_capped() {
        let (roots, file, cwd) = roots("cap");
        std::fs::write(&file, prompt("u1", "hello there") + &say("a1", "hi")).unwrap();
        let chat = DeskChat::default();
        for k in 0..WATCH_MAX + 2 {
            let mut a = ask("page", &cwd);
            a["key"] = Value::from(format!("k{k}"));
            chat.op(&roots, &a);
        }
        assert_eq!(chat.watching(), WATCH_MAX);
    }

    fn texts(v: &Value) -> Vec<String> {
        v["items"].as_array().unwrap().iter().filter_map(|i| i["text"].as_str().map(str::to_owned)).collect()
    }

    #[test]
    fn seek_returns_the_page_that_ends_with_the_turn() {
        let (roots, file, cwd) = roots("seek");
        let a = prompt("u1", "first question here") + &say("a1", "first answer");
        let b = prompt("u2", "second question") + &say("a2", "second answer");
        let c = prompt("u3", "third question") + &say("a3", "third answer");
        std::fs::write(&file, a.clone() + &b + &c).unwrap();
        let chat = DeskChat::default();
        let mut q = ask("seek", &cwd);
        assert_eq!(chat.op(&roots, &q), fail("invalid"));
        q["turn"] = Value::from(a.len());
        let mid = chat.op(&roots, &q);
        assert_eq!((mid["ok"].clone(), mid["newer"].as_u64()), (Value::Bool(true), Some((a.len() + b.len()) as u64)));
        let got = texts(&mid);
        assert!(got.contains(&"second question".to_owned()) && got.contains(&"first answer".to_owned()) && !got.contains(&"third question".to_owned()), "{got:?}");
        assert_eq!(chat.watching(), 0);
        q["turn"] = Value::from(a.len() + b.len());
        let last = chat.op(&roots, &q);
        assert_eq!(last["newer"], Value::Null);
        assert!(texts(&last).contains(&"third answer".to_owned()));
        assert_eq!(chat.watching(), 1);
        q["turn"] = Value::from(3);
        assert_eq!(chat.op(&roots, &q)["missing"], true);
    }

    fn iso(ms: i64) -> String {
        let secs = ms.div_euclid(1000);
        let (y, m, d) = core_transcript::time::civil_from_days(secs.div_euclid(86_400));
        let s = secs.rem_euclid(86_400);
        format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.000Z", s / 3600, (s / 60) % 60, s % 60)
    }

    #[test]
    fn commits_get_chips_from_agent_turns_and_parse_checks_input() {
        let base = std::env::temp_dir().join(format!("ccm-chat-ipc-{}-commits", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let root = base.join("repo").to_string_lossy().replace('\\', "/");
        std::fs::create_dir_all(&root).unwrap();
        let pdir = base.join("projects").join(core_transcript::tracker::cwd_slug(&root));
        std::fs::create_dir_all(&pdir).unwrap();
        let now = core_transcript::time::now_ms() / 1000 * 1000;
        let at = iso(now - 600_000);
        let user = line(json!({ "type": "user", "cwd": root, "timestamp": at, "message": { "role": "user", "content": "Fix login" } }));
        let edit = |id: &str, p: &str| line(json!({ "type": "assistant", "timestamp": at, "message": { "id": id, "model": "m", "content": [{ "type": "tool_use", "id": id, "name": "Edit", "input": { "file_path": format!("{root}/{p}"), "old_string": "a", "new_string": "b" } }] } }));
        std::fs::write(pdir.join(format!("{SID}.jsonl")), user + &edit("e1", "src/a.ts") + &edit("e2", "src/b.ts")).unwrap();
        let roots = Roots { projects: base.join("projects"), sessions: base.join("sessions") };
        let t = now / 1000;
        let git = |_: &str, hashes: &[String]| hashes.iter().map(|h| (h.clone(), vec!["src/a.ts".to_owned(), "src/b.ts".to_owned()])).collect::<HashMap<_, _>>();
        let v = json!({ "op": "commits", "root": root, "commits": [
            { "hash": "aaaaaaa1", "time": t },
            { "hash": "bbbbbbb2", "time": t, "files": ["src/a.ts", "x", "y"] },
            { "hash": "ccccccc3", "time": t, "files": ["README.md"] },
            { "hash": "ddddddd4", "time": t - 7 * 3600 },
        ] });
        let got = commits(&base.join("edits"), &roots, &v, &git);
        assert_eq!(got["ok"], true);
        let map = got["commits"].as_object().unwrap();
        assert_eq!(map.keys().cloned().collect::<Vec<_>>(), ["aaaaaaa1", "bbbbbbb2"]);
        let strong = &map["aaaaaaa1"]["chips"][0];
        assert_eq!((strong["agent"].as_str(), strong["session"].as_str(), strong["turn"].as_u64(), strong["strong"].as_bool()), (Some("claude"), Some(SID), Some(0), Some(true)));
        assert_eq!(strong["label"], "repo");
        assert_eq!(map["bbbbbbb2"]["chips"][0]["strong"], false);
        assert!(base.join("edits").read_dir().unwrap().count() >= 2);
        assert_eq!(commits(&base.join("edits"), &roots, &json!({ "op": "commits", "root": "rel/path", "commits": [] }), &git), fail("invalid"));
        assert_eq!(commits(&base.join("edits"), &roots, &json!({ "op": "commits", "root": root, "commits": [{ "hash": "--help", "time": t }] }), &git), fail("invalid"));
        assert_eq!(parse_show("\u{1}ABCDEF12\n\nsrc/a.ts\nsrc/b.ts\n\u{1}abcdef34\n\n")["abcdef12"], vec!["src/a.ts", "src/b.ts"]);
    }
}
