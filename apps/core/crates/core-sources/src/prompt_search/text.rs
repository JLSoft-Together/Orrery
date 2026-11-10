use core_transcript::jsstr::{js_slice, js_space, js_trim};
use serde_json::Value;

use super::nfc::fold_text;

pub const PROMPT_SEARCH_MAX_SESSIONS: usize = 200;
pub const PROMPT_SEARCH_SNIPPETS: usize = 3;
pub const PROMPT_SEARCH_SNIPPET_MAX: usize = 200;
pub const PROMPT_SEARCH_QUERY_MAX: usize = 200;
pub const PROMPT_SEARCH_TERMS_MAX: usize = 8;

const NOT_PROMPT: [&str; 14] = [
    "<command-name>",
    "<command-message>",
    "<command-args>",
    "<local-command-stdout>",
    "<local-command-stderr>",
    "<local-command-caveat>",
    "<bash-input>",
    "<bash-stdout>",
    "<bash-stderr>",
    "<task-notification>",
    "<system-reminder>",
    "<user-prompt-submit-hook>",
    "[Request interrupted",
    "Caveat:",
];

pub fn prompt_search_terms(query: &str) -> Vec<String> {
    let folded = fold_text(js_slice(query, PROMPT_SEARCH_QUERY_MAX));
    let mut out: Vec<String> = Vec::new();
    for raw in folded.split(js_space) {
        if !raw.is_empty() && !out.iter().any(|t| t == raw) {
            out.push(raw.to_owned());
        }
    }
    out.truncate(PROMPT_SEARCH_TERMS_MAX);
    out
}

pub fn prompt_text_of(rec: &Value) -> Option<String> {
    let rec = rec.as_object()?;
    if rec.get("type")?.as_str()? != "user" {
        return None;
    }
    for k in ["isSidechain", "isMeta", "isCompactSummary", "isVisibleInTranscriptOnly"] {
        if rec.get(k) == Some(&Value::Bool(true)) {
            return None;
        }
    }
    if rec.contains_key("toolUseResult") || rec.contains_key("sourceToolUseID") {
        return None;
    }
    if rec.get("promptSource").and_then(Value::as_str) == Some("system") {
        return None;
    }
    if let Some(origin) = rec.get("origin") {
        if origin.as_object().and_then(|o| o.get("kind")).and_then(Value::as_str) != Some("human") {
            return None;
        }
    }
    let message = rec.get("message")?.as_object()?;
    let text = match message.get("content")? {
        Value::String(s) => s.clone(),
        Value::Array(blocks) => {
            let mut parts: Vec<&str> = Vec::new();
            for block in blocks {
                let Some(b) = block.as_object() else { continue };
                let ty = b.get("type").and_then(Value::as_str);
                if ty == Some("tool_result") {
                    return None;
                }
                if ty == Some("text") {
                    if let Some(t) = b.get("text").and_then(Value::as_str) {
                        parts.push(t);
                    }
                }
            }
            parts.join("\n")
        }
        _ => return None,
    };
    let trimmed = js_trim(&text);
    if trimmed.is_empty() || NOT_PROMPT.iter().any(|p| trimmed.starts_with(p)) {
        return None;
    }
    Some(trimmed.to_owned())
}

pub fn matches_all(folded: &str, terms: &[String]) -> bool {
    !terms.is_empty() && terms.iter().all(|t| folded.contains(t.as_str()))
}

fn collapse(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut in_space = false;
    for c in text.chars() {
        if js_space(c) {
            if !in_space {
                out.push(' ');
            }
            in_space = true;
        } else {
            out.push(c);
            in_space = false;
        }
    }
    js_trim(&out).to_owned()
}

fn index_of(hay: &[u16], needle: &[u16]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

pub fn snippet_of(text: &str, terms: &[String], max: usize) -> String {
    let flat = collapse(text);
    let flat16: Vec<u16> = flat.encode_utf16().collect();
    if flat16.len() <= max {
        return flat;
    }
    let lower16: Vec<u16> = fold_text(&flat).encode_utf16().collect();
    let mut at: Option<usize> = None;
    if lower16.len() == flat16.len() {
        for t in terms {
            let t16: Vec<u16> = t.encode_utf16().collect();
            if let Some(i) = index_of(&lower16, &t16) {
                if at.is_none_or(|a| i < a) {
                    at = Some(i);
                }
            }
        }
    }
    let room = max - 2;
    let mut start = match at {
        Some(a) if a > 0 => a.saturating_sub(room / 3),
        _ => 0,
    };
    start = start.min(flat16.len() - room);
    let end = flat16.len().min(start + room);
    let mut out: Vec<u16> = Vec::new();
    if start > 0 {
        out.push(0x2026);
    }
    out.extend_from_slice(&flat16[start..end]);
    if end < flat16.len() {
        out.push(0x2026);
    }
    out.truncate(max);
    String::from_utf16_lossy(&out)
}

pub fn project_dir_name(cwd: &str) -> String {
    let mut out = String::with_capacity(cwd.len());
    for c in cwd.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else {
            for _ in 0..c.len_utf16() {
                out.push('-');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn user(content: Value, extra: Value) -> Value {
        let mut rec = json!({"parentUuid": null, "isSidechain": false, "cwd": "D:\\work\\app", "type": "user", "message": {"role": "user", "content": content}});
        if let (Some(r), Some(e)) = (rec.as_object_mut(), extra.as_object()) {
            for (k, v) in e {
                r.insert(k.clone(), v.clone());
            }
        }
        rec
    }

    #[test]
    fn keeps_typed_queued_sdk_and_legacy_prompts() {
        assert_eq!(prompt_text_of(&user(json!("fix the paywall"), json!({"promptSource": "typed", "origin": {"kind": "human"}}))).as_deref(), Some("fix the paywall"));
        assert_eq!(prompt_text_of(&user(json!([{"type": "image", "source": {}}, {"type": "text", "text": "what is in this screenshot"}]), json!({"promptSource": "sdk"}))).as_deref(), Some("what is in this screenshot"));
        assert_eq!(prompt_text_of(&user(json!("older prompt"), json!({}))).as_deref(), Some("older prompt"));
    }

    #[test]
    fn drops_every_non_prompt_user_record_kind() {
        let excluded = vec![
            user(json!([{"type": "tool_result", "content": "paywall"}]), json!({"toolUseResult": {}})),
            user(json!([{"type": "tool_result", "content": "paywall"}]), json!({})),
            user(json!("x"), json!({"isMeta": true})),
            user(json!("x"), json!({"isSidechain": true})),
            user(json!("x"), json!({"isCompactSummary": true})),
            user(json!("x"), json!({"toolUseResult": null})),
            user(json!("<command-name>/compact</command-name>"), json!({})),
            user(json!("<bash-stdout>paywall</bash-stdout>"), json!({})),
            user(json!("x"), json!({"promptSource": "system"})),
            user(json!("x"), json!({"origin": {"kind": "coordinator"}})),
            user(json!("x"), json!({"origin": "human"})),
            user(json!("[Request interrupted by user]"), json!({})),
            user(json!("   "), json!({})),
            json!({"type": "assistant", "message": {"content": "x"}}),
            json!("x"),
        ];
        for rec in excluded {
            assert_eq!(prompt_text_of(&rec), None, "{rec}");
        }
    }

    #[test]
    fn terms_snippets_and_project_dir_names() {
        assert_eq!(prompt_search_terms("  Paywall  IAP paywall "), vec!["paywall".to_owned(), "iap".to_owned()]);
        let text = format!("{}the PAYWALL screen {}", "a ".repeat(300), "b ".repeat(300));
        let s = snippet_of(&text, &["paywall".to_owned()], 200);
        assert!(s.encode_utf16().count() <= 200 && s.contains("PAYWALL") && s.starts_with('…'));
        assert_eq!(snippet_of("short  text\n here", &["x".to_owned()], 200), "short text here");
        assert_eq!(project_dir_name("D:\\work\\app"), "D--work-app");
        assert_eq!(project_dir_name("a😀b"), "a--b");
    }
}
