use super::base::{topic_key, Entry, St, Tg};
use super::html::{cell, cell_t, markup, Row};
use super::sanitize::{parse_topic_link, session_id_ok, ROUTES_MAX, SESSION_ROUTES_MAX};
use super::settings::{After, Notice, STARTUP_SWEEP_MS};
use super::shape::{Route, SessionRoute};
use super::text_more::more1;
use serde_json::{json, Map, Value};
use std::sync::Arc;

impl Tg {
    pub fn norm_project(p: &str) -> Option<String> {
        std::path::Path::new(p).is_absolute().then(|| crate::util::path_str(&crate::util::resolve(p)))
    }

    fn same_project(&self, a: &str, b: &str) -> bool {
        if self.deps.win {
            a.to_lowercase() == b.to_lowercase()
        } else {
            a == b
        }
    }

    fn target_of(input: &Map<String, Value>) -> Option<(String, Option<i64>)> {
        if let Some(link) = input.get("link").and_then(Value::as_str) {
            return parse_topic_link(link);
        }
        let chat = input.get("chatId").and_then(Value::as_str).filter(|c| !c.is_empty())?;
        Some((chat.to_string(), input.get("threadId").and_then(Value::as_i64)))
    }

    pub fn set_route(self: &Arc<Self>, input: &Map<String, Value>) -> Value {
        let Some(project) = input.get("project").and_then(Value::as_str).and_then(Tg::norm_project) else { return json!({"ok": false, "error": "invalid-project"}) };
        let Some((chat, thread)) = Tg::target_of(input) else { return json!({"ok": false, "error": "invalid-link"}) };
        let mut after = After::default();
        {
            let mut st = self.lock();
            let rest: Vec<Route> = st.data.routes.iter().filter(|r| !self.same_project(&r.project, &project)).cloned().collect();
            if rest.len() >= ROUTES_MAX {
                return json!({"ok": false, "error": "too-many"});
            }
            let moved: Vec<Route> = st.data.routes.iter().filter(|r| self.same_project(&r.project, &project) && (r.chat_id != chat || r.thread_id != thread)).cloned().collect();
            let base = crate::util::basename(&project);
            let label = input.get("label").and_then(Value::as_str).filter(|l| !l.is_empty()).map(str::to_string).unwrap_or_else(|| if base.is_empty() { project.clone() } else { base });
            st.data.routes = rest;
            st.data.routes.push(Route { project, label, chat_id: chat, thread_id: thread });
            for old in moved {
                Tg::release_pin(&mut st, &old.chat_id, old.thread_id, &mut after);
            }
        }
        self.finish(after);
        self.commit(false)
    }

    pub fn set_session_route(self: &Arc<Self>, input: &Map<String, Value>) -> Value {
        let Some(project) = input.get("project").and_then(Value::as_str).and_then(Tg::norm_project) else { return json!({"ok": false, "error": "invalid-project"}) };
        let Some((chat, thread)) = Tg::target_of(input) else { return json!({"ok": false, "error": "invalid-link"}) };
        let route = SessionRoute {
            session: input.get("session").and_then(Value::as_str).unwrap_or("").to_string(),
            name: input.get("name").and_then(Value::as_str).unwrap_or("").to_string(),
            project,
            chat_id: chat,
            thread_id: thread,
            keep: false,
        };
        self.link_session(route, input.get("keep").and_then(Value::as_bool))
    }

    pub fn link_session(self: &Arc<Self>, route: SessionRoute, keep: Option<bool>) -> Value {
        if !session_id_ok(&route.session) {
            return json!({"ok": false, "error": "invalid-project"});
        }
        let mut after = After::default();
        {
            let mut st = self.lock();
            let on_topic = |r: &SessionRoute| r.session != route.session && r.chat_id == route.chat_id && r.thread_id == route.thread_id;
            let rest: Vec<SessionRoute> = st.data.session_routes.iter().filter(|r| r.session != route.session && !on_topic(r)).cloned().collect();
            if rest.len() >= SESSION_ROUTES_MAX {
                return json!({"ok": false, "error": "too-many"});
            }
            let old = st.data.session_routes.iter().find(|r| r.session == route.session).cloned();
            let keep = keep.or(old.as_ref().map(|o| o.keep)).unwrap_or(false);
            st.data.session_routes = rest;
            st.data.session_routes.push(SessionRoute { keep, ..route.clone() });
            if let Some(o) = old.filter(|o| o.chat_id != route.chat_id || o.thread_id != route.thread_id) {
                Tg::release_pin(&mut st, &o.chat_id, o.thread_id, &mut after);
            }
            st.ended_topics.remove(&topic_key(&route.chat_id, route.thread_id));
        }
        self.finish(after);
        self.commit(false)
    }

    pub fn remove_session_route(self: &Arc<Self>, session: &str) -> Value {
        let key = session.to_lowercase();
        let mut after = After::default();
        {
            let mut st = self.lock();
            let gone = st.data.session_routes.iter().find(|r| r.session == key).cloned();
            st.data.session_routes.retain(|r| r.session != key);
            if let Some(g) = gone {
                Tg::release_pin(&mut st, &g.chat_id, g.thread_id, &mut after);
            }
        }
        self.finish(after);
        self.commit(false)
    }

    pub fn remove_route(self: &Arc<Self>, project: &str) -> Value {
        let Some(norm) = Tg::norm_project(project) else { return json!({"ok": false, "error": "invalid-project"}) };
        let mut after = After::default();
        {
            let mut st = self.lock();
            let gone: Vec<Route> = st.data.routes.iter().filter(|r| self.same_project(&r.project, &norm)).cloned().collect();
            st.data.routes.retain(|r| !self.same_project(&r.project, &norm));
            for r in gone {
                Tg::release_pin(&mut st, &r.chat_id, r.thread_id, &mut after);
            }
        }
        self.finish(after);
        self.commit(false)
    }

    pub fn release_pin(st: &mut St, chat: &str, thread: Option<i64>, after: &mut After) {
        let key = topic_key(chat, thread);
        if Tg::routed(st, &key) {
            return;
        }
        st.pin_text.remove(&key);
        st.pin_at.remove(&key);
        if let Some(pin) = super::jsx::js_remove(&mut st.data.pins, &key) {
            after.unpins.push(pin);
        }
    }

    pub fn finish(self: &Arc<Self>, after: After) {
        if after.unpins.is_empty() && after.notices.is_empty() {
            return;
        }
        self.spawn(move |tg| {
            if !after.unpins.is_empty() && tg.token().is_some() {
                for (chat, id) in &after.unpins {
                    tg.call_json("unpinChatMessage", json!({"chat_id": chat, "message_id": id}));
                }
            }
            for n in after.notices {
                tg.notice_unlinked(&n.route, n.why, n.next.as_ref());
            }
        });
    }

    pub fn set_panes(self: &Arc<Self>, links: &[Value]) {
        let now = self.now();
        let mut after = After::default();
        let changed = {
            let mut st = self.lock();
            let before = std::mem::take(&mut st.agent_of_terminal);
            st.agent_of_pane.clear();
            st.terminal_of_agent.clear();
            st.open_sessions = links
                .iter()
                .filter_map(|l| {
                    let cwd = l["cwd"].as_str()?;
                    l["terminal"].as_str()?;
                    let name = l["name"].as_str().filter(|n| !n.is_empty()).map(str::to_string).unwrap_or_else(|| crate::util::basename(cwd));
                    Some((l["agent"].as_str().unwrap_or("").to_string(), name, cwd.to_string()))
                })
                .collect();
            for l in links {
                let agent = l["agent"].as_str().unwrap_or("").to_string();
                if let Some(p) = l["pane"].as_str() {
                    st.agent_of_pane.insert(p.into(), agent.clone());
                }
                if let Some(t) = l["terminal"].as_str() {
                    st.agent_of_terminal.insert(t.into(), agent.clone());
                    st.terminal_of_agent.insert(agent, t.into());
                }
            }
            st.panes_seen = true;
            let followed = Tg::follow_sessions(&mut st, links, &before, &mut after);
            let ended = Tg::drop_ended(&mut st, links, &before, &mut after);
            let swept = Tg::sweep_stale(&mut st, now, &mut after);
            swept || ended || followed
        };
        self.finish(after);
        if changed {
            self.commit(false);
        }
    }

    pub fn sweep_stale(st: &mut St, now: i64, after: &mut After) -> bool {
        if st.swept || !st.panes_seen || now - st.started_at < STARTUP_SWEEP_MS {
            return false;
        }
        st.swept = true;
        let stale: Vec<SessionRoute> = st.data.session_routes.iter().filter(|r| !r.keep && !st.terminal_of_agent.contains_key(&r.session)).cloned().collect();
        for r in &stale {
            Tg::unlink_ended(st, r, "ended", None, after);
        }
        !stale.is_empty()
    }

    fn follow_sessions(st: &mut St, links: &[Value], before: &std::collections::HashMap<String, String>, after: &mut After) -> bool {
        let mut changed = false;
        for l in links {
            let agent = l["agent"].as_str().unwrap_or("");
            let Some(prev) = l["terminal"].as_str().and_then(|t| before.get(t)).cloned() else { continue };
            if prev == agent || st.data.session_routes.iter().any(|r| r.session == agent) {
                continue;
            }
            let Some(pos) = st.data.session_routes.iter().position(|r| r.session == prev) else { continue };
            if st.data.session_routes[pos].keep {
                st.data.session_routes[pos].session = agent.to_string();
            } else {
                let old = st.data.session_routes[pos].clone();
                Tg::unlink_ended(st, &old, "cleared", Some(l.clone()), after);
            }
            changed = true;
        }
        for l in links {
            let Some(name) = l["name"].as_str().filter(|n| !n.is_empty()) else { continue };
            let agent = l["agent"].as_str().unwrap_or("");
            if let Some(r) = st.data.session_routes.iter_mut().find(|r| r.session == agent) {
                if r.name != name {
                    r.name = name.to_string();
                    changed = true;
                }
            }
        }
        changed
    }

    fn drop_ended(st: &mut St, links: &[Value], before: &std::collections::HashMap<String, String>, after: &mut After) -> bool {
        let now: std::collections::HashSet<&str> = links.iter().filter(|l| l["terminal"].is_string()).filter_map(|l| l["agent"].as_str()).collect();
        let gone: std::collections::HashSet<&String> = before.values().filter(|a| !now.contains(a.as_str())).collect();
        let ended: Vec<SessionRoute> = st.data.session_routes.iter().filter(|r| gone.contains(&r.session) && !r.keep).cloned().collect();
        for r in &ended {
            Tg::unlink_ended(st, r, "ended", None, after);
        }
        !ended.is_empty()
    }

    fn unlink_ended(st: &mut St, r: &SessionRoute, why: &'static str, next: Option<Value>, after: &mut After) {
        if let Some(pos) = st.data.session_routes.iter().position(|x| x == r) {
            st.data.session_routes.remove(pos);
        }
        Tg::release_pin(st, &r.chat_id, r.thread_id, after);
        st.ended_topics.insert(topic_key(&r.chat_id, r.thread_id), r.project.clone());
        after.notices.push(Notice { route: r.clone(), why, next });
    }

    pub fn notice_unlinked(&self, r: &SessionRoute, why: &str, next: Option<&Value>) {
        let enabled = self.lock().data.enabled;
        if !enabled || self.token().is_none() || !self.take(&r.chat_id) {
            return;
        }
        let vi = self.vi();
        let base = Entry { terminal_id: String::new(), chat_id: r.chat_id.clone(), thread_id: r.thread_id, ..Entry::default() };
        let mut rows: Vec<Row> = Vec::new();
        if self.lock().receiving {
            if why == "ended" {
                let key = self.add_action(Entry { kind: "rs".into(), project: r.project.clone(), cwd: r.project.clone(), name: r.name.clone(), extra: Some(r.session.clone()), ..base.clone() });
                rows.push(vec![cell_t("rs", &key, &r.name)]);
                rows.extend(self.link_rows(&r.chat_id, r.thread_id, true).into_iter().filter(|row| row[0].0 == "lk"));
            } else if let Some(n) = next {
                let cwd = n["cwd"].as_str().unwrap_or(&r.project).to_string();
                let name = n["name"].as_str().unwrap_or(&r.name).to_string();
                let key = self.add_action(Entry { kind: "lk".into(), project: cwd.clone(), cwd, name: name.clone(), extra: n["agent"].as_str().map(str::to_string), ..base.clone() });
                rows.push(vec![cell_t("lk", &key, &name)]);
            }
        }
        let mut extra = Map::new();
        extra.insert("disable_notification".into(), json!(true));
        if !rows.is_empty() {
            extra.insert("reply_markup".into(), markup(vi, &rows));
        }
        self.post(&r.chat_id, r.thread_id, &more1(vi, why, &r.name), extra);
    }

    pub fn link_rows(&self, chat: &str, thread: Option<i64>, only_free: bool) -> Vec<Row> {
        let (sessions, linked) = {
            let st = self.lock();
            let linked: Vec<String> = if only_free { st.data.session_routes.iter().map(|r| r.session.clone()).collect() } else { Vec::new() };
            (st.open_sessions.clone(), linked)
        };
        let base = Entry { terminal_id: String::new(), chat_id: chat.to_string(), thread_id: thread, ..Entry::default() };
        let mut rows: Vec<Row> = Vec::new();
        for (agent, name, cwd) in sessions.iter().filter(|(a, _, _)| !linked.contains(a)).take(6) {
            let key = self.add_action(Entry { kind: "lk".into(), project: cwd.clone(), cwd: cwd.clone(), name: name.clone(), extra: Some(agent.clone()), ..base.clone() });
            rows.push(vec![cell_t("lk", &key, name)]);
        }
        let mut projects: Vec<String> = Vec::new();
        for (_, _, cwd) in &sessions {
            if !projects.contains(cwd) {
                projects.push(cwd.clone());
            }
        }
        for p in projects.into_iter().take(4) {
            let b = crate::util::basename(&p);
            let name = if b.is_empty() { p.clone() } else { b };
            let key = self.add_action(Entry { kind: "lp".into(), project: p.clone(), cwd: p.clone(), name: name.clone(), ..base.clone() });
            rows.push(vec![cell_t("lp", &key, &name)]);
        }
        rows
    }

    pub fn ask(&self, mut req: Value) -> Value {
        let project = req["project"].as_str().unwrap_or("").to_string();
        let nested: Vec<String> = {
            let st = self.lock();
            st.data.routes.iter().filter(|r| !self.same_project(&r.project, &project) && crate::util::is_inside_or_same(&project, &r.project)).map(|r| r.project.clone()).collect()
        };
        if !nested.is_empty() {
            req["nested"] = json!(nested);
        }
        match (self.deps.host)("telegram.ask", json!({"req": req})) {
            Ok(v) => super::sanitize::parse_inbound_result(&v),
            Err(_) => json!({"ok": false, "error": "unavailable"}),
        }
    }

    pub fn req(action: &str, project: &str) -> Value {
        json!({"id": crate::util::uuid(), "action": action, "project": project})
    }

    pub fn keyboard(vi: bool, rows: &[Row]) -> Value {
        markup(vi, rows)
    }

    pub fn one(kind: &str, key: &str) -> Row {
        vec![cell(kind, key)]
    }
}
