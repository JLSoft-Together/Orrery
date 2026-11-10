use super::api::{form, Body, Part};
use super::buttons::{parse_data, Claim};
use super::base::{Entry, RunWatch, Target, Tg, UPLOAD_TIMEOUT_MS};
use super::html::{result_html, thread_fields};
use super::inbound::{has_terminal, Say};
use super::message::{TgFile, TgMessage};
use super::sanitize::{FILE_MAX, TAIL_DEFAULT, TAIL_MAX};
use super::shape::SessionRoute;
use super::text::inbound;
use super::text_act::{act, act1, chose, diff_caption};
use super::text_more::{more1, prompt, reply_text, KINDS};
use serde_json::{json, Map, Value};
use std::sync::Arc;

const DOWNLOAD_MAX: f64 = 20.0 * 1024.0 * 1024.0;
const CONFIRM_TTL_MS: i64 = 5 * 60_000;
const OWNER_BUTTONS: [&str; 8] = ["ap", "dn", "lk", "lp", "bg", "bx", "ul", "cl"];
const READ_BUTTONS: [&str; 3] = ["lt", "tl", "df"];

pub fn parse_action(data: &str) -> Option<(&str, &str)> {
    let (kind, key) = data.split_once(':')?;
    (KINDS.contains(&kind) && (6..=16).contains(&key.len()) && key.bytes().all(|b| b.is_ascii_alphanumeric())).then_some((kind, key))
}

pub fn may_press(role: &str, kind: &str) -> bool {
    match role {
        "owner" => true,
        "full" => !OWNER_BUTTONS.contains(&kind),
        _ => READ_BUTTONS.contains(&kind),
    }
}

impl Tg {
    pub fn on_callback(self: &Arc<Self>, cb: &Value) {
        let Some(id) = cb["id"].as_str() else { return };
        let from = cb["from"]["id"].as_f64().map(|f| f as i64);
        let data = cb["data"].as_str().unwrap_or("");
        if parse_data(data).is_some() {
            return self.on_reply_tap(id, data, from, cb);
        }
        let vi = self.vi();
        let role = Tg::role_of(&self.lock(), from);
        let Some(role) = role else {
            self.note_ignored();
            let now = self.now() as f64;
            if self.lock().not_owner_limit.take(now) {
                self.answer(id, Some(act(vi, "notOwner")));
            }
            return;
        };
        let m = parse_action(data);
        if let Some((kind, _)) = m {
            if !may_press(role, kind) {
                let now = self.now() as f64;
                if self.lock().read_only_limit.take(now) {
                    self.answer(id, Some(act(vi, "notAllowed")));
                }
                return;
            }
        }
        let (entry, receiving) = {
            let st = self.lock();
            (m.and_then(|(_, k)| super::jsx::js_get(&st.actions, k).cloned()), st.receiving)
        };
        let (Some((kind, key)), Some(entry), true) = (m, entry, receiving) else {
            self.answer(id, Some(act(vi, "expired")));
            return;
        };
        let from = from.unwrap_or(0);
        let who = Tg::name_of(&self.lock(), from);
        match kind {
            "lk" | "lp" => return self.link_topic(id, kind, &entry, cb),
            "ul" => return self.unlink_topic(id, key, &entry, cb),
            "bg" | "bx" => return self.on_backlog(id, kind, key, &entry, cb),
            _ => {}
        }
        let msg_id = cb["message"]["message_id"].as_i64();
        let to = Say::Post(entry.chat_id.clone(), entry.thread_id);
        let target = Target::Terminal(entry.terminal_id.clone());
        match kind {
            "rs" | "rn" => {
                super::jsx::js_remove(&mut self.lock().actions, key);
                self.answer(id, None);
                self.clear_markup(&entry.chat_id, msg_id);
                if kind == "rn" {
                    return self.start_run(&entry.project, entry.extra.as_deref().unwrap_or(""), &entry.name, &entry.chat_id, entry.thread_id);
                }
                let mut req = Tg::req("resume", &entry.project);
                req["sessionId"] = json!(entry.extra.clone().unwrap_or_default());
                req["cwd"] = json!(entry.cwd);
                let res = self.ask(req);
                let html = if res["ok"] == true && res["action"] == "resume" { more1(vi, "resumed", res["name"].as_str().unwrap_or("")) } else { result_html(&res, vi) };
                self.say(&to, &html, None);
                return;
            }
            "df" => {
                self.answer(id, None);
                return self.send_diff(&entry);
            }
            "lt" => {
                self.answer(id, None);
                self.say(&to, &self.last_html(&entry.terminal_id, &entry.project), None);
                return;
            }
            "tl" => {
                self.answer(id, None);
                let n = entry.extra.as_deref().and_then(|e| e.parse::<f64>().ok()).filter(|n| *n != 0.0 && n.is_finite()).map(|n| n as i64).unwrap_or(TAIL_DEFAULT);
                self.say(&to, &self.tail_text(&entry.terminal_id, &entry.project, n.clamp(1, TAIL_MAX)), None);
                return;
            }
            _ => {}
        }
        if let Some(by) = entry.by {
            if role != "owner" && from != by {
                self.answer(id, Some(act(vi, "notAllowed")));
                return;
            }
        }
        if (kind == "cp" || kind == "cl") && self.now() - entry.at.unwrap_or(0) > CONFIRM_TTL_MS {
            self.retire(key, &entry);
            self.answer(id, Some(act(vi, "expired")));
            return;
        }
        match kind {
            "cp" | "cl" | "fc" => return self.run_pane_command(kind, &entry, &to, Some((id, msg_id, key))),
            "pj" => {
                self.retire(key, &entry);
                self.answer(id, None);
                self.clear_markup(&entry.chat_id, msg_id);
                let open: Vec<Value> = self.panes(&entry.project, None).into_iter().filter(has_terminal).collect();
                return self.start_new(&to, &entry.project, entry.extra.as_deref().unwrap_or(""), &who, from, &open);
            }
            "nw" | "np" => {
                {
                    let mut st = self.lock();
                    if entry.group.is_some() {
                        drop(st);
                        self.retire(key, &entry);
                    } else {
                        st.actions.retain(|(_, e)| !((e.kind == "nw" || e.kind == "np") && e.chat_id == entry.chat_id && e.extra == entry.extra && e.thread_id == entry.thread_id));
                    }
                }
                self.answer(id, None);
                self.clear_markup(&entry.chat_id, msg_id);
                let text = entry.extra.clone().unwrap_or_default();
                let req = if kind == "nw" {
                    let mut r = Tg::req("new", &entry.project);
                    r["from"] = json!(who);
                    if !text.is_empty() {
                        r["text"] = json!(text);
                    }
                    r
                } else {
                    let mut r = Tg::req("send", &entry.project);
                    r["text"] = json!(text);
                    r["from"] = json!(who);
                    r["target"] = target.value();
                    r
                };
                let result = self.ask(req);
                let html = if result["ok"] == true && result["action"] == "new" { more1(vi, "opened", result["name"].as_str().unwrap_or("")) } else { result_html(&result, vi) };
                self.say(&to, &html, None);
                return;
            }
            "go" | "ts" => {
                let mut r = Tg::req("send", &entry.project);
                r["text"] = json!(prompt(vi, kind));
                r["from"] = json!(who);
                r["target"] = target.value();
                let result = self.ask(r);
                self.answer(id, None);
                self.say(&to, &result_html(&result, vi), None);
                return;
            }
            _ => {}
        }
        self.press_key(id, kind, key, &entry, &target, msg_id, vi);
    }

    #[allow(clippy::too_many_arguments)]
    fn press_key(&self, id: &str, kind: &str, key: &str, entry: &Entry, target: &Target, msg_id: Option<i64>, vi: bool) {
        let k = match kind {
            "ap" => "approve",
            "dn" => "deny",
            "qa" => "option",
            "es" => "esc",
            _ => "stop",
        };
        let option = if kind == "qa" { entry.extra.as_deref().and_then(|e| e.parse::<f64>().ok()) } else { None };
        let mut req = Tg::req("keys", &entry.project);
        req["key"] = json!(k);
        if let Some(o) = option.filter(|o| *o != 0.0 && !o.is_nan()) {
            req["option"] = json!(o as i64);
        }
        req["target"] = target.value();
        if let Some(s) = entry.since {
            req["since"] = json!(s);
        }
        if let Some(t) = entry.tool.as_ref().filter(|_| !matches!(k, "stop" | "option" | "esc")) {
            req["tool"] = json!(t);
        }
        let result = self.ask(req);
        if result["ok"] != true {
            let html = match result["error"].as_str() {
                Some("state") => act(vi, "state").to_string(),
                Some("busy") => act(vi, "keysBusy").to_string(),
                _ => result_html(&result, vi),
            };
            self.answer(id, Some(&html));
            return;
        }
        {
            let mut st = self.lock();
            super::jsx::js_remove(&mut st.actions, key);
            if kind == "qa" {
                st.actions.retain(|(_, e)| !(e.kind == "qa" && e.terminal_id == entry.terminal_id && e.since == entry.since));
            }
        }
        self.clear_markup(&entry.chat_id, msg_id);
        let done = match kind {
            "ap" => act1(vi, "approved", &entry.name),
            "dn" => act1(vi, "denied", &entry.name),
            "qa" => chose(vi, &entry.name, &entry.label.clone().unwrap_or_else(|| option.map(crate::js::number).unwrap_or_else(|| "NaN".into()))),
            "es" => act1(vi, "escaped", &entry.name),
            _ => act1(vi, "stopped", &entry.name),
        };
        self.answer(id, None);
        self.post(&entry.chat_id, entry.thread_id, &done, Map::new());
    }

    fn on_reply_tap(&self, id: &str, data: &str, from: Option<i64>, cb: &Value) {
        let vi = self.vi();
        let role = Tg::role_of(&self.lock(), from);
        if role != Some("owner") {
            if role.is_none() {
                self.note_ignored();
            }
            let now = self.now() as f64;
            if self.lock().not_owner_limit.take(now) {
                self.answer(id, Some(act(vi, "notOwner")));
            }
            return;
        }
        let now = self.now();
        let got = self.lock().reply_buttons.claim(now, data);
        let (token, group, label, value) = match got {
            Claim::Expired => return self.answer(id, Some(act(vi, "expired"))),
            Claim::Answered(l) => return self.answer(id, Some(&reply_text(vi, "already", &l))),
            Claim::Busy => return self.answer(id, Some(act(vi, "keysBusy"))),
            Claim::Ok { token, group, label, value } => (token, group, label, value),
        };
        let mut req = Tg::req("answer", &group.project);
        req["text"] = json!(value);
        req["target"] = Target::Pane(group.pane.clone()).value();
        req["from"] = json!(Tg::name_of(&self.lock(), from.unwrap_or(0)));
        let res = match (self.deps.host)("telegram.ask", json!({"req": req})) {
            Ok(v) => super::sanitize::parse_inbound_result(&v),
            Err(_) => json!({"ok": false, "error": "unavailable"}),
        };
        let msg_id = cb["message"]["message_id"].as_i64();
        let done = res["ok"] == true && res["action"] == "answer";
        let closed = res["ok"] != true && (res["error"] == "gone" || res["error"] == "none");
        if !done && !closed {
            self.lock().reply_buttons.release(&token);
            let html = if res["ok"] != true && res["error"] == "busy" { reply_text(vi, "busy", "") } else { inbound(vi, "unavailable").to_string() };
            self.answer(id, Some(&html));
            return;
        }
        if done {
            self.lock().reply_buttons.finish(&token, &label);
        } else {
            self.lock().reply_buttons.forget(&token);
        }
        let toast = if done {
            reply_text(vi, if res["submitted"] == true { "sent" } else { "typed" }, &label)
        } else {
            reply_text(vi, "closed", "")
        };
        self.answer(id, Some(&toast));
        if let Some(mid) = msg_id {
            let keys = if done { json!([[{"text": super::jsx::slice(&format!("✓ {label}"), 60), "callback_data": data}]]) } else { json!([]) };
            self.call_json("editMessageReplyMarkup", json!({"chat_id": group.chat_id, "message_id": mid, "reply_markup": {"inline_keyboard": keys}}));
        }
    }

    fn unlink_topic(self: &Arc<Self>, id: &str, key: &str, entry: &Entry, cb: &Value) {
        super::jsx::js_remove(&mut self.lock().actions, key);
        let res = match &entry.extra {
            Some(s) => self.remove_session_route(s),
            None => self.remove_route(&entry.project),
        };
        self.answer(id, None);
        self.clear_markup(&entry.chat_id, cb["message"]["message_id"].as_i64());
        let vi = self.vi();
        let html = if res["ok"] == true { more1(vi, "unlinked", &entry.name) } else { act(vi, "expired").to_string() };
        self.post(&entry.chat_id, entry.thread_id, &html, Map::new());
    }

    fn link_topic(self: &Arc<Self>, id: &str, kind: &str, entry: &Entry, cb: &Value) {
        self.lock().actions.retain(|(_, e)| !((e.kind == "lk" || e.kind == "lp") && e.chat_id == entry.chat_id && e.thread_id == entry.thread_id));
        let res = match Tg::norm_project(&entry.project) {
            None => json!({"ok": false, "error": "invalid-project"}),
            Some(project) if kind == "lk" => self.link_session(SessionRoute { session: entry.extra.clone().unwrap_or_default(), name: entry.name.clone(), project, chat_id: entry.chat_id.clone(), thread_id: entry.thread_id, keep: false }, None),
            Some(project) => {
                let mut input = Map::new();
                input.insert("project".into(), json!(project));
                input.insert("chatId".into(), json!(entry.chat_id));
                input.insert("threadId".into(), json!(entry.thread_id));
                self.set_route(&input)
            }
        };
        self.answer(id, None);
        self.clear_markup(&entry.chat_id, cb["message"]["message_id"].as_i64());
        let vi = self.vi();
        let html = if res["ok"] == true { more1(vi, "linked", &entry.name) } else { act(vi, "expired").to_string() };
        self.post(&entry.chat_id, entry.thread_id, &html, Map::new());
    }

    pub fn photo_form(chat: &str, thread: Option<i64>, caption: &str, field: &str, bytes: Vec<u8>, name: &str, mime: Option<&'static str>) -> Body {
        let mut m = thread_fields(chat, thread);
        m.insert("caption".into(), json!(caption));
        m.insert("parse_mode".into(), json!("HTML"));
        let mut parts = form(&m);
        parts.push((field.to_string(), Part::File { bytes, name: name.into(), mime }));
        Body::Form(parts)
    }

    pub fn send_screen(&self, msg: &TgMessage, serial: Option<&str>) {
        let vi = self.vi();
        let shot = self.host("telegram.screenshot", json!({"serial": serial})).unwrap_or_else(|| json!({"ok": false, "error": "failed"}));
        if shot["ok"] != true {
            self.reply(msg, act(vi, if shot["error"] == "none" { "screenNone" } else { "screenFail" }), None);
            return;
        }
        let Some(buf) = shot["png"].as_str().and_then(|p| std::fs::read(p).ok()) else {
            self.reply(msg, act(vi, "screenFail"), None);
            return;
        };
        let body = Tg::photo_form(&msg.chat_id, msg.thread_id, &act1(vi, "screenCaption", shot["device"].as_str().unwrap_or("")), "photo", buf, "screen.png", Some("image/png"));
        if !self.call("sendPhoto", body, UPLOAD_TIMEOUT_MS).ok() {
            self.reply(msg, act(vi, "screenFail"), None);
        }
    }

    pub fn send_diff(&self, entry: &Entry) {
        let vi = self.vi();
        let post = |html: &str| {
            self.post(&entry.chat_id, entry.thread_id, html, Map::new());
        };
        let diff = super::files::project_diff(&entry.cwd, &self.deps.masker);
        let (text, files) = match diff {
            Ok(d) => d,
            Err(e) => return post(act(vi, if e == "clean" { "diffClean" } else if e == "not-repo" { "diffNotRepo" } else { "diffFail" })),
        };
        if text.len() as u64 > FILE_MAX {
            return post(act(vi, "diffFail"));
        }
        let body = Tg::photo_form(&entry.chat_id, entry.thread_id, &diff_caption(vi, &entry.name, files), "document", text.into_bytes(), "changes.diff", Some("text/x-diff"));
        if !self.call("sendDocument", body, UPLOAD_TIMEOUT_MS).ok() {
            post(act(vi, "diffFail"));
        }
    }

    pub fn download(&self, project: &str, file: &TgFile) -> Result<String, &'static str> {
        if file.size > DOWNLOAD_MAX {
            return Err("big");
        }
        let token = self.token().ok_or("failed")?;
        let info = self.call_json("getFile", json!({"file_id": file.file_id}));
        let path = info.result()["file_path"].as_str().map(str::to_string).filter(|p| (1..=300).contains(&p.len()) && p.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b'/' | b'-')) && !p.contains("..")).ok_or("failed")?;
        let url = format!("{}/file/bot{token}/{path}", self.deps.base.trim_end_matches('/'));
        let (status, buf) = self.deps.http.get(&url, UPLOAD_TIMEOUT_MS, DOWNLOAD_MAX as u64 + 1).map_err(|_| "failed")?;
        if !(200..300).contains(&status) {
            return Err("failed");
        }
        if buf.len() as f64 > DOWNLOAD_MAX {
            return Err("big");
        }
        super::files::save_inbound(project, &file.name, &buf, &self.local(self.now())).ok_or("failed")
    }

    pub fn start_run(&self, project: &str, config_id: &str, name: &str, chat: &str, thread: Option<i64>) {
        let vi = self.vi();
        let res = self.host("telegram.runStart", json!({"project": project, "configId": config_id})).unwrap_or_else(|| json!({"ok": false, "error": "failed"}));
        let session = res["sessionId"].as_str().filter(|_| res["ok"] == true).map(str::to_string);
        let Some(session) = session else {
            let code = res["error"].as_str().unwrap_or("failed").to_string();
            self.post(chat, thread, &more1(vi, "runFail", &code), Map::new());
            return;
        };
        {
            let mut st = self.lock();
            super::jsx::js_remove(&mut st.run_watch, &session);
            st.run_watch.push((session, RunWatch { project: project.into(), chat_id: chat.into(), thread_id: thread, config: name.into(), shot: false }));
            if st.run_watch.len() > 50 {
                st.run_watch.remove(0);
            }
        }
        self.post(chat, thread, &more1(vi, "runStarted", name), Map::new());
    }
}
