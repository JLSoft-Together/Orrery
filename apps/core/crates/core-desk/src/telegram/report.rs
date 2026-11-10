use super::api::{Body, Part};
use super::buttons::ReplyButtons;
use super::base::{Target, Tg, UPLOAD_TIMEOUT_MS};
use super::html::{big_done_html, big_failed_html, big_file_html, big_uploading_html, report_html, thread_fields, CAPTION_MAX, MESSAGE_MAX};
use super::sanitize::{is_secret_file_name, BIG_FILE_MAX, FILE_MAX};
use super::shape::Route;
use serde_json::{json, Value};
use std::path::Component;
use std::sync::{Arc, Mutex};

const PROGRESS_EDIT_MS: i64 = 15_000;

fn relative(from: &str, to: &str) -> String {
    let a = crate::util::resolve(from);
    let b = crate::util::resolve(to);
    let ac: Vec<Component> = a.components().collect();
    let bc: Vec<Component> = b.components().collect();
    let eq = |x: &Component, y: &Component| if cfg!(windows) { x.as_os_str().to_string_lossy().to_lowercase() == y.as_os_str().to_string_lossy().to_lowercase() } else { x == y };
    let common = ac.iter().zip(bc.iter()).take_while(|(x, y)| eq(x, y)).count();
    let mut out = std::path::PathBuf::new();
    for _ in common..ac.len() {
        out.push("..");
    }
    for c in &bc[common..] {
        out.push(c.as_os_str());
    }
    crate::util::path_str(&out)
}

fn ok(topic: String) -> Value {
    json!({"ok": true, "topic": topic})
}

fn err(e: &str) -> Value {
    json!({"ok": false, "error": e})
}

impl Tg {
    pub fn report(self: &Arc<Self>, cwd: &str, text: &str, file: Option<&str>, session: Option<&str>, buttons: Option<Vec<(String, String)>>) -> Value {
        let target = session.map(|s| Target::Pane(s.to_string()));
        if !self.lock().data.enabled {
            return err("telegram-off");
        }
        if self.token().is_none() {
            return err("telegram-config");
        }
        let (route, topic, owner) = {
            let st = self.lock();
            let agent = session.and_then(|s| st.agent_of_pane.get(s).cloned());
            let Some(route) = Tg::session_route_of(&st, agent.as_deref()).or_else(|| Tg::route_for(&st, cwd)) else { return err("no-topic") };
            let topic = Tg::topic_name(&st, &route.chat_id, route.thread_id);
            (route, topic, st.data.owner.is_some())
        };
        let vi = self.vi();
        let m = self.deps.masker.clone();
        let name = route.label.clone();
        if let (Some(list), Some(pane)) = (buttons.as_ref().filter(|b| !b.is_empty()), session) {
            if file.is_some() {
                return err("buttons-file");
            }
            if !owner {
                return err("not-paired");
            }
            let now = self.now();
            let token = self.lock().reply_buttons.add(now, pane, cwd, &route.chat_id, route.thread_id, list);
            let labels: Vec<String> = list.iter().map(|(l, _)| m.clean(l)).collect();
            let keys = ReplyButtons::markup(&token, &labels);
            let sent = self.send_text_id(&route, &report_html(&name, text, &m, MESSAGE_MAX), target, Some(keys), None);
            if let Some(e) = sent.error {
                self.lock().reply_buttons.forget(&token);
                return err(e);
            }
            self.lock().reported_at.insert(route.project.clone(), self.now());
            self.sync_poll(false);
            return json!({"ok": true, "topic": topic, "buttons": list.len()});
        }
        let Some(file) = file else {
            if let Some(e) = self.send_text(&route, &report_html(&name, text, &m, MESSAGE_MAX), target, None, None) {
                return err(e);
            }
            self.lock().reported_at.insert(route.project.clone(), self.now());
            return ok(topic);
        };
        if !crate::util::is_inside_or_same(cwd, file) {
            return err("outside");
        }
        let base = crate::util::basename(file);
        if is_secret_file_name(&base) {
            return err("secret");
        }
        let Some(size) = std::fs::metadata(file).ok().filter(|m| m.is_file()).map(|m| m.len()) else { return err("missing") };
        let rel = {
            let r = relative(cwd, file);
            if r.is_empty() { base.clone() } else { r }
        };
        if size > FILE_MAX && size <= BIG_FILE_MAX {
            if let Some(creds) = self.mt_creds() {
                let caption = report_html(&name, text, &m, CAPTION_MAX);
                let sent = self.send_text_id(&route, &big_uploading_html(vi, &rel, size, &m, None), target.clone(), None, None);
                if let Some(e) = sent.error {
                    return err(e);
                }
                let req = json!({"chatId": route.chat_id, "threadId": route.thread_id, "file": file, "name": base, "size": size, "caption": caption});
                let (route2, rel2, status) = (route.clone(), rel.clone(), sent.id);
                self.spawn(move |tg| tg.big_upload(route2, req, creds, rel2, size, status, target));
                self.lock().reported_at.insert(route.project.clone(), self.now());
                return json!({"ok": true, "topic": topic, "file": rel, "uploading": true});
            }
        }
        if size > FILE_MAX {
            let html = format!("{}\n{}", report_html(&name, text, &m, 3500), big_file_html(vi, &rel, size, &m));
            if let Some(e) = self.send_text(&route, &html, target, None, None) {
                return err(e);
            }
            self.lock().reported_at.insert(route.project.clone(), self.now());
            return json!({"ok": true, "topic": topic, "file": rel, "linkOnly": true});
        }
        if !self.take(&route.chat_id) {
            return err(self.fail("rate"));
        }
        let Ok(buf) = std::fs::read(file) else { return err("missing") };
        let mut fields = thread_fields(&route.chat_id, route.thread_id);
        fields.insert("caption".into(), json!(report_html(&name, text, &m, CAPTION_MAX)));
        fields.insert("parse_mode".into(), json!("HTML"));
        let mut parts = super::api::form(&fields);
        parts.push(("document".into(), Part::File { bytes: buf, name: base, mime: None }));
        let res = self.call("sendDocument", Body::Form(parts), UPLOAD_TIMEOUT_MS);
        if let Some(e) = res.error() {
            return err(self.fail(e));
        }
        self.ok();
        self.track(&route.chat_id, &res.result(), target.as_ref());
        self.lock().reported_at.insert(route.project.clone(), self.now());
        json!({"ok": true, "topic": topic, "file": rel})
    }

    fn mt_creds(&self) -> Option<Value> {
        let (big, hash, id, session) = {
            let st = self.lock();
            (st.data.big_files, st.data.api_hash.clone(), st.data.api_id, st.data.mt_session.clone())
        };
        if !big {
            return None;
        }
        let token = self.token()?;
        let hash = self.unseal(hash.as_deref())?;
        let id = id?;
        Some(json!({"apiId": id, "apiHash": hash, "token": token, "session": self.unseal(session.as_deref())}))
    }

    #[allow(clippy::too_many_arguments)]
    fn big_upload(self: &Arc<Self>, route: Route, mut req: Value, creds: Value, rel: String, size: u64, status: Option<i64>, target: Option<Target>) {
        let vi = self.vi();
        let m = self.deps.masker.clone();
        let edit = {
            let me = Arc::downgrade(self);
            let chat = route.chat_id.clone();
            move |html: &str| {
                if let (Some(tg), Some(id)) = (me.upgrade(), status) {
                    tg.call_json("editMessageText", json!({"chat_id": chat, "message_id": id, "text": html, "parse_mode": "HTML"}));
                }
            }
        };
        let gate = Arc::new(Mutex::new((self.now(), 0i64)));
        let id = {
            let mut st = self.lock();
            let id = st.upload_next;
            st.upload_next += 1;
            let gate = gate.clone();
            let me = Arc::downgrade(self);
            let (rel, m, edit) = (rel.clone(), m.clone(), edit.clone());
            st.uploads.insert(
                id,
                Arc::new(move |fraction: f64| {
                    let Some(tg) = me.upgrade() else { return };
                    let pct = (fraction.clamp(0.0, 1.0) * 100.0).floor() as i64;
                    let mut g = gate.lock().unwrap_or_else(|e| e.into_inner());
                    let now = tg.now();
                    if pct <= g.1 || pct >= 100 || now - g.0 < PROGRESS_EDIT_MS {
                        return;
                    }
                    g.1 = pct;
                    g.0 = now;
                    edit(&big_uploading_html(vi, &rel, size, &m, Some(pct)));
                }),
            );
            id
        };
        req["progress"] = json!(id);
        let res = match self.deps.mt.clone() {
            Some(c) => Some(self.mt_upload(&c, &req, &creds)),
            None => self.host("telegram.bigUpload", json!({"req": req, "creds": creds})),
        };
        self.lock().uploads.remove(&id);
        let _hold = gate.lock().unwrap_or_else(|e| e.into_inner());
        let res = res.unwrap_or_else(|| json!({"result": {"ok": false, "error": "failed"}}));
        if res.get("session").is_some() {
            let sealed = res["session"].as_str().filter(|s| !s.is_empty()).filter(|_| self.keychain()).and_then(|s| self.seal(s));
            let mut st = self.lock();
            st.data.mt_session = sealed;
            Tg::save_st(&self.deps, &st);
        }
        let result = &res["result"];
        if result["ok"] == true {
            self.ok();
            {
                let mut st = self.lock();
                st.health.last_sent = Some(self.now());
                st.health.sent_today += 1;
            }
            edit(&big_done_html(vi, &rel, size, &m));
            return;
        }
        self.lock().health.errors_today += 1;
        let why = result["error"].as_str().filter(|e| ["auth", "chat", "network", "failed"].contains(e)).unwrap_or("failed");
        let failed = big_failed_html(vi, &rel, why, &m);
        edit(&failed);
        self.send_text(&route, &failed, target, None, None);
    }

    pub fn upload_progress(&self, id: u64, fraction: f64) {
        let cb = self.lock().uploads.get(&id).cloned();
        if let Some(cb) = cb {
            cb(fraction);
        }
    }
}
