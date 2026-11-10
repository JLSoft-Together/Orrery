use memchr::memmem;

pub const HEAD_BYTES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineClass {
    Parse,
    Skip,
    Compacted,
}

const KEEP_EVENTS: [&[u8]; 5] = [b"task_started", b"task_complete", b"turn_aborted", b"token_count", b"context_compacted"];
const KEEP_ITEMS: [&[u8]; 5] = [b"function_call", b"custom_tool_call", b"local_shell_call", b"function_call_output", b"custom_tool_call_output"];
const TYPE_KEY: &[u8] = b"\"type\":\"";
const PAYLOAD_TYPE: &[u8] = b",\"payload\":{\"type\":\"";
const TIMESTAMP_KEY: &str = "\"timestamp\":\"";

fn word_run(h: &[u8], from: usize) -> usize {
    h.get(from..).map_or(0, |s| s.iter().take_while(|b| b.is_ascii_alphanumeric() || **b == b'_').count())
}

fn quoted_word(h: &[u8], from: usize) -> Option<&[u8]> {
    let n = word_run(h, from);
    (n > 0 && h.get(from + n) == Some(&b'"')).then(|| &h[from..from + n])
}

pub fn head_types(h: &[u8]) -> Option<(&[u8], Option<&[u8]>)> {
    let mut search = 0;
    while let Some(i) = memmem::find(&h[search..], TYPE_KEY) {
        let at = search + i;
        search = at + 1;
        let from = at + TYPE_KEY.len();
        let Some(top) = quoted_word(h, from) else { continue };
        let after = from + top.len() + 1;
        let sub = h.get(after..).filter(|s| s.starts_with(PAYLOAD_TYPE)).and_then(|_| quoted_word(h, after + PAYLOAD_TYPE.len()));
        return Some((top, sub));
    }
    None
}

pub fn classify_line(head: &[u8]) -> LineClass {
    let Some((top, sub)) = head_types(head) else { return LineClass::Parse };
    match top {
        b"session_meta" | b"turn_context" => LineClass::Parse,
        b"compacted" => LineClass::Compacted,
        b"event_msg" if sub.is_none_or(|s| KEEP_EVENTS.contains(&s)) => LineClass::Parse,
        b"response_item" if sub.is_none_or(|s| KEEP_ITEMS.contains(&s)) => LineClass::Parse,
        _ => LineClass::Skip,
    }
}

pub fn timestamp_in(head: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(head);
    let mut search = 0;
    while let Some(i) = text[search..].find(TIMESTAMP_KEY) {
        let at = search + i;
        search = at + 1;
        let from = at + TIMESTAMP_KEY.len();
        let rest = &text[from..];
        let Some(end) = rest.find('"') else { continue };
        let units: usize = rest[..end].chars().map(char::len_utf16).sum();
        if (10..=40).contains(&units) {
            return Some(rest[..end].to_owned());
        }
    }
    None
}
