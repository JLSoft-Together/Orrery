use super::jsx::{len, round, s, slice, tail, to_fixed1, trim};
use super::text::{self, esc};
use super::text_act;
use super::text_more::label;
use serde_json::{json, Value};

pub const MESSAGE_MAX: usize = 4096;
pub const CAPTION_MAX: usize = 1024;
const TAIL_HTML_MAX: usize = 3500;
const TAIL_ESCAPED_MAX: usize = 3900;

#[derive(Clone)]
pub struct Masker {
    pub home: String,
    pub platform: &'static str,
}

impl Masker {
    pub fn clean(&self, s: &str) -> String {
        core_sources::mask::mask_secrets(s, &self.home, self.platform)
    }
}

pub fn clip(html: &str, max: usize) -> String {
    if len(html) <= max {
        return html.to_string();
    }
    let mut cut = slice(html, max - 1);
    if let Some(at) = cut.rfind('&') {
        if cut[at + 1..].bytes().all(|b| b.is_ascii_alphabetic()) {
            cut.truncate(at);
        }
    }
    format!("{cut}…")
}

pub fn quote(body: &str, m: &Masker, room: i64) -> String {
    let text = esc(trim(&m.clean(body)));
    if text.is_empty() {
        return String::new();
    }
    let open = "\n<blockquote expandable>";
    let close = "</blockquote>";
    let space = room - len(open) as i64 - len(close) as i64;
    if space < 20 {
        return String::new();
    }
    format!("{open}{}{close}", clip(&text, space as usize))
}

fn ask_block(ask: &Value, m: &Masker) -> String {
    let opts = ask["options"].as_array().cloned().unwrap_or_default();
    let mut lines: Vec<String> = opts.iter().enumerate().map(|(i, o)| format!("{}. {}", i + 1, esc(&m.clean(o.as_str().unwrap_or(""))))).collect();
    if ask.get("more") == Some(&Value::Bool(true)) {
        lines.push("…".into());
    }
    format!("\n{}\n{}", esc(&m.clean(s(ask, "question").unwrap_or(""))), lines.join("\n"))
}

pub fn episode_html(e: &Value, vi: bool, m: &Masker, reply: bool) -> String {
    let name = esc(&m.clean(s(e, "name").unwrap_or("")));
    let reason = s(e, "reason").unwrap_or("waiting");
    let head = match reason {
        "done" => text::done(vi, &name, e.get("worked").and_then(Value::as_f64).map(|w| round(w / 60_000.0).max(0)).unwrap_or(0)),
        "question" => text::question(vi, &name),
        "permission" => text::permission(vi, &name, s(e, "tool")),
        _ => text::waiting(vi, &name),
    };
    if reason == "question" {
        if let Some(ask) = e.get("ask").filter(|a| a.is_object()) {
            return format!("{head}{}", ask_block(ask, m));
        }
    }
    match s(e, "reply").filter(|r| reply && !r.is_empty()) {
        Some(r) => format!("{head}{}", quote(r, m, MESSAGE_MAX as i64 - len(&head) as i64)),
        None => head,
    }
}

pub fn digest_html(e: &Value, vi: bool, m: &Masker, reply: bool, n: i64, at: &str) -> String {
    let head = text::digest_head(vi, &esc(&m.clean(s(e, "name").unwrap_or(""))), n, at);
    let full = format!("{head}\n{}", episode_html(e, vi, m, reply));
    if len(&full) <= MESSAGE_MAX || !reply {
        full
    } else {
        format!("{head}\n{}", episode_html(e, vi, m, false))
    }
}

pub fn report_html(name: &str, body: &str, m: &Masker, max: usize) -> String {
    let head = text::report(&esc(&m.clean(name)));
    let body = esc(trim(&m.clean(body)));
    clip(&if body.is_empty() { head.clone() } else { format!("{head}\n{body}") }, max)
}

fn mb(bytes: u64) -> String {
    to_fixed1(bytes as f64 / (1024.0 * 1024.0))
}

pub fn big_uploading_html(vi: bool, file: &str, bytes: u64, m: &Masker, pct: Option<i64>) -> String {
    text::big_uploading(vi, &esc(&m.clean(file)), &mb(bytes), pct)
}

pub fn big_done_html(vi: bool, file: &str, bytes: u64, m: &Masker) -> String {
    text::big_done(vi, &esc(&m.clean(file)), &mb(bytes))
}

pub fn big_failed_html(vi: bool, file: &str, why: &str, m: &Masker) -> String {
    text::big_failed(vi, &esc(&m.clean(file)), why)
}

pub fn big_file_html(vi: bool, file: &str, bytes: u64, m: &Masker) -> String {
    text::big_file(vi, &esc(&m.clean(file)), &mb(bytes))
}

fn drop_units(s: &str, n: usize) -> String {
    let total = len(s);
    tail(s, total.saturating_sub(n))
}

pub fn tail_html(name: &str, body: &str, vi: bool, m: &Masker, token: Option<&str>) -> String {
    let unmasked = match token.filter(|t| !t.is_empty()) {
        Some(t) => body.split(t).collect::<Vec<_>>().join("[masked]"),
        None => body.to_string(),
    };
    let mut raw = m.clean(&unmasked).trim_end_matches(core_transcript::jsstr::js_space).to_string();
    if trim(&raw).is_empty() {
        return format!("{}\n{}", text::tail_head(vi, name), text::tail_empty(vi));
    }
    let mut cut = false;
    if len(&raw) > TAIL_HTML_MAX {
        raw = tail(&raw, TAIL_HTML_MAX);
        cut = true;
    }
    let mut html = esc(&raw);
    while len(&html) > TAIL_ESCAPED_MAX {
        raw = drop_units(&raw, (len(&html) - TAIL_ESCAPED_MAX).div_ceil(5) + 1);
        cut = true;
        html = esc(&raw);
    }
    if cut {
        if let Some(at) = raw.find('\n') {
            html = esc(&raw[at + 1..]);
        }
    }
    format!("{}\n<pre>{html}</pre>", text::tail_head(vi, name))
}

pub fn pane_line(p: &Value, vi: bool) -> String {
    format!("• <b>{}</b> — {}", esc(s(p, "name").unwrap_or("")), text::pane_status(s(p, "status").unwrap_or(""), vi))
}

pub fn result_html(res: &Value, vi: bool) -> String {
    let name = s(res, "name").unwrap_or("");
    if res.get("ok") == Some(&Value::Bool(true)) {
        let panes = res["panes"].as_array().cloned().unwrap_or_default();
        return match s(res, "action").unwrap_or("") {
            "tail" => text::tail_head(vi, name),
            "status" if panes.is_empty() => text::inbound(vi, "statusEmpty").into(),
            "status" => text::status(vi, &panes.iter().map(|p| pane_line(p, vi)).collect::<Vec<_>>().join("\n")),
            "send" if res.get("queued") == Some(&Value::Bool(true)) => text::queued(vi, name),
            "all" => {
                let names: Vec<String> = res["names"].as_array().map(|l| l.iter().filter_map(|n| n.as_str().map(str::to_string)).collect()).unwrap_or_default();
                if names.is_empty() {
                    text::inbound(vi, "none").into()
                } else {
                    text::sent(vi, &names.join(", "))
                }
            }
            "resumeList" => {
                let items = res["items"].as_array().cloned().unwrap_or_default();
                if items.is_empty() {
                    text::inbound(vi, "statusEmpty").into()
                } else {
                    text::status(vi, &items.iter().map(|i| format!("• {}", esc(s(i, "name").unwrap_or("")))).collect::<Vec<_>>().join("\n"))
                }
            }
            _ => text::sent(vi, name),
        };
    }
    let panes = res["panes"].as_array().cloned().unwrap_or_default();
    match s(res, "error").unwrap_or("unavailable") {
        "state" => text::inbound(vi, "gone").into(),
        "many" => text::many(vi, &panes.iter().map(|p| pane_line(p, vi)).collect::<Vec<_>>().join("\n")),
        "busy" => match panes.first() {
            Some(p) => text::busy(vi, s(p, "name").unwrap_or("")),
            None => text::inbound(vi, "open").into(),
        },
        e => text::inbound(vi, e).into(),
    }
}

pub type Row = Vec<(String, String, Option<String>)>;

pub fn markup(vi: bool, rows: &[Row]) -> Value {
    let keyboard: Vec<Value> = rows
        .iter()
        .map(|row| {
            Value::Array(
                row.iter()
                    .map(|(kind, key, t)| {
                        let l = label(vi, kind);
                        let text = match t.as_deref().filter(|t| !t.is_empty()) {
                            Some(t) => format!("{l} {t}"),
                            None => l.to_string(),
                        };
                        json!({"text": slice(&text, 60), "callback_data": format!("{kind}:{key}")})
                    })
                    .collect(),
            )
        })
        .collect();
    json!({"inline_keyboard": keyboard})
}

pub fn cell(kind: &str, key: &str) -> (String, String, Option<String>) {
    (kind.to_string(), key.to_string(), None)
}

pub fn cell_t(kind: &str, key: &str, t: &str) -> (String, String, Option<String>) {
    (kind.to_string(), key.to_string(), Some(t.to_string()))
}

#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct DayCounts {
    pub done: i64,
    pub worked_ms: f64,
    pub waits: i64,
    pub permissions: i64,
    pub run_fails: i64,
}

pub fn summary_html(vi: bool, day: &str, rows: &[(String, DayCounts)]) -> String {
    let active: Vec<&(String, DayCounts)> = rows.iter().filter(|(_, c)| c.done > 0 || c.waits > 0 || c.permissions > 0 || c.run_fails > 0).collect();
    let head = text_act::act1(vi, "summaryHead", day);
    if active.is_empty() {
        return format!("{head}\n{}", text_act::act(vi, "summaryEmpty"));
    }
    let mut lines = vec![head];
    lines.extend(active.iter().map(|(n, c)| text_act::summary_line(vi, n, c)));
    lines.join("\n")
}

pub fn thread_fields(chat: &str, thread: Option<i64>) -> serde_json::Map<String, Value> {
    let mut m = serde_json::Map::new();
    m.insert("chat_id".into(), json!(chat));
    if let Some(t) = thread.filter(|t| *t != 0) {
        m.insert("message_thread_id".into(), json!(t));
    }
    m
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk() -> Masker {
        Masker { home: "/Users/me".into(), platform: "darwin" }
    }

    #[test]
    fn episodes_and_clips() {
        let m = mk();
        let e = json!({"type": "telegram", "episode": "1", "reason": "done", "terminalId": "t", "name": "a<b", "cwd": "/x", "worked": 150_000, "reply": "  ok  "});
        assert_eq!(episode_html(&e, false, &m, true), "✅ <b>a&lt;b</b> finished after 3 min\n<blockquote expandable>ok</blockquote>");
        assert_eq!(episode_html(&e, true, &m, false), "✅ <b>a&lt;b</b> đã xong sau 3 phút");
        assert_eq!(clip("abc&amp;def", 6), "abc…");
        let r = json!({"ok": false, "error": "many", "panes": [{"name": "x", "status": "idle"}]});
        assert_eq!(result_html(&r, true), "Project này có nhiều session, chủ nhân reply vào tin báo của session muốn gửi:\n• <b>x</b> — rảnh");
        let k = markup(false, &[vec![cell("ap", "k1"), cell_t("lk", "k2", "proj")]]);
        assert_eq!(k["inline_keyboard"][0][1]["text"], "🔗 proj");
        assert_eq!(k["inline_keyboard"][0][0]["callback_data"], "ap:k1");
        let t = tail_html("n", &"x\n".repeat(3000), false, &m, Some("tok"));
        assert!(t.starts_with("📜 Terminal of <b>n</b>\n<pre>x\n"));
    }
}
