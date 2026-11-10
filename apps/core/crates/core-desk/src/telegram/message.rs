use super::jsx::{slice, tail, trim};
use super::sanitize::{TAIL_DEFAULT, TAIL_MAX};
use super::text::MENU_EN;
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct TgFile {
    pub file_id: String,
    pub kind: &'static str,
    pub name: String,
    pub size: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TgMessage {
    pub message_id: i64,
    pub chat_id: String,
    pub chat_type: String,
    pub thread_id: Option<i64>,
    pub from_id: i64,
    pub from_name: String,
    pub from_bot: bool,
    pub text: String,
    pub reply_to: Option<i64>,
    pub reply_to_bot: bool,
    pub file: Option<TgFile>,
    pub group: Option<String>,
    pub date: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Pair(String),
    Status,
    Where,
    Help,
    Stop,
    Compact,
    Close,
    Focus,
    New(String),
    Resume,
    Run(String),
    All(String),
    Last,
    Tail(i64),
    Screen(Option<String>),
    Send(String),
    File(TgFile, String),
}

impl Command {
    pub fn kind(&self) -> &'static str {
        match self {
            Command::Pair(_) => "pair",
            Command::Status => "status",
            Command::Where => "where",
            Command::Help => "help",
            Command::Stop => "stop",
            Command::Compact => "compact",
            Command::Close => "close",
            Command::Focus => "focus",
            Command::New(_) => "new",
            Command::Resume => "resume",
            Command::Run(_) => "run",
            Command::All(_) => "all",
            Command::Last => "last",
            Command::Tail(_) => "tail",
            Command::Screen(_) => "screen",
            Command::Send(_) => "send",
            Command::File(..) => "file",
        }
    }
}

fn safe_name(raw: &str, fallback: &str) -> String {
    let mut out = String::new();
    let mut run = false;
    for c in raw.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
            out.push(c);
            run = false;
        } else if !run {
            out.push('_');
            run = true;
        }
    }
    let out = tail(out.trim_start_matches('.'), 80);
    if out.is_empty() {
        fallback.to_string()
    } else {
        out
    }
}

fn size(o: &Value) -> f64 {
    o.get("file_size").and_then(Value::as_f64).unwrap_or(0.0)
}

fn file_of(m: &Value) -> Option<TgFile> {
    if let Some(list) = m.get("photo").and_then(Value::as_array).filter(|l| !l.is_empty()) {
        let mut photos: Vec<&Value> = list.iter().filter(|p| p.is_object()).collect();
        photos.sort_by(|a, b| size(b).partial_cmp(&size(a)).unwrap_or(std::cmp::Ordering::Equal));
        if let Some(best) = photos.first() {
            if let Some(id) = best.get("file_id").and_then(Value::as_str) {
                return Some(TgFile { file_id: id.into(), kind: "photo", name: "photo.jpg".into(), size: size(best) });
            }
        }
    }
    for kind in ["voice", "audio", "video", "document"] {
        let Some(o) = m.get(kind).filter(|o| o.is_object()) else { continue };
        let Some(id) = o.get("file_id").and_then(Value::as_str) else { continue };
        let ext = match kind {
            "voice" => "voice.ogg",
            "video" => "video.mp4",
            "audio" => "audio.mp3",
            _ => "file.bin",
        };
        let raw = o.get("file_name").and_then(Value::as_str).filter(|n| !n.is_empty()).unwrap_or(ext);
        return Some(TgFile { file_id: id.into(), kind, name: safe_name(raw, ext), size: size(o) });
    }
    None
}

pub fn read_message(update: &Value, bot_id: Option<i64>) -> Option<TgMessage> {
    let m = update.get("message").filter(|m| m.is_object())?;
    let chat = m.get("chat").filter(|c| c.is_object())?;
    let from = m.get("from").filter(|f| f.is_object())?;
    let message_id = m.get("message_id").and_then(Value::as_f64)? as i64;
    let chat_id = chat.get("id").filter(|v| v.is_number())?;
    let from_id = from.get("id").and_then(Value::as_f64)? as i64;
    let text = m.get("text").and_then(Value::as_str).or_else(|| m.get("caption").and_then(Value::as_str)).unwrap_or("").to_string();
    let file = file_of(m);
    if text.is_empty() && file.is_none() {
        return None;
    }
    let reply = m.get("reply_to_message").filter(|r| r.is_object());
    let reply_from = reply.and_then(|r| r.get("from")).filter(|f| f.is_object());
    let topic_start = reply.is_some_and(|r| r.get("forum_topic_created").is_some_and(Value::is_object));
    let parts: Vec<&str> = ["first_name", "last_name"].iter().filter_map(|k| from.get(*k).and_then(Value::as_str).filter(|s| !s.is_empty())).collect();
    let name = if parts.is_empty() { from.get("username").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| crate::js::number(from_id as f64)) } else { parts.join(" ") };
    let group = file.as_ref().and(m.get("media_group_id").and_then(Value::as_str).filter(|g| crate::js::len(g) < 64)).map(str::to_string);
    Some(TgMessage {
        message_id,
        chat_id: crate::js::number(chat_id.as_f64().unwrap_or(0.0)),
        chat_type: chat.get("type").and_then(Value::as_str).unwrap_or("").to_string(),
        thread_id: if m.get("is_topic_message") == Some(&Value::Bool(true)) { m.get("message_thread_id").and_then(Value::as_f64).map(|t| t as i64) } else { None },
        from_id,
        from_name: slice(&name, 80),
        from_bot: from.get("is_bot") == Some(&Value::Bool(true)),
        text,
        reply_to: if topic_start { None } else { reply.and_then(|r| r.get("message_id")).and_then(Value::as_f64).map(|v| v as i64) },
        reply_to_bot: reply_from.is_some_and(|f| bot_id.is_some() && f.get("id").and_then(Value::as_f64) == bot_id.map(|b| b as f64)),
        file,
        group,
        date: m.get("date").and_then(Value::as_f64).filter(|d| d.is_finite()).map(|d| d * 1000.0),
    })
}

fn menu_head(lower: &str) -> Option<&str> {
    let rest = lower.strip_prefix('/')?;
    let end = rest.find(|c: char| !c.is_ascii_lowercase()).unwrap_or(rest.len());
    if end == 0 {
        return None;
    }
    let tail = &rest[end..];
    (tail.is_empty() || tail.starts_with(core_transcript::jsstr::js_space)).then(|| &rest[..end])
}

fn strip_mention(text: &str, bot: &str) -> String {
    let at: Vec<char> = format!("@{bot}").to_lowercase().chars().collect();
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    let word = |c: char| c.is_ascii_alphanumeric() || c == '_';
    while i < chars.len() {
        let end = i + at.len();
        if end <= chars.len() && chars[i..end].iter().zip(at.iter()).all(|(a, b)| a.to_lowercase().eq(b.to_lowercase())) && (end == chars.len() || !word(chars[end]) || !word(chars[end - 1])) {
            out.push(' ');
            i = end;
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

pub fn command_of(msg: &TgMessage, bot: &str) -> Option<Command> {
    let at = format!("@{bot}").to_lowercase();
    let lower = msg.text.to_lowercase();
    let menu = menu_head(&lower);
    let mentioned = lower.contains(&at) || menu.is_some_and(|m| MENU_EN.iter().any(|(n, _)| *n == m));
    if !mentioned && !msg.reply_to_bot && msg.chat_type != "private" {
        return None;
    }
    let stripped = trim(&strip_mention(&msg.text, bot)).to_string();
    let body = stripped.strip_prefix('/').unwrap_or(&stripped);
    let end = body.find(|c: char| !c.is_ascii_alphabetic()).unwrap_or(body.len());
    let (head, rest) = if end > 0 && (end == body.len() || body[end..].starts_with(core_transcript::jsstr::js_space)) {
        (Some(body[..end].to_ascii_lowercase()), trim(&body[end..]).to_string())
    } else {
        (None, String::new())
    };
    let h = head.as_deref();
    let no_rest = rest.is_empty();
    if h == Some("pair") && rest.len() == 6 && rest.bytes().all(|b| b.is_ascii_digit()) {
        return Some(Command::Pair(rest));
    }
    if matches!(h, Some("status") | Some("list")) && no_rest {
        return Some(Command::Status);
    }
    if matches!(h, Some("help") | Some("start")) && no_rest && msg.file.is_none() {
        return Some(Command::Help);
    }
    if msg.chat_type == "private" {
        return None;
    }
    if let Some(f) = &msg.file {
        return Some(Command::File(f.clone(), stripped));
    }
    match (h, no_rest) {
        (Some("stop"), true) => return Some(Command::Stop),
        (Some("compact"), true) => return Some(Command::Compact),
        (Some("close"), true) => return Some(Command::Close),
        (Some("focus"), true) => return Some(Command::Focus),
        (Some("where"), true) => return Some(Command::Where),
        (Some("new"), _) => return Some(Command::New(rest)),
        (Some("resume"), true) => return Some(Command::Resume),
        (Some("run"), _) => return Some(Command::Run(slice(&rest, 80))),
        (Some("all"), _) => return Some(if no_rest { Command::Help } else { Command::All(rest) }),
        (Some("last"), true) => return Some(Command::Last),
        _ => {}
    }
    if h == Some("tail") && rest.len() <= 4 && rest.bytes().all(|b| b.is_ascii_digit()) {
        let n = rest.parse::<i64>().ok().filter(|n| *n != 0).unwrap_or(TAIL_DEFAULT);
        return Some(Command::Tail(n.clamp(1, TAIL_MAX)));
    }
    if matches!(h, Some("screen") | Some("screenshot")) && rest.len() <= 80 && rest.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-')) {
        return Some(Command::Screen((!rest.is_empty()).then_some(rest)));
    }
    Some(if stripped.is_empty() { Command::Help } else { Command::Send(stripped) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn msg(text: &str) -> TgMessage {
        read_message(&json!({"message": {"message_id": 5, "chat": {"id": -1001, "type": "supergroup"}, "from": {"id": 7, "first_name": "Jin"}, "text": text, "is_topic_message": true, "message_thread_id": 3}}), Some(1)).unwrap()
    }

    #[test]
    fn reads_and_parses_commands() {
        let m = msg("@MyBot status");
        assert_eq!((m.chat_id.as_str(), m.thread_id, m.from_name.as_str()), ("-1001", Some(3), "Jin"));
        assert_eq!(command_of(&m, "MyBot"), Some(Command::Status));
        assert_eq!(command_of(&msg("/status"), "MyBot"), Some(Command::Status));
        assert_eq!(command_of(&msg("hello"), "MyBot"), None);
        assert_eq!(command_of(&msg("@mybot tail 500"), "MyBot"), Some(Command::Tail(80)));
        assert_eq!(command_of(&msg("@mybot tail"), "MyBot"), Some(Command::Tail(30)));
        assert_eq!(command_of(&msg("@MyBot new do x"), "MyBot"), Some(Command::New("do x".into())));
        assert_eq!(command_of(&msg("@MyBot fix the bug"), "MyBot"), Some(Command::Send("fix the bug".into())));
        assert_eq!(command_of(&msg("@MyBot all"), "MyBot"), Some(Command::Help));
        assert_eq!(command_of(&msg("@MyBot pair 123456"), "MyBot"), Some(Command::Pair("123456".into())));
        assert_eq!(command_of(&msg("@MyBot screen emulator-5554"), "MyBot"), Some(Command::Screen(Some("emulator-5554".into()))));
        let doc = read_message(&json!({"message": {"message_id": 1, "chat": {"id": -1, "type": "group"}, "from": {"id": 2}, "caption": "@b x", "document": {"file_id": "F", "file_name": "../a b.txt", "file_size": 3}}}), None).unwrap();
        assert_eq!(doc.file.as_ref().unwrap().name, "_a_b.txt");
    }
}
