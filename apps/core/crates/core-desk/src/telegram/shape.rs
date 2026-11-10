use super::jsx::{digits, int_of, js_insert, s, slice};
use super::sanitize::{chat_id, chat_id_ok, parse_members, session_id, session_id_ok, thread_id, DEFAULT_DONE_MIN, DONE_CHOICES, ROUTES_MAX, SESSION_ROUTES_MAX, STRANGER_MAX};
use serde_json::{json, Value};
use std::path::Path;

pub use super::shape_out::*;

pub const ERRORS: [&str; 10] = ["network", "status", "timeout", "rate", "keychain", "config", "token", "chat", "too-large", "conflict"];
const KNOWN_TOPICS_MAX: usize = 200;
const TOPIC_HEALTH_MAX: usize = 200;
const REPLIES_MAX: usize = 300;

#[derive(Clone, Debug, PartialEq)]
pub struct Route {
    pub project: String,
    pub label: String,
    pub chat_id: String,
    pub thread_id: Option<i64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SessionRoute {
    pub session: String,
    pub name: String,
    pub project: String,
    pub chat_id: String,
    pub thread_id: Option<i64>,
    pub keep: bool,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Known {
    pub title: String,
    pub is_forum: bool,
    pub topics: Vec<(String, String)>,
    pub bot_admin: bool,
    pub can_manage: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Health {
    pub at: Option<f64>,
    pub error: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Shape {
    pub enabled: bool,
    pub token: Option<String>,
    pub bot: Option<String>,
    pub include_reply: bool,
    pub digest: bool,
    pub waiting: bool,
    pub done_minutes: i64,
    pub routes: Vec<Route>,
    pub known: Vec<(String, Known)>,
    pub owner: Option<(i64, String)>,
    pub quiet: Value,
    pub summary: Value,
    pub alerts: bool,
    pub pin_status: bool,
    pub topic_icons: bool,
    pub pins: Vec<(String, (String, i64))>,
    pub summary_day: Option<String>,
    pub big_files: bool,
    pub session_routes: Vec<SessionRoute>,
    pub broken: Option<Vec<String>>,
    pub topic_health: Vec<(String, Health)>,
    pub stranger: Value,
    pub members: Vec<Value>,
    pub replies: Vec<(String, String)>,
    pub last_update: Option<i64>,
    pub commands_hash: Option<String>,
    pub api_id: Option<i64>,
    pub api_hash: Option<String>,
    pub mt_session: Option<String>,
    pub user_session: Option<String>,
    pub user_name: Option<String>,
}

pub fn default_quiet() -> Value {
    json!({"enabled": false, "from": "23:00", "to": "07:00"})
}

pub fn default_summary() -> Value {
    json!({"enabled": false, "at": "21:00"})
}

pub fn default_stranger() -> Value {
    json!({"enabled": true, "text": ""})
}

pub fn pin_key_ok(k: &str) -> bool {
    let Some((a, b)) = k.rsplit_once(':') else { return false };
    let head_ok = (1..=40).contains(&a.chars().count()) && a.chars().all(|c| c == '-' || c == '@' || c.is_ascii_alphanumeric() || c == '_');
    head_ok && (b == "null" || (!b.is_empty() && b.bytes().all(|x| x.is_ascii_digit())))
}

impl Default for Shape {
    fn default() -> Self {
        Shape {
            enabled: false,
            token: None,
            bot: None,
            include_reply: false,
            digest: false,
            waiting: true,
            done_minutes: DEFAULT_DONE_MIN,
            routes: Vec::new(),
            known: Vec::new(),
            owner: None,
            quiet: default_quiet(),
            summary: default_summary(),
            alerts: true,
            pin_status: false,
            topic_icons: false,
            pins: Vec::new(),
            summary_day: None,
            big_files: false,
            session_routes: Vec::new(),
            broken: None,
            topic_health: Vec::new(),
            stranger: default_stranger(),
            members: Vec::new(),
            replies: Vec::new(),
            last_update: None,
            commands_hash: None,
            api_id: None,
            api_hash: None,
            mt_session: None,
            user_session: None,
            user_name: None,
        }
    }
}

fn basename(p: &str) -> String {
    crate::util::basename(p)
}

fn read_routes(raw: &Value) -> Vec<Route> {
    raw.as_array()
        .map(|l| {
            l.iter()
                .take(ROUTES_MAX)
                .filter_map(|o| {
                    let project = s(o, "project").filter(|p| !p.is_empty())?;
                    let chat = chat_id(&o["chatId"])?;
                    let label = s(o, "label").filter(|l| !l.is_empty()).map(|l| slice(l, 80)).unwrap_or_else(|| basename(project));
                    Some(Route { project: project.into(), label, chat_id: chat.into(), thread_id: thread_id(&o["threadId"]) })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn read_session_routes(raw: &Value) -> Vec<SessionRoute> {
    raw.as_array()
        .map(|l| {
            l.iter()
                .take(SESSION_ROUTES_MAX)
                .filter_map(|o| {
                    let session = session_id(&o["session"])?;
                    let project = s(o, "project").filter(|p| !p.is_empty())?;
                    let chat = chat_id(&o["chatId"])?;
                    let name = s(o, "name").filter(|n| !n.is_empty()).map(|n| slice(n, 80)).unwrap_or_else(|| "Claude".into());
                    Some(SessionRoute { session: session.into(), name, project: project.into(), chat_id: chat.into(), thread_id: thread_id(&o["threadId"]), keep: o.get("keep") == Some(&Value::Bool(true)) })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn read_known(raw: &Value) -> Vec<(String, Known)> {
    let Some(m) = raw.as_object() else { return Vec::new() };
    let mut out = Vec::new();
    for (id, v) in crate::js::entries(m).into_iter().take(50) {
        if !chat_id_ok(id) || !v.is_object() {
            continue;
        }
        let mut topics: Vec<(String, String)> = Vec::new();
        if let Some(t) = v.get("topics").and_then(Value::as_object) {
            for (k, n) in crate::js::entries(t).into_iter().take(KNOWN_TOPICS_MAX) {
                if digits(k, 1, 10) {
                    if let Some(n) = n.as_str() {
                        js_insert(&mut topics, k, slice(n, 128));
                    }
                }
            }
        }
        let title = s(v, "title").map(|t| slice(t, 120)).unwrap_or_else(|| id.clone());
        js_insert(&mut out, id, Known { title, is_forum: v.get("isForum") == Some(&Value::Bool(true)), topics, bot_admin: v.get("botAdmin") == Some(&Value::Bool(true)), can_manage: v.get("canManageTopics") == Some(&Value::Bool(true)) });
    }
    out
}

fn read_pins(raw: &Value, routes: &[Route]) -> Vec<(String, (String, i64))> {
    let Some(m) = raw.as_object() else { return Vec::new() };
    let msg_id = |v: &Value| int_of(v).filter(|n| *n > 0);
    let mut out = Vec::new();
    for (k, v) in crate::js::entries(m).into_iter().take(ROUTES_MAX + SESSION_ROUTES_MAX) {
        if v.is_object() {
            if let (Some(c), Some(id)) = (chat_id(&v["chatId"]), msg_id(&v["messageId"])) {
                js_insert(&mut out, k, (c.to_string(), id));
            }
        } else if let Some(id) = msg_id(v) {
            if let Some(r) = routes.iter().find(|r| r.project == *k) {
                js_insert(&mut out, k, (r.chat_id.clone(), id));
            }
        }
    }
    out
}

fn read_health(raw: &Value) -> Vec<(String, Health)> {
    let Some(m) = raw.as_object() else { return Vec::new() };
    let mut out = Vec::new();
    for (k, v) in crate::js::entries(m).into_iter().take(TOPIC_HEALTH_MAX) {
        if !pin_key_ok(k) {
            continue;
        }
        let at = v.get("at").and_then(Value::as_f64).filter(|a| a.is_finite() && *a > 0.0);
        let error = v.get("error").and_then(Value::as_str).and_then(|e| ERRORS.iter().find(|x| **x == e).copied());
        if at.is_some() || error.is_some() {
            js_insert(&mut out, k, Health { at, error });
        }
    }
    out
}

fn read_quiet(v: &Value) -> Value {
    match (super::sanitize::is_hhmm(&v["from"]), super::sanitize::is_hhmm(&v["to"])) {
        (Some(f), Some(t)) if v.is_object() => json!({"enabled": v.get("enabled") == Some(&Value::Bool(true)), "from": f, "to": t}),
        _ => default_quiet(),
    }
}

fn read_summary(v: &Value) -> Value {
    match super::sanitize::is_hhmm(&v["at"]) {
        Some(a) if v.is_object() => json!({"enabled": v.get("enabled") == Some(&Value::Bool(true)), "at": a}),
        _ => default_summary(),
    }
}

fn read_stranger(v: &Value) -> Value {
    match (v.get("enabled").and_then(Value::as_bool), s(v, "text")) {
        (Some(e), Some(t)) => json!({"enabled": e, "text": slice(t, STRANGER_MAX)}),
        _ => default_stranger(),
    }
}

fn read_members(raw: &Value) -> Vec<Value> {
    let Some(list) = raw.as_array() else { return Vec::new() };
    let mut out: Vec<Value> = Vec::new();
    for m in list.iter().take(super::sanitize::MEMBERS_MAX) {
        if let Some(one) = parse_members(&json!([m])).and_then(|l| l.into_iter().next()) {
            if !out.iter().any(|x| x["id"] == one["id"]) {
                out.push(one);
            }
        }
    }
    out
}

fn read_replies(raw: &Value) -> Vec<(String, String)> {
    let Some(m) = raw.as_object() else { return Vec::new() };
    let all = crate::js::entries(m);
    let skip = all.len().saturating_sub(REPLIES_MAX);
    let mut out = Vec::new();
    for (k, v) in all.into_iter().skip(skip) {
        let key_ok = k.split_once(':').is_some_and(|(a, b)| digits(a.strip_prefix('-').unwrap_or(a), 1, 30) && digits(b, 1, 30));
        if key_ok {
            if let Some(sv) = v.as_str().filter(|v| session_id_ok(v)) {
                js_insert(&mut out, k, sv.to_lowercase());
            }
        }
    }
    out
}

fn str_under(raw: &Value, k: &str, max: usize) -> Option<String> {
    s(raw, k).filter(|v| crate::js::len(v) < max).map(str::to_string)
}

pub fn read(file: Option<&Path>) -> Shape {
    let Some(file) = file else { return Shape::default() };
    let Some(raw) = std::fs::read_to_string(file).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) else { return Shape::default() };
    if raw.is_null() {
        return Shape::default();
    }
    let routes = read_routes(&raw["routes"]);
    let pins = read_pins(&raw["pins"], &routes);
    let hash = s(&raw, "commandsHash").filter(|h| h.len() == 16 && h.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))).map(str::to_string);
    let day = s(&raw, "summaryDay").filter(|d| d.len() == 10 && d.as_bytes()[4] == b'-' && d.as_bytes()[7] == b'-' && digits(&d[..4], 4, 4) && digits(&d[5..7], 2, 2) && digits(&d[8..], 2, 2)).map(str::to_string);
    let owner = raw.get("owner").filter(|o| o.is_object()).and_then(|o| Some((o.get("id").and_then(Value::as_i64).filter(|_| int_of(&o["id"]).is_some())?, slice(s(o, "name")?, 80))));
    Shape {
        enabled: raw.get("enabled") == Some(&Value::Bool(true)),
        token: str_under(&raw, "token", 4096),
        bot: s(&raw, "bot").map(|b| slice(b, 64)),
        include_reply: raw.get("includeReply") == Some(&Value::Bool(true)),
        digest: raw.get("digest") == Some(&Value::Bool(true)),
        waiting: raw.get("waiting") != Some(&Value::Bool(false)),
        done_minutes: raw.get("doneMinutes").and_then(Value::as_f64).and_then(|f| DONE_CHOICES.iter().copied().find(|c| *c as f64 == f)).unwrap_or(DEFAULT_DONE_MIN),
        routes,
        known: read_known(&raw["known"]),
        owner,
        quiet: read_quiet(&raw["quiet"]),
        summary: read_summary(&raw["summary"]),
        alerts: raw.get("alerts") != Some(&Value::Bool(false)),
        pin_status: raw.get("pinStatus") == Some(&Value::Bool(true)),
        topic_icons: raw.get("topicIcons") == Some(&Value::Bool(true)),
        pins,
        summary_day: day,
        big_files: raw.get("bigFiles") == Some(&Value::Bool(true)),
        session_routes: read_session_routes(&raw["sessionRoutes"]),
        broken: raw.get("broken").and_then(Value::as_array).map(|l| l.iter().filter_map(Value::as_str).filter(|k| pin_key_ok(k)).take(ROUTES_MAX * 2).map(str::to_string).collect()),
        topic_health: read_health(&raw["topicHealth"]),
        stranger: read_stranger(&raw["stranger"]),
        members: read_members(&raw["members"]),
        replies: read_replies(&raw["replies"]),
        last_update: raw.get("lastUpdate").and_then(int_of).filter(|n| *n >= 0),
        commands_hash: hash,
        api_id: raw.get("apiId").and_then(int_of).filter(|n| *n > 0),
        api_hash: str_under(&raw, "apiHash", 4096),
        mt_session: str_under(&raw, "mtSession", 8192),
        user_session: str_under(&raw, "userSession", 8192),
        user_name: s(&raw, "userName").map(|n| slice(n, 64)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_what_it_writes() {
        let dir = std::env::temp_dir().join(format!("ccm-tg-shape-{}", crate::util::uuid()));
        let file = dir.join("telegram.json");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(&file, r#"{"enabled":true,"token":"c2VhbGVk","routes":[{"project":"/p/app","chatId":"-1001234567","threadId":5},{"project":"","chatId":"-1"}],"known":{"-1001234567":{"title":"G","topics":{"9":"b","5":"a","x":"no"}}},"pins":{"/p/app":42},"doneMinutes":3,"owner":{"id":7,"name":"Jin"},"replies":{"-100:5":"0123ABCD-0000-4000-8000-00000000abcd"},"topicHealth":{"-1001234567:5":{"at":10,"error":"chat"},"bad":{"at":1}}}"#).unwrap();
        let s = read(Some(&file));
        assert_eq!(s.routes.len(), 1);
        assert_eq!(s.routes[0].label, "app");
        assert_eq!(s.known[0].1.topics, vec![("5".to_string(), "a".to_string()), ("9".to_string(), "b".to_string())]);
        assert_eq!(s.pins, vec![("/p/app".to_string(), ("-1001234567".to_string(), 42))]);
        assert_eq!(s.replies[0].1, "0123abcd-0000-4000-8000-00000000abcd");
        assert_eq!(s.topic_health.len(), 1);
        assert!(s.save(&file));
        assert_eq!(read(Some(&file)), s);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
