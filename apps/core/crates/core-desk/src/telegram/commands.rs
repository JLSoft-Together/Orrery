use super::base::{topic_key, Entry, Target, Tg};
use super::html::{cell, cell_t, markup, result_html, Row};
use super::inbound::{group_id, pane_terminal, Say};
use super::message::{command_of, Command, TgFile, TgMessage};
use super::sanitize::inbound_prompt;
use super::shape::{Route, SessionRoute};
use super::text::{help, inbound, paired, stranger};
use super::text_act::{act, act1, file_prompt, image_prompt};
use super::text_more::{all_sent, file_kind, more, pick, project_pick, via_telegram};
use serde_json::{json, Value};
use std::sync::Arc;

const PAIR_TRIES: u32 = 5;
const PAIR_TRIES_TOTAL: u32 = 50;
const SESSION_BOUND: [&str; 8] = ["send", "file", "stop", "last", "tail", "compact", "close", "focus"];
const READ_COMMANDS: [&str; 5] = ["status", "last", "tail", "help", "where"];

fn is_image(f: &TgFile) -> bool {
    let low = f.name.to_lowercase();
    f.kind == "photo" || (f.kind == "document" && [".png", ".jpg", ".jpeg", ".gif", ".webp"].iter().any(|e| low.ends_with(e)))
}

fn stranger_html(custom: &str, owner: &str, vi: bool) -> String {
    if custom.is_empty() {
        return stranger(vi, owner);
    }
    super::text::esc(custom).split("{owner}").collect::<Vec<_>>().join(&super::text::esc(owner))
}

impl Tg {
    fn on_pair(self: &Arc<Self>, msg: &TgMessage, code: &str, vi: bool) {
        let now = self.now();
        {
            let mut st = self.lock();
            if !Tg::pairing(&st, now) {
                return;
            }
            let tries = st.pair_tries.get(&msg.from_id).copied().unwrap_or(0);
            if tries >= PAIR_TRIES {
                return;
            }
            if st.pair_code.as_deref() != Some(code) {
                st.pair_tries.insert(msg.from_id, tries + 1);
                st.pair_tries_total += 1;
                if st.pair_tries_total < PAIR_TRIES_TOTAL {
                    return;
                }
                st.pair_code = None;
                st.pair_until = None;
                drop(st);
                self.commit(false);
                return;
            }
            st.data.owner = Some((msg.from_id, msg.from_name.clone()));
            st.data.members.retain(|m| m["id"].as_i64() != Some(msg.from_id));
            st.strangers.retain(|(id, _)| *id != msg.from_id);
            st.pair_code = None;
            st.pair_until = None;
        }
        self.commit(false);
        self.reply(msg, &paired(vi, &msg.from_name), None);
    }

    pub fn on_message(self: &Arc<Self>, msg: &TgMessage, more_files: Vec<TgFile>) {
        let Some(bot) = self.lock().data.bot.clone() else { return };
        let Some(cmd) = command_of(msg, &bot) else { return };
        let vi = self.vi();
        if let Command::Pair(code) = &cmd {
            return self.on_pair(msg, code, vi);
        }
        let kind = cmd.kind();
        let role = Tg::role_of(&self.lock(), Some(msg.from_id));
        let Some(role) = role else {
            self.see_stranger(msg);
            self.note_ignored();
            let now = self.now() as f64;
            let answer = {
                let mut st = self.lock();
                let owner = st.data.owner.clone();
                let on = st.data.stranger["enabled"] == true;
                let text = st.data.stranger["text"].as_str().unwrap_or("").to_string();
                match owner {
                    Some((_, name)) if on && st.stranger_limit.take(now) => Some(stranger_html(&text, &name, vi)),
                    _ => None,
                }
            };
            if let Some(h) = answer {
                self.reply(msg, &h, None);
            }
            return;
        };
        let now = self.now();
        let receiving = {
            let mut st = self.lock();
            st.health.last_received = Some(now);
            st.receiving
        };
        if !receiving || msg.chat_type == "private" {
            if kind == "help" {
                self.reply(msg, &help(vi, &bot), None);
            }
            return;
        }
        if kind == "help" {
            self.reply(msg, &help(vi, &bot), None);
            return;
        }
        let limited = |tg: &Tg| tg.lock().read_only_limit.take(tg.now() as f64);
        if role == "read" && !READ_COMMANDS.contains(&kind) {
            if limited(self) {
                self.reply(msg, inbound(vi, "readOnly"), None);
            }
            return;
        }
        if kind == "close" && role != "owner" {
            if limited(self) {
                self.reply(msg, act(vi, "notAllowed"), None);
            }
            return;
        }
        let from = if role == "owner" { msg.from_name.clone() } else { Tg::name_of(&self.lock(), msg.from_id) };
        let (session, route) = self.route_of(msg, kind == "resume");
        if kind == "where" {
            return self.where_reply(msg, role == "owner", session.as_ref(), route.as_ref());
        }
        let new_text = if let Command::New(t) = &cmd { t.clone() } else { String::new() };
        let choices = if kind == "new" && session.is_none() { self.project_choices(msg) } else { Vec::new() };
        if !choices.is_empty() {
            if !self.take_inbound(&msg.chat_id) {
                self.reply(msg, inbound(vi, "rate"), None);
                return;
            }
            let text = if new_text.is_empty() { String::new() } else { via_telegram(&inbound_prompt(&new_text), vi) };
            let group = group_id();
            let rows: Vec<Row> = choices
                .iter()
                .map(|r| {
                    let key = self.add_action(Entry { kind: "pj".into(), project: r.project.clone(), cwd: r.project.clone(), name: r.label.clone(), chat_id: msg.chat_id.clone(), thread_id: msg.thread_id, extra: Some(text.clone()), by: Some(msg.from_id), group: Some(group.clone()), ..Entry::default() });
                    vec![cell_t("pj", &key, &r.label)]
                })
                .collect();
            self.reply(msg, &project_pick(vi, !text.is_empty()), Some(markup(vi, &rows)));
            return;
        }
        let Some(route) = route else {
            let keys = if role == "owner" { self.link_markup(msg) } else { None };
            let html = if keys.is_some() { more(vi, "linkPick") } else { inbound(vi, "noTopic") };
            self.reply(msg, html, keys);
            return;
        };
        if !self.take_inbound(&msg.chat_id) {
            self.reply(msg, inbound(vi, "rate"), None);
            return;
        }
        self.run_command(msg, cmd, more_files, &route, session.as_ref(), &from, vi);
    }

    fn route_of(&self, msg: &TgMessage, resume: bool) -> (Option<SessionRoute>, Option<Route>) {
        let st = self.lock();
        let on_topic = Tg::session_routes_on(&st, &msg.chat_id, msg.thread_id);
        let session = on_topic.iter().rev().find(|r| st.terminal_of_agent.contains_key(&r.session)).cloned().or_else(|| on_topic.last().cloned());
        let ended = if resume { st.ended_topics.get(&topic_key(&msg.chat_id, msg.thread_id)).cloned() } else { None };
        let route = match &session {
            Some(s) => Some(Route { project: s.project.clone(), label: s.name.clone(), chat_id: s.chat_id.clone(), thread_id: s.thread_id }),
            None => st.data.routes.iter().find(|r| r.chat_id == msg.chat_id && r.thread_id == msg.thread_id).cloned().or_else(|| {
                ended.map(|p| {
                    let b = crate::util::basename(&p);
                    Route { label: if b.is_empty() { p.clone() } else { b }, project: p, chat_id: msg.chat_id.clone(), thread_id: msg.thread_id }
                })
            }),
        };
        (session, route)
    }

    #[allow(clippy::too_many_arguments)]
    fn run_command(self: &Arc<Self>, msg: &TgMessage, cmd: Command, more_files: Vec<TgFile>, route: &Route, session: Option<&SessionRoute>, from: &str, vi: bool) {
        let kind = cmd.kind();
        let project = route.project.clone();
        let (session_terminal, replied_agent, sent) = {
            let st = self.lock();
            let t = session.and_then(|s| st.terminal_of_agent.get(&s.session).cloned());
            let key = msg.reply_to.map(|r| format!("{}:{r}", msg.chat_id));
            let agent = key.as_ref().and_then(|k| super::jsx::js_get(&st.data.replies, k).cloned());
            let sent = key.as_ref().and_then(|k| super::jsx::js_get(&st.sent_to, k).cloned());
            (t, agent, sent)
        };
        let replied = msg.reply_to.and_then(|_| self.live_target(replied_agent.as_deref(), sent.as_ref()));
        let target = replied.clone().or_else(|| session_terminal.clone().map(Target::Terminal));
        let bound = SESSION_BOUND.contains(&kind);
        if replied_agent.is_some() && replied.is_none() && bound {
            let name = self.lock().data.session_routes.iter().find(|r| Some(&r.session) == replied_agent.as_ref()).map(|r| r.name.clone()).unwrap_or_default();
            self.reply(msg, &act1(vi, "sessionClosed", &name), None);
            return;
        }
        if let Some(s) = session.filter(|_| session_terminal.is_none() && bound) {
            self.reply(msg, &act1(vi, "sessionClosed", &s.name), None);
            return;
        }
        if let Command::Screen(serial) = &cmd {
            return self.heavy_slot(|| self.send_screen(msg, serial.as_deref()));
        }
        let only = session_terminal.as_deref();
        let terminal_of = |t: &Option<Target>| match t {
            Some(Target::Terminal(id)) => Some(id.clone()),
            _ => None,
        };
        match &cmd {
            Command::New(text) => {
                let text = if text.is_empty() { String::new() } else { via_telegram(&inbound_prompt(text), vi) };
                let open: Vec<Value> = self.panes(&project, only).into_iter().filter(super::inbound::has_terminal).collect();
                return self.start_new(&Say::Reply(msg), &project, &text, from, msg.from_id, &open);
            }
            Command::All(text) => {
                let text = via_telegram(&inbound_prompt(text), vi);
                if text.is_empty() {
                    return;
                }
                let mut req = Tg::req("all", &project);
                req["text"] = json!(text);
                req["from"] = json!(from);
                let res = self.ask(req);
                let html = if res["ok"] == true && res["action"] == "all" {
                    let names: Vec<String> = res["names"].as_array().map(|l| l.iter().filter_map(|n| n.as_str().map(str::to_string)).collect()).unwrap_or_default();
                    if names.is_empty() { more(vi, "allNone").to_string() } else { all_sent(vi, &names) }
                } else if res["ok"] != true && res["error"] == "busy" {
                    more(vi, "allNone").to_string()
                } else {
                    result_html(&res, vi)
                };
                self.reply(msg, &html, None);
                return;
            }
            Command::Resume => {
                let res = self.ask(Tg::req("resumeList", &project));
                let items = if res["ok"] == true && res["action"] == "resumeList" { res["items"].as_array().cloned().unwrap_or_default() } else { Vec::new() };
                if items.is_empty() {
                    let html = if res["ok"] == true { more(vi, "resumeNone").to_string() } else { result_html(&res, vi) };
                    self.reply(msg, &html, None);
                    return;
                }
                let rows: Vec<Row> = items
                    .iter()
                    .map(|i| {
                        let name = i["name"].as_str().unwrap_or("").to_string();
                        let key = self.add_action(Entry { kind: "rs".into(), project: project.clone(), cwd: i["cwd"].as_str().unwrap_or("").into(), name: name.clone(), chat_id: msg.chat_id.clone(), thread_id: msg.thread_id, extra: i["id"].as_str().map(str::to_string), ..Entry::default() });
                        vec![cell_t("rs", &key, &name)]
                    })
                    .collect();
                self.reply(msg, more(vi, "resumePick"), Some(markup(vi, &rows)));
                return;
            }
            Command::Last | Command::Tail(_) => {
                let panes = self.panes(&project, only);
                let pick_id = terminal_of(&target).or_else(|| if panes.len() == 1 { pane_terminal(&panes[0]) } else { None });
                let (bk, what) = if kind == "last" { ("lt", "last") } else { ("tl", "tail") };
                let lines = if let Command::Tail(n) = cmd { n } else { 0 };
                let Some(pick_id) = pick_id else {
                    let open: Vec<Value> = panes.into_iter().filter(super::inbound::has_terminal).collect();
                    if open.len() > 1 {
                        let extra = (kind == "tail").then(|| lines.to_string());
                        let keys = self.pick_markup(bk, &open, &project, msg, extra.as_deref());
                        self.reply(msg, &pick(vi, open.len(), what), Some(keys));
                    } else {
                        self.reply(msg, &result_html(&json!({"ok": false, "error": "none"}), vi), None);
                    }
                    return;
                };
                let html = if kind == "last" { self.last_html(&pick_id, &project) } else { self.tail_text(&pick_id, &project, lines) };
                self.reply(msg, &html, None);
                return;
            }
            Command::Run(config) => return self.run_cmd(msg, &project, config, vi),
            Command::Stop | Command::Compact | Command::Close | Command::Focus => {
                let panes = self.panes(&project, only);
                let pick_pane = match (&target, kind) {
                    (Some(Target::Terminal(t)), _) => panes.iter().find(|p| p["terminalId"].as_str() == Some(t)).cloned(),
                    (Some(_), "stop") if panes.len() == 1 => Some(panes[0].clone()),
                    (Some(_), _) => None,
                    (None, _) if panes.len() == 1 => Some(panes[0].clone()),
                    _ => None,
                };
                let bk = match kind {
                    "stop" => "st",
                    "compact" => "cp",
                    "close" => "cl",
                    _ => "fc",
                };
                let Some(p) = pick_pane.filter(super::inbound::has_terminal) else {
                    let open: Vec<Value> = panes.into_iter().filter(super::inbound::has_terminal).collect();
                    if target.is_none() && open.len() > 1 {
                        let keys = self.pick_markup(bk, &open, &project, msg, None);
                        self.reply(msg, &pick(vi, open.len(), kind), Some(keys));
                    } else {
                        let e = if target.is_some() { "gone" } else { "none" };
                        self.reply(msg, &result_html(&json!({"ok": false, "error": e}), vi), None);
                    }
                    return;
                };
                let name = p["name"].as_str().unwrap_or("").to_string();
                let since = p["since"].as_f64().filter(|s| *s != 0.0);
                let mut entry = Entry { kind: bk.into(), terminal_id: pane_terminal(&p).unwrap_or_default(), project: project.clone(), cwd: project.clone(), name: name.clone(), chat_id: msg.chat_id.clone(), thread_id: msg.thread_id, since, ..Entry::default() };
                if kind == "stop" {
                    let key = self.add_action(entry);
                    self.reply(msg, &act1(vi, "stopAsk", &name), Some(markup(vi, &[vec![cell("st", &key)]])));
                    return;
                }
                entry.by = Some(msg.from_id);
                entry.group = Some(group_id());
                if bk == "fc" {
                    return self.run_pane_command("fc", &entry, &Say::Reply(msg), None);
                }
                let ask_html = if bk == "cl" { act1(vi, "closeAsk", &name) } else { act1(vi, "compactAsk", &name) };
                let key = self.add_action(entry);
                self.reply(msg, &ask_html, Some(markup(vi, &[vec![cell(bk, &key)]])));
                return;
            }
            _ => {}
        }
        let req = match &cmd {
            Command::Status => Tg::req("status", &project),
            Command::File(file, text) => {
                let mut files = vec![file.clone()];
                files.extend(more_files.iter().cloned());
                let results: Vec<Result<String, &'static str>> = files.iter().map(|f| self.heavy_slot(|| self.download(&project, f))).collect();
                let paths: Vec<String> = results.iter().filter_map(|r| r.clone().ok()).collect();
                if paths.is_empty() {
                    let html = if results.contains(&Err("big")) { act(vi, "fileTooBig") } else { act(vi, "fileFail") };
                    self.reply(msg, html, None);
                    return;
                }
                let caption = inbound_prompt(text);
                let fk = if more_files.is_empty() { file_kind(vi, file.kind) } else { file_kind(vi, "album") };
                let body_text = via_telegram(&inbound_prompt(&file_prompt(vi, &fk, &paths.join(", "), &caption)), vi);
                let ok_at = |i: usize| results[i].is_ok();
                let images: Vec<String> = (0..files.len()).filter(|i| ok_at(*i) && is_image(&files[*i])).filter_map(|i| results[i].clone().ok()).collect();
                let others: Vec<String> = (0..files.len()).filter(|i| ok_at(*i) && !is_image(&files[*i])).filter_map(|i| results[i].clone().ok()).collect();
                let other = (0..files.len()).find(|i| ok_at(*i) && !is_image(&files[*i])).map(|i| files[i].clone());
                let lead = image_prompt(vi, images.len());
                let body = match &other {
                    Some(f) => file_prompt(vi, &file_kind(vi, f.kind), &others.join(", "), &caption),
                    None => caption.clone(),
                };
                let image_text = if images.is_empty() { String::new() } else { via_telegram(&inbound_prompt(&if body.is_empty() { lead.clone() } else { format!("{lead}\n{body}") }), vi) };
                let mut req = Tg::req("send", &project);
                req["text"] = json!(body_text);
                req["from"] = json!(from);
                if let Some(t) = &target {
                    req["target"] = t.value();
                }
                if !image_text.is_empty() {
                    req["images"] = json!(images);
                    req["imageText"] = json!(image_text);
                }
                req
            }
            Command::Send(text) => {
                let text = via_telegram(&inbound_prompt(text), vi);
                if text.is_empty() {
                    return;
                }
                let mut req = Tg::req("send", &project);
                req["text"] = json!(text);
                req["from"] = json!(from);
                if let Some(t) = &target {
                    req["target"] = t.value();
                }
                req
            }
            _ => return,
        };
        let is_send = req["action"] == "send";
        let req_text = req["text"].as_str().unwrap_or("").to_string();
        let result = self.ask(req);
        if is_send && result["ok"] != true && result["error"] == "many" {
            let open: Vec<Value> = result["panes"].as_array().cloned().unwrap_or_default().into_iter().filter(super::inbound::has_terminal).collect();
            if !open.is_empty() {
                let keys = self.pick_markup("np", &open, &project, msg, Some(&req_text));
                self.reply(msg, &pick(vi, open.len(), "send"), Some(keys));
                return;
            }
        }
        let sent = self.reply(msg, &result_html(&result, vi), None);
        if let (Some(id), true, Some(t)) = (sent, result["ok"] == true && result["action"] == "send", result["terminalId"].as_str()) {
            self.remember_sent(&msg.chat_id, id, Target::Terminal(t.to_string()));
        }
    }

}
