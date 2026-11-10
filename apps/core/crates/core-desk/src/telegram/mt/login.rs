use super::super::jsx::slice;
use super::{classify_user, Connector, Link, MtError, Offset, Sent, SignIn};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

pub const LOGIN_TTL_MS: i64 = 10 * 60_000;
pub const MAX_CHATS: usize = 50;
pub const MAX_TOPICS: usize = 300;
pub const PAGE: i32 = 100;
pub const GENERAL_TOPIC: i32 = 1;
pub const MAX_DIALOGS: usize = 200;

#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Next(&'static str),
    Done { session: String, name: String },
    Fail(&'static str),
}

struct Pending {
    link: Arc<dyn Link>,
    phone: String,
    hash: String,
    at: i64,
}

#[derive(Default)]
pub struct Logins {
    pending: Mutex<Option<Pending>>,
}

impl Logins {
    fn slot(&self) -> std::sync::MutexGuard<'_, Option<Pending>> {
        self.pending.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn clear(&self) {
        if let Some(p) = self.slot().take() {
            p.link.close();
        }
    }

    pub fn active(&self, now: i64) -> bool {
        self.slot().as_ref().is_some_and(|p| now - p.at <= LOGIN_TTL_MS)
    }

    fn live(&self, now: i64) -> Option<(Arc<dyn Link>, String, String)> {
        let mut slot = self.slot();
        if slot.as_ref().is_some_and(|p| now - p.at > LOGIN_TTL_MS) {
            if let Some(p) = slot.take() {
                p.link.close();
            }
        }
        slot.as_ref().map(|p| (p.link.clone(), p.phone.clone(), p.hash.clone()))
    }

    fn finish(&self, link: &Arc<dyn Link>, name: String) -> Outcome {
        let session = link.export();
        self.clear();
        link.close();
        match session {
            Some(session) => Outcome::Done { session, name },
            None => Outcome::Fail("failed"),
        }
    }

    pub fn send_code(&self, c: &dyn Connector, api_id: i32, api_hash: &str, phone: &str, now: i64) -> Outcome {
        self.clear();
        let link = match c.connect(api_id, api_hash, None) {
            Ok(l) => l,
            Err(e) => return Outcome::Fail(classify_user(&e)),
        };
        match link.send_code(phone) {
            Ok(Sent::Code(hash)) => {
                *self.slot() = Some(Pending { link, phone: phone.to_string(), hash, at: now });
                Outcome::Next("code")
            }
            Ok(Sent::Done(name)) => self.finish(&link, name),
            Err(e) => {
                link.close();
                Outcome::Fail(classify_user(&e))
            }
        }
    }

    pub fn sign_in(&self, code: &str, now: i64) -> Outcome {
        let Some((link, phone, hash)) = self.live(now) else { return Outcome::Fail("expired") };
        match link.sign_in(&phone, &hash, code) {
            Ok(SignIn::Done(name)) => self.finish(&link, name),
            Ok(SignIn::SignUp) => {
                self.clear();
                Outcome::Fail("phone")
            }
            Err(e) if e.is("SESSION_PASSWORD_NEEDED") => Outcome::Next("password"),
            Err(e) => Outcome::Fail(classify_user(&e)),
        }
    }

    pub fn password(&self, password: &str, now: i64) -> Outcome {
        let Some((link, _, _)) = self.live(now) else { return Outcome::Fail("expired") };
        match link.check_password(password) {
            Ok(name) => self.finish(&link, name),
            Err(e) => Outcome::Fail(classify_user(&e)),
        }
    }
}

pub fn scan(link: &dyn Link) -> Result<Vec<Value>, MtError> {
    let mut chats = Vec::new();
    for chat in link.forum_chats(MAX_DIALOGS)? {
        let mut topics: Vec<Value> = Vec::new();
        let mut at = Offset::default();
        while topics.len() < MAX_TOPICS {
            let page = link.forum_topics(&chat, at, PAGE)?;
            for t in page.topics.iter().filter(|t| t.id != GENERAL_TOPIC) {
                topics.push(json!({"threadId": t.id, "name": slice(&t.title, 128)}));
            }
            let Some(last) = page.topics.last() else { break };
            if page.raw_len < PAGE as usize {
                break;
            }
            at = Offset { topic: last.id, id: last.top_message, date: page.dates.iter().find(|(id, _)| *id == last.top_message).map(|(_, d)| *d).unwrap_or(0) };
        }
        chats.push(json!({"chatId": format!("-100{}", chat.id), "title": slice(&chat.title, 120), "topics": topics}));
        if chats.len() >= MAX_CHATS {
            break;
        }
    }
    Ok(chats)
}

pub fn topics(c: &dyn Connector, api_id: i32, api_hash: &str, session: &str) -> Result<Vec<Value>, &'static str> {
    let link = c.connect(api_id, api_hash, Some(session)).map_err(|e| classify_user(&e))?;
    let res = scan(&*link);
    link.close();
    res.map_err(|e| classify_user(&e))
}

pub fn logout(c: &dyn Connector, api_id: i32, api_hash: &str, session: &str) {
    if let Ok(link) = c.connect(api_id, api_hash, Some(session)) {
        let _ = link.log_out();
        link.close();
    }
}
