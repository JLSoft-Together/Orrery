use super::jsx::{bad_path, collapse, digits, int_of, is_bidi, is_ctl, is_ctl_c1, len, s, slice, trim};
use serde_json::{json, Map, Value};

pub const DONE_CHOICES: [i64; 5] = [0, 1, 3, 5, 10];
pub const DEFAULT_DONE_MIN: i64 = 1;
pub const ROUTES_MAX: usize = 100;
pub const SESSION_ROUTES_MAX: usize = 50;
pub const PANE_LINKS_MAX: usize = 100;
pub const MEMBERS_MAX: usize = 20;
pub const STRANGER_MAX: usize = 300;
pub const NAME_MAX: usize = 80;
pub const INBOUND_MAX: usize = 2000;
pub const TAIL_DEFAULT: i64 = 30;
pub const TAIL_MAX: i64 = 80;
pub use super::parse::*;

pub const FILE_MAX: u64 = 50 * 1024 * 1024;
pub const BIG_FILE_MAX: u64 = 2000 * 1024 * 1024;

fn word_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn username(v: &str) -> bool {
    let b = v.as_bytes();
    (4..=32).contains(&b.len()) && b[0].is_ascii_alphabetic() && b.iter().all(|c| word_char(*c))
}

pub fn chat_id_ok(v: &str) -> bool {
    if let Some(rest) = v.strip_prefix('-') {
        return digits(rest, 5, 20);
    }
    v.strip_prefix('@').is_some_and(username)
}

pub fn chat_id(v: &Value) -> Option<&str> {
    v.as_str().filter(|s| chat_id_ok(s))
}

pub fn thread_id(v: &Value) -> Option<i64> {
    let n = v.as_f64()?;
    (n.fract() == 0.0 && n > 0.0 && n < 2_147_483_648.0).then_some(n as i64)
}

pub fn session_id_ok(v: &str) -> bool {
    let b = v.as_bytes();
    b.len() == 36 && b.iter().enumerate().all(|(i, c)| if [8, 13, 18, 23].contains(&i) { *c == b'-' } else { c.is_ascii_hexdigit() })
}

pub fn session_id(v: &Value) -> Option<&str> {
    v.as_str().filter(|s| session_id_ok(s))
}

pub fn token_ok(v: &str) -> bool {
    let Some((a, b)) = v.split_once(':') else { return false };
    digits(a, 5, 15) && (30..=64).contains(&b.len()) && b.bytes().all(|c| word_char(c) || c == b'-')
}

pub fn user_id(v: &Value) -> Option<i64> {
    int_of(v).filter(|n| *n > 0)
}

pub fn is_hhmm(v: &Value) -> Option<&str> {
    let t = v.as_str()?;
    let b = t.as_bytes();
    let ok = b.len() == 5 && b[2] == b':' && b[0].is_ascii_digit() && b[1].is_ascii_digit() && b[3].is_ascii_digit() && b[4].is_ascii_digit();
    let ok = ok && ((b[0] <= b'1') || (b[0] == b'2' && b[1] <= b'3')) && b[3] <= b'5';
    ok.then_some(t)
}

pub fn hhmm_minute(v: &str) -> i64 {
    v[..2].parse::<i64>().unwrap_or(0) * 60 + v[3..].parse::<i64>().unwrap_or(0)
}

pub fn in_quiet(q: &Value, minute: i64) -> bool {
    if q.get("enabled") != Some(&Value::Bool(true)) {
        return false;
    }
    let from = hhmm_minute(s(q, "from").unwrap_or("00:00"));
    let to = hhmm_minute(s(q, "to").unwrap_or("00:00"));
    if from == to {
        return false;
    }
    if from < to {
        minute >= from && minute < to
    } else {
        minute >= from || minute < to
    }
}

pub fn parse_topic_link(raw: &str) -> Option<(String, Option<i64>)> {
    let text = trim(raw);
    let num = |s: Option<&str>| s.filter(|s| digits(s, 1, 10)).map(|s| s.parse::<i64>().unwrap_or(0));
    let (head, thread) = match text.split_once(':') {
        Some((h, t)) if !h.contains('/') => (h, Some(t)),
        _ => (text, None),
    };
    let pair_head = head.strip_prefix("-100").is_some_and(|r| digits(r, 5, 15)) || head.strip_prefix('@').is_some_and(username);
    if pair_head && thread.is_none_or(|t| digits(t, 1, 10)) {
        return Some((head.to_string(), num(thread)));
    }
    let lower = text.to_ascii_lowercase();
    let mut rest = text;
    for p in ["https://", "http://"] {
        if lower.starts_with(p) {
            rest = &text[p.len()..];
            break;
        }
    }
    if rest.to_ascii_lowercase().starts_with("www.") {
        rest = &rest[4..];
    }
    let low = rest.to_ascii_lowercase();
    let after = if low.starts_with("t.me/") {
        &rest[5..]
    } else if low.starts_with("telegram.me/") {
        &rest[12..]
    } else {
        return None;
    };
    if after.chars().any(char::is_whitespace) {
        return None;
    }
    let (before_hash, _) = after.split_once('#').unwrap_or((after, ""));
    let (path, query) = before_hash.split_once('?').unwrap_or((before_hash, ""));
    let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
    let thread_q = query.split('&').find_map(|kv| kv.strip_prefix("thread=")).filter(|v| digits(v, 1, 10));
    if parts.first() == Some(&"c") {
        let id = parts.get(1).filter(|p| digits(p, 5, 15))?;
        let chat = format!("-100{id}");
        return match parts.len() {
            n if n >= 4 => (num(parts.get(2).copied()).is_some() && num(parts.get(3).copied()).is_some()).then(|| (chat, num(parts.get(2).copied()))),
            3 => num(parts.get(2).copied()).is_some().then(|| (chat, num(thread_q))),
            2 => Some((chat, None)),
            _ => None,
        };
    }
    let first = parts.first().filter(|p| username(p))?;
    let chat = format!("@{first}");
    match parts.len() {
        n if n >= 3 => (num(parts.get(1).copied()).is_some() && num(parts.get(2).copied()).is_some()).then(|| (chat, num(parts.get(1).copied()))),
        2 => num(parts.get(1).copied()).is_some().then(|| (chat, num(thread_q))),
        _ => Some((chat, None)),
    }
}

fn one_name(raw: &str, max: usize) -> String {
    slice(trim(&collapse(raw, |c| is_ctl_c1(c) || is_bidi(c), " ")), max)
}

pub fn parse_members(raw: &Value) -> Option<Vec<Value>> {
    let list = raw.as_array().filter(|l| l.len() <= MEMBERS_MAX)?;
    let mut out: Vec<Value> = Vec::new();
    for m in list {
        let id = user_id(&m["id"])?;
        let name = m.get("name").and_then(Value::as_str)?;
        let level = m.get("level").and_then(Value::as_str).filter(|l| *l == "read" || *l == "full")?;
        if out.iter().any(|x| x["id"].as_i64() == Some(id)) {
            return None;
        }
        let name = one_name(name, NAME_MAX);
        let name = if name.is_empty() { id.to_string() } else { name };
        out.push(json!({"id": id, "name": name, "level": level}));
    }
    Some(out)
}

pub fn sanitize_patch(raw: &Value) -> Option<Map<String, Value>> {
    let r = raw.as_object()?;
    let mut out = Map::new();
    for k in ["enabled", "includeReply", "digest", "waiting", "receiving", "alerts", "pinStatus", "topicIcons", "bigFiles"] {
        match r.get(k) {
            None => continue,
            Some(Value::Bool(b)) => {
                out.insert(k.into(), json!(b));
            }
            Some(_) => return None,
        }
    }
    if let Some(v) = r.get("doneMinutes") {
        let n = int_of(v).filter(|n| DONE_CHOICES.contains(n) && v.as_f64() == Some(*n as f64))?;
        out.insert("doneMinutes".into(), json!(n));
    }
    if let Some(v) = r.get("token") {
        let t = v.as_str().filter(|t| len(t) <= 200)?;
        out.insert("token".into(), json!(trim(t)));
    }
    if let Some(v) = r.get("apiId") {
        if v.is_null() {
            out.insert("apiId".into(), Value::Null);
        } else {
            let n = int_of(v).filter(|n| *n > 0 && *n < 2_147_483_648)?;
            out.insert("apiId".into(), json!(n));
        }
    }
    if let Some(v) = r.get("apiHash") {
        let h = trim(v.as_str()?);
        if !(h.is_empty() || (h.len() == 32 && h.bytes().all(|b| b.is_ascii_hexdigit()))) {
            return None;
        }
        out.insert("apiHash".into(), json!(h.to_ascii_lowercase()));
    }
    if let Some(v) = r.get("stranger") {
        let enabled = v.get("enabled").and_then(Value::as_bool)?;
        let text = v.get("text").and_then(Value::as_str).filter(|t| len(t) <= STRANGER_MAX)?;
        let text = collapse(text, |c| is_ctl(c) && c != '\n', " ");
        out.insert("stranger".into(), json!({"enabled": enabled, "text": trim(&text)}));
    }
    if let Some(v) = r.get("members") {
        out.insert("members".into(), Value::Array(parse_members(v)?));
    }
    if let Some(v) = r.get("quiet") {
        let enabled = v.get("enabled").and_then(Value::as_bool)?;
        let from = is_hhmm(&v["from"])?;
        let to = is_hhmm(&v["to"])?;
        out.insert("quiet".into(), json!({"enabled": enabled, "from": from, "to": to}));
    }
    if let Some(v) = r.get("summary") {
        let enabled = v.get("enabled").and_then(Value::as_bool)?;
        let at = is_hhmm(&v["at"])?;
        out.insert("summary".into(), json!({"enabled": enabled, "at": at}));
    }
    Some(out)
}

fn project_ok(v: &Value) -> Option<&str> {
    v.as_str().filter(|p| !p.is_empty() && len(p) <= 4096 && !bad_path(p))
}

fn target_of(r: &Map<String, Value>, base: &mut Map<String, Value>) -> Option<()> {
    if let Some(link) = r.get("link").and_then(Value::as_str) {
        if len(link) > 300 {
            return None;
        }
        base.insert("link".into(), json!(link));
        return Some(());
    }
    let chat = chat_id(r.get("chatId").unwrap_or(&Value::Null))?;
    let thread = match r.get("threadId") {
        None | Some(Value::Null) => Value::Null,
        Some(v) => json!(thread_id(v)?),
    };
    base.insert("chatId".into(), json!(chat));
    base.insert("threadId".into(), thread);
    Some(())
}

pub fn sanitize_route_input(raw: &Value) -> Option<Map<String, Value>> {
    let r = raw.as_object()?;
    let project = project_ok(r.get("project")?)?;
    let mut base = Map::new();
    base.insert("project".into(), json!(project));
    if let Some(label) = r.get("label").and_then(Value::as_str) {
        let l = slice(trim(&collapse(label, is_ctl, " ")), NAME_MAX);
        if !l.is_empty() {
            base.insert("label".into(), json!(l));
        }
    }
    target_of(r, &mut base)?;
    Some(base)
}

pub fn sanitize_session_route_input(raw: &Value) -> Option<Map<String, Value>> {
    let r = raw.as_object()?;
    let session = session_id(r.get("session")?)?;
    let project = project_ok(r.get("project")?)?;
    let name = r.get("name").and_then(Value::as_str).map(|n| slice(trim(&collapse(n, is_ctl, " ")), NAME_MAX)).unwrap_or_default();
    let mut base = Map::new();
    base.insert("session".into(), json!(session.to_lowercase()));
    base.insert("name".into(), json!(if name.is_empty() { "Claude".to_string() } else { name }));
    base.insert("project".into(), json!(project));
    if let Some(Value::Bool(k)) = r.get("keep") {
        base.insert("keep".into(), json!(k));
    }
    target_of(r, &mut base)?;
    Some(base)
}

pub fn sanitize_pane_links(raw: &Value) -> Option<Vec<Value>> {
    let list = raw.as_array()?;
    let id = |v: Option<&Value>| v.and_then(Value::as_str).filter(|s| !s.is_empty() && len(s) <= 128 && !s.chars().any(|c| (c as u32) < 0x20)).map(str::to_string);
    Some(
        list.iter()
            .take(PANE_LINKS_MAX)
            .filter_map(|l| {
                let o = l.as_object()?;
                let agent = session_id(o.get("agent")?)?;
                let mut m = Map::new();
                m.insert("agent".into(), json!(agent.to_lowercase()));
                if let Some(p) = id(o.get("pane")) {
                    m.insert("pane".into(), json!(p));
                }
                if let Some(t) = id(o.get("terminal")) {
                    m.insert("terminal".into(), json!(t));
                }
                let name = o.get("name").and_then(Value::as_str).map(|n| slice(trim(&collapse(n, is_ctl, " ")), 80)).unwrap_or_default();
                if !name.is_empty() {
                    m.insert("name".into(), json!(name));
                }
                if let Some(c) = o.get("cwd").and_then(Value::as_str).filter(|c| !c.is_empty() && len(c) <= 4096 && !bad_path(c)) {
                    m.insert("cwd".into(), json!(c));
                }
                Some(Value::Object(m))
            })
            .collect(),
    )
}

pub fn sanitize_user(raw: &Value) -> Option<Value> {
    let r = raw.as_object()?;
    let action = r.get("action").and_then(Value::as_str)?;
    match action {
        "logout" => Some(json!({"action": "logout"})),
        "code" => {
            let phone: String = r.get("phone")?.as_str()?.chars().filter(|c| !c.is_whitespace() && !matches!(c, '(' | ')' | '.' | '-')).collect();
            let bare = phone.strip_prefix('+').unwrap_or(&phone);
            digits(bare, 6, 16).then(|| json!({"action": "code", "phone": if phone.starts_with('+') { phone.clone() } else { format!("+{phone}") }}))
        }
        "signin" => {
            let code: String = r.get("code")?.as_str()?.chars().filter(|c| c.is_ascii_digit()).collect();
            digits(&code, 4, 8).then(|| json!({"action": "signin", "code": code}))
        }
        "password" => {
            let p = r.get("password")?.as_str()?;
            (!p.is_empty() && len(p) <= 256).then(|| json!({"action": "password", "password": p}))
        }
        _ => None,
    }
}

pub fn sanitize_create_topic(raw: &Value) -> Option<(String, String, String)> {
    let r = raw.as_object()?;
    let chat = chat_id(r.get("chatId")?)?;
    let project = project_ok(r.get("project")?)?;
    let name = slice(trim(&collapse(r.get("name")?.as_str()?, is_ctl, " ")), 128);
    (!name.is_empty()).then(|| (chat.to_string(), project.to_string(), name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn links_tokens_and_ids() {
        assert_eq!(parse_topic_link("https://t.me/c/1234567/89/100"), Some(("-1001234567".into(), Some(89))));
        assert_eq!(parse_topic_link("t.me/c/1234567/100?thread=5"), Some(("-1001234567".into(), Some(5))));
        assert_eq!(parse_topic_link("https://t.me/c/1234567"), Some(("-1001234567".into(), None)));
        assert_eq!(parse_topic_link("-1001234567:12"), Some(("-1001234567".into(), Some(12))));
        assert_eq!(parse_topic_link("@my_group"), Some(("@my_group".into(), None)));
        assert_eq!(parse_topic_link("https://t.me/my_group/7/8"), Some(("@my_group".into(), Some(7))));
        assert_eq!(parse_topic_link("https://evil.com/c/1234567"), None);
        assert_eq!(parse_topic_link("-123:4"), None);
        assert!(token_ok("12345:ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"));
        assert!(!token_ok("1234:ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"));
        assert!(chat_id_ok("-1001234567") && chat_id_ok("@abcd") && !chat_id_ok("@1abc") && !chat_id_ok("123"));
        assert!(session_id_ok("0123abcd-0000-4000-8000-00000000abcd"));
        assert!(is_secret_file_name(".env.local") && is_secret_file_name("release.JKS") && !is_secret_file_name("main.dart"));
    }

    #[test]
    fn prompts_and_patches() {
        assert_eq!(inbound_prompt("  a\x1b[31mb\x1b[0m\n\n\n\nc\u{202e}\x07 "), "ab\n\nc");
        assert!(sanitize_patch(&json!({"enabled": 1})).is_none());
        assert!(sanitize_patch(&json!({"doneMinutes": 2})).is_none());
        let p = sanitize_patch(&json!({"token": " 1 ", "apiHash": "ABCDEFABCDEFABCDEFABCDEFABCDEFAB", "quiet": {"enabled": true, "from": "23:00", "to": "07:00"}})).unwrap();
        assert_eq!(p["token"], "1");
        assert_eq!(p["apiHash"], "abcdefabcdefabcdefabcdefabcdefab");
        assert!(sanitize_patch(&json!({"quiet": {"enabled": true, "from": "24:00", "to": "07:00"}})).is_none());
        assert!(in_quiet(&json!({"enabled": true, "from": "23:00", "to": "07:00"}), 23 * 60 + 5));
        assert!(!in_quiet(&json!({"enabled": true, "from": "23:00", "to": "07:00"}), 12 * 60));
        assert_eq!(sanitize_user(&json!({"action": "code", "phone": "(84) 912-345-678"})).unwrap()["phone"], "+84912345678");
        assert!(sanitize_user(&json!({"action": "signin", "code": "12"})).is_none());
    }
}
