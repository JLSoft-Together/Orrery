use core_store::model::TerminalSession;
use core_store::num::JsNum;
use core_store::persist::jsstr::slice_units;
use core_store::persist::notify_file::ToastLocale;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

use crate::text::Text;
use crate::util::{name_of, toast_tag};

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToastContent {
    pub tag: String,
    pub title: String,
    pub body: String,
    pub url: String,
    pub open: String,
    pub dismiss: String,
    pub persistent: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snooze_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snooze_label: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PhoneEpisode {
    pub episode: String,
    pub reason: &'static str,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<JsNum>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub free_bytes: Option<JsNum>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_pct: Option<JsNum>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<JsNum>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hit: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelegramEpisode {
    pub episode: String,
    pub reason: &'static str,
    pub terminal_id: String,
    pub name: String,
    pub cwd: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub since: Option<JsNum>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub worked: Option<JsNum>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ask: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HostMessage {
    Show(ToastContent),
    Hide(String),
    Phone(PhoneEpisode),
    Telegram(TelegramEpisode),
}

fn typed(kind: &str, body: impl Serialize) -> Value {
    let mut out = Map::new();
    out.insert("type".into(), Value::String(kind.into()));
    if let Ok(Value::Object(m)) = serde_json::to_value(body) {
        out.extend(m);
    }
    Value::Object(out)
}

impl HostMessage {
    pub fn to_message(&self, full: bool) -> Value {
        match self {
            HostMessage::Show(c) if full => typed("toast.show", c),
            HostMessage::Show(c) => json!({ "type": "toast.show", "tag": c.tag, "title": c.title, "persistent": c.persistent }),
            HostMessage::Hide(tag) => json!({ "type": "toast.hide", "tag": tag }),
            HostMessage::Phone(p) => typed("phone", p),
            HostMessage::Telegram(t) => typed("telegram", t),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskAlert {
    pub label: String,
    pub level: String,
    pub free_bytes: f64,
    pub total_bytes: f64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentNote {
    pub episode: String,
    pub level: String,
    pub text: String,
    pub name: String,
    #[serde(default)]
    pub terminal_id: Option<String>,
    #[serde(default)]
    pub merged: f64,
}

pub fn toast_content(terminal: &TerminalSession, locale: ToastLocale, url: &str, tool: Option<&str>) -> ToastContent {
    let text = Text(locale);
    let reason = terminal.waiting_for.as_deref().map(|w| slice_units(w, 120));
    ToastContent {
        tag: toast_tag(&format!("waiting:{}", terminal.id)),
        title: text.waiting_title(&name_of(terminal)),
        body: text.waiting_body(reason.as_deref(), tool),
        url: url.to_string(),
        open: text.open(),
        dismiss: text.dismiss(),
        persistent: true,
        snooze_url: None,
        snooze_label: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn content() -> ToastContent {
        ToastContent { tag: "t".into(), title: "Title".into(), body: "secret".into(), url: "http://x".into(), open: "o".into(), dismiss: "d".into(), persistent: true, snooze_url: None, snooze_label: None }
    }

    #[test]
    fn main_gets_only_the_minimum_unless_it_renders_the_toast() {
        assert_eq!(HostMessage::Show(content()).to_message(false), json!({ "type": "toast.show", "tag": "t", "title": "Title", "persistent": true }));
        assert_eq!(HostMessage::Hide("t".into()).to_message(false), json!({ "type": "toast.hide", "tag": "t" }));
        assert_eq!(
            HostMessage::Show(content()).to_message(true),
            json!({ "type": "toast.show", "tag": "t", "title": "Title", "body": "secret", "url": "http://x", "open": "o", "dismiss": "d", "persistent": true })
        );
        let p = PhoneEpisode { episode: "d:a|".into(), reason: "done", name: "app".into(), since: Some(JsNum(5.0)), ..Default::default() };
        assert_eq!(HostMessage::Phone(p).to_message(false), json!({ "type": "phone", "episode": "d:a|", "reason": "done", "name": "app", "since": 5 }));
    }
}
