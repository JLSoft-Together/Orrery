use serde_json::{json, Value};

pub const REPLY_TTL_MS: i64 = 24 * 3_600_000;
pub const REPLY_GROUPS_MAX: usize = 200;

#[derive(Clone, Debug)]
pub struct ReplyGroup {
    pub pane: String,
    pub project: String,
    pub chat_id: String,
    pub thread_id: Option<i64>,
    pub buttons: Vec<(String, String)>,
    pub at: i64,
    pub done: Option<String>,
    pub busy: bool,
}

pub enum Claim {
    Expired,
    Answered(String),
    Busy,
    Ok { token: String, group: ReplyGroup, label: String, value: String },
}

#[derive(Default)]
pub struct ReplyButtons {
    groups: Vec<(String, ReplyGroup)>,
}

pub fn parse_data(data: &str) -> Option<(&str, usize)> {
    let rest = data.strip_prefix("rb:")?;
    if rest.len() != 13 {
        return None;
    }
    let (token, idx) = rest.split_at(12);
    let ok = token.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    let n = idx.as_bytes()[0];
    (ok && (b'0'..=b'5').contains(&n)).then(|| (token, (n - b'0') as usize))
}

fn token() -> String {
    use base64::Engine;
    let mut b = [0u8; 9];
    let _ = getrandom::fill(&mut b);
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(b)
}

impl ReplyButtons {
    fn prune(&mut self, now: i64) {
        self.groups.retain(|(_, g)| now - g.at <= REPLY_TTL_MS);
    }

    pub fn add(&mut self, now: i64, pane: &str, project: &str, chat_id: &str, thread_id: Option<i64>, buttons: &[(String, String)]) -> String {
        self.prune(now);
        let mut t = token();
        while self.groups.iter().any(|(k, _)| *k == t) {
            t = token();
        }
        self.groups.push((t.clone(), ReplyGroup { pane: pane.into(), project: project.into(), chat_id: chat_id.into(), thread_id, buttons: buttons.to_vec(), at: now, done: None, busy: false }));
        while self.groups.len() > REPLY_GROUPS_MAX {
            self.groups.remove(0);
        }
        t
    }

    pub fn markup(token: &str, labels: &[String]) -> Value {
        json!({"inline_keyboard": labels.iter().enumerate().map(|(i, text)| json!([{"text": text, "callback_data": format!("rb:{token}{i}")}])).collect::<Vec<_>>()})
    }

    pub fn forget(&mut self, token: &str) {
        self.groups.retain(|(k, _)| k != token);
    }

    pub fn claim(&mut self, now: i64, data: &str) -> Claim {
        let Some((token, index)) = parse_data(data) else { return Claim::Expired };
        let Some(pos) = self.groups.iter().position(|(k, _)| k == token) else { return Claim::Expired };
        if now - self.groups[pos].1.at > REPLY_TTL_MS {
            self.groups.remove(pos);
            return Claim::Expired;
        }
        let g = &mut self.groups[pos].1;
        let Some((label, value)) = g.buttons.get(index).cloned() else { return Claim::Expired };
        if let Some(d) = &g.done {
            return Claim::Answered(d.clone());
        }
        if g.busy {
            return Claim::Busy;
        }
        g.busy = true;
        Claim::Ok { token: token.to_string(), group: g.clone(), label, value }
    }

    pub fn finish(&mut self, token: &str, label: &str) {
        if let Some((_, g)) = self.groups.iter_mut().find(|(k, _)| k == token) {
            g.done = Some(label.to_string());
            g.busy = false;
        }
    }

    pub fn release(&mut self, token: &str) {
        if let Some((_, g)) = self.groups.iter_mut().find(|(k, _)| k == token) {
            g.busy = false;
        }
    }

    pub fn pending(&mut self, now: i64) -> bool {
        self.prune(now);
        self.groups.iter().any(|(_, g)| g.done.is_none())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_tap_wins_and_expires() {
        let mut rb = ReplyButtons::default();
        let t = rb.add(0, "pane", "/p", "-1001", None, &[("Yes".into(), "yes".into()), ("No".into(), "no".into())]);
        assert_eq!(t.len(), 12);
        assert!(rb.pending(0));
        let data = format!("rb:{t}1");
        let Claim::Ok { label, value, .. } = rb.claim(1, &data) else { panic!() };
        assert_eq!((label.as_str(), value.as_str()), ("No", "no"));
        assert!(matches!(rb.claim(2, &data), Claim::Busy));
        rb.finish(&t, "No");
        assert!(matches!(rb.claim(3, &format!("rb:{t}0")), Claim::Answered(l) if l == "No"));
        assert!(!rb.pending(4));
        assert!(matches!(rb.claim(REPLY_TTL_MS + 10, &data), Claim::Expired));
        assert!(matches!(rb.claim(0, &format!("rb:{t}5")), Claim::Expired));
    }
}
