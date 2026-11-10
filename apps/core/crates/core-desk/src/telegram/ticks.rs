use super::base::{topic_key, Tg};
use super::html::{summary_html, DayCounts};
use super::sanitize::hhmm_minute;
use super::shape::{pin_key_ok, Route, SessionRoute};
use super::text_more::{more, more1, pin_line};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

const PIN_EDIT_MS: i64 = 20_000;
const ICON_EDIT_MS: i64 = 20_000;
const MOODS: [(&str, &[&str]); 3] = [("working", &["⚡", "🔥", "💻"]), ("waiting", &["❓", "❗", "⁉", "‼"]), ("idle", &["✔", "✅", "💤", "🏁"])];

fn rank(m: &str) -> i32 {
    match m {
        "waiting" => 2,
        "working" => 1,
        _ => 0,
    }
}

type Mood = (String, i64, &'static str);
type IconTarget = (String, Option<i64>, String, Option<String>, bool);

struct Cache<'a> {
    tg: &'a Tg,
    map: HashMap<String, Option<Vec<Value>>>,
}

impl Cache<'_> {
    fn panes(&mut self, project: &str) -> Option<Vec<Value>> {
        if !self.map.contains_key(project) {
            let res = self.tg.ask(Tg::req("status", project));
            let v = (res["ok"] == true && res["action"] == "status").then(|| res["panes"].as_array().cloned().unwrap_or_default());
            self.map.insert(project.to_string(), v);
        }
        self.map.get(project).cloned().flatten()
    }
}

struct Topic {
    chat: String,
    thread: Option<i64>,
    label: String,
    routes: Vec<Route>,
    sessions: Vec<SessionRoute>,
}

impl Tg {
    pub fn tick(self: &Arc<Self>) -> bool {
        self.spawn(|tg| tg.flush_held());
        self.spawn(|tg| tg.pin_tick());
        self.spawn(|tg| tg.icon_tick());
        let now = self.now();
        let mut after = super::settings::After::default();
        let swept = Tg::sweep_stale(&mut self.lock(), now, &mut after);
        self.finish(after);
        if swept {
            self.commit(false);
        }
        let (enabled, summary) = {
            let st = self.lock();
            (st.data.enabled, st.data.summary.clone())
        };
        if !enabled || summary["enabled"] != true || self.token().is_none() {
            return false;
        }
        let at = summary["at"].as_str().map(hhmm_minute).unwrap_or(0);
        let day = self.day_key(now);
        let rows: Vec<(String, Vec<(String, DayCounts)>)> = {
            let mut st = self.lock();
            if st.data.summary_day.as_deref() == Some(day.as_str()) || self.local(now).minute_of_day() < at {
                return false;
            }
            st.data.summary_day = Some(day.clone());
            Tg::save_st(&self.deps, &st);
            drop(st);
            if self.quiet_now() {
                return false;
            }
            let st = self.lock();
            let counts = if day == st.counts_day { st.counts.clone() } else { Vec::new() };
            let started = st.started_at;
            if counts.is_empty() && self.day_key(started) == day && self.local(started).minute_of_day() >= at {
                return false;
            }
            let mut by_chat: Vec<(String, Vec<(String, DayCounts)>)> = Vec::new();
            for r in &st.data.routes {
                let c = super::jsx::js_get(&counts, &r.project).copied().unwrap_or_default();
                match by_chat.iter_mut().find(|(k, _)| *k == r.chat_id) {
                    Some((_, l)) => l.push((r.label.clone(), c)),
                    None => by_chat.push((r.chat_id.clone(), vec![(r.label.clone(), c)])),
                }
            }
            by_chat
        };
        let vi = self.vi();
        for (chat, list) in rows {
            if self.take(&chat) {
                self.post(&chat, None, &summary_html(vi, &day, &list), Map::new());
            }
        }
        true
    }

    fn migrate_pins(&self) {
        let old: Vec<(String, i64)> = {
            let mut st = self.lock();
            let old: Vec<(String, i64)> = st.data.pins.iter().filter(|(k, _)| !pin_key_ok(k)).map(|(_, p)| p.clone()).collect();
            if old.is_empty() {
                return;
            }
            st.data.pins.retain(|(k, _)| pin_key_ok(k));
            Tg::save_st(&self.deps, &st);
            old
        };
        for (chat, id) in old {
            self.call_json("unpinChatMessage", json!({"chat_id": chat, "message_id": id}));
        }
    }

    pub fn pin_tick(self: &Arc<Self>) {
        {
            let mut st = self.lock();
            if st.pin_busy || !st.data.enabled {
                return;
            }
            st.pin_busy = true;
        }
        if self.token().is_some() {
            self.pin_work();
        }
        self.lock().pin_busy = false;
    }

    fn pin_work(self: &Arc<Self>) {
        self.migrate_pins();
        if !self.lock().data.pin_status {
            return;
        }
        let vi = self.vi();
        let mut cache = Cache { tg: self, map: HashMap::new() };
        let (topics, own, agents) = {
            let st = self.lock();
            let own: HashSet<String> = st.data.session_routes.iter().filter_map(|s| st.terminal_of_agent.get(&s.session).cloned()).collect();
            let mut topics: Vec<(String, Topic)> = Vec::new();
            let mut topic = |chat: &str, thread: Option<i64>, label: &str| -> usize {
                let key = topic_key(chat, thread);
                match topics.iter().position(|(k, _)| *k == key) {
                    Some(p) => p,
                    None => {
                        topics.push((key, Topic { chat: chat.into(), thread, label: label.into(), routes: Vec::new(), sessions: Vec::new() }));
                        topics.len() - 1
                    }
                }
            };
            let mut placed: Vec<(usize, Option<Route>, Option<SessionRoute>)> = Vec::new();
            for r in &st.data.routes {
                placed.push((topic(&r.chat_id, r.thread_id, &r.label), Some(r.clone()), None));
            }
            for s in &st.data.session_routes {
                placed.push((topic(&s.chat_id, s.thread_id, &s.name), None, Some(s.clone())));
            }
            for (i, r, s) in placed {
                if let Some(r) = r {
                    topics[i].1.routes.push(r);
                }
                if let Some(s) = s {
                    topics[i].1.sessions.push(s);
                }
            }
            (topics, own, st.terminal_of_agent.clone())
        };
        let now = self.now();
        let minutes = |p: &Value| p["since"].as_f64().filter(|s| *s != 0.0).map(|s| super::jsx::round((now as f64 - s) / 60_000.0).max(0));
        let state = |p: &Value| match p["status"].as_str() {
            Some(s @ ("working" | "waiting" | "idle")) => s.to_string(),
            _ => "unknown".to_string(),
        };
        for (key, t) in topics {
            let mut lines: Vec<String> = Vec::new();
            let mut seen: HashSet<String> = HashSet::new();
            let mut complete = true;
            for r in &t.routes {
                let Some(all) = cache.panes(&r.project) else {
                    complete = false;
                    break;
                };
                for p in &all {
                    let id = p["terminalId"].as_str().map(str::to_string).unwrap_or_else(|| format!("{}\n{}", r.project, p["name"].as_str().unwrap_or("")));
                    let mine = p["terminalId"].as_str().is_some_and(|t| !t.is_empty() && own.contains(t));
                    if seen.contains(&id) || mine {
                        continue;
                    }
                    seen.insert(id);
                    lines.push(pin_line(vi, p["name"].as_str().unwrap_or(""), &state(p), p["tool"].as_str(), minutes(p)));
                }
            }
            for s in &t.sessions {
                if !complete {
                    break;
                }
                let Some(all) = cache.panes(&s.project) else {
                    complete = false;
                    break;
                };
                let p = agents.get(&s.session).and_then(|term| all.iter().find(|x| x["terminalId"].as_str() == Some(term)));
                match p {
                    Some(p) => {
                        let tid = p["terminalId"].as_str().unwrap_or("").to_string();
                        if seen.contains(&tid) {
                            continue;
                        }
                        seen.insert(tid);
                        let name = if s.name.is_empty() { p["name"].as_str().unwrap_or("").to_string() } else { s.name.clone() };
                        lines.push(pin_line(vi, &name, &state(p), p["tool"].as_str(), minutes(p)));
                    }
                    None => lines.push(pin_line(vi, &s.name, "closed", None, None)),
                }
            }
            if !complete {
                continue;
            }
            let head = more1(vi, "pinHead", t.routes.first().map(|r| r.label.as_str()).unwrap_or(&t.label));
            let body = if lines.is_empty() { more(vi, "pinEmpty").to_string() } else { lines.join("\n") };
            let html = format!("{head}\n{body}");
            let skip = {
                let st = self.lock();
                st.pin_text.get(&key) == Some(&html) || st.pin_at.get(&key).is_some_and(|at| now - at < PIN_EDIT_MS)
            };
            if skip || self.hold_left(&t.chat) > 0 {
                continue;
            }
            self.lock().pin_at.insert(key.clone(), now);
            let pin = super::jsx::js_get(&self.lock().data.pins, &key).cloned();
            if let Some((chat, mid)) = pin.filter(|p| p.0 == t.chat) {
                let edit = self.call_json("editMessageText", json!({"chat_id": chat, "message_id": mid, "text": html, "parse_mode": "HTML", "disable_web_page_preview": true}));
                if edit.ok() || edit.error() == Some("status") {
                    self.lock().pin_text.insert(key.clone(), html.clone());
                    let shown = if edit.ok() { edit } else { super::api::Tg::Ok(Value::Null) };
                    self.note_topic(Some(key.clone()), &shown);
                    continue;
                }
                self.note_topic(Some(key.clone()), &edit);
                if edit.error() != Some("chat") {
                    continue;
                }
            }
            let mut extra = Map::new();
            extra.insert("disable_notification".into(), json!(true));
            let Some(sent) = self.post(&t.chat, t.thread, &html, extra) else { continue };
            {
                let mut st = self.lock();
                st.pin_text.insert(key.clone(), html.clone());
                super::jsx::js_insert(&mut st.data.pins, &key, (t.chat.clone(), sent));
                Tg::save_st(&self.deps, &st);
            }
            self.call_json("pinChatMessage", json!({"chat_id": t.chat, "message_id": sent, "disable_notification": true}));
        }
    }

    pub fn icon_tick(self: &Arc<Self>) {
        {
            let mut st = self.lock();
            if st.icon_busy || !st.data.enabled || !st.data.topic_icons {
                return;
            }
            st.icon_busy = true;
        }
        if self.token().is_some() {
            self.icon_work();
        }
        self.lock().icon_busy = false;
    }

    fn icon_work(&self) {
        let Some(ids) = self.topic_icon_ids() else { return };
        let Some(moods) = self.topic_moods() else { return };
        let stale: Vec<String> = self.lock().icon_set.iter().filter(|(k, _)| !moods.iter().any(|(m, _)| m == k)).map(|(k, _)| k.clone()).collect();
        self.clear_icons(&stale);
        let now = self.now();
        for (key, (chat, thread, mood)) in moods {
            let Some(id) = ids.iter().find(|(m, _)| *m == mood).map(|(_, id)| id.clone()) else { continue };
            let skip = {
                let st = self.lock();
                super::jsx::js_get(&st.icon_set, &key).is_some_and(|s| s.2 == id) || st.icon_off.contains(&chat) || now - st.icon_at.get(&key).copied().unwrap_or(0) < ICON_EDIT_MS
            };
            if skip {
                continue;
            }
            self.lock().icon_at.insert(key.clone(), now);
            let res = self.call_json("editForumTopic", json!({"chat_id": chat, "message_thread_id": thread, "icon_custom_emoji_id": id}));
            let mut st = self.lock();
            if res.ok() || res.error() == Some("status") {
                super::jsx::js_insert(&mut st.icon_set, &key, (chat, thread, id));
            } else if res.error() == Some("chat") {
                st.icon_off.insert(chat);
            }
        }
    }

    fn topic_icon_ids(&self) -> Option<Vec<(&'static str, String)>> {
        if let Some(ids) = self.lock().icon_ids.clone() {
            return Some(ids);
        }
        let res = self.call_json("getForumTopicIconStickers", json!({}));
        let list = res.result().as_array().cloned().filter(|_| res.ok())?;
        let mut by: Vec<(String, String)> = Vec::new();
        for s in &list {
            if let (Some(e), Some(id)) = (s["emoji"].as_str(), s["custom_emoji_id"].as_str()) {
                super::jsx::js_insert(&mut by, &e.replace('\u{fe0f}', ""), id.to_string());
            }
        }
        let ids: Vec<(&'static str, String)> = MOODS.iter().filter_map(|(mood, list)| list.iter().find_map(|e| super::jsx::js_get(&by, e).cloned()).map(|id| (*mood, id))).collect();
        self.lock().icon_ids = Some(ids.clone());
        Some(ids)
    }

    fn topic_moods(&self) -> Option<Vec<(String, Mood)>> {
        let (targets, own) = {
            let st = self.lock();
            let own: HashSet<String> = st.data.session_routes.iter().filter_map(|s| st.terminal_of_agent.get(&s.session).cloned()).collect();
            let mut t: Vec<IconTarget> = st.data.routes.iter().map(|r| (r.chat_id.clone(), r.thread_id, r.project.clone(), None, false)).collect();
            t.extend(st.data.session_routes.iter().map(|s| (s.chat_id.clone(), s.thread_id, s.project.clone(), st.terminal_of_agent.get(&s.session).cloned(), true)));
            (t, own)
        };
        let mut cache = Cache { tg: self, map: HashMap::new() };
        let mut out: Vec<(String, Mood)> = Vec::new();
        for (chat, thread, project, terminal, agent) in targets {
            let Some(thread) = thread else { continue };
            let all = cache.panes(&project)?;
            let panes: Vec<&Value> = if agent {
                all.iter().filter(|p| terminal.is_some() && p["terminalId"].as_str() == terminal.as_deref()).collect()
            } else {
                all.iter().filter(|p| p["terminalId"].as_str().filter(|t| !t.is_empty()).is_none_or(|t| !own.contains(t))).collect()
            };
            let mood: &'static str = if panes.iter().any(|p| p["status"] == "waiting") {
                "waiting"
            } else if panes.iter().any(|p| p["status"] == "working") {
                "working"
            } else {
                "idle"
            };
            let key = topic_key(&chat, Some(thread));
            let better = super::jsx::js_get(&out, &key).is_none_or(|prev| rank(mood) > rank(prev.2));
            if better {
                super::jsx::js_insert(&mut out, &key, (chat, thread, mood));
            }
        }
        Some(out)
    }

    pub fn clear_icons(&self, keys: &[String]) {
        for key in keys {
            let set = {
                let mut st = self.lock();
                st.icon_at.remove(key);
                super::jsx::js_remove(&mut st.icon_set, key)
            };
            if let Some((chat, thread, _)) = set {
                self.call_json("editForumTopic", json!({"chat_id": chat, "message_thread_id": thread, "icon_custom_emoji_id": ""}));
            }
        }
    }
}
