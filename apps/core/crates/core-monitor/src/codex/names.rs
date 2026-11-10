use std::collections::HashMap;

use core_transcript::jsstr::{js_slice, js_trim};
use serde_json::Value;

const MAX_NAME: usize = 120;

pub fn parse_thread_names(text: &str) -> HashMap<String, String> {
    let mut names = HashMap::new();
    for line in text.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if js_trim(line).is_empty() {
            continue;
        }
        let Ok(Value::Object(r)) = serde_json::from_str::<Value>(line) else { continue };
        let (Some(Value::String(id)), Some(Value::String(name))) = (r.get("id"), r.get("thread_name")) else { continue };
        let name = js_trim(name);
        if name.is_empty() {
            continue;
        }
        let cleaned: String = name.chars().map(|c| if (c as u32) < 0x20 || c as u32 == 0x7f { ' ' } else { c }).collect();
        names.insert(id.clone(), js_slice(&cleaned, MAX_NAME).to_owned());
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_threads_from_the_session_index() {
        let text = format!(
            "{}\nnot json\n{}\n{}\r\n",
            r#"{"id":"t1","thread_name":"Fix build","updated_at":"x"}"#,
            r#"{"id":"b","thread_name":"  "}"#,
            r#"{"id":"t2","thread_name":" a\u0001b "}"#
        );
        let names = parse_thread_names(&text);
        assert_eq!(names.len(), 2);
        assert_eq!(names["t1"], "Fix build");
        assert_eq!(names["t2"], "a b");
    }
}
