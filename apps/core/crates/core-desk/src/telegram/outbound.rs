use super::api::{Body, Tg as Res};
use super::base::{Held, Target, Tg, TIMEOUT_MS};
use super::html::{thread_fields, DayCounts};
use super::shape::Route;
use serde_json::{json, Value};
use std::sync::Arc;

const HELD_MAX: usize = 20;
pub const MAX_EPISODES: usize = 500;
pub const SENT_MAP_MAX: usize = 500;
const REPLIES_MAX: usize = 300;

pub struct Sent {
    pub error: Option<&'static str>,
    pub id: Option<i64>,
}

impl Tg {
    pub fn send_text(self: &Arc<Self>, route: &Route, html: &str, target: Option<Target>, keys: Option<Value>, important: Option<bool>) -> Option<&'static str> {
        self.send_text_id(route, html, target, keys, important).error
    }

    pub fn send_text_id(self: &Arc<Self>, route: &Route, html: &str, target: Option<Target>, keys: Option<Value>, important: Option<bool>) -> Sent {
        let important = important.unwrap_or(keys.is_some());
        let item = Held { route: route.clone(), html: html.into(), target, keys, important };
        if self.hold_left(&route.chat_id) > 0 {
            return self.enqueue(item);
        }
        if !self.take(&route.chat_id) {
            return Sent { error: Some(self.fail("rate")), id: None };
        }
        match self.deliver(&item) {
            None => self.enqueue(item),
            Some(out) => out,
        }
    }

    fn deliver(&self, item: &Held) -> Option<Sent> {
        let mut m = thread_fields(&item.route.chat_id, item.route.thread_id);
        m.insert("text".into(), json!(item.html));
        m.insert("parse_mode".into(), json!("HTML"));
        m.insert("disable_web_page_preview".into(), json!(true));
        if let Some(k) = &item.keys {
            m.insert("reply_markup".into(), k.clone());
        }
        let res = self.call("sendMessage", Body::Json(m), TIMEOUT_MS);
        if let Res::Err(e, _) = res {
            if e == "rate" && self.hold_left(&item.route.chat_id) > 0 {
                return None;
            }
            if e == "chat" {
                self.mark_topic(&item.route.chat_id, item.route.thread_id, true);
            }
            return Some(Sent { error: Some(self.fail(e)), id: None });
        }
        self.mark_topic(&item.route.chat_id, item.route.thread_id, false);
        self.ok();
        let result = res.result();
        self.track(&item.route.chat_id, &result, item.target.as_ref());
        Some(Sent { error: None, id: result["message_id"].as_i64() })
    }

    fn enqueue(self: &Arc<Self>, item: Held) -> Sent {
        let chat = item.route.chat_id.clone();
        {
            let mut st = self.lock();
            let pos = match st.held.iter().position(|(k, _)| *k == chat) {
                Some(p) => p,
                None => {
                    st.held.push((chat.clone(), Vec::new()));
                    st.held.len() - 1
                }
            };
            let list = &mut st.held[pos].1;
            if list.len() >= HELD_MAX {
                let at = list.iter().position(|h| !h.important).unwrap_or(0);
                list.remove(at);
            }
            list.push(item);
        }
        self.arm_held(&chat, 10_000);
        Sent { error: None, id: None }
    }

    fn arm_held(self: &Arc<Self>, chat: &str, retry_ms: i64) {
        let left = self.hold_left(chat);
        let gen = {
            let mut st = self.lock();
            if st.held_timers.contains_key(chat) {
                return;
            }
            st.timer_gen += 1;
            let gen = st.timer_gen;
            st.held_timers.insert(chat.to_string(), gen);
            gen
        };
        let wait = if left > 0 { left + 200 } else { retry_ms };
        let chat = chat.to_string();
        self.spawn(move |tg| {
            (tg.deps.sleep)(wait.max(0) as u64);
            {
                let mut st = tg.lock();
                if st.held_timers.get(&chat) != Some(&gen) {
                    return;
                }
                st.held_timers.remove(&chat);
            }
            tg.flush_held();
        });
    }

    pub fn flush_held(self: &Arc<Self>) {
        {
            let mut st = self.lock();
            if st.flushing || !st.data.enabled {
                return;
            }
            st.flushing = true;
        }
        if self.token().is_none() {
            self.lock().flushing = false;
            return;
        }
        let chats: Vec<String> = self.lock().held.iter().map(|(k, _)| k.clone()).collect();
        for chat in chats {
            loop {
                let has = self.lock().held.iter().find(|(k, _)| *k == chat).is_some_and(|(_, l)| !l.is_empty());
                if !has || self.hold_left(&chat) != 0 || !self.take(&chat) {
                    break;
                }
                let item = {
                    let mut st = self.lock();
                    let Some((_, list)) = st.held.iter_mut().find(|(k, _)| *k == chat) else { break };
                    list.remove(0)
                };
                if self.deliver(&item).is_none() {
                    let mut st = self.lock();
                    if let Some((_, list)) = st.held.iter_mut().find(|(k, _)| *k == chat) {
                        list.insert(0, item);
                    }
                    break;
                }
            }
            let left = self.lock().held.iter().find(|(k, _)| *k == chat).map(|(_, l)| l.len()).unwrap_or(0);
            if left > 0 {
                self.arm_held(&chat, 10_000);
            } else {
                self.lock().held.retain(|(k, _)| *k != chat);
            }
        }
        self.lock().flushing = false;
    }

    pub fn track(&self, chat: &str, result: &Value, target: Option<&Target>) {
        if let (Some(t), Some(id)) = (target, result["message_id"].as_i64()) {
            self.remember_sent(chat, id, t.clone());
        }
    }

    pub fn remember_sent(&self, chat: &str, message_id: i64, target: Target) {
        let key = format!("{chat}:{message_id}");
        let mut st = self.lock();
        super::jsx::js_remove(&mut st.sent_to, &key);
        st.sent_to.push((key.clone(), target.clone()));
        if st.sent_to.len() > SENT_MAP_MAX {
            st.sent_to.remove(0);
        }
        let agent = match &target {
            Target::Terminal(t) => st.agent_of_terminal.get(t).cloned(),
            Target::Pane(p) => st.agent_of_pane.get(p).cloned(),
        };
        let Some(agent) = agent else { return };
        super::jsx::js_insert(&mut st.data.replies, &key, agent);
        let extra = st.data.replies.len().saturating_sub(REPLIES_MAX);
        st.data.replies.drain(..extra);
        Tg::save_st(&self.deps, &st);
    }

    pub fn quiet_now(&self) -> bool {
        let minute = self.local(self.now()).minute_of_day();
        super::sanitize::in_quiet(&self.lock().data.quiet, minute)
    }

    pub fn counts_add(&self, project: &str, f: impl FnOnce(&mut DayCounts)) {
        let day = self.day_key(self.now());
        let mut st = self.lock();
        if day != st.counts_day {
            st.counts.clear();
            st.counts_day = day;
        }
        if super::jsx::js_get(&st.counts, project).is_none() {
            st.counts.push((project.to_string(), DayCounts::default()));
        }
        if let Some((_, c)) = st.counts.iter_mut().find(|(k, _)| k == project) {
            f(c);
        }
    }

    pub fn seen_add(st: &mut super::base::St, key: String) {
        if st.sent_set.insert(key.clone()) {
            st.sent.push_back(key);
        }
    }

}
