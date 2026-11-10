use super::base::{Entry, Target, Tg};
use super::html::{cell, cell_t, episode_html, markup, result_html, tail_html, Row};
use super::message::TgMessage;
use super::shape::{Route, SessionRoute};
use super::text::{inbound, pane_status};
use super::text_act::act1;
use super::text_more::{more, more1, new_pick, where_project, where_session};
use serde_json::{json, Map, Value};
use std::sync::Arc;

pub enum Say<'a> {
    Reply(&'a TgMessage),
    Post(String, Option<i64>),
}

pub fn group_id() -> String {
    crate::util::uuid().chars().take(8).collect()
}

pub fn pane_terminal(p: &Value) -> Option<String> {
    p["terminalId"].as_str().filter(|t| !t.is_empty()).map(str::to_string)
}

pub fn has_terminal(p: &Value) -> bool {
    pane_terminal(p).is_some()
}

impl Tg {
    pub fn say(&self, to: &Say, html: &str, keys: Option<Value>) -> Option<i64> {
        match to {
            Say::Reply(msg) => self.reply(msg, html, keys),
            Say::Post(chat, thread) => {
                let mut extra = Map::new();
                if let Some(k) = keys {
                    extra.insert("reply_markup".into(), k);
                }
                self.post(chat, *thread, html, extra)
            }
        }
    }

    pub fn panes(&self, project: &str, only: Option<&str>) -> Vec<Value> {
        let res = self.ask(Tg::req("status", project));
        let panes = if res["ok"] == true && res["action"] == "status" { res["panes"].as_array().cloned().unwrap_or_default() } else { Vec::new() };
        match only {
            Some(t) => panes.into_iter().filter(|p| p["terminalId"].as_str() == Some(t)).collect(),
            None => panes,
        }
    }

    pub fn project_choices(&self, msg: &TgMessage) -> Vec<Route> {
        let st = self.lock();
        let here: Vec<Route> = st.data.routes.iter().filter(|r| r.chat_id == msg.chat_id && r.thread_id == msg.thread_id).cloned().collect();
        if here.len() == 1 || (here.is_empty() && msg.thread_id.is_some()) {
            return Vec::new();
        }
        let list = if here.is_empty() { st.data.routes.iter().filter(|r| r.chat_id == msg.chat_id).cloned().collect() } else { here };
        let mut seen: Vec<String> = Vec::new();
        let mut out = Vec::new();
        for r in list {
            if !seen.contains(&r.project) {
                seen.push(r.project.clone());
                out.push(r);
            }
        }
        out.truncate(8);
        out
    }

    pub fn pane_label(&self, p: &Value, vi: bool) -> String {
        let now = self.now() as f64;
        let min = p["since"].as_f64().filter(|s| *s != 0.0).map(|s| ((now - s) / 60_000.0).floor().max(0.0) as i64);
        let age = match min {
            None => String::new(),
            Some(m) if m < 1 => (if vi { "vừa xong" } else { "just now" }).to_string(),
            Some(m) if m < 60 => if vi { format!("{m} phút") } else { format!("{m}m") },
            Some(m) if m < 1440 => if vi { format!("{} giờ", m / 60) } else { format!("{}h", m / 60) },
            Some(m) => if vi { format!("{} ngày", m / 1440) } else { format!("{}d", m / 1440) },
        };
        let tail = [pane_status(p["status"].as_str().unwrap_or(""), vi), age].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ");
        let room = 48 - tail.chars().count() as i64 - 3;
        let name: Vec<char> = p["name"].as_str().unwrap_or("").chars().collect();
        let shown = if name.len() as i64 > room { format!("{}…", name.iter().take((room - 1).max(1) as usize).collect::<String>()) } else { name.iter().collect() };
        format!("{shown} · {tail}")
    }

    pub fn pick_markup(&self, kind: &str, panes: &[Value], project: &str, msg: &TgMessage, text: Option<&str>) -> Value {
        let vi = self.vi();
        let own = matches!(kind, "cp" | "cl" | "fc" | "np");
        let group = own.then(group_id);
        let rows: Vec<Row> = panes
            .iter()
            .take(8)
            .map(|p| {
                let e = Entry {
                    kind: kind.into(),
                    terminal_id: pane_terminal(p).unwrap_or_default(),
                    project: project.into(),
                    cwd: project.into(),
                    name: p["name"].as_str().unwrap_or("").into(),
                    chat_id: msg.chat_id.clone(),
                    thread_id: msg.thread_id,
                    by: own.then_some(msg.from_id),
                    group: group.clone(),
                    since: p["since"].as_f64().filter(|s| *s != 0.0),
                    extra: text.map(str::to_string),
                    ..Entry::default()
                };
                vec![cell_t(kind, &self.add_action(e), &self.pane_label(p, vi))]
            })
            .collect();
        markup(vi, &rows)
    }

    pub fn last_html(&self, terminal: &str, project: &str) -> String {
        let vi = self.vi();
        let last = super::jsx::js_get(&self.lock().last_replies, terminal).cloned();
        let Some((name, reply)) = last else { return more(vi, "lastNone").into() };
        let e = json!({"type": "telegram", "episode": "", "reason": "done", "terminalId": terminal, "name": name, "cwd": project, "reply": reply});
        let html = episode_html(&e, vi, &self.deps.masker, true);
        let rest = html.find('\n').map(|i| &html[i..]).unwrap_or("");
        format!("{}{rest}", more1(vi, "last", &name))
    }

    pub fn tail_text(&self, terminal: &str, project: &str, lines: i64) -> String {
        let vi = self.vi();
        let mut req = Tg::req("tail", project);
        req["lines"] = json!(lines);
        req["target"] = Target::Terminal(terminal.into()).value();
        let res = self.ask(req);
        if res["ok"] == true && res["action"] == "tail" {
            let token = self.token();
            tail_html(res["name"].as_str().unwrap_or(""), res["text"].as_str().unwrap_or(""), vi, &self.deps.masker, token.as_deref())
        } else {
            result_html(&res, vi)
        }
    }

    pub fn retire(&self, key: &str, entry: &Entry) {
        let mut st = self.lock();
        super::jsx::js_remove(&mut st.actions, key);
        if let Some(g) = &entry.group {
            st.actions.retain(|(_, e)| e.group.as_ref() != Some(g));
        }
    }

    pub fn link_markup(&self, msg: &TgMessage) -> Option<Value> {
        let open = {
            let st = self.lock();
            st.receiving && msg.thread_id.is_some() && !st.open_sessions.is_empty()
        };
        open.then(|| markup(self.vi(), &self.link_rows(&msg.chat_id, msg.thread_id, false)))
    }

    pub fn where_reply(&self, msg: &TgMessage, owner: bool, session: Option<&SessionRoute>, route: Option<&Route>) {
        let vi = self.vi();
        let Some(route) = route else {
            let keys = if owner { self.link_markup(msg) } else { None };
            let html = if keys.is_some() { more(vi, "linkPick") } else { inbound(vi, "noTopic") };
            self.reply(msg, html, keys);
            return;
        };
        let name = session.map(|s| s.name.clone()).unwrap_or_else(|| route.label.clone());
        let receiving = self.lock().receiving;
        let mut rows: Vec<Row> = Vec::new();
        if owner && receiving {
            let key = self.add_action(Entry { kind: "ul".into(), chat_id: msg.chat_id.clone(), thread_id: msg.thread_id, project: route.project.clone(), cwd: route.project.clone(), name, extra: session.map(|s| s.session.clone()), ..Entry::default() });
            rows.push(vec![cell("ul", &key)]);
        }
        let html = match session {
            Some(s) => {
                let open = self.lock().terminal_of_agent.contains_key(&s.session);
                where_session(vi, &s.name, &s.project, open, s.keep)
            }
            None => where_project(vi, &route.label, &route.project),
        };
        self.reply(msg, &html, (!rows.is_empty()).then(|| markup(vi, &rows)));
    }

    pub fn start_new(self: &Arc<Self>, to: &Say, project: &str, text: &str, from: &str, by: i64, open: &[Value]) {
        let vi = self.vi();
        if !open.is_empty() {
            let base = Entry { project: project.into(), cwd: project.into(), extra: Some(text.into()), by: Some(by), group: Some(group_id()), ..Entry::default() };
            let base = match to {
                Say::Reply(m) => Entry { chat_id: m.chat_id.clone(), thread_id: m.thread_id, ..base },
                Say::Post(c, t) => Entry { chat_id: c.clone(), thread_id: *t, ..base },
            };
            let mut rows: Vec<Row> = vec![vec![cell("nw", &self.add_action(Entry { kind: "nw".into(), ..base.clone() }))]];
            if !text.is_empty() {
                for p in open.iter().take(8) {
                    let key = self.add_action(Entry { kind: "np".into(), terminal_id: pane_terminal(p).unwrap_or_default(), name: p["name"].as_str().unwrap_or("").into(), ..base.clone() });
                    rows.push(vec![cell_t("np", &key, &self.pane_label(p, vi))]);
                }
            }
            self.say(to, &new_pick(vi, open.len(), !text.is_empty()), Some(markup(vi, &rows)));
            return;
        }
        let mut req = Tg::req("new", project);
        req["from"] = json!(from);
        if !text.is_empty() {
            req["text"] = json!(text);
        }
        let res = self.ask(req);
        let html = if res["ok"] == true && res["action"] == "new" { more1(vi, "opened", res["name"].as_str().unwrap_or("")) } else { result_html(&res, vi) };
        self.say(to, &html, None);
    }

    pub fn run_pane_command(self: &Arc<Self>, kind: &str, entry: &Entry, to: &Say, cb: Option<(&str, Option<i64>, &str)>) {
        let vi = self.vi();
        let action = match kind {
            "cp" => "compact",
            "cl" => "close",
            _ => "focus",
        };
        let mut req = Tg::req(action, &entry.project);
        req["target"] = Target::Terminal(entry.terminal_id.clone()).value();
        if kind != "fc" {
            if let Some(s) = entry.since {
                req["since"] = json!(s);
            }
        }
        let res = self.ask(req);
        if res["ok"] != true || res["action"] != action {
            let html = if res["ok"] == true {
                inbound(vi, "unavailable").to_string()
            } else if res["error"] == "state" {
                super::text_act::act(vi, "state").to_string()
            } else {
                result_html(&res, vi)
            };
            match cb {
                Some((id, _, _)) => self.answer(id, Some(&html)),
                None => {
                    self.say(to, &html, None);
                }
            }
            return;
        }
        if kind == "fc" {
            self.host("telegram.focus", Value::Null);
        }
        if let Some((id, msg_id, key)) = cb {
            self.retire(key, entry);
            self.answer(id, None);
            self.clear_markup(&entry.chat_id, msg_id);
        }
        let name = res["name"].as_str().unwrap_or("");
        let done = match kind {
            "cp" => act1(vi, "compacted", name),
            "cl" => act1(vi, "closed", name),
            _ => act1(vi, "focused", name),
        };
        self.say(to, &done, None);
    }

    pub fn live_target(&self, agent: Option<&str>, sent: Option<&Target>) -> Option<Target> {
        let st = self.lock();
        if let Some(t) = agent.and_then(|a| st.terminal_of_agent.get(a)) {
            return Some(Target::Terminal(t.clone()));
        }
        if agent.is_some() {
            return None;
        }
        if let Some(Target::Pane(p)) = sent {
            let agent = st.agent_of_pane.get(p).cloned().unwrap_or_default();
            if let Some(live) = st.terminal_of_agent.get(&agent) {
                return Some(Target::Terminal(live.clone()));
            }
        }
        sent.cloned()
    }
}
