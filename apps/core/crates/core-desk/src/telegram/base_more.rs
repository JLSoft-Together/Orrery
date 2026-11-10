use super::api::Body;
use super::base::*;
use super::html::thread_fields;
use super::message::TgMessage;
use super::shape::{Route, SessionRoute};
use crate::phone::send::RateLimiter;
use serde_json::{json, Map, Value};
use std::sync::Arc;

impl Tg {
    pub fn mark_topic(&self, chat: &str, thread: Option<i64>, broken: bool) {
        {
            let mut st = self.lock();
            let key = topic_key(chat, thread);
            let is = st.data.broken.as_ref().is_some_and(|b| b.contains(&key));
            if is == broken {
                return;
            }
            let mut rest: Vec<String> = st.data.broken.clone().unwrap_or_default().into_iter().filter(|k| *k != key).collect();
            if broken {
                rest.push(key);
            }
            st.data.broken = Some(rest);
            Tg::save_st(&self.deps, &st);
        }
        self.emit_state();
    }

    pub fn role_of(st: &St, id: Option<i64>) -> Option<&'static str> {
        let (owner, _) = st.data.owner.as_ref()?;
        let id = id?;
        if id == *owner {
            return Some("owner");
        }
        st.data.members.iter().find(|m| m["id"].as_i64() == Some(id)).and_then(|m| match m["level"].as_str() {
            Some("read") => Some("read"),
            Some("full") => Some("full"),
            _ => None,
        })
    }

    pub fn name_of(st: &St, id: i64) -> String {
        if let Some((o, n)) = &st.data.owner {
            if *o == id {
                return n.clone();
            }
        }
        st.data.members.iter().find(|m| m["id"].as_i64() == Some(id)).and_then(|m| m["name"].as_str().map(str::to_string)).unwrap_or_default()
    }

    pub fn see_stranger(&self, msg: &TgMessage) {
        let now = self.now();
        let mut st = self.lock();
        st.strangers.retain(|(id, _)| *id != msg.from_id);
        let name = super::jsx::slice(super::jsx::trim(&super::jsx::collapse(&msg.from_name, |c| super::jsx::is_ctl_c1(c) || super::jsx::is_bidi(c), " ")), 80);
        let name = if name.is_empty() { msg.from_id.to_string() } else { name };
        let chat = super::jsx::js_get(&st.data.known, &msg.chat_id).map(|k| k.title.clone()).unwrap_or_else(|| if msg.chat_type == "private" { String::new() } else { msg.chat_id.clone() });
        st.strangers.push((msg.from_id, json!({"id": msg.from_id, "name": name, "at": now, "chat": chat})));
        if st.strangers.len() > STRANGERS_SEEN_MAX {
            st.strangers.remove(0);
        }
    }

    pub fn save_st(deps: &Deps, st: &St) -> bool {
        match &deps.file {
            Some(f) => st.data.save(f),
            None => true,
        }
    }

    pub fn save(&self) -> bool {
        let st = self.lock();
        Tg::save_st(&self.deps, &st)
    }

    pub fn commit(&self, clear_error: bool) -> Value {
        let saved = {
            let mut st = self.lock();
            if clear_error {
                st.last_error = None;
            }
            Tg::save_st(&self.deps, &st)
        };
        if !saved {
            return json!({"ok": false, "error": "failed"});
        }
        let state = self.state();
        (self.deps.emit)(EVENT, json!({"state": state.clone()}));
        json!({"ok": true, "state": state})
    }

    pub fn ok(&self) {
        {
            let mut st = self.lock();
            if st.last_error.is_none() {
                return;
            }
            st.last_error = None;
        }
        self.emit_state();
    }

    pub fn fail(&self, error: &'static str) -> &'static str {
        let first = {
            let mut st = self.lock();
            let first = st.last_error.is_none();
            st.last_error = Some(error);
            first
        };
        if first {
            (self.deps.emit)(EVENT, json!({"failed": error}));
            self.emit_state();
        }
        error
    }

    pub fn take(&self, chat: &str) -> bool {
        let now = self.now() as f64;
        let mut st = self.lock();
        let ok = st.per_chat.entry(chat.to_string()).or_insert_with(|| RateLimiter::new(PER_CHAT_MIN, 60_000.0)).take(now);
        ok && st.total.take(now)
    }

    pub fn take_inbound(&self, chat: &str) -> bool {
        let now = self.now() as f64;
        self.lock().inbound_limit.entry(chat.to_string()).or_insert_with(|| RateLimiter::new(INBOUND_PER_CHAT_MIN, 60_000.0)).take(now)
    }

    pub fn note_ignored(&self) {
        let now = self.now() as f64;
        {
            let mut st = self.lock();
            st.ignored += 1;
            if now - st.ignored_at < IGNORED_STATE_MS {
                return;
            }
            st.ignored_at = now;
        }
        self.emit_state();
    }

    pub fn add_action(&self, mut entry: Entry) -> String {
        let key: String = crate::util::uuid().replace('-', "").chars().take(12).collect();
        if entry.at.is_none() {
            entry.at = Some(self.now());
        }
        let mut st = self.lock();
        st.actions.push((key.clone(), entry));
        if st.actions.len() > ACTIONS_MAX {
            st.actions.remove(0);
        }
        key
    }

    pub fn answer(&self, id: &str, text: Option<&str>) {
        let mut body = json!({"callback_query_id": id});
        if let Some(t) = text {
            let plain = strip_tags(t);
            let plain = plain.replace("&lt;", "<").replace("&gt;", ">").replace("&amp;", "&");
            let plain = super::jsx::slice(&plain, 190);
            if !plain.is_empty() {
                body["text"] = json!(plain);
            }
        }
        self.call_json("answerCallbackQuery", body);
    }

    pub fn post(&self, chat: &str, thread: Option<i64>, html: &str, extra: Map<String, Value>) -> Option<i64> {
        let mut m = thread_fields(chat, thread);
        m.insert("text".into(), json!(html));
        m.insert("parse_mode".into(), json!("HTML"));
        m.insert("disable_web_page_preview".into(), json!(true));
        m.extend(extra);
        self.call("sendMessage", Body::Json(m), TIMEOUT_MS).result().get("message_id").and_then(Value::as_i64)
    }

    pub fn reply(&self, msg: &TgMessage, html: &str, keys: Option<Value>) -> Option<i64> {
        let mut m = thread_fields(&msg.chat_id, msg.thread_id);
        m.insert("text".into(), json!(html));
        m.insert("parse_mode".into(), json!("HTML"));
        m.insert("disable_web_page_preview".into(), json!(true));
        m.insert("reply_parameters".into(), json!({"message_id": msg.message_id, "allow_sending_without_reply": true}));
        if let Some(k) = keys {
            m.insert("reply_markup".into(), k);
        }
        self.call("sendMessage", Body::Json(m), TIMEOUT_MS).result().get("message_id").and_then(Value::as_i64)
    }

    pub fn clear_markup(&self, chat: &str, message_id: Option<i64>) {
        if let Some(id) = message_id {
            self.call_json("editMessageReplyMarkup", json!({"chat_id": chat, "message_id": id, "reply_markup": {"inline_keyboard": []}}));
        }
    }

    pub fn route_for(st: &St, cwd: &str) -> Option<Route> {
        let mut best: Option<&Route> = None;
        for r in &st.data.routes {
            if crate::util::is_inside_or_same(&r.project, cwd) && best.is_none_or(|b| super::jsx::len(&r.project) > super::jsx::len(&b.project)) {
                best = Some(r);
            }
        }
        best.cloned()
    }

    pub fn session_route_of(st: &St, agent: Option<&str>) -> Option<Route> {
        let agent = agent?;
        st.data.session_routes.iter().find(|r| r.session == agent).map(|s| Route { project: s.project.clone(), label: s.name.clone(), chat_id: s.chat_id.clone(), thread_id: s.thread_id })
    }

    pub fn session_routes_on(st: &St, chat: &str, thread: Option<i64>) -> Vec<SessionRoute> {
        st.data.session_routes.iter().filter(|r| r.chat_id == chat && r.thread_id == thread).cloned().collect()
    }

    pub fn spawn(self: &Arc<Self>, f: impl FnOnce(Arc<Tg>) + Send + 'static) {
        let me = self.clone();
        std::thread::spawn(move || f(me));
    }
}
