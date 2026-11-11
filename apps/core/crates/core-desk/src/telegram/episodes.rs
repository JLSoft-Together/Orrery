use super::base::{topic_key, DigestMsg, Entry, Target, Tg, UPLOAD_TIMEOUT_MS};
use super::html::{cell, cell_t, digest_html, episode_html, markup, quote, Row, CAPTION_MAX, MESSAGE_MAX};
use super::outbound::{MAX_EPISODES, SENT_MAP_MAX};
use super::shape::Route;
use super::text_act::{disk, limit, run_failed};
use super::text_more::{plan_label, run_shot, tail_button};
use serde_json::{json, Map, Value};
use std::sync::Arc;

const RUN_SHOT_DELAY_MS: u64 = 8000;
const RUN_LOG_LINES: usize = 200;
const RUN_DETAIL_MAX: usize = 1500;

impl Tg {
    pub fn handle(self: &Arc<Self>, e: &Value) -> bool {
        let reason = e["reason"].as_str().unwrap_or("");
        let terminal = e["terminalId"].as_str().unwrap_or("").to_string();
        let episode = e["episode"].as_str().unwrap_or("").to_string();
        let cwd = e["cwd"].as_str().unwrap_or("");
        let worked = e["worked"].as_f64();
        let now = self.now();
        let counted = Tg::route_for(&self.lock(), cwd);
        if reason == "done" {
            if let Some(reply) = e["reply"].as_str().filter(|r| !r.is_empty()) {
                let mut st = self.lock();
                super::jsx::js_insert(&mut st.last_replies, &terminal, (e["name"].as_str().unwrap_or("").to_string(), reply.to_string()));
                if st.last_replies.len() > 200 {
                    st.last_replies.remove(0);
                }
            }
        }
        if let Some(c) = &counted {
            let key = format!("c:{episode}");
            let fresh = {
                let mut st = self.lock();
                let fresh = !st.sent_set.contains(&key);
                if fresh {
                    Tg::seen_add(&mut st, key);
                }
                fresh
            };
            if fresh {
                self.counts_add(&c.project, |d| match reason {
                    "done" => {
                        d.done += 1;
                        d.worked_ms += worked.unwrap_or(0.0);
                    }
                    "permission" => d.permissions += 1,
                    _ => d.waits += 1,
                });
            }
        }
        let (enabled, seen, waiting, done_min, include_reply, digest, receiving) = {
            let st = self.lock();
            (st.data.enabled, st.sent_set.contains(&episode), st.data.waiting, st.data.done_minutes, st.data.include_reply, st.data.digest, st.receiving)
        };
        if !enabled || seen || (reason != "done" && !waiting) {
            return false;
        }
        if reason == "done" && worked.unwrap_or(0.0) < (done_min * 60_000) as f64 {
            return false;
        }
        if self.quiet_now() {
            return false;
        }
        let route = {
            let st = self.lock();
            let agent = st.agent_of_terminal.get(&terminal).cloned();
            Tg::session_route_of(&st, agent.as_deref()).or(counted)
        };
        let Some(route) = route else { return false };
        if reason == "done" {
            if let Some(r) = self.lock().reported_at.get(&route.project) {
                if (*r as f64) >= now as f64 - worked.unwrap_or(0.0) {
                    return false;
                }
            }
        }
        {
            let mut st = self.lock();
            Tg::seen_add(&mut st, episode.clone());
            if st.sent.len() > MAX_EPISODES {
                if let Some(old) = st.sent.pop_front() {
                    st.sent_set.remove(&old);
                }
            }
        }
        let vi = self.vi();
        let keys = self.with_pane_link(receiving.then(|| self.episode_keys(e, &route, vi)), e, vi);
        let target = Target::Terminal(terminal);
        if reason == "done" && digest {
            return self.digest_done(&route, e, vi, target, keys, include_reply);
        }
        self.send_text(&route, &episode_html(e, vi, &self.deps.masker, include_reply), Some(target), keys, Some(reason != "done")).is_none()
    }

    fn episode_keys(&self, e: &Value, route: &Route, vi: bool) -> Value {
        let since = e["since"].as_f64();
        let entry = Entry { terminal_id: e["terminalId"].as_str().unwrap_or("").into(), project: route.project.clone(), cwd: e["cwd"].as_str().unwrap_or("").into(), name: e["name"].as_str().unwrap_or("").into(), chat_id: route.chat_id.clone(), thread_id: route.thread_id, since, tool: e["tool"].as_str().filter(|t| !t.is_empty()).map(str::to_string), ..Entry::default() };
        let k = |kind: &str| self.add_action(Entry { kind: kind.into(), ..entry.clone() });
        let tail = || cell_t("tl", &self.add_action(Entry { kind: "tl".into(), extra: Some("20".into()), ..entry.clone() }), tail_button(vi));
        let esc_tail = || -> Row { if since.is_some() { vec![cell("es", &k("es")), tail()] } else { vec![tail()] } };
        match e["reason"].as_str().unwrap_or("") {
            "done" => markup(vi, &[vec![cell("go", &k("go")), cell("ts", &k("ts")), cell("df", &k("df"))]]),
            "question" => {
                let mut rows: Vec<Row> = Vec::new();
                let ask = &e["ask"];
                if ask.is_object() && ask["multi"] != true && ask["more"] != true && since.is_some() {
                    for (i, o) in ask["options"].as_array().cloned().unwrap_or_default().iter().enumerate() {
                        let label = self.deps.masker.clean(o.as_str().unwrap_or(""));
                        let key = self.add_action(Entry { kind: "qa".into(), extra: Some((i + 1).to_string()), label: Some(label.clone()), ..entry.clone() });
                        rows.push(vec![cell_t("qa", &key, &format!("{} · {label}", i + 1))]);
                    }
                }
                rows.push(esc_tail());
                markup(vi, &rows)
            }
            "permission" => {
                let mut rows: Vec<Row> = Vec::new();
                if since.is_some() {
                    rows.push(vec![cell("ap", &k("ap")), cell("dn", &k("dn"))]);
                }
                rows.push(esc_tail());
                let mut keys = markup(vi, &rows);
                if since.is_some() && e["tool"] == "ExitPlanMode" {
                    keys["inline_keyboard"][0][0]["text"] = json!(plan_label(vi, true));
                    keys["inline_keyboard"][0][1]["text"] = json!(plan_label(vi, false));
                }
                keys
            }
            _ => markup(vi, &[esc_tail()]),
        }
    }

    fn digest_done(self: &Arc<Self>, route: &Route, e: &Value, vi: bool, target: Target, keys: Option<Value>, include_reply: bool) -> bool {
        let terminal = e["terminalId"].as_str().unwrap_or("").to_string();
        let key = self.lock().agent_of_terminal.get(&terminal).cloned().unwrap_or(terminal);
        let now = self.now();
        let day = self.day_key(now);
        let prev = super::jsx::js_get(&self.lock().digest_msgs, &key).map(|p| (p.chat_id.clone(), p.message_id, p.day.clone(), p.n));
        if let Some((_, mid, _, n)) = prev.filter(|p| p.2 == day && p.0 == route.chat_id) {
            let n = n + 1;
            let hhmm = self.local(now).hhmm();
            let edit = self.call_json("editMessageText", json!({"chat_id": route.chat_id, "message_id": mid, "text": digest_html(e, vi, &self.deps.masker, include_reply, n, &hhmm), "parse_mode": "HTML", "disable_web_page_preview": true, "reply_markup": keys.clone().unwrap_or_else(|| json!({"inline_keyboard": []}))}));
            if edit.ok() {
                self.ok();
                if let Some((_, p)) = self.lock().digest_msgs.iter_mut().find(|(k, _)| *k == key) {
                    p.n = n;
                }
                self.track(&route.chat_id, &json!({"message_id": mid}), Some(&target));
                return true;
            }
        }
        let sent = self.send_text_id(route, &episode_html(e, vi, &self.deps.masker, include_reply), Some(target), keys, Some(false));
        let mut st = self.lock();
        match sent.id {
            Some(id) => {
                super::jsx::js_insert(&mut st.digest_msgs, &key, DigestMsg { chat_id: route.chat_id.clone(), message_id: id, day, n: 1 });
                if st.digest_msgs.len() > SENT_MAP_MAX {
                    st.digest_msgs.remove(0);
                }
            }
            None => {
                super::jsx::js_remove(&mut st.digest_msgs, &key);
            }
        }
        sent.error.is_none()
    }

    pub fn alert(self: &Arc<Self>, kind: &str, detail: &Value) -> bool {
        let (enabled, alerts) = {
            let st = self.lock();
            (st.data.enabled, st.data.alerts)
        };
        if !enabled || !alerts || self.token().is_none() {
            return false;
        }
        if kind == "limit" {
            return self.limit_alert(detail);
        }
        let html = disk(self.vi(), detail["label"].as_str().unwrap_or(""), detail["free"].as_str().unwrap_or(""));
        let mut chats: Vec<String> = Vec::new();
        for r in &self.lock().data.routes {
            if !chats.contains(&r.chat_id) {
                chats.push(r.chat_id.clone());
            }
        }
        for chat in chats {
            if self.take(&chat) {
                self.post(&chat, None, &html, Map::new());
            }
        }
        true
    }

    fn reset_label(&self, at: i64) -> String {
        let d = self.local(at);
        let hhmm = d.hhmm();
        if self.day_key(at) == self.day_key(self.now()) {
            hhmm
        } else {
            format!("{:02}/{:02} {hhmm}", d.d, d.mo)
        }
    }

    fn limit_alert(self: &Arc<Self>, info: &Value) -> bool {
        let vi = self.vi();
        let window = info["window"].as_str().filter(|w| *w == "five_hour" || *w == "seven_day");
        let used = info["usedPct"].as_f64();
        let hit = info["hit"] == true;
        let resets = info["resetsAt"].as_f64().filter(|r| *r != 0.0);
        let mut text = Map::new();
        if let Some(w) = window {
            text.insert("window".into(), json!(w));
        }
        if let Some(u) = used {
            text.insert("usedPct".into(), json!(u));
        }
        if hit {
            text.insert("hit".into(), json!(true));
        }
        if let Some(r) = resets {
            text.insert("reset".into(), json!(self.reset_label(r as i64)));
        }
        let any = window.is_some() || used.is_some() || resets.is_some() || hit;
        let text = Value::Object(text);
        let html = limit(vi, any.then_some(&text));
        let now = self.now() as f64;
        let period = match resets {
            Some(r) => (r / 60_000.0).floor(),
            None => (now / if window == Some("seven_day") { 7.0 * 24.0 * 3_600_000.0 } else { 5.0 * 3_600_000.0 }).floor(),
        };
        let kind_key = format!("{}|{}|{}", if hit { "hit" } else { "forecast" }, window.unwrap_or("?"), crate::js::number(period));
        let targets: Vec<(String, (String, Option<i64>))> = {
            let mut st = self.lock();
            let mut out: Vec<(String, (String, Option<i64>))> = Vec::new();
            for (agent, _, cwd) in &st.open_sessions {
                if let Some(r) = Tg::session_route_of(&st, Some(agent)).or_else(|| Tg::route_for(&st, cwd)) {
                    super::jsx::js_insert(&mut out, &topic_key(&r.chat_id, r.thread_id), (r.chat_id.clone(), r.thread_id));
                }
            }
            if out.is_empty() {
                for r in &st.data.routes {
                    super::jsx::js_insert(&mut out, &format!("{}:null", r.chat_id), (r.chat_id.clone(), None));
                }
            }
            if st.limit_sent.len() > 500 {
                st.limit_sent.clear();
            }
            out
        };
        let mut sent = false;
        for (key, (chat, thread)) in targets {
            let once = format!("{kind_key}|{key}");
            if self.lock().limit_sent.contains(&once) {
                continue;
            }
            let route = Route { project: String::new(), label: String::new(), chat_id: chat, thread_id: thread };
            if self.send_text(&route, &html, None, None, Some(true)).is_some() {
                continue;
            }
            self.lock().limit_sent.insert(once);
            sent = true;
        }
        sent
    }

    pub fn run_state(self: &Arc<Self>, session: &Value) {
        let id = session["id"].as_str().unwrap_or("");
        let w = {
            let mut st = self.lock();
            let Some((_, w)) = st.run_watch.iter_mut().find(|(k, _)| k == id) else { return };
            if w.shot || session["state"] != "running" {
                return;
            }
            w.shot = true;
            (w.chat_id.clone(), w.thread_id, w.config.clone())
        };
        (self.deps.sleep)(RUN_SHOT_DELAY_MS);
        let Some(shot) = self.host("telegram.screenshot", json!({"serial": session["deviceId"]})).filter(|s| s["ok"] == true) else { return };
        let Some(buf) = shot["png"].as_str().and_then(|p| std::fs::read(p).ok()) else { return };
        let body = Tg::photo_form(&w.0, w.1, &run_shot(self.vi(), &w.2, shot["device"].as_str().unwrap_or("")), "photo", buf, "screen.png", Some("image/png"));
        self.call("sendPhoto", body, UPLOAD_TIMEOUT_MS);
    }

    pub fn run_failed_report(self: &Arc<Self>, session: &Value) -> bool {
        let id = session["id"].as_str().unwrap_or("").to_string();
        {
            let mut st = self.lock();
            if st.run_failed.contains(&id) {
                return false;
            }
            st.run_failed.push_back(id.clone());
            if st.run_failed.len() > MAX_EPISODES {
                st.run_failed.pop_front();
            }
        }
        let route = Tg::route_for(&self.lock(), session["cwd"].as_str().unwrap_or(""));
        if let Some(r) = &route {
            self.counts_add(&r.project, |c| c.run_fails += 1);
        }
        let (enabled, alerts) = {
            let st = self.lock();
            (st.data.enabled, st.data.alerts)
        };
        let Some(route) = route.filter(|_| enabled && alerts) else { return false };
        let vi = self.vi();
        let failure = &session["failure"];
        let detail = super::jsx::slice(failure["detail"].as_str().unwrap_or(""), RUN_DETAIL_MAX);
        let head = run_failed(vi, session["configName"].as_str().unwrap_or(""), failure["code"].as_str().unwrap_or("failed"));
        let html = format!("{head}{}", quote(&detail, &self.deps.masker, MESSAGE_MAX as i64 - super::jsx::len(&head) as i64));
        let lines: Vec<String> = match self.host("telegram.runLog", json!({"sessionId": id})) {
            Some(Value::Array(l)) => l.into_iter().filter_map(|v| v.as_str().map(str::to_string)).collect(),
            _ => Vec::new(),
        };
        if lines.is_empty() {
            return self.send_text(&route, &html, None, None, None).is_none();
        }
        if !self.take(&route.chat_id) {
            return false;
        }
        let log = lines[lines.len().saturating_sub(RUN_LOG_LINES)..].join("\n");
        let caption = format!("{head}{}", quote(&detail, &self.deps.masker, CAPTION_MAX as i64 - super::jsx::len(&head) as i64));
        let body = Tg::photo_form(&route.chat_id, route.thread_id, &caption, "document", self.deps.masker.clean(&log).into_bytes(), "run.log", Some("text/plain"));
        if !self.call("sendDocument", body, UPLOAD_TIMEOUT_MS).ok() {
            return self.send_text(&route, &html, None, None, None).is_none();
        }
        true
    }
}
