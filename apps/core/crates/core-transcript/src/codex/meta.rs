use super::extract::{meta_of, Meta};
use super::value::{parse_record, unescape, J};
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub const MAX_META_LINE: u64 = 4 * 1024 * 1024;
const HEAD_TEXT: usize = 64 * 1024;
const FIELDS: [&str; 7] = ["id", "session_id", "timestamp", "cwd", "originator", "cli_version", "source"];

fn line_end(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

pub fn raw_field<'a>(body: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("\"{name}\":\"");
    let mut search = 0;
    'outer: while let Some(i) = body[search..].find(&key) {
        let from = search + i + key.len();
        search += i + 1;
        let mut chars = body[from..].char_indices();
        while let Some((at, c)) = chars.next() {
            match c {
                '"' => return Some(&body[from..from + at]),
                '\\' => match chars.next() {
                    Some((_, n)) if !line_end(n) => {}
                    _ => continue 'outer,
                },
                _ => {}
            }
        }
    }
    None
}

fn from_head(head: &str) -> Option<Meta> {
    let payload_at = head.find("\"payload\":")?;
    if !head.contains("\"type\":\"session_meta\"") {
        return None;
    }
    let body = &head[payload_at..];
    let fields = FIELDS.iter().filter_map(|name| raw_field(body, name).and_then(unescape).map(|v| ((*name).to_owned(), J::Str(v)))).collect();
    meta_of(Some(&J::Obj(fields)))
}

pub fn read_session_meta(file: &Path) -> Option<Meta> {
    let mut buf = Vec::new();
    File::open(file).ok()?.take(MAX_META_LINE).read_to_end(&mut buf).ok()?;
    if let Some(end) = memchr::memchr(b'\n', &buf) {
        let rec = parse_record(&String::from_utf8_lossy(&buf[..end]))?;
        if rec.get("type").and_then(J::as_str) != Some("session_meta") {
            return None;
        }
        return meta_of(rec.get("payload"));
    }
    let text = String::from_utf8_lossy(&buf[..buf.len().min(HEAD_TEXT)]);
    from_head(&text)
}
