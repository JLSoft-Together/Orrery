pub mod account;
pub mod blob;
pub mod grammers;
pub mod grammers_tl;
pub mod html;
pub mod login;
pub mod service;
pub mod store;
pub mod upload;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_tg;

use std::sync::Arc;

#[derive(Clone, Debug, PartialEq)]
pub enum MtError {
    Rpc { code: i32, name: String, value: Option<u32> },
    Net,
    Other(&'static str),
}

impl MtError {
    pub fn rpc(code: i32, name: &str, value: Option<u32>) -> Self {
        MtError::Rpc { code, name: name.to_string(), value }
    }

    pub fn is(&self, want: &str) -> bool {
        matches!(self, MtError::Rpc { name, .. } if name == want)
    }

    pub fn value(&self) -> Option<u32> {
        match self {
            MtError::Rpc { value, .. } => *value,
            _ => None,
        }
    }

    pub fn code(&self) -> String {
        match self {
            MtError::Rpc { name, .. } => name.chars().take(200).collect(),
            MtError::Net => "NETWORK".into(),
            MtError::Other(why) => why.to_string(),
        }
    }

    pub fn flood_wait(&self) -> Option<u64> {
        match self {
            MtError::Rpc { name, value: Some(s), .. } if name == "FLOOD_WAIT" || name == "FLOOD_PREMIUM_WAIT" || name == "SLOWMODE_WAIT" => Some(*s as u64),
            _ => None,
        }
    }

    pub fn transient(&self) -> bool {
        match self {
            MtError::Net => true,
            MtError::Rpc { code, name, .. } => *code >= 500 || *code == -503 || name == "RPC_CALL_FAIL" || name == "TIMEOUT" || name.starts_with("INTERDC_"),
            MtError::Other(_) => false,
        }
    }
}

fn any(msg: &str, keys: &[&str]) -> bool {
    keys.iter().any(|k| msg.contains(k))
}

pub fn classify_upload(e: &MtError) -> &'static str {
    let msg = e.code();
    if matches!(e, MtError::Rpc { code: 401, .. }) || any(&msg, &["AUTH_KEY", "ACCESS_TOKEN", "API_ID", "BOT_", "SESSION_REVOKED", "USER_DEACTIVATED"]) {
        return "auth";
    }
    if any(&msg, &["CHANNEL_", "CHAT_", "PEER_ID", "TOPIC_", "MESSAGE_THREAD", "USER_BANNED", "RIGHT_FORBIDDEN", "SEND_"]) {
        return "chat";
    }
    if e.transient() {
        return "network";
    }
    "failed"
}

pub fn classify_user(e: &MtError) -> &'static str {
    let msg = e.code();
    if any(&msg, &["FLOOD", "SLOWMODE"]) {
        return "flood";
    }
    if any(&msg, &["PHONE_NUMBER"]) {
        return "phone";
    }
    if any(&msg, &["PHONE_CODE"]) {
        return "code";
    }
    if any(&msg, &["PASSWORD_HASH", "SRP_"]) {
        return "password";
    }
    if matches!(e, MtError::Rpc { code: 401, .. }) || any(&msg, &["AUTH_KEY", "SESSION_REVOKED", "SESSION_EXPIRED", "USER_DEACTIVATED", "API_ID"]) {
        return "auth";
    }
    if e.transient() {
        return "network";
    }
    "failed"
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Peer {
    Channel { id: i64, hash: i64 },
    Chat(i64),
}

pub fn chat_peer(chat_id: &str) -> Option<Peer> {
    if let Some(rest) = chat_id.strip_prefix("-100") {
        return rest.parse::<i64>().ok().filter(|n| *n > 0).map(|id| Peer::Channel { id, hash: 0 });
    }
    chat_id.strip_prefix('-').and_then(|r| r.parse::<i64>().ok()).filter(|n| *n > 0).map(Peer::Chat)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Doc {
    pub peer: Peer,
    pub thread: Option<i32>,
    pub file_id: i64,
    pub parts: i32,
    pub name: String,
    pub caption: String,
    pub entities: Vec<html::Entity>,
    pub random_id: i64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Sent {
    Code(String),
    Done(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum SignIn {
    Done(String),
    SignUp,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ForumChat {
    pub id: i64,
    pub hash: i64,
    pub title: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Topic {
    pub id: i32,
    pub title: String,
    pub top_message: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TopicPage {
    pub topics: Vec<Topic>,
    pub raw_len: usize,
    pub dates: Vec<(i32, i32)>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Offset {
    pub date: i32,
    pub id: i32,
    pub topic: i32,
}

pub trait Link: Send + Sync {
    fn bot_sign_in(&self, token: &str) -> Result<(), MtError>;
    fn save_part(&self, file_id: i64, part: i32, total: i32, bytes: Vec<u8>) -> Result<(), MtError>;
    fn resolve_channel(&self, id: i64) -> Result<i64, MtError>;
    fn send_document(&self, doc: &Doc) -> Result<(), MtError>;
    fn send_code(&self, phone: &str) -> Result<Sent, MtError>;
    fn sign_in(&self, phone: &str, hash: &str, code: &str) -> Result<SignIn, MtError>;
    fn check_password(&self, password: &str) -> Result<String, MtError>;
    fn forum_chats(&self, max_dialogs: usize) -> Result<Vec<ForumChat>, MtError>;
    fn forum_topics(&self, chat: &ForumChat, at: Offset, limit: i32) -> Result<TopicPage, MtError>;
    fn log_out(&self) -> Result<(), MtError>;
    fn export(&self) -> Option<String>;
    fn close(&self);
}

pub trait Connector: Send + Sync {
    fn connect(&self, api_id: i32, api_hash: &str, session: Option<&str>) -> Result<Arc<dyn Link>, MtError>;
}

pub fn random_i64() -> i64 {
    let mut b = [0u8; 8];
    if getrandom::fill(&mut b).is_err() {
        return crate::util::now_ms() as i64;
    }
    i64::from_le_bytes(b)
}

pub fn display_name(first: Option<&str>, last: Option<&str>, username: Option<&str>) -> String {
    let name = [first, last].into_iter().flatten().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" ");
    let name = if name.is_empty() { username.filter(|u| !u.is_empty()).map(|u| format!("@{u}")).unwrap_or_else(|| "Telegram".into()) } else { name };
    let mut out = String::new();
    let mut gap = false;
    for c in name.chars() {
        if (c as u32) < 0x20 || c as u32 == 0x7f {
            if !gap {
                out.push(' ');
            }
            gap = true;
        } else {
            out.push(c);
            gap = false;
        }
    }
    out.chars().take(64).collect()
}
