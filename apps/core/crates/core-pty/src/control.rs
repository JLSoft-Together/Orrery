use serde_json::{json, Map, Value};

pub const LINE_MAX: usize = 1024 * 1024;
pub const RESOLVE_TIMEOUT_MS: u64 = 15_000;
pub const SPAWN_ERRORS: [&str; 4] = ["preset", "cwd", "limit", "spawn"];

#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub rid: String,
    pub id: String,
    pub file: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: String,
    pub agent_session: Option<String>,
    pub si: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MainToCore {
    Init { token: String, origins: Vec<String>, app_version: String },
    Resolved(Resolved),
    ResolveFailed { rid: String, message: String },
    CwdOk { rid: String, cwd: Option<String> },
    WtReply { rid: String, reply: Value },
    Branch { id: String, branch: Option<String> },
    Census,
    KillAll,
    Shutdown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CoreToMain {
    Listening { port: u16 },
    Resolve { rid: String, msg: Value },
    Spawned { id: String, pid: u32 },
    Pid { id: String, pid: u32 },
    Exited { id: String, code: i64 },
    Removed { id: String },
    Cwd { rid: String, id: String, cwd: String },
    Wt { rid: String, msg: Value },
    CensusReply { sessions: Vec<Value> },
}

pub fn is_id(v: &str) -> bool {
    !v.is_empty() && v.chars().count() <= 128
}

fn id_of(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str).filter(|s| is_id(s)).map(str::to_owned)
}

fn str_of(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str).map(str::to_owned)
}

fn str_or_null(v: Option<&Value>) -> Option<Option<String>> {
    match v {
        Some(Value::Null) => Some(None),
        Some(Value::String(s)) => Some(Some(s.clone())),
        _ => None,
    }
}

fn opt_str(v: Option<&Value>) -> Option<Option<String>> {
    match v {
        None => Some(None),
        Some(Value::String(s)) => Some(Some(s.clone())),
        _ => None,
    }
}

fn str_vec(v: Option<&Value>) -> Option<Vec<String>> {
    v?.as_array()?.iter().map(|x| x.as_str().map(str::to_owned)).collect()
}

fn env_of(v: Option<&Value>) -> Option<Vec<(String, String)>> {
    v?.as_object()?.iter().map(|(k, x)| x.as_str().map(|s| (k.clone(), s.to_owned()))).collect()
}

pub fn parse_value(raw: &Value) -> Option<MainToCore> {
    let o = raw.as_object()?;
    match o.get("t")?.as_str()? {
        "init" => Some(MainToCore::Init {
            token: str_of(o.get("token"))?,
            origins: str_vec(o.get("origins"))?,
            app_version: str_of(o.get("appVersion"))?,
        }),
        "resolved" => parse_resolved(o),
        "cwd-ok" => Some(MainToCore::CwdOk { rid: id_of(o.get("rid"))?, cwd: str_or_null(o.get("cwd"))? }),
        "wt-reply" => {
            let rid = id_of(o.get("rid"))?;
            let reply = o.get("reply")?;
            reply.get("t")?.as_str().filter(|t| t.starts_with("wt."))?;
            Some(MainToCore::WtReply { rid, reply: reply.clone() })
        }
        "branch" => Some(MainToCore::Branch { id: id_of(o.get("id"))?, branch: str_or_null(o.get("branch"))? }),
        "census" => Some(MainToCore::Census),
        "kill-all" => Some(MainToCore::KillAll),
        "shutdown" => Some(MainToCore::Shutdown),
        _ => None,
    }
}

fn parse_resolved(o: &Map<String, Value>) -> Option<MainToCore> {
    let rid = id_of(o.get("rid"))?;
    match o.get("ok")? {
        Value::Bool(false) => {
            let message = o.get("message")?.as_str()?;
            SPAWN_ERRORS.contains(&message).then(|| MainToCore::ResolveFailed { rid, message: message.to_owned() })
        }
        Value::Bool(true) => Some(MainToCore::Resolved(Resolved {
            rid,
            id: id_of(o.get("id"))?,
            file: str_of(o.get("file"))?,
            args: str_vec(o.get("args"))?,
            env: env_of(o.get("env"))?,
            cwd: str_of(o.get("cwd"))?,
            agent_session: opt_str(o.get("agentSession"))?,
            si: opt_str(o.get("si"))?,
        })),
        _ => None,
    }
}

pub fn parse_line(line: &str) -> Option<MainToCore> {
    if line.len() > LINE_MAX {
        return None;
    }
    let v: Value = serde_json::from_str(line.trim_end_matches('\r')).ok()?;
    parse_value(&v)
}

impl CoreToMain {
    pub fn to_value(&self) -> Value {
        match self {
            CoreToMain::Listening { port } => json!({ "t": "listening", "port": port }),
            CoreToMain::Resolve { rid, msg } => json!({ "t": "resolve", "rid": rid, "msg": msg }),
            CoreToMain::Spawned { id, pid } => json!({ "t": "spawned", "id": id, "pid": pid }),
            CoreToMain::Pid { id, pid } => json!({ "t": "pid", "id": id, "pid": pid }),
            CoreToMain::Exited { id, code } => json!({ "t": "exited", "id": id, "code": code }),
            CoreToMain::Removed { id } => json!({ "t": "removed", "id": id }),
            CoreToMain::Cwd { rid, id, cwd } => json!({ "t": "cwd", "rid": rid, "id": id, "cwd": cwd }),
            CoreToMain::Wt { rid, msg } => json!({ "t": "wt", "rid": rid, "msg": msg }),
            CoreToMain::CensusReply { sessions } => json!({ "t": "census-reply", "sessions": sessions }),
        }
    }
}

pub fn encode(msg: &CoreToMain) -> String {
    let mut s = msg.to_value().to_string();
    s.push('\n');
    s
}

#[derive(Default)]
pub struct LineSplitter {
    buf: Vec<u8>,
    skipping: bool,
}

impl LineSplitter {
    pub fn push(&mut self, chunk: &[u8]) -> Vec<Option<MainToCore>> {
        let mut out = Vec::new();
        let mut rest = chunk;
        while let Some(nl) = rest.iter().position(|b| *b == b'\n') {
            let part = &rest[..nl];
            rest = &rest[nl + 1..];
            if self.skipping || self.buf.len() + part.len() > LINE_MAX {
                self.buf.clear();
                self.skipping = false;
                out.push(None);
                continue;
            }
            self.buf.extend_from_slice(part);
            let line = std::mem::take(&mut self.buf);
            match std::str::from_utf8(&line) {
                Ok(s) if s.trim().is_empty() => {}
                Ok(s) => out.push(parse_line(s)),
                Err(_) => out.push(None),
            }
        }
        if self.skipping {
            return out;
        }
        if self.buf.len() + rest.len() > LINE_MAX {
            self.buf.clear();
            self.skipping = true;
        } else {
            self.buf.extend_from_slice(rest);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../../../desktop/test/fixtures/pty-control.json");

    #[test]
    fn parses_init() {
        let m = parse_line(r#"{"t":"init","token":"abc","origins":["http://127.0.0.1:4317"],"appVersion":"3.4.0"}"#);
        assert_eq!(
            m,
            Some(MainToCore::Init {
                token: "abc".into(),
                origins: vec!["http://127.0.0.1:4317".into()],
                app_version: "3.4.0".into()
            })
        );
    }

    #[test]
    fn unknown_or_broken_lines_are_dropped() {
        assert_eq!(parse_line(r#"{"t":"future-thing"}"#), None);
        assert_eq!(parse_line("not json"), None);
    }

    #[test]
    fn encodes_listening() {
        assert_eq!(encode(&CoreToMain::Listening { port: 4242 }), "{\"t\":\"listening\",\"port\":4242}\n");
    }

    #[test]
    fn fixture_main_to_core_all_parse_and_invalid_are_refused() {
        let v: Value = serde_json::from_str(FIXTURE).unwrap();
        for msg in v["mainToCore"].as_array().unwrap() {
            assert!(parse_value(msg).is_some(), "valid fixture refused: {msg}");
        }
        for msg in v["invalidMainToCore"].as_array().unwrap() {
            assert!(parse_value(msg).is_none(), "invalid fixture accepted: {msg}");
        }
        let resolved = v["mainToCore"].as_array().unwrap().iter().find_map(|m| match parse_value(m) {
            Some(MainToCore::Resolved(r)) if r.si.is_some() => Some(r),
            _ => None,
        });
        let r = resolved.unwrap();
        assert_eq!(r.env, vec![("ORRERY_SI_NONCE".to_owned(), "00112233445566778899aabbccddeeff".to_owned())]);
        assert_eq!(r.agent_session, None);
    }

    #[test]
    fn env_keeps_main_order() {
        let m = parse_line(r#"{"t":"resolved","rid":"r","ok":true,"id":"x","file":"f","args":[],"env":{"Z":"1","A":"2","m":"3"},"cwd":"/"}"#);
        let Some(MainToCore::Resolved(r)) = m else { panic!() };
        let keys: Vec<_> = r.env.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, ["Z", "A", "m"]);
    }

    #[test]
    fn core_to_main_shapes_match_fixture() {
        let v: Value = serde_json::from_str(FIXTURE).unwrap();
        let fixture = v["coreToMain"].as_array().unwrap();
        let ours = [
            CoreToMain::Listening { port: 50123 },
            CoreToMain::Spawned { id: "6f1c1d2e-0000-4000-8000-000000000001".into(), pid: 4242 },
            CoreToMain::Exited { id: "6f1c1d2e-0000-4000-8000-000000000002".into(), code: 3221225794 },
            CoreToMain::Exited { id: "6f1c1d2e-0000-4000-8000-000000000002".into(), code: -1073741502 },
            CoreToMain::Removed { id: "6f1c1d2e-0000-4000-8000-000000000001".into() },
            CoreToMain::Cwd { rid: "c-1".into(), id: "6f1c1d2e-0000-4000-8000-000000000001".into(), cwd: "/Users/me/work/app/src".into() },
        ];
        for m in ours {
            assert!(fixture.contains(&m.to_value()), "not in fixture: {}", m.to_value());
        }
    }

    #[test]
    fn splitter_handles_chunks_cr_blank_and_oversize() {
        let mut s = LineSplitter::default();
        assert!(s.push(b"{\"t\":\"cen").is_empty());
        assert_eq!(s.push(b"sus\"}\r\n\n{\"t\":\"kill-all\"}\n"), vec![Some(MainToCore::Census), Some(MainToCore::KillAll)]);
        let big = vec![b'x'; LINE_MAX + 10];
        assert!(s.push(&big).is_empty());
        assert_eq!(s.push(b"tail\n{\"t\":\"shutdown\"}\n"), vec![None, Some(MainToCore::Shutdown)]);
    }
}
