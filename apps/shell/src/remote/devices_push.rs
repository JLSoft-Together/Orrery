use super::devices::{Devices, WebSub};

impl Devices {
    pub fn set_push(&mut self, id: &str, token: &str, locale: &str) -> bool {
        self.set_push_mute(id, token, locale, Vec::new())
    }

    pub fn set_push_mute(&mut self, id: &str, token: &str, locale: &str, mute: Vec<String>) -> bool {
        if self.find(id).is_none() {
            return false;
        }
        for d in self.list.iter_mut().filter(|d| d.device_id != id && d.push_token == token) {
            d.push_token.clear();
            d.push_locale.clear();
        }
        if let Some(d) = self.list.iter_mut().find(|d| d.device_id == id) {
            d.push_token = token.to_owned();
            d.push_locale = locale.to_owned();
            d.push_mute = mute;
            d.web_push = None;
        }
        self.save();
        true
    }

    pub fn set_web_push(&mut self, id: &str, sub: WebSub, locale: &str, mute: Vec<String>, content: bool) -> bool {
        if self.find(id).is_none() {
            return false;
        }
        for d in self.list.iter_mut().filter(|d| d.device_id != id && d.web_push.as_ref().is_some_and(|w| w.endpoint == sub.endpoint)) {
            d.web_push = None;
        }
        if let Some(d) = self.list.iter_mut().find(|d| d.device_id == id) {
            d.push_token.clear();
            d.web_push = Some(sub);
            d.push_locale = locale.to_owned();
            d.push_mute = mute;
            d.push_content = content;
        }
        self.save();
        true
    }

    pub fn set_push_content(&mut self, id: &str, content: bool) {
        if let Some(d) = self.list.iter_mut().find(|d| d.device_id == id && d.push_content != content) {
            d.push_content = content;
            self.save();
        }
    }

    pub fn set_push_seal(&mut self, id: &str, seal: bool, ok: bool, now: u64) {
        let ok = seal && ok;
        let at = if ok { now } else { 0 };
        if let Some(d) = self.list.iter_mut().find(|d| d.device_id == id && (d.push_seal, d.push_seal_ok, d.push_seal_at) != (seal, ok, at)) {
            d.push_seal = seal;
            d.push_seal_ok = ok;
            d.push_seal_at = at;
            self.save();
        }
    }

    pub fn set_push_off(&mut self, id: &str, off: bool) -> bool {
        let Some(d) = self.list.iter_mut().find(|d| d.device_id == id) else { return false };
        if d.push_off != off {
            d.push_off = off;
            self.save();
        }
        true
    }

    pub fn clear_web_endpoint(&mut self, endpoint: &str) -> bool {
        let mut changed = false;
        for d in self.list.iter_mut().filter(|d| d.web_push.as_ref().is_some_and(|w| w.endpoint == endpoint)) {
            d.web_push = None;
            changed = true;
        }
        if changed {
            self.save();
        }
        changed
    }

    pub fn clear_push(&mut self, id: &str) {
        if let Some(d) = self.list.iter_mut().find(|d| d.device_id == id && (!d.push_token.is_empty() || d.web_push.is_some())) {
            d.push_token.clear();
            d.push_locale.clear();
            d.web_push = None;
            self.save();
        }
    }

    pub fn clear_all_push(&mut self) {
        let mut changed = false;
        for d in self.list.iter_mut().filter(|d| !d.push_token.is_empty()) {
            d.push_token.clear();
            d.push_locale.clear();
            changed = true;
        }
        if changed {
            self.save();
        }
    }

    pub fn clear_push_token(&mut self, token: &str) -> bool {
        if token.is_empty() {
            return false;
        }
        let mut changed = false;
        for d in self.list.iter_mut().filter(|d| d.push_token == token) {
            d.push_token.clear();
            d.push_locale.clear();
            changed = true;
        }
        if changed {
            self.save();
        }
        changed
    }
}
