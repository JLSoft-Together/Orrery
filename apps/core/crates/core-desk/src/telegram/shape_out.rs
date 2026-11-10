use super::jsx::js_sort_keys;
use super::shape::{Known, Route, SessionRoute, Shape};
use serde_json::{json, Map, Value};
use std::path::Path;

pub fn route_value(r: &Route) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("project".into(), json!(r.project));
    m.insert("label".into(), json!(r.label));
    m.insert("chatId".into(), json!(r.chat_id));
    m.insert("threadId".into(), json!(r.thread_id));
    m
}

pub fn session_value(r: &SessionRoute) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("session".into(), json!(r.session));
    m.insert("name".into(), json!(r.name));
    m.insert("project".into(), json!(r.project));
    m.insert("chatId".into(), json!(r.chat_id));
    m.insert("threadId".into(), json!(r.thread_id));
    if r.keep {
        m.insert("keep".into(), json!(true));
    }
    m
}

pub fn known_value(k: &Known) -> Value {
    let mut topics = k.topics.clone();
    js_sort_keys(&mut topics);
    let t: Map<String, Value> = topics.into_iter().map(|(a, b)| (a, json!(b))).collect();
    json!({"title": k.title, "isForum": k.is_forum, "topics": t, "botAdmin": k.bot_admin, "canManageTopics": k.can_manage})
}

impl Shape {
    pub fn to_value(&self) -> Value {
        let mut m = Map::new();
        let opt = |m: &mut Map<String, Value>, k: &str, v: &Option<String>| {
            if let Some(v) = v {
                m.insert(k.into(), json!(v));
            }
        };
        m.insert("enabled".into(), json!(self.enabled));
        opt(&mut m, "token", &self.token);
        opt(&mut m, "bot", &self.bot);
        m.insert("includeReply".into(), json!(self.include_reply));
        m.insert("digest".into(), json!(self.digest));
        m.insert("waiting".into(), json!(self.waiting));
        m.insert("doneMinutes".into(), json!(self.done_minutes));
        m.insert("routes".into(), Value::Array(self.routes.iter().map(|r| Value::Object(route_value(r))).collect()));
        m.insert("known".into(), Value::Object(self.known.iter().map(|(k, v)| (k.clone(), known_value(v))).collect()));
        m.insert("quiet".into(), self.quiet.clone());
        m.insert("summary".into(), self.summary.clone());
        m.insert("alerts".into(), json!(self.alerts));
        m.insert("pinStatus".into(), json!(self.pin_status));
        m.insert("topicIcons".into(), json!(self.topic_icons));
        m.insert("pins".into(), Value::Object(self.pins.iter().map(|(k, (c, id))| (k.clone(), json!({"chatId": c, "messageId": id}))).collect()));
        m.insert("bigFiles".into(), json!(self.big_files));
        m.insert("sessionRoutes".into(), Value::Array(self.session_routes.iter().map(|r| Value::Object(session_value(r))).collect()));
        let health: Map<String, Value> = self
            .topic_health
            .iter()
            .map(|(k, h)| {
                let mut o = Map::new();
                if let Some(at) = h.at {
                    o.insert("at".into(), json!(at as i64));
                }
                if let Some(e) = h.error {
                    o.insert("error".into(), json!(e));
                }
                (k.clone(), Value::Object(o))
            })
            .collect();
        m.insert("topicHealth".into(), Value::Object(health));
        if let Some(b) = &self.broken {
            m.insert("broken".into(), json!(b));
        }
        m.insert("stranger".into(), self.stranger.clone());
        m.insert("members".into(), Value::Array(self.members.clone()));
        m.insert("replies".into(), Value::Object(self.replies.iter().map(|(k, v)| (k.clone(), json!(v))).collect()));
        if let Some(n) = self.last_update {
            m.insert("lastUpdate".into(), json!(n));
        }
        opt(&mut m, "commandsHash", &self.commands_hash);
        if let Some(n) = self.api_id {
            m.insert("apiId".into(), json!(n));
        }
        opt(&mut m, "apiHash", &self.api_hash);
        opt(&mut m, "mtSession", &self.mt_session);
        opt(&mut m, "userSession", &self.user_session);
        opt(&mut m, "userName", &self.user_name);
        opt(&mut m, "summaryDay", &self.summary_day);
        if let Some((id, name)) = &self.owner {
            m.insert("owner".into(), json!({"id": id, "name": name}));
        }
        Value::Object(m)
    }

    pub fn save(&self, file: &Path) -> bool {
        let text = crate::js::stringify_indent(&self.to_value(), "  ");
        let Some(dir) = file.parent() else { return false };
        if std::fs::create_dir_all(dir).is_err() {
            return false;
        }
        let mut tmp = file.as_os_str().to_owned();
        tmp.push(".tmp");
        let tmp = std::path::PathBuf::from(tmp);
        if write_private(&tmp, &text).is_err() {
            return false;
        }
        std::fs::rename(&tmp, file).is_ok()
    }
}

#[cfg(unix)]
fn write_private(file: &Path, text: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(file)?;
    f.write_all(text.as_bytes())
}

#[cfg(not(unix))]
fn write_private(file: &Path, text: &str) -> std::io::Result<()> {
    std::fs::write(file, text)
}
