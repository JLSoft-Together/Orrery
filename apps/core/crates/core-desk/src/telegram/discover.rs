use super::api::{chats_from, scan_updates, Rights, Seen};
use super::base::Tg;
use super::sanitize::thread_id;
use super::shape::Known;
use super::text::test as test_html;
use serde_json::{json, Map, Value};
use std::sync::Arc;

impl Tg {
    pub fn failed(&self, error: &'static str) -> Value {
        json!({"ok": false, "error": self.fail(error)})
    }

    pub fn who_am_i(&self) -> Value {
        let me = self.call_json("getMe", json!({}));
        if let Some(e) = me.error() {
            return self.failed(e);
        }
        let r = me.result();
        let (Some(id), Some(name)) = (r["id"].as_f64().filter(|_| r["id"].is_number()), r["username"].as_str()) else { return self.failed("status") };
        let id = id as i64;
        let changed = {
            let mut st = self.lock();
            st.bot_id = Some(id);
            if st.data.bot.as_deref() != Some(name) {
                st.data.bot = Some(super::jsx::slice(name, 64));
                true
            } else {
                false
            }
        };
        if changed {
            self.commit(false);
        }
        json!({"ok": true, "bot": name, "id": id})
    }

    pub fn test(self: &Arc<Self>, project: Option<&str>, session: Option<&str>) -> Value {
        if self.token().is_none() {
            return json!({"ok": false, "error": "config"});
        }
        let me = self.who_am_i();
        if me["ok"] != true {
            return me;
        }
        let route = {
            let st = self.lock();
            match (session, project) {
                (Some(s), _) => Tg::session_route_of(&st, Some(&s.to_lowercase())),
                (None, Some(p)) => Tg::route_for(&st, p),
                (None, None) => st.data.routes.first().cloned(),
            }
        };
        if let Some(r) = route {
            if let Some(e) = self.send_text(&r, test_html(self.vi()), None, None, None) {
                return json!({"ok": false, "error": e});
            }
        }
        self.ok();
        json!({"ok": true, "bot": me["bot"]})
    }

    fn seen_known(&self) -> (Seen, Vec<(String, Rights)>) {
        let st = self.lock();
        let mut seen = Seen::default();
        let mut rights = Vec::new();
        for (id, k) in &st.data.known {
            seen.set_chat(id, &k.title, k.is_forum);
            let mut topics = k.topics.clone();
            super::jsx::js_sort_keys(&mut topics);
            seen.set_topics(id, topics.into_iter().filter_map(|(t, n)| t.parse::<i64>().ok().map(|t| (t, n))).collect());
            rights.push((id.clone(), Rights { bot_admin: k.bot_admin, can_manage: k.can_manage, title: None, is_forum: None }));
        }
        (seen, rights)
    }

    pub fn discover(self: &Arc<Self>, cached: bool) -> Value {
        if self.token().is_none() {
            return json!({"ok": false, "error": "config"});
        }
        if cached {
            let (seen, rights) = self.seen_known();
            return json!({"ok": true, "chats": chats_from(&seen, &rights)});
        }
        let me = self.who_am_i();
        if me["ok"] != true {
            return me;
        }
        let (mut seen, _) = self.seen_known();
        let (routes, polling, poll_seen) = {
            let st = self.lock();
            (st.data.routes.clone(), st.poll_run != 0, st.poll_seen.clone())
        };
        for r in &routes {
            if seen.chat(&r.chat_id).is_none() {
                seen.set_chat(&r.chat_id, &r.chat_id, r.thread_id.is_some());
            }
        }
        match poll_seen.filter(|_| polling) {
            Some(p) => {
                for (id, (title, forum)) in &p.chats {
                    seen.set_chat(id, title, *forum);
                }
                for (id, list) in &p.topics {
                    let mine = seen.topics_mut(id);
                    for (t, n) in list {
                        super::api::topic_set(mine, *t, n.clone());
                    }
                }
            }
            None => {
                let updates = self.call_json("getUpdates", json!({"limit": 100, "timeout": 0, "allowed_updates": ["message", "edited_message", "my_chat_member"]}));
                if let Some(e) = updates.error() {
                    return self.failed(e);
                }
                scan_updates(&updates.result(), &mut seen);
            }
        }
        self.scan_as_user(&mut seen);
        let me_id = me["id"].clone();
        let mut rights: Vec<(String, Rights)> = Vec::new();
        let ids: Vec<String> = seen.chats.iter().map(|(k, _)| k.clone()).collect();
        for chat_id in ids {
            let chat = self.call_json("getChat", json!({"chat_id": chat_id}));
            if let Some(e) = chat.error() {
                if e == "chat" {
                    seen.chats.retain(|(k, _)| *k != chat_id);
                    continue;
                }
                return self.failed(e);
            }
            let c = chat.result();
            let member = self.call_json("getChatMember", json!({"chat_id": chat_id, "user_id": me_id}));
            let m = if member.ok() { member.result() } else { json!({}) };
            let status = m["status"].as_str().unwrap_or("");
            let admin = status == "administrator" || status == "creator";
            rights.push((chat_id, Rights { bot_admin: admin, can_manage: status == "creator" || (admin && m["can_manage_topics"] == true), title: c["title"].as_str().map(|t| super::jsx::slice(t, 120)), is_forum: Some(c["is_forum"] == true) }));
        }
        let chats = chats_from(&seen, &rights);
        self.remember(&chats);
        self.ok();
        json!({"ok": true, "chats": chats})
    }

    fn remember(&self, chats: &[Value]) {
        let mut st = self.lock();
        let mut known: Vec<(String, Known)> = Vec::new();
        for c in chats.iter().take(50) {
            let mut topics: Vec<(String, String)> = Vec::new();
            for t in c["topics"].as_array().cloned().unwrap_or_default().iter().take(200) {
                super::jsx::js_insert(&mut topics, &crate::js::number(t["threadId"].as_f64().unwrap_or(0.0)), t["name"].as_str().unwrap_or("").to_string());
            }
            super::jsx::js_insert(&mut known, c["chatId"].as_str().unwrap_or(""), Known { title: c["title"].as_str().unwrap_or("").into(), is_forum: c["isForum"] == true, topics, bot_admin: c["botAdmin"] == true, can_manage: c["canManageTopics"] == true });
        }
        st.data.known = known;
        Tg::save_st(&self.deps, &st);
    }

    pub fn create_topic(self: &Arc<Self>, chat: &str, project: &str, name: &str) -> Value {
        let Some(project) = Tg::norm_project(project) else { return json!({"ok": false, "error": "invalid-project"}) };
        let res = self.call_json("createForumTopic", json!({"chat_id": chat, "name": name}));
        if let Some(e) = res.error() {
            return if e == "chat" { json!({"ok": false, "error": "no-rights"}) } else { self.failed(e) };
        }
        let r = res.result();
        let Some(thread) = thread_id(&r["message_thread_id"]) else { return json!({"ok": false, "error": "status"}) };
        let topic_name = r["name"].as_str().map(|n| super::jsx::slice(n, 128)).unwrap_or_else(|| name.to_string());
        {
            let mut st = self.lock();
            let mut k = super::jsx::js_get(&st.data.known, chat).cloned().unwrap_or(Known { title: chat.into(), is_forum: true, ..Known::default() });
            super::jsx::js_insert(&mut k.topics, &thread.to_string(), topic_name.clone());
            super::jsx::js_insert(&mut st.data.known, chat, k);
        }
        let mut input = Map::new();
        input.insert("project".into(), json!(project));
        input.insert("chatId".into(), json!(chat));
        input.insert("threadId".into(), json!(thread));
        input.insert("label".into(), json!(crate::util::basename(&project)));
        let set = self.set_route(&input);
        if set["ok"] != true {
            return json!({"ok": false, "error": if set["error"] == "invalid-project" { "invalid-project" } else { "status" }});
        }
        json!({"ok": true, "state": set["state"], "topic": {"threadId": thread, "name": topic_name}})
    }

    fn user_keys(&self) -> Option<Value> {
        let (hash, id) = {
            let st = self.lock();
            (st.data.api_hash.clone(), st.data.api_id)
        };
        let hash = self.unseal(hash.as_deref())?;
        id.map(|id| json!({"apiId": id, "apiHash": hash}))
    }

    fn user_session(&self) -> Option<String> {
        let s = self.lock().data.user_session.clone();
        self.unseal(s.as_deref())
    }

    fn drop_user(&self) {
        let mut st = self.lock();
        st.data.user_session = None;
        st.data.user_name = None;
    }

    fn scan_as_user(&self, seen: &mut Seen) {
        let (Some(keys), Some(session)) = (self.user_keys(), self.user_session()) else { return };
        let res = self.host("telegram.user", json!({"op": "topics", "keys": keys, "session": session})).unwrap_or_else(|| json!({"ok": false, "error": "failed"}));
        if res["ok"] != true {
            if res["error"] == "auth" {
                self.drop_user();
                self.save();
            }
            return;
        }
        for c in res["chats"].as_array().cloned().unwrap_or_default() {
            let Some(id) = c["chatId"].as_str().filter(|id| super::sanitize::chat_id_ok(id)) else { continue };
            let title = c["title"].as_str().filter(|t| !t.is_empty()).map(str::to_string).or_else(|| seen.chat(id).map(|x| x.0.clone())).unwrap_or_else(|| id.to_string());
            seen.set_chat(id, &title, true);
            let topics: Vec<(i64, String)> = c["topics"].as_array().cloned().unwrap_or_default().iter().filter_map(|t| Some((thread_id(&t["threadId"])?, super::jsx::slice(t["name"].as_str()?, 128)))).collect();
            seen.set_topics(id, topics);
        }
    }

    pub fn user(self: &Arc<Self>, input: &Value) -> Value {
        let failed = json!({"ok": false, "error": "failed"});
        let action = input["action"].as_str().unwrap_or("");
        if action == "logout" {
            let keys = self.user_keys();
            let session = self.user_session();
            self.drop_user();
            let result = self.commit(false);
            if let (Some(k), Some(s)) = (keys, session) {
                self.host("telegram.user", json!({"op": "logout", "keys": k, "session": s}));
            }
            return if result["ok"] == true { json!({"ok": true, "state": result["state"]}) } else { failed };
        }
        let Some(keys) = self.user_keys() else { return json!({"ok": false, "error": "keys"}) };
        if !self.keychain() {
            return json!({"ok": false, "error": "keychain"});
        }
        let req = match action {
            "code" => json!({"op": "sendCode", "keys": keys, "phone": input["phone"]}),
            "signin" => json!({"op": "signIn", "code": input["code"]}),
            _ => json!({"op": "password", "password": input["password"]}),
        };
        let res = self.host("telegram.user", req).unwrap_or_else(|| json!({"ok": false, "error": "failed"}));
        if res["ok"] != true {
            const ERRORS: [&str; 10] = ["keys", "keychain", "phone", "code", "password", "expired", "flood", "auth", "network", "failed"];
            let e = res["error"].as_str().filter(|e| ERRORS.contains(e)).unwrap_or("failed");
            return json!({"ok": false, "error": e});
        }
        if let Some(next) = res["next"].as_str().filter(|n| *n == "code" || *n == "password") {
            return json!({"ok": true, "state": self.state(), "next": next});
        }
        let Some(session) = res["session"].as_str().and_then(|s| self.seal(s)) else { return failed };
        {
            let mut st = self.lock();
            st.data.user_session = Some(session);
            st.data.user_name = Some(super::jsx::slice(res["name"].as_str().unwrap_or("Telegram"), 64));
        }
        let result = self.commit(false);
        if result["ok"] == true {
            json!({"ok": true, "state": result["state"]})
        } else {
            failed
        }
    }
}
