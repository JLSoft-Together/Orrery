use super::base::Tg;
use super::text::esc;
use serde_json::{json, Value};
use std::sync::Arc;

const NAMES_MAX: usize = 5;

fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}

pub fn ci_html(vi: bool, ev: &Value) -> Option<String> {
    let branch = esc(&super::jsx::slice(s(ev, "branch"), 120));
    let project = esc(&crate::util::basename(s(ev, "root")));
    let failed: Vec<String> = ev.get("failed").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).take(NAMES_MAX).map(|n| esc(&super::jsx::slice(n, 80))).collect();
    let pr = ev.get("pr").and_then(Value::as_u64).map(|n| format!(" #{n}")).unwrap_or_default();
    let html = match (s(ev, "state"), vi) {
        ("success", true) => format!("✅ CI đã qua: <b>{branch}</b>{pr} ({project})"),
        ("success", false) => format!("✅ CI passed: <b>{branch}</b>{pr} ({project})"),
        ("failure", true) => format!("❌ CI lỗi: <b>{branch}</b>{pr} ({project})"),
        ("failure", false) => format!("❌ CI failed: <b>{branch}</b>{pr} ({project})"),
        ("neutral", true) => format!("➖ CI đã xong: <b>{branch}</b>{pr} ({project})"),
        ("neutral", false) => format!("➖ CI finished: <b>{branch}</b>{pr} ({project})"),
        _ => return None,
    };
    if failed.is_empty() {
        Some(html)
    } else {
        Some(format!("{html}\n{}", failed.join(", ")))
    }
}

pub fn ci_keys(vi: bool, ev: &Value) -> Option<Value> {
    let url = s(ev, "url");
    let ok = url.starts_with("https://github.com/") && url.len() < 400 && url.chars().all(|c| c.is_ascii_graphic());
    if !ok {
        return None;
    }
    let label = match (ev.get("pr").and_then(Value::as_u64).is_some(), vi) {
        (true, true) => "Mở PR",
        (true, false) => "Open PR",
        (false, true) => "Mở trên GitHub",
        (false, false) => "Open on GitHub",
    };
    Some(json!({"inline_keyboard": [[{"text": label, "url": url}]]}))
}

impl Tg {
    pub fn ci_done(self: &Arc<Self>, ev: &Value) -> bool {
        let enabled = self.lock().data.enabled;
        if !enabled || self.token().is_none() {
            return false;
        }
        let route = {
            let st = self.lock();
            Tg::route_for(&st, s(ev, "root"))
        };
        let Some(route) = route else { return false };
        let vi = self.vi();
        let Some(html) = ci_html(vi, ev) else { return false };
        self.send_text(&route, &html, None, ci_keys(vi, ev), Some(true)).is_none()
    }
}
