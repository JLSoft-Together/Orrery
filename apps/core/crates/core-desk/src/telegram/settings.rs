use super::base::{St, Tg};
use super::sanitize::{token_ok, MEMBERS_MAX};
use super::shape::SessionRoute;
use super::text::{MENU_EN, MENU_VI};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub const PAIR_MS: i64 = 5 * 60_000;
pub const STARTUP_SWEEP_MS: i64 = 3 * 60_000;

pub struct Notice {
    pub route: SessionRoute,
    pub why: &'static str,
    pub next: Option<Value>,
}

#[derive(Default)]
pub struct After {
    pub unpins: Vec<(String, i64)>,
    pub notices: Vec<Notice>,
}

impl Tg {
    pub fn set(self: &Arc<Self>, patch: &Map<String, Value>) -> Value {
        let give = |k: &str| patch.get(k);
        let truthy = |k: &str| give(k).and_then(Value::as_str).is_some_and(|s| !s.is_empty());
        if (truthy("apiHash") || truthy("token")) && !self.keychain() {
            return json!({"ok": false, "error": "keychain"});
        }
        if truthy("token") && !token_ok(give("token").and_then(Value::as_str).unwrap_or("")) {
            return json!({"ok": false, "error": "invalid-token"});
        }
        let sealed_token = match give("token").and_then(Value::as_str).filter(|t| !t.is_empty()) {
            Some(t) => match self.seal(t) {
                Some(s) => Some(Some(s)),
                None => return json!({"ok": false, "error": "failed"}),
            },
            None => give("token").map(|_| None),
        };
        let sealed_hash = match give("apiHash").and_then(Value::as_str).filter(|t| !t.is_empty()) {
            Some(h) => match self.seal(h) {
                Some(s) => Some(Some(s)),
                None => return json!({"ok": false, "error": "failed"}),
            },
            None => give("apiHash").map(|_| None),
        };
        let mut clear_icons = Vec::new();
        {
            let mut st = self.lock();
            if let Some(tok) = &sealed_token {
                st.data.token = tok.clone();
                st.data.mt_session = None;
                st.data.last_update = None;
                st.data.bot = None;
                st.data.commands_hash = None;
                st.bot_id = None;
                st.poll_run = 0;
                if tok.is_none() {
                    st.data.enabled = false;
                    st.receiving = false;
                }
            }
        }
        if let Some(r) = give("receiving").and_then(Value::as_bool) {
            let configured = self.token().is_some();
            let mut st = self.lock();
            st.receiving = r && configured && st.data.owner.is_some();
        }
        {
            let mut st = self.lock();
            let b = |k: &str| give(k).and_then(Value::as_bool);
            if let Some(v) = b("includeReply") {
                st.data.include_reply = v;
            }
            if let Some(v) = b("digest") {
                st.data.digest = v;
            }
            if let Some(v) = b("waiting") {
                st.data.waiting = v;
            }
            if let Some(v) = give("doneMinutes").and_then(Value::as_i64) {
                st.data.done_minutes = v;
            }
            if let Some(v) = b("enabled") {
                st.data.enabled = v;
            }
            if let Some(q) = give("quiet") {
                st.data.quiet = q.clone();
            }
            if let Some(q) = give("summary") {
                st.data.summary = q.clone();
            }
            if let Some(v) = b("alerts") {
                st.data.alerts = v;
            }
            if let Some(v) = b("pinStatus") {
                st.data.pin_status = v;
            }
            if let Some(v) = b("topicIcons") {
                if !v && st.data.topic_icons {
                    clear_icons = st.icon_set.iter().map(|(k, _)| k.clone()).collect();
                }
                st.data.topic_icons = v;
            }
            if let Some(h) = &sealed_hash {
                st.data.api_hash = h.clone();
                st.data.mt_session = None;
            }
            if let Some(v) = give("apiId") {
                let id = v.as_i64();
                if id != st.data.api_id {
                    st.data.mt_session = None;
                    st.data.user_session = None;
                    st.data.user_name = None;
                }
                st.data.api_id = id;
            }
            if let Some(v) = b("bigFiles") {
                st.data.big_files = v;
            }
            if let Some(s) = give("stranger") {
                st.data.stranger = s.clone();
            }
            if let Some(Value::Array(list)) = give("members") {
                let owner = st.data.owner.as_ref().map(|o| o.0);
                st.data.members = list.iter().filter(|m| m["id"].as_i64() != owner).take(MEMBERS_MAX).cloned().collect();
            }
        }
        if !clear_icons.is_empty() {
            self.spawn(move |tg| tg.clear_icons(&clear_icons));
        }
        let result = self.commit(sealed_token.is_some());
        let new_token = matches!(sealed_token, Some(Some(_)));
        if new_token {
            self.spawn(|tg| {
                tg.who_am_i();
            });
        }
        let receive_on = give("receiving") == Some(&Value::Bool(true));
        self.sync_poll(receive_on || new_token);
        if self.lock().receiving && (receive_on || new_token) {
            self.spawn(|tg| tg.sync_commands());
        }
        result
    }

    pub fn sync_commands(&self) {
        let sets = {
            let en: Vec<Value> = MENU_EN.iter().map(|(a, b)| json!([a, b])).collect();
            let mut v = vec![json!({"lang": "", "list": en})];
            if self.vi() {
                let vi: Vec<Value> = MENU_VI.iter().map(|(a, b)| json!([a, b])).collect();
                v.push(json!({"lang": "vi", "list": vi}));
            }
            v
        };
        let hash: String = Sha256::digest(crate::js::stringify(&Value::Array(sets.clone())).as_bytes()).iter().map(|b| format!("{b:02x}")).collect::<String>()[..16].to_string();
        let sealed = {
            let mut st = self.lock();
            if st.commands_busy || st.data.commands_hash.as_deref() == Some(hash.as_str()) {
                return;
            }
            st.commands_busy = true;
            st.data.token.clone()
        };
        let mut done = true;
        for s in &sets {
            let commands: Vec<Value> = s["list"].as_array().cloned().unwrap_or_default().iter().map(|p| json!({"command": p[0], "description": p[1]})).collect();
            let mut body = json!({"commands": commands, "scope": {"type": "all_group_chats"}});
            if s["lang"] != "" {
                body["language_code"] = s["lang"].clone();
            }
            if !self.call_json("setMyCommands", body).ok() {
                done = false;
                break;
            }
        }
        let mut st = self.lock();
        st.commands_busy = false;
        if done && st.data.token == sealed {
            st.data.commands_hash = Some(hash);
            Tg::save_st(&self.deps, &st);
        }
    }

    pub fn pair(self: &Arc<Self>, on: bool) -> Value {
        if on && self.token().is_none() {
            return json!({"ok": false, "error": "failed"});
        }
        let now = self.now();
        {
            let mut st = self.lock();
            if on {
                let mut b = [0u8; 4];
                let _ = getrandom::fill(&mut b);
                st.pair_code = Some(format!("{:06}", u32::from_le_bytes(b) % 1_000_000));
                st.pair_until = Some(now + PAIR_MS);
            } else {
                st.pair_code = None;
                st.pair_until = None;
            }
            st.pair_tries.clear();
            st.pair_tries_total = 0;
        }
        let res = self.commit(false);
        self.sync_poll(false);
        res
    }

    pub fn unpair(self: &Arc<Self>) -> Value {
        {
            let mut st = self.lock();
            st.data.owner = None;
            st.receiving = false;
        }
        let res = self.commit(false);
        self.sync_poll(false);
        res
    }

    pub fn pairing(st: &St, now: i64) -> bool {
        st.pair_code.is_some() && st.pair_until.unwrap_or(0) > now
    }

    pub fn want_updates(&self) -> bool {
        let now = self.now();
        let mut st = self.lock();
        let owner = st.data.owner.is_some();
        st.receiving || Tg::pairing(&st, now) || (owner && st.reply_buttons.pending(now))
    }

    pub fn sync_poll(self: &Arc<Self>, retry: bool) {
        let want = self.token().is_some() && self.want_updates();
        let start = {
            let mut st = self.lock();
            if want && st.poll_run == 0 && (!st.poll_conflict || retry) {
                st.timer_gen += 1;
                let run = st.timer_gen;
                st.poll_run = run;
                st.conflict_runs = 0;
                Some(run)
            } else {
                if !want {
                    st.poll_run = 0;
                    if st.poll_conflict {
                        st.poll_conflict = false;
                        drop(st);
                        self.emit_state();
                    }
                }
                None
            }
        };
        if let Some(run) = start {
            self.spawn(move |tg| tg.poll_loop(run));
        }
    }

    pub fn poll_failed(&self, error: &'static str, run: u64) -> bool {
        if error != "conflict" {
            return false;
        }
        let first = {
            let mut st = self.lock();
            let first = !st.poll_conflict;
            st.poll_conflict = true;
            first
        };
        self.fail("conflict");
        let stop = {
            let mut st = self.lock();
            st.conflict_runs += 1;
            let stop = st.conflict_runs >= 3;
            if stop && st.poll_run == run {
                st.poll_run = 0;
            }
            stop
        };
        if first || stop {
            self.emit_state();
        }
        stop
    }

    pub fn poll_ok(&self) {
        {
            let mut st = self.lock();
            st.conflict_runs = 0;
            if !st.poll_conflict {
                return;
            }
            st.poll_conflict = false;
            if st.last_error == Some("conflict") {
                st.last_error = None;
            }
        }
        self.emit_state();
    }

    pub fn stop(&self) {
        let mut st = self.lock();
        st.poll_run = 0;
        st.held_timers.clear();
    }

}
