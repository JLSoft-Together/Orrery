use crate::{js, util};
use serde_json::{json, Value};
use std::collections::VecDeque;

#[derive(Debug, Clone, PartialEq)]
pub struct Dest {
    pub channel: &'static str,
    pub url: String,
    pub topic: Option<String>,
}

pub struct Text {
    pub title: String,
    pub body: String,
    pub link: Option<String>,
    pub link_label: &'static str,
}

fn has_query(u: &url::Url) -> bool {
    u.query().is_some_and(|q| !q.is_empty()) || u.fragment().is_some_and(|f| !f.is_empty())
}

pub fn origin_path(u: &url::Url, trim_all: bool) -> String {
    let path = u.path();
    let path = if trim_all { path.trim_end_matches('/') } else { path.strip_suffix('/').unwrap_or(path) };
    format!("{}{}", u.origin().ascii_serialization(), path)
}

pub fn clean_ntfy_server(raw: &str) -> Option<String> {
    let u = url::Url::parse(raw).ok()?;
    if !matches!(u.scheme(), "http" | "https") || !u.username().is_empty() || u.password().is_some() || has_query(&u) {
        return None;
    }
    Some(origin_path(&u, true))
}

pub fn clean_topic(raw: &str) -> Option<String> {
    let ok = (1..=64).contains(&raw.len()) && raw.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-');
    ok.then(|| raw.to_string())
}

const DISCORD_HOSTS: [&str; 4] = ["discord.com", "discordapp.com", "ptb.discord.com", "canary.discord.com"];

fn webhook_path(p: &str) -> bool {
    let Some(rest) = p.strip_prefix("/api/webhooks/") else { return false };
    let rest = rest.strip_suffix('/').unwrap_or(rest);
    let Some((id, tok)) = rest.split_once('/') else { return false };
    !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()) && !tok.is_empty() && tok.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

pub fn clean_discord_url(raw: &str) -> Option<String> {
    let u = url::Url::parse(raw).ok()?;
    if u.scheme() != "https" || !u.host_str().is_some_and(|h| DISCORD_HOSTS.contains(&h)) || !webhook_path(u.path()) || !u.username().is_empty() {
        return None;
    }
    Some(origin_path(&u, false))
}

pub fn host_of(raw: &str) -> Option<String> {
    url::Url::parse(raw).ok().and_then(|u| u.host_str().map(str::to_string))
}

fn js_round(x: f64) -> f64 {
    (x + 0.5).floor()
}

pub fn format_gb(bytes: f64) -> String {
    let gb = bytes / 1024f64.powi(3);
    if gb >= 100.0 {
        format!("{} GB", js::number(js_round(gb)))
    } else {
        format!("{} GB", util::to_fixed1(gb))
    }
}

fn body_en(m: &Value, reason: &str, w: Option<f64>) -> String {
    let w = w.filter(|n| *n != 0.0).map(js::number);
    let tool = m.get("tool").and_then(Value::as_str).filter(|t| !t.is_empty());
    match reason {
        "waiting" => w.map(|w| format!("Waiting for your input for {w} min")).unwrap_or_else(|| "Waiting for your input".into()),
        "question" => w.map(|w| format!("Asking you a question ({w} min)")).unwrap_or_else(|| "Asking you a question".into()),
        "permission" => format!("{}{}", tool.map(|t| format!("Needs approval: {t}")).unwrap_or_else(|| "Needs approval".into()), w.map(|w| format!(" ({w} min)")).unwrap_or_default()),
        "done" => w.map(|w| format!("Finished after {w} min of work")).unwrap_or_else(|| "Finished".into()),
        "job" => "A background job needs you".into(),
        "limit" => "Usage limit is about to run out".into(),
        "disk" => match m.get("freeBytes").and_then(Value::as_f64) {
            Some(b) => format!("Disk almost full: {} left", format_gb(b)),
            None => "Disk almost full".into(),
        },
        _ => m.get("text").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| "Message from the agent".into()),
    }
}

fn body_vi(m: &Value, reason: &str, w: Option<f64>) -> String {
    let w = w.filter(|n| *n != 0.0).map(js::number);
    let tool = m.get("tool").and_then(Value::as_str).filter(|t| !t.is_empty());
    match reason {
        "waiting" => w.map(|w| format!("Đang chờ chủ nhân trả lời {w} phút")).unwrap_or_else(|| "Đang chờ chủ nhân trả lời".into()),
        "question" => w.map(|w| format!("Đang hỏi chủ nhân ({w} phút)")).unwrap_or_else(|| "Đang hỏi chủ nhân".into()),
        "permission" => format!("{}{}", tool.map(|t| format!("Cần duyệt: {t}")).unwrap_or_else(|| "Cần duyệt".into()), w.map(|w| format!(" ({w} phút)")).unwrap_or_default()),
        "done" => w.map(|w| format!("Đã xong sau {w} phút làm việc")).unwrap_or_else(|| "Đã xong".into()),
        "job" => "Job nền cần chủ nhân".into(),
        "limit" => "Hạn mức sắp hết".into(),
        "disk" => match m.get("freeBytes").and_then(Value::as_f64) {
            Some(b) => format!("Ổ sắp đầy: còn {}", format_gb(b)),
            None => "Ổ sắp đầy".into(),
        },
        _ => m.get("text").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| "Agent nhắn chủ nhân".into()),
    }
}

fn mask(s: &str, home: &str, platform: &str) -> String {
    let masked = core_sources::mask::mask_secrets(s, home, platform);
    let mut flat = String::new();
    let mut in_break = false;
    for c in masked.chars() {
        if c == '\r' || c == '\n' {
            if !in_break {
                flat.push(' ');
            }
            in_break = true;
        } else {
            flat.push(c);
            in_break = false;
        }
    }
    js::slice(&flat, 200)
}

pub fn phone_text(m: &Value, en: bool, now_ms: f64, home: &str, platform: &str) -> Text {
    let reason = m.get("reason").and_then(Value::as_str).unwrap_or("note");
    let waited = m.get("since").and_then(Value::as_f64).map(|s| js_round((now_ms - s) / 60_000.0).max(0.0));
    let name = m.get("name").and_then(Value::as_str).unwrap_or("");
    let body = if en { body_en(m, reason, waited) } else { body_vi(m, reason, waited) };
    Text { title: mask(&format!("Orrery · {name}"), home, platform), body: mask(&body, home, platform), link: None, link_label: "" }
}

pub fn test_text(en: bool) -> Text {
    let body = if en { "Test from Orrery: phone notifications work." } else { "Thử từ Orrery: báo ra điện thoại đang chạy." };
    Text { title: "Orrery".into(), body: body.into(), link: None, link_label: "" }
}

pub fn build_request(dest: &Dest, msg: &Text, over: Option<&str>) -> (String, String) {
    let url = over.filter(|o| !o.is_empty()).unwrap_or(&dest.url).to_string();
    let body = if dest.channel == "ntfy" {
        let mut m = serde_json::Map::new();
        if let Some(t) = &dest.topic {
            m.insert("topic".into(), json!(t));
        }
        m.insert("title".into(), json!(msg.title));
        m.insert("message".into(), json!(msg.body));
        m.insert("tags".into(), json!(["robot"]));
        if let Some(link) = &msg.link {
            m.insert("click".into(), json!(link));
            m.insert("actions".into(), json!([{"action": "view", "label": msg.link_label, "url": link}]));
        }
        Value::Object(m)
    } else {
        json!({"username": "Orrery", "content": format!("**{}**\n{}", msg.title, msg.body), "allowed_mentions": {"parse": []}})
    };
    (url, js::stringify(&body))
}

pub struct RateLimiter {
    max: usize,
    window_ms: f64,
    times: VecDeque<f64>,
}

impl RateLimiter {
    pub fn new(max: usize, window_ms: f64) -> Self {
        RateLimiter { max, window_ms, times: VecDeque::new() }
    }

    pub fn take(&mut self, now: f64) -> bool {
        while self.times.front().is_some_and(|t| now - t >= self.window_ms) {
            self.times.pop_front();
        }
        if self.times.len() >= self.max {
            return false;
        }
        self.times.push_back(now);
        true
    }
}

pub fn e2e_override(raw: Option<&str>) -> Option<String> {
    let u = url::Url::parse(raw.filter(|r| !r.is_empty())?).ok()?;
    (u.scheme() == "http" && matches!(u.host_str(), Some("127.0.0.1" | "localhost"))).then(|| u.as_str().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_and_texts() {
        assert_eq!(clean_ntfy_server("https://ntfy.sh///").as_deref(), Some("https://ntfy.sh"));
        assert_eq!(clean_ntfy_server("https://x.org:443/a/b/").as_deref(), Some("https://x.org/a/b"));
        assert_eq!(clean_ntfy_server("https://u:p@x.org"), None);
        assert_eq!(clean_ntfy_server("https://x.org/?a=1"), None);
        assert_eq!(clean_ntfy_server("ftp://x.org"), None);
        assert_eq!(clean_discord_url("https://discord.com/api/webhooks/123/abc_-/").as_deref(), Some("https://discord.com/api/webhooks/123/abc_-"));
        assert_eq!(clean_discord_url("https://evil.com/api/webhooks/123/abc"), None);
        assert_eq!(clean_topic("a-b_c").as_deref(), Some("a-b_c"));
        assert_eq!(clean_topic("a b"), None);
        assert_eq!(format_gb(1.5 * 1024f64.powi(3)), "1.5 GB");
        assert_eq!(format_gb(150.4 * 1024f64.powi(3)), "150 GB");
        let m = json!({"reason": "permission", "name": "proj", "tool": "Bash", "since": 0});
        let t = phone_text(&m, true, 150_000.0, "/h", "darwin");
        assert_eq!(t.title, "Orrery · proj");
        assert_eq!(t.body, "Needs approval: Bash (3 min)");
        let dest = Dest { channel: "ntfy", url: "https://ntfy.sh".into(), topic: Some("t".into()) };
        let plain: Value = serde_json::from_str(&build_request(&dest, &t, None).1).unwrap();
        assert!(plain.get("click").is_none() && plain.get("actions").is_none());
        let linked = Text { link: Some("https://r.dev/o/m/p1".into()), link_label: "Open on phone", ..t };
        let with: Value = serde_json::from_str(&build_request(&dest, &linked, None).1).unwrap();
        assert_eq!(with["click"], "https://r.dev/o/m/p1");
        assert_eq!(with["actions"][0]["url"], "https://r.dev/o/m/p1");
        assert_eq!(with["actions"][0]["label"], "Open on phone");
        let mut r = RateLimiter::new(2, 10.0);
        assert!(r.take(0.0) && r.take(1.0) && !r.take(2.0) && r.take(10.0));
        assert_eq!(e2e_override(Some("http://localhost:9/x")).as_deref(), Some("http://localhost:9/x"));
        assert_eq!(e2e_override(Some("https://localhost:9/x")), None);
    }
}
