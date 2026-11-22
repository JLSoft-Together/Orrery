use std::sync::{LazyLock, Mutex};

use core_transcript::chat::page::{Live, Poll};
use core_transcript::chat::{Agent, PAGE, PAGE_MAX};
use serde_json::{json, Value};

use crate::remote::chat_src::{page_of, Ask, Hub, Roots, ShellClean, Source, Target};
use crate::state::lock;

const CHANNEL: &str = "ccm:chat";
const WATCH_MAX: usize = 4;
const KEY_MAX: usize = 200;
const GEN_MAX: usize = 64;
const ID_MAX: usize = 100;
const CWD_MAX: usize = 1024;

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

pub fn dispatch(args: &[Value]) -> Result<Value, String> {
    Ok(CHAT.op(&Roots::from_env(), args.first().unwrap_or(&Value::Null)))
}

#[derive(Debug, PartialEq)]
pub struct Req {
    pub op: String,
    pub key: String,
    pub target: Target,
    pub session: String,
    pub gen: Option<String>,
    pub before: Option<u64>,
    pub limit: usize,
}

fn fail(code: &str) -> Value {
    json!({ "ok": false, "code": code })
}

pub fn parse(v: &Value) -> Result<Req, &'static str> {
    let op = v["op"].as_str().filter(|o| matches!(*o, "page" | "poll" | "unwatch")).ok_or("invalid")?;
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
    Ok(Req { op: op.to_owned(), key: key.to_owned(), target: Target { agent, terminal: None, cwd: cwd.to_owned() }, session: session.to_owned(), gen, before, limit })
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
            _ => self.page(&r, roots),
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
}
