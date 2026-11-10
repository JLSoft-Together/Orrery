use super::login::{self, Outcome};
use super::Connector;
use crate::telegram::base::Tg;
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub const NOTICE_HOST: &str = "telegram.notice";

impl Tg {
    fn mt_keys(&self) -> Option<(i32, String)> {
        let keys = self.user_keys()?;
        let id = keys["apiId"].as_i64().and_then(|n| i32::try_from(n).ok())?;
        Some((id, keys["apiHash"].as_str()?.to_string()))
    }

    fn mt_user_sealed(&self) -> Option<String> {
        let api_id = self.lock().data.api_id;
        self.mt.saved().user.clone().filter(|(id, _)| Some(*id) == api_id).map(|(_, s)| s)
    }

    pub fn mt_account(&self) -> Option<String> {
        self.mt_user_sealed()?;
        Some(self.lock().data.user_name.clone().unwrap_or_else(|| "Telegram".into()))
    }

    pub fn mt_notice(&self) {
        if self.mt.noticed.load(Ordering::SeqCst) {
            return;
        }
        let (has_user, noticed) = {
            let s = self.mt.saved();
            (s.user.is_some(), s.noticed)
        };
        if has_user || noticed {
            self.mt.noticed.store(true, Ordering::SeqCst);
            return;
        }
        if self.lock().data.user_session.is_none() {
            return;
        }
        self.mt.noticed.store(true, Ordering::SeqCst);
        self.mt.update(|s| s.noticed = true);
        let _ = (self.deps.host)(NOTICE_HOST, json!({"kind": "relogin"}));
    }

    pub fn mt_topics(&self, c: &Arc<dyn Connector>) -> Option<Vec<Value>> {
        let (id, hash) = self.mt_keys()?;
        let session = self.unseal(self.mt_user_sealed().as_deref())?;
        match login::topics(&**c, id, &hash, &session) {
            Ok(chats) => Some(chats),
            Err(e) => {
                if e == "auth" {
                    self.mt.update(|s| s.user = None);
                    self.emit_state();
                }
                None
            }
        }
    }

    pub fn mt_user(self: &Arc<Self>, c: &Arc<dyn Connector>, input: &Value) -> Value {
        let failed = json!({"ok": false, "error": "failed"});
        let action = input["action"].as_str().unwrap_or("");
        if action == "logout" {
            let keys = self.mt_keys();
            let session = self.unseal(self.mt_user_sealed().as_deref());
            self.mt.logins.clear();
            let dropped = self.mt.update(|s| s.user = None);
            self.drop_user();
            let result = self.commit(false);
            if let (Some((id, hash)), Some(s)) = (keys, session) {
                login::logout(&**c, id, &hash, &s);
            }
            return if result["ok"] == true && dropped { json!({"ok": true, "state": result["state"]}) } else { failed };
        }
        let Some((id, hash)) = self.mt_keys() else { return json!({"ok": false, "error": "keys"}) };
        if !self.keychain() {
            return json!({"ok": false, "error": "keychain"});
        }
        let text = |k: &str| input[k].as_str().unwrap_or("").to_string();
        let now = self.now();
        let out = match action {
            "code" => self.mt.logins.send_code(&**c, id, &hash, &text("phone"), now),
            "signin" => self.mt.logins.sign_in(&text("code"), now),
            _ => self.mt.logins.password(&text("password"), now),
        };
        match out {
            Outcome::Next(next) => json!({"ok": true, "state": self.state(), "next": next}),
            Outcome::Fail(e) => json!({"ok": false, "error": e}),
            Outcome::Done { session, name } => {
                let Some(sealed) = self.seal(&session) else { return failed };
                if !self.mt.update(|s| s.user = Some((id as i64, sealed))) {
                    return failed;
                }
                self.lock().data.user_name = Some(crate::telegram::jsx::slice(&name, 64));
                let result = self.commit(false);
                if result["ok"] == true {
                    json!({"ok": true, "state": result["state"]})
                } else {
                    failed
                }
            }
        }
    }
}
