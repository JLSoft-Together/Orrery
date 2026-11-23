use super::api::{Body, Part};
use super::base::{Target, Tg, UPLOAD_TIMEOUT_MS};
use super::html::{report_html, thread_fields, CAPTION_MAX};
use super::sanitize::FILE_MAX;
use serde_json::{json, Value};
use std::sync::Arc;

fn err(e: &str) -> Value {
    json!({"ok": false, "error": e})
}

fn safe_name(name: &str) -> String {
    let n: String = name.chars().map(|c| if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') { c } else { '-' }).collect();
    let n = n.trim_matches(['-', '.']).to_owned();
    if n.is_empty() {
        "session.md".to_owned()
    } else if n.ends_with(".md") {
        n
    } else {
        format!("{n}.md")
    }
}

impl Tg {
    pub fn send_doc(self: &Arc<Self>, cwd: &str, session: Option<&str>, name: &str, text: &str, caption: &str) -> Value {
        let target = session.map(|s| Target::Pane(s.to_string()));
        if !self.lock().data.enabled {
            return err("telegram-off");
        }
        if self.token().is_none() {
            return err("telegram-config");
        }
        if text.is_empty() || text.len() as u64 > FILE_MAX {
            return err("size");
        }
        let (route, topic) = {
            let st = self.lock();
            let agent = session.and_then(|s| st.agent_of_pane.get(s).cloned());
            let Some(route) = Tg::session_route_of(&st, agent.as_deref()).or_else(|| Tg::route_for(&st, cwd)) else { return err("no-topic") };
            let topic = Tg::topic_name(&st, &route.chat_id, route.thread_id);
            (route, topic)
        };
        if !self.take(&route.chat_id) {
            return err(self.fail("rate"));
        }
        let m = self.deps.masker.clone();
        let mut fields = thread_fields(&route.chat_id, route.thread_id);
        fields.insert("caption".into(), json!(report_html(&route.label, caption, &m, CAPTION_MAX)));
        fields.insert("parse_mode".into(), json!("HTML"));
        let mut parts = super::api::form(&fields);
        parts.push(("document".into(), Part::File { bytes: text.as_bytes().to_vec(), name: safe_name(name), mime: Some("text/markdown") }));
        let res = self.call("sendDocument", Body::Form(parts), UPLOAD_TIMEOUT_MS);
        if let Some(e) = res.error() {
            return err(self.fail(e));
        }
        self.ok();
        self.track(&route.chat_id, &res.result(), target.as_ref());
        self.lock().reported_at.insert(route.project.clone(), self.now());
        json!({"ok": true, "topic": topic})
    }
}

#[cfg(test)]
mod tests {
    use super::safe_name;

    #[test]
    fn names_are_safe_markdown_files() {
        assert_eq!(safe_name("claude-api-2026.md"), "claude-api-2026.md");
        assert_eq!(safe_name("../a b"), "a-b.md");
        assert_eq!(safe_name(""), "session.md");
    }
}
