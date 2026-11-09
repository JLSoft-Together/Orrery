use crate::jsstr::{js_slice, js_trim, units};
use crate::signal::Signal;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

const UDS_PREFIX: &str = "uds:";
const MAX_SOCKET_PATH: usize = 300;

fn all_in(s: &str, min: usize, max: usize, ok: impl Fn(u8) -> bool) -> bool {
    (min..=max).contains(&s.len()) && s.bytes().all(ok)
}

pub fn agent_id(s: &str) -> bool {
    s.len() == 17 && s.starts_with('a') && s[1..].bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn id_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'-'
}

pub fn record_id(v: Option<&Value>) -> Option<String> {
    v?.as_str().filter(|s| all_in(s, 1, 128, id_byte)).map(str::to_owned)
}

pub fn msg_id(v: Option<&Value>) -> Option<String> {
    v?.as_str().filter(|s| all_in(s, 8, 64, id_byte)).map(str::to_owned)
}

pub fn effort_ok(s: &str) -> bool {
    all_in(s, 2, 12, |b| b.is_ascii_lowercase())
}

pub fn window_ok(s: &str) -> bool {
    all_in(s, 1, 40, |b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
}

fn hex(b: u8) -> Option<u8> {
    (b as char).to_digit(16).map(|d| d as u8)
}

pub fn decode_uri_component(raw: &str) -> Option<String> {
    let b = raw.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hi = hex(*b.get(i + 1)?)?;
            let lo = hex(*b.get(i + 2)?)?;
            out.push(hi * 16 + lo);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

pub fn socket_key(path: &str) -> Option<String> {
    let n = units(path);
    if n == 0 || n > MAX_SOCKET_PATH {
        return None;
    }
    let digest = Sha256::digest(path.to_lowercase().as_bytes());
    Some(digest.iter().take(8).map(|b| format!("{b:02x}")).collect())
}

pub fn origin_signals(origin: Option<&Value>, id: Option<String>, at: Option<&String>) -> Vec<Signal> {
    let key = id.or_else(|| at.filter(|a| !a.is_empty()).map(|a| format!("@{a}")));
    let (Some(o), Some(key)) = (origin.and_then(Value::as_object), key) else { return Vec::new() };
    let at = at.cloned();
    let kind = o.get("kind").and_then(Value::as_str);
    if kind == Some("coordinator") {
        return vec![Signal::CoordinatorMsg { key, at }];
    }
    let from = o.get("from").and_then(Value::as_str);
    if kind == Some("peer") && o.get("handback") == Some(&Value::Bool(true)) {
        if let Some(f) = from.filter(|f| agent_id(f)) {
            return vec![Signal::Handback { from: f.to_owned(), key, at }];
        }
    }
    if kind == Some("peer") {
        if let Some(rest) = from.and_then(|f| f.strip_prefix(UDS_PREFIX)) {
            let addr = decode_uri_component(rest).unwrap_or_else(|| rest.to_owned());
            if let Some(from_key) = socket_key(&addr) {
                return vec![Signal::PeerIn { from_key, msg_id: msg_id(o.get("msg_id")), dedupe_key: key, at }];
            }
        }
    }
    Vec::new()
}

pub fn peer_sent(result: Option<&Value>, at: Option<&String>) -> Vec<Signal> {
    let Some(r) = result.and_then(Value::as_object) else { return Vec::new() };
    if r.get("success") != Some(&Value::Bool(true)) || r.contains_key("pin") || r.contains_key("resumedAgentId") {
        return Vec::new();
    }
    msg_id(r.get("msg_id")).map_or_else(Vec::new, |msg_id| vec![Signal::PeerSent { msg_id, at: at.cloned() }])
}

const OPEN: &str = "<task-notification>";
const CLOSE: &str = "</task-notification>";

fn tag(body: &str, name: &str) -> Option<String> {
    let open = format!("<{name}>");
    let close = format!("</{name}>");
    let mut from = 0;
    while let Some(i) = body[from..].find(&open) {
        let start = from + i + open.len();
        let end = body[start..].find('<').map_or(body.len(), |j| start + j);
        let n = units(&body[start..end]);
        if (1..=200).contains(&n) && body[end..].starts_with(&close) {
            return Some(js_trim(&body[start..end]).to_owned());
        }
        from += i + 1;
    }
    None
}

pub fn task_notifications(text: &str, at: Option<&String>) -> Vec<Signal> {
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(i) = text[from..].find(OPEN) {
        let start = from + i + OPEN.len();
        let Some(j) = text[start..].find(CLOSE) else { break };
        let body = &text[start..start + j];
        from = start + j + CLOSE.len();
        let Some(status) = tag(body, "status").filter(|s| !s.is_empty()) else { continue };
        out.push(Signal::TaskNotification { tool_use_id: tag(body, "tool-use-id"), task_id: tag(body, "task-id"), status, at: at.cloned() });
    }
    out
}

fn one_line(v: &str, max: usize) -> String {
    let mut s = String::with_capacity(v.len());
    let mut run = false;
    for c in v.chars() {
        if c <= '\u{1f}' || c == '\u{7f}' {
            if !run {
                s.push(' ');
            }
            run = true;
        } else {
            s.push(c);
            run = false;
        }
    }
    js_slice(js_trim(&s), max).to_owned()
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ask {
    pub question: String,
    pub options: Vec<String>,
    pub multi: bool,
    pub more: bool,
}

impl Ask {
    pub fn to_json(&self) -> Value {
        let mut m = Map::new();
        m.insert("question".into(), Value::from(self.question.as_str()));
        m.insert("options".into(), Value::from(self.options.clone()));
        m.insert("multi".into(), Value::Bool(self.multi));
        if self.more {
            m.insert("more".into(), Value::Bool(true));
        }
        Value::Object(m)
    }
}

pub fn ask_of(name: &str, input: &Value) -> Option<Ask> {
    if name != "AskUserQuestion" {
        return None;
    }
    let q = input.as_object()?.get("questions")?.as_array()?.first()?.as_object()?;
    let opts = q.get("options")?.as_array()?;
    let question = one_line(q.get("question")?.as_str()?, 200);
    if question.is_empty() {
        return None;
    }
    let mut options = Vec::with_capacity(opts.len());
    for o in opts {
        let label = one_line(o.as_object()?.get("label")?.as_str()?, 60);
        if label.is_empty() {
            return None;
        }
        options.push(label);
    }
    if options.is_empty() {
        return None;
    }
    let more = options.len() > 6;
    options.truncate(6);
    Some(Ask { question, options, multi: q.get("multiSelect") == Some(&Value::Bool(true)), more })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socket_key_matches_node_sha256() {
        assert_eq!(socket_key("abc").unwrap(), "ba7816bf8f01cfea");
        assert_eq!(socket_key("ABC").unwrap(), "ba7816bf8f01cfea");
        assert!(socket_key("").is_none());
        assert!(socket_key(&"x".repeat(301)).is_none());
    }

    #[test]
    fn decodes_like_decode_uri_component() {
        assert_eq!(decode_uri_component("/tmp/cc%20msg.sock").unwrap(), "/tmp/cc msg.sock");
        assert!(decode_uri_component("/tmp/%E0%A4%A.sock").is_none());
        assert!(decode_uri_component("%ED%A0%80").is_none());
        assert!(decode_uri_component("%zz").is_none());
    }

    #[test]
    fn reads_notification_tags() {
        let s = task_notifications("<task-notification><status> done </status><tool-use-id>t1</tool-use-id></task-notification><task-notification><status>   </status></task-notification>", None);
        assert_eq!(s, vec![Signal::TaskNotification { tool_use_id: Some("t1".into()), task_id: None, status: "done".into(), at: None }]);
        assert_eq!(tag("<status>a<b></status><status>ok</status>", "status").as_deref(), Some("ok"));
        assert_eq!(tag(&format!("<status>{}</status>", "x".repeat(201)), "status"), None);
    }
}
