use super::api::{tg_call, Body, Tg as Res};
use super::base::*;
use super::shape::Health;
use serde_json::{json, Map, Value};

impl Tg {
    pub fn call(&self, method: &str, body: Body, timeout_ms: u64) -> Res {
        let Some(token) = self.token() else { return Res::Err("config", None) };
        let chat = if outgoing(method) { body.field("chat_id") } else { None };
        if let Some(c) = &chat {
            if self.hold_left(c) > 0 {
                return Res::Err("rate", None);
            }
        }
        let res = tg_call(&*self.deps.http, &self.deps.base, &token, method, &body, timeout_ms);
        self.note_health(method, &res);
        if let Some(c) = chat {
            if let Res::Err("rate", retry) = &res {
                self.hold(&c, *retry);
            }
            if method.starts_with("send") {
                let t = body.field("message_thread_id").and_then(|v| v.parse::<f64>().ok()).filter(|n| n.fract() == 0.0 && *n > 0.0 && *n <= 9_007_199_254_740_991.0).map(|n| n as i64);
                self.note_topic(Some(topic_key(&c, t)), &res);
            }
        }
        res
    }

    pub fn call_json(&self, method: &str, body: Value) -> Res {
        let m = match body {
            Value::Object(m) => m,
            _ => Map::new(),
        };
        self.call(method, Body::Json(m), TIMEOUT_MS)
    }

    pub fn hold_left(&self, chat: &str) -> i64 {
        let now = self.now();
        let mut st = self.lock();
        let Some(until) = st.hold_until.get(chat).copied() else { return 0 };
        let left = until - now;
        if left <= 0 {
            st.hold_until.remove(chat);
        }
        left.max(0)
    }

    fn hold(&self, chat: &str, retry: Option<f64>) {
        let secs = retry.unwrap_or(HOLD_DEFAULT_S).clamp(1.0, HOLD_MAX_S);
        let until = self.now() + (secs * 1000.0) as i64;
        let mut st = self.lock();
        if until > st.hold_until.get(chat).copied().unwrap_or(0) {
            st.hold_until.insert(chat.to_string(), until);
        }
    }

    pub fn routed(st: &St, key: &str) -> bool {
        st.data.routes.iter().any(|r| topic_key(&r.chat_id, r.thread_id) == key) || st.data.session_routes.iter().any(|r| topic_key(&r.chat_id, r.thread_id) == key)
    }

    pub fn note_topic(&self, key: Option<String>, res: &Res) {
        let now = self.now();
        let push = {
            let mut st = self.lock();
            let Some(key) = key.filter(|k| Tg::routed(&st, k)) else { return };
            let prev = super::jsx::js_get(&st.data.topic_health, &key).cloned();
            let next = match res {
                Res::Ok(_) => Health { at: Some(now as f64), error: None },
                Res::Err(e, _) => Health { at: prev.as_ref().and_then(|p| p.at), error: Some(e) },
            };
            let changed = prev.as_ref().and_then(|p| p.error) != res.error();
            super::jsx::js_insert(&mut st.data.topic_health, &key, next);
            if st.data.topic_health.len() > TOPIC_HEALTH_MAX {
                let keep: Vec<(String, Health)> = st.data.topic_health.iter().filter(|(k, _)| Tg::routed(&st, k)).cloned().collect();
                st.data.topic_health = keep;
            }
            if !changed && now - st.topic_saved_at <= TOPIC_PUSH_MS {
                false
            } else {
                st.topic_saved_at = now;
                Tg::save_st(&self.deps, &st);
                true
            }
        };
        if push {
            self.emit_state();
        }
    }

    fn note_health(&self, method: &str, res: &Res) {
        let now = self.now();
        let day = self.day_key(now);
        let mut st = self.lock();
        if day != st.health_day {
            st.health_day = day;
            st.health.sent_today = 0;
            st.health.errors_today = 0;
        }
        if method == "getUpdates" {
            match res {
                Res::Ok(_) => st.health.last_poll = Some(now),
                Res::Err(e, _) if *e != "timeout" => st.health.errors_today += 1,
                _ => {}
            }
            return;
        }
        if !method.starts_with("send") && method != "editMessageText" {
            return;
        }
        if res.ok() {
            st.health.last_sent = Some(now);
            st.health.sent_today += 1;
        } else {
            st.health.errors_today += 1;
        }
    }

    pub fn topic_name(st: &St, chat: &str, thread: Option<i64>) -> String {
        let known = super::jsx::js_get(&st.data.known, chat);
        let topic = match thread.filter(|t| *t != 0) {
            Some(t) => known.and_then(|k| super::jsx::js_get(&k.topics, &t.to_string()).cloned()).unwrap_or_else(|| format!("#{t}")),
            None => "General".into(),
        };
        format!("{} › {topic}", known.map(|k| k.title.clone()).unwrap_or_else(|| chat.to_string()))
    }

    fn route_state(st: &St, mut m: Map<String, Value>, chat: &str, thread: Option<i64>) -> Value {
        let key = topic_key(chat, thread);
        m.insert("where".into(), json!(Tg::topic_name(st, chat, thread)));
        if st.data.broken.as_ref().is_some_and(|b| b.contains(&key)) {
            m.insert("broken".into(), json!(true));
        }
        if let Some(h) = super::jsx::js_get(&st.data.topic_health, &key) {
            if let Some(at) = h.at {
                m.insert("lastSentAt".into(), json!(at as i64));
            }
            if let Some(e) = h.error {
                m.insert("lastError".into(), json!(e));
            }
        }
        Value::Object(m)
    }

    pub fn command_installed(&self, name: &str) -> bool {
        self.deps.claude_root.join("commands").join(format!("{name}.md")).exists()
    }

    pub fn state(&self) -> Value {
        let configured = self.token().is_some();
        let keychain = self.keychain();
        let hash = self.lock().data.api_hash.clone();
        let has_hash = self.unseal(hash.as_deref()).is_some();
        let cmd = self.command_installed(SEND_COMMAND);
        let recv = self.command_installed(RECEIVE_COMMAND);
        let now = self.now();
        let rust_account = self.deps.mt.as_ref().map(|_| {
            self.mt_notice();
            self.mt_account()
        });
        let st = self.lock();
        let d = &st.data;
        let pairing = st.pair_code.is_some() && st.pair_until.unwrap_or(0) > now;
        let owner_id = d.owner.as_ref().map(|o| o.0);
        let members: Vec<Value> = d.members.iter().filter(|m| m["id"].as_i64() != owner_id).cloned().collect();
        let strangers: Vec<Value> = st.strangers.iter().rev().filter(|(id, _)| Some(*id) != owner_id && !d.members.iter().any(|m| m["id"].as_i64() == Some(*id))).map(|(_, v)| v.clone()).collect();
        let h = &st.health;
        json!({
            "receiving": st.receiving,
            "owner": d.owner.as_ref().map(|(id, n)| json!({"id": id, "name": n})),
            "pairCode": if pairing { json!(st.pair_code) } else { Value::Null },
            "pairUntil": if pairing { json!(st.pair_until) } else { Value::Null },
            "ignored": st.ignored,
            "receiveInstalled": recv,
            "enabled": d.enabled,
            "configured": configured,
            "keychain": keychain,
            "bot": d.bot,
            "includeReply": d.include_reply,
            "digest": d.digest,
            "waiting": d.waiting,
            "doneMinutes": d.done_minutes,
            "routes": d.routes.iter().map(|r| Tg::route_state(&st, super::shape::route_value(r), &r.chat_id, r.thread_id)).collect::<Vec<_>>(),
            "commandInstalled": cmd,
            "lastError": st.last_error,
            "pollConflict": st.poll_conflict,
            "quiet": d.quiet,
            "summary": d.summary,
            "alerts": d.alerts,
            "pinStatus": d.pin_status,
            "topicIcons": d.topic_icons,
            "health": {"lastSentAt": h.last_sent, "lastReceivedAt": h.last_received, "lastPollAt": h.last_poll, "sentToday": h.sent_today, "errorsToday": h.errors_today},
            "userAccount": rust_account.unwrap_or_else(|| d.user_session.as_ref().map(|_| d.user_name.clone().unwrap_or_else(|| "Telegram".into()))),
            "bigFiles": {"enabled": d.big_files, "apiId": d.api_id, "hasHash": has_hash},
            "sessionRoutes": d.session_routes.iter().map(|r| Tg::route_state(&st, super::shape::session_value(r), &r.chat_id, r.thread_id)).collect::<Vec<_>>(),
            "stranger": d.stranger,
            "members": members,
            "recentStrangers": strangers,
        })
    }
}
