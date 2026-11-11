use serde_json::Value;

const LINK_REASONS: [&str; 4] = ["waiting", "question", "permission", "done"];

pub fn terminal_of(m: &Value) -> Option<String> {
    let reason = m.get("reason").and_then(Value::as_str)?;
    if !LINK_REASONS.contains(&reason) {
        return None;
    }
    let key = m.get("episode").and_then(Value::as_str)?;
    let rest = key.strip_prefix("w:").or_else(|| key.strip_prefix("d:"))?;
    let (id, _) = rest.rsplit_once('|')?;
    (!id.is_empty()).then(|| id.to_string())
}

fn id_ok(s: &str, extra: &[u8]) -> bool {
    (1..=128).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-' || extra.contains(&b))
}

pub fn valid_link(raw: &str) -> bool {
    let Ok(u) = url::Url::parse(raw) else { return false };
    if u.scheme() != "https" || !u.username().is_empty() || u.password().is_some() || u.query().is_some() || u.fragment().is_some() || u.host_str().is_none() {
        return false;
    }
    let Some(parts) = u.path_segments().map(|s| s.collect::<Vec<_>>()) else { return false };
    matches!(parts.as_slice(), ["o", machine, pane] if id_ok(machine, b"") && id_ok(pane, b".:"))
}

pub fn from_host(res: Option<Value>) -> Option<String> {
    let url = res?.get("url")?.as_str()?.to_string();
    valid_link(&url).then_some(url)
}

pub fn label(en: bool) -> &'static str {
    if en { "Open on phone" } else { "Mở trên điện thoại" }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn terminal_comes_from_the_episode_key_of_pane_reasons_only() {
        assert_eq!(terminal_of(&json!({"reason": "permission", "episode": "w:t-1|2026"})).as_deref(), Some("t-1"));
        assert_eq!(terminal_of(&json!({"reason": "done", "episode": "d:a|b|c"})).as_deref(), Some("a|b"));
        assert_eq!(terminal_of(&json!({"reason": "disk", "episode": "disk:C:|1"})), None);
        assert_eq!(terminal_of(&json!({"reason": "note", "episode": "n:x|1"})), None);
        assert_eq!(terminal_of(&json!({"reason": "done", "episode": "j:x|1"})), None);
        assert_eq!(terminal_of(&json!({"reason": "done", "episode": "d:|1"})), None);
    }

    #[test]
    fn only_relay_pane_links_pass() {
        assert!(valid_link("https://orrery-relay.jinvu.workers.dev/o/abc_DEF-123/p1.a:b"));
        assert!(!valid_link("http://orrery-relay.jinvu.workers.dev/o/abc/p1"));
        assert!(!valid_link("https://x.dev/o/abc"));
        assert!(!valid_link("https://x.dev/o/abc/p1/extra"));
        assert!(!valid_link("https://x.dev/o/abc/p1?x=1"));
        assert!(!valid_link("https://x.dev/o/abc/p%201"));
        assert!(!valid_link("https://u:p@x.dev/o/abc/p1"));
        assert!(!valid_link("https://x.dev/p/abc/p1"));
        assert!(!valid_link("javascript:alert(1)"));
    }

    #[test]
    fn host_answer_must_carry_a_valid_url() {
        assert_eq!(from_host(Some(json!({"url": "https://x.dev/o/abc/p1"}))).as_deref(), Some("https://x.dev/o/abc/p1"));
        assert_eq!(from_host(Some(json!({"url": null}))), None);
        assert_eq!(from_host(Some(json!({"url": "https://evil.dev/anything"}))), None);
        assert_eq!(from_host(None), None);
    }
}
