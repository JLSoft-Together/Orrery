use regex::Regex;
use std::sync::OnceLock;

pub const AGENT_NOTE_MAX: usize = 200;
pub const PANE_OPEN_PROMPT_MAX: usize = 8000;

fn re(cell: &'static OnceLock<Regex>, src: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(src).unwrap())
}

pub fn u16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

pub fn u16_slice(s: &str, max: usize) -> String {
    let mut n = 0;
    let mut out = String::new();
    for c in s.chars() {
        let w = c.len_utf16();
        if n + w > max {
            break;
        }
        n += w;
        out.push(c);
    }
    out
}

pub fn chars_slice(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

pub fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

pub fn js_trim_end(s: &str) -> &str {
    s.trim_end_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

const INVISIBLE: &str = r"[\x{061c}\x{180e}\x{200b}-\x{200f}\x{202a}-\x{202e}\x{2066}-\x{2069}\x{feff}\x{e0000}-\x{e007f}]";

pub fn clean_note_text(v: &str) -> Option<String> {
    static A: OnceLock<Regex> = OnceLock::new();
    static B: OnceLock<Regex> = OnceLock::new();
    static C: OnceLock<Regex> = OnceLock::new();
    static D: OnceLock<Regex> = OnceLock::new();
    let t = re(&A, r"[\x00-\x1f\x7f-\x{9f}]+").replace_all(v, " ");
    let t = re(&B, r"[\x{2028}\x{2029}]+").replace_all(&t, " ");
    let t = re(&C, INVISIBLE).replace_all(&t, "");
    let t = re(&D, r"[\s\x{feff}]{2,}").replace_all(&t, " ");
    let t = js_trim(&t);
    if t.is_empty() {
        None
    } else {
        Some(chars_slice(t, AGENT_NOTE_MAX))
    }
}

pub fn clean_note_capped(v: Option<&serde_json::Value>, max_in: usize, max_out: usize) -> Option<String> {
    let s = v?.as_str()?;
    if u16_len(s) > max_in {
        return None;
    }
    clean_note_text(s).map(|t| u16_slice(&t, max_out)).filter(|t| !t.is_empty())
}

pub fn clean_pane_prompt(v: &str) -> Option<String> {
    static A: OnceLock<Regex> = OnceLock::new();
    static B: OnceLock<Regex> = OnceLock::new();
    let t = re(&A, r"\r\n?").replace_all(v, "\n");
    let t = re(&B, r"[\x00-\x08\x0b-\x1f\x7f-\x{9f}]").replace_all(&t, "");
    let t = js_trim(&t);
    if t.is_empty() {
        None
    } else {
        Some(u16_slice(t, PANE_OPEN_PROMPT_MAX))
    }
}

pub fn clean_run_text(v: &str, max: usize) -> Option<String> {
    static A: OnceLock<Regex> = OnceLock::new();
    static B: OnceLock<Regex> = OnceLock::new();
    static C: OnceLock<Regex> = OnceLock::new();
    static D: OnceLock<Regex> = OnceLock::new();
    let head = u16_slice(v, max * 4);
    let t = re(&A, r"\x1b\[[0-?]*[ -/]*[@-~]").replace_all(&head, "");
    let t = re(&B, r"\x1b\][^\x07\x1b]*(\x07|\x1b\\)?").replace_all(&t, "");
    let t = re(&C, r"[\x00-\x08\x0b-\x1f\x7f-\x{9f}\x{2028}\x{2029}]+").replace_all(&t, " ");
    let t = re(&D, INVISIBLE).replace_all(&t, "");
    let t = t.replace('\t', "  ");
    let t = js_trim_end(&t);
    if t.is_empty() {
        None
    } else {
        Some(chars_slice(t, max))
    }
}

pub fn num(v: f64) -> serde_json::Value {
    if v.fract() == 0.0 && v.abs() < 9e15 {
        serde_json::json!(v as i64)
    } else {
        serde_json::json!(v)
    }
}

pub fn quote(v: &str) -> String {
    serde_json::to_string(v).unwrap_or_default()
}

pub fn home_dir() -> String {
    std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).unwrap_or_default()
}

pub fn platform() -> &'static str {
    if cfg!(windows) {
        "win32"
    } else {
        "darwin"
    }
}

pub fn mask_run(text: &str) -> String {
    core_transcript::mask::mask_log(text, &home_dir(), platform())
}

pub fn is_agent_session_id(v: &str) -> bool {
    static A: OnceLock<Regex> = OnceLock::new();
    re(&A, r"^(?i)[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").is_match(v)
}

pub fn is_preset(v: &str) -> bool {
    matches!(v, "claude" | "codex" | "deepseek" | "shell")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notes_lose_controls_and_bidi() {
        assert_eq!(clean_note_text("  a\u{202e}b\n\nc   d ").as_deref(), Some("ab c d"));
        assert_eq!(clean_note_text("\u{200b}\u{feff}"), None);
        assert_eq!(clean_note_text(&"x".repeat(300)).map(|s| s.len()), Some(200));
    }

    #[test]
    fn prompts_keep_newlines_and_drop_controls() {
        assert_eq!(clean_pane_prompt("\r\nhi\x07\r\nthere\t ").as_deref(), Some("hi\nthere"));
        assert_eq!(clean_pane_prompt("  "), None);
    }

    #[test]
    fn run_text_strips_ansi_and_tabs() {
        assert_eq!(clean_run_text("\x1b[31mred\x1b[0m\tx  ", 1000).as_deref(), Some("red  x"));
        assert_eq!(clean_run_text("\x1b]0;title\x07ok", 1000).as_deref(), Some("ok"));
    }

    #[test]
    fn utf16_slicing_counts_units() {
        assert_eq!(u16_len("a😀"), 3);
        assert_eq!(u16_slice("a😀b", 2), "a");
        assert_eq!(quote("a\"b"), "\"a\\\"b\"");
    }
}
