use super::jsx::{locale_compare, s, slice};
use serde_json::{json, Map, Value};
use std::io::Read;
use std::time::Duration;

pub const API_BASE: &str = "https://api.telegram.org";

#[derive(Clone, Debug)]
pub enum Part {
    Text(String),
    File { bytes: Vec<u8>, name: String, mime: Option<&'static str> },
}

#[derive(Clone, Debug)]
pub enum Body {
    Json(Map<String, Value>),
    Form(Vec<(String, Part)>),
}

impl Body {
    pub fn field(&self, name: &str) -> Option<String> {
        match self {
            Body::Json(m) => match m.get(name)? {
                Value::String(s) => Some(s.clone()),
                Value::Number(n) => Some(crate::js::number(n.as_f64().unwrap_or(0.0))),
                _ => None,
            },
            Body::Form(parts) => parts.iter().find(|(k, _)| k == name).and_then(|(_, p)| match p {
                Part::Text(t) => Some(t.clone()),
                Part::File { .. } => None,
            }),
        }
    }
}

pub fn form(fields: &Map<String, Value>) -> Vec<(String, Part)> {
    fields
        .iter()
        .map(|(k, v)| {
            let t = match v {
                Value::String(s) => s.clone(),
                other => crate::js::stringify(other),
            };
            (k.clone(), Part::Text(t))
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tg {
    Ok(Value),
    Err(&'static str, Option<f64>),
}

impl Tg {
    pub fn ok(&self) -> bool {
        matches!(self, Tg::Ok(_))
    }

    pub fn error(&self) -> Option<&'static str> {
        match self {
            Tg::Err(e, _) => Some(e),
            Tg::Ok(_) => None,
        }
    }

    pub fn result(&self) -> Value {
        match self {
            Tg::Ok(v) => v.clone(),
            Tg::Err(..) => Value::Null,
        }
    }
}

pub trait Transport: Send + Sync {
    fn post(&self, url: &str, body: &Body, timeout_ms: u64) -> Result<(u16, Vec<u8>), &'static str>;
    fn get(&self, url: &str, timeout_ms: u64, max: u64) -> Result<(u16, Vec<u8>), &'static str>;
}

pub struct Ureq;

fn agent(timeout_ms: u64) -> ureq::Agent {
    let tls = ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build();
    ureq::Agent::config_builder().timeout_global(Some(Duration::from_millis(timeout_ms))).http_status_as_error(false).tls_config(tls).build().into()
}

fn err_of(e: &ureq::Error) -> &'static str {
    match e {
        ureq::Error::Timeout(_) => "timeout",
        ureq::Error::Io(io) if io.kind() == std::io::ErrorKind::TimedOut => "timeout",
        _ => "network",
    }
}

fn read_all(mut res: ureq::http::Response<ureq::Body>, max: u64) -> Result<(u16, Vec<u8>), &'static str> {
    let status = res.status().as_u16();
    let mut bytes = Vec::new();
    let mut reader = res.body_mut().with_config().limit(max).reader();
    match reader.read_to_end(&mut bytes) {
        Ok(_) => Ok((status, bytes)),
        Err(e) if e.kind() == std::io::ErrorKind::TimedOut || e.to_string().to_lowercase().contains("timeout") => Err("timeout"),
        Err(_) => Err("network"),
    }
}

fn crlf(t: &str) -> String {
    let mut out = String::with_capacity(t.len());
    let mut chars = t.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push_str("\r\n");
            }
            '\n' => out.push_str("\r\n"),
            other => out.push(other),
        }
    }
    out
}

pub fn multipart(parts: &[(String, Part)]) -> (String, Vec<u8>) {
    let boundary = format!("----orrery{}", crate::util::uuid().replace('-', ""));
    let mut out = Vec::new();
    for (name, p) in parts {
        out.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
        match p {
            Part::Text(t) => {
                out.extend_from_slice(format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n").as_bytes());
                out.extend_from_slice(crlf(t).as_bytes());
            }
            Part::File { bytes, name: file, mime } => {
                let safe = file.replace(['"', '\r', '\n'], "_");
                out.extend_from_slice(format!("Content-Disposition: form-data; name=\"{name}\"; filename=\"{safe}\"\r\nContent-Type: {}\r\n\r\n", mime.unwrap_or("application/octet-stream")).as_bytes());
                out.extend_from_slice(bytes);
            }
        }
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
    (format!("multipart/form-data; boundary={boundary}"), out)
}

impl Transport for Ureq {
    fn post(&self, url: &str, body: &Body, timeout_ms: u64) -> Result<(u16, Vec<u8>), &'static str> {
        let a = agent(timeout_ms);
        let res = match body {
            Body::Json(m) => a.post(url).header("content-type", "application/json").send(crate::js::stringify(&Value::Object(m.clone()))),
            Body::Form(parts) => {
                let (ct, bytes) = multipart(parts);
                a.post(url).header("content-type", &ct).send(&bytes[..])
            }
        };
        read_all(res.map_err(|e| err_of(&e))?, 64 * 1024 * 1024)
    }

    fn get(&self, url: &str, timeout_ms: u64, max: u64) -> Result<(u16, Vec<u8>), &'static str> {
        let res = agent(timeout_ms).get(url).call().map_err(|e| err_of(&e))?;
        read_all(res, max)
    }
}

pub fn error_of(status: u16, body: &Value) -> &'static str {
    let desc = s(body, "description").map(str::to_lowercase).unwrap_or_default();
    match status {
        401 | 404 => "token",
        429 => "rate",
        409 => "conflict",
        403 => "chat",
        _ if ["chat not found", "message to edit not found", "message can't be edited", "thread not found", "not enough rights"].iter().any(|d| desc.contains(d)) => "chat",
        413 => "too-large",
        _ => "status",
    }
}

pub fn tg_call(http: &dyn Transport, base: &str, token: &str, method: &str, body: &Body, timeout_ms: u64) -> Tg {
    let url = format!("{}/bot{token}/{method}", base.trim_end_matches('/'));
    let (status, bytes) = match http.post(&url, body, timeout_ms) {
        Ok(r) => r,
        Err(e) => return Tg::Err(e, None),
    };
    let json: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    if (200..300).contains(&status) && json.is_object() && json.get("ok") == Some(&Value::Bool(true)) {
        return Tg::Ok(json.get("result").cloned().unwrap_or(Value::Null));
    }
    let retry = json.get("parameters").filter(|p| p.is_object()).and_then(|p| p.get("retry_after")).and_then(Value::as_f64).filter(|r| *r != 0.0);
    Tg::Err(error_of(status, &json), retry)
}

#[derive(Default, Clone)]
pub struct Seen {
    pub chats: Vec<(String, (String, bool))>,
    pub topics: Vec<(String, Vec<(i64, String)>)>,
}

impl Seen {
    pub fn set_chat(&mut self, id: &str, title: &str, forum: bool) {
        match self.chats.iter_mut().find(|(k, _)| k == id) {
            Some(slot) => slot.1 = (title.to_string(), forum),
            None => self.chats.push((id.to_string(), (title.to_string(), forum))),
        }
    }

    pub fn chat(&self, id: &str) -> Option<&(String, bool)> {
        self.chats.iter().find(|(k, _)| k == id).map(|(_, v)| v)
    }

    pub fn remove(&mut self, id: &str) {
        self.chats.retain(|(k, _)| k != id);
        self.topics.retain(|(k, _)| k != id);
    }

    pub fn topics_mut(&mut self, id: &str) -> &mut Vec<(i64, String)> {
        if let Some(pos) = self.topics.iter().position(|(k, _)| k == id) {
            return &mut self.topics[pos].1;
        }
        self.topics.push((id.to_string(), Vec::new()));
        &mut self.topics.last_mut().unwrap().1
    }

    pub fn set_topics(&mut self, id: &str, list: Vec<(i64, String)>) {
        *self.topics_mut(id) = list;
    }

    pub fn topics(&self, id: &str) -> Option<&Vec<(i64, String)>> {
        self.topics.iter().find(|(k, _)| k == id).map(|(_, v)| v)
    }
}

pub fn topic_set(list: &mut Vec<(i64, String)>, id: i64, name: String) {
    match list.iter_mut().find(|(k, _)| *k == id) {
        Some(slot) => slot.1 = name,
        None => list.push((id, name)),
    }
}

pub fn scan_updates(updates: &Value, into: &mut Seen) {
    let Some(list) = updates.as_array() else { return };
    for u in list.iter().filter(|u| u.is_object()) {
        let member = u.get("my_chat_member").filter(|m| m.is_object());
        let msg = ["message", "edited_message", "channel_post"].iter().find_map(|k| u.get(*k).filter(|m| m.is_object()));
        let chat = member.and_then(|m| m.get("chat")).filter(|c| c.is_object()).or_else(|| msg.and_then(|m| m.get("chat")).filter(|c| c.is_object()));
        let Some(chat) = chat else { continue };
        if !matches!(s(chat, "type"), Some("supergroup") | Some("group")) {
            continue;
        }
        let Some(id) = chat.get("id").and_then(Value::as_f64).map(crate::js::number) else { continue };
        if let Some(st) = member.and_then(|m| m.get("new_chat_member")).filter(|n| n.is_object()).and_then(|n| s(n, "status")) {
            if st == "left" || st == "kicked" {
                into.remove(&id);
                continue;
            }
        }
        let title = s(chat, "title").map(|t| slice(t, 120)).unwrap_or_else(|| id.clone());
        into.set_chat(&id, &title, chat.get("is_forum") == Some(&Value::Bool(true)));
        let Some(msg) = msg else { continue };
        let topics = into.topics_mut(&id);
        let obj = |v: Option<&Value>| v.filter(|x| x.is_object()).cloned();
        let created = obj(msg.get("forum_topic_created"));
        let reply = obj(msg.get("reply_to_message"));
        let from_reply = reply.as_ref().and_then(|r| obj(r.get("forum_topic_created")));
        let edited = obj(msg.get("forum_topic_edited"));
        let thread = if msg.get("is_topic_message") == Some(&Value::Bool(true)) { msg.get("message_thread_id").and_then(Value::as_f64).map(|t| t as i64) } else { None };
        let Some(thread) = thread else { continue };
        let name = [created, edited, from_reply].iter().find_map(|c| c.as_ref().and_then(|c| s(c, "name")).filter(|n| !n.is_empty()).map(str::to_string));
        match name {
            Some(n) => topic_set(topics, thread, slice(&n, 128)),
            None if !topics.iter().any(|(k, _)| *k == thread) => topics.push((thread, format!("#{thread}"))),
            None => {}
        }
    }
}

#[derive(Clone, Default)]
pub struct Rights {
    pub bot_admin: bool,
    pub can_manage: bool,
    pub title: Option<String>,
    pub is_forum: Option<bool>,
}

pub fn chats_from(seen: &Seen, rights: &[(String, Rights)]) -> Vec<Value> {
    seen.chats
        .iter()
        .map(|(id, (title, forum))| {
            let r = rights.iter().find(|(k, _)| k == id).map(|(_, r)| r);
            let mut topics: Vec<(i64, String)> = seen.topics(id).cloned().unwrap_or_default();
            topics.sort_by(|a, b| locale_compare(&a.1, &b.1));
            json!({
                "chatId": id,
                "title": r.and_then(|r| r.title.clone()).unwrap_or_else(|| title.clone()),
                "isForum": r.and_then(|r| r.is_forum).unwrap_or(*forum),
                "botAdmin": r.map(|r| r.bot_admin).unwrap_or(false),
                "canManageTopics": r.map(|r| r.can_manage).unwrap_or(false),
                "topics": topics.into_iter().map(|(t, n)| json!({"threadId": t, "name": n})).collect::<Vec<_>>(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn errors_and_scan() {
        assert_eq!(error_of(400, &json!({"description": "Bad Request: chat not found"})), "chat");
        assert_eq!(error_of(429, &Value::Null), "rate");
        assert_eq!(error_of(400, &Value::Null), "status");
        let mut seen = Seen::default();
        scan_updates(&json!([
            {"message": {"chat": {"id": -1001, "type": "supergroup", "title": "G", "is_forum": true}, "is_topic_message": true, "message_thread_id": 5, "forum_topic_created": {"name": "Build"}}},
            {"message": {"chat": {"id": -1001, "type": "supergroup", "title": "G", "is_forum": true}, "is_topic_message": true, "message_thread_id": 9}},
            {"my_chat_member": {"chat": {"id": -1002, "type": "group", "title": "H"}, "new_chat_member": {"status": "member"}}},
            {"my_chat_member": {"chat": {"id": -1002, "type": "group", "title": "H"}, "new_chat_member": {"status": "left"}}},
            {"message": {"chat": {"id": 7, "type": "private"}}}
        ]), &mut seen);
        let chats = chats_from(&seen, &[]);
        assert_eq!(chats.len(), 1);
        assert_eq!(chats[0]["topics"], json!([{"threadId": 9, "name": "#9"}, {"threadId": 5, "name": "Build"}]));
    }
}
