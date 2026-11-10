use crate::appearance::{parse_look, Look};
use crate::pages_style::{STILL, STILL_A, STILL_B, STYLE_A, STYLE_B, STYLE_C};
use crate::{arg, Desk, DeskEnv};
use serde_json::Value;

pub const RETRY_URL: &str = "ccm-desktop://retry";
pub const INTRO_MS: u64 = 2450;
pub const LEAVE_MS: u64 = 280;
pub const LEAVE_SCRIPT: &str = "document.documentElement.classList.add('out')";

pub struct Texts {
    pub app_name: String,
    pub loading: String,
    pub loading_hint: String,
    pub error_title: String,
    pub retry: String,
    pub log_hint: String,
}

impl Texts {
    pub fn from_value(v: &Value) -> Self {
        let s = |k: &str| v.get(k).and_then(Value::as_str).unwrap_or("").to_string();
        Texts { app_name: s("appName"), loading: s("loading"), loading_hint: s("loadingHint"), error_title: s("errorTitle"), retry: s("retry"), log_hint: s("logHint") }
    }
}

pub fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' | '<' | '>' | '"' | '\'' => out.push_str(&format!("&#{};", c as u32)),
            _ => out.push(c),
        }
    }
    out
}

fn light() -> Look {
    Look::new("#ececee", "#ffffff", "#1d1d1f", "#5c5c61", "#86868b", "#0071e3", false)
}

fn dark() -> Look {
    Look::new("#1e1e1e", "#262628", "#f5f5f7", "#a1a1a6", "#7c7c80", "#0a84ff", true)
}

pub fn on_accent(hex: &str) -> &'static str {
    let lin = |i: usize| {
        let c = u8::from_str_radix(hex.get(i..i + 2).unwrap_or("00"), 16).map(f64::from).unwrap_or(f64::NAN) / 255.0;
        if c <= 0.04045 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    let l = 0.2126 * lin(1) + 0.7152 * lin(3) + 0.0722 * lin(5);
    if 1.05 / (l + 0.05) >= (l + 0.05) / 0.06 {
        "#ffffff"
    } else {
        "#1d1d1f"
    }
}

fn vars(l: &Look) -> String {
    format!(
        "--canvas:{}; --surface:{}; --ink:{}; --muted:{}; --faint:{}; --accent:{}; --line:color-mix(in srgb, var(--ink) 12%, transparent); --accent-soft:color-mix(in srgb, var(--accent) 22%, transparent); --on-accent:{}; --error:{}; color-scheme:{};",
        l.canvas,
        l.surface,
        l.ink,
        l.muted,
        l.faint,
        l.accent,
        on_accent(&l.accent),
        if l.dark { "#ff453a" } else { "#d70015" },
        if l.dark { "dark" } else { "light" }
    )
}

fn tokens(look: Option<&Look>) -> String {
    match look {
        Some(l) => format!(":root {{ {} }}", vars(l)),
        None => format!(":root {{ {} }}\n@media (prefers-color-scheme: dark) {{ :root {{ {} }} }}", vars(&light()), vars(&dark())),
    }
}

fn still(scope: &str) -> String {
    STILL.join(scope)
}

fn style() -> String {
    format!("{STYLE_A}{}{STYLE_B}{}{STYLE_C}", still(STILL_A), still(STILL_B))
}

const INK: &str = "#E8E6E3";
const BG: &str = "#16181D";

fn orbit(r: u32, at: u32) -> String {
    format!(r#"<circle class="orbit" pathLength="1" cx="512" cy="512" r="{r}" stroke="{INK}" stroke-width="36" transform="rotate(-90 512 512)" style="--at:{at}ms"/>"#)
}

fn planet(x: u32, y: u32, r: u32, gap: u32, at: u32, turn: u32) -> String {
    format!(r#"<g class="spin" style="--turn:{turn}s"><g class="part" style="--at:{at}ms; --from:scale(0)"><circle cx="{x}" cy="{y}" r="{gap}" fill="{BG}"/><circle cx="{x}" cy="{y}" r="{r}" fill="{INK}"/></g></g>"#)
}

fn mark() -> String {
    format!(
        r##"<div class="stage" aria-hidden="true"><div class="float"><svg class="mark" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024" fill="none"><rect width="1024" height="1024" fill="{BG}"/>{}{}{}{}{}<circle class="sun" cx="512" cy="512" r="96" fill="#E8915A"/></svg><i class="ring"></i></div></div>"##,
        orbit(220, 450),
        orbit(360, 620),
        planet(681, 371, 50, 82, 950, 18),
        planet(174, 635, 62, 96, 1060, 34),
        planet(635, 850, 40, 70, 1160, 34)
    )
}

pub fn encode_uri_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

fn page(title: &str, body: &str, look: Option<&Look>, n: usize, intro: bool) -> String {
    let html = format!(
        r#"<!doctype html><html{}><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>{}</title><style>{}
:root {{ --n: {n}; }}{}</style></head><body><main>{body}</main></body></html>"#,
        if intro { "" } else { r#" class="static""# },
        esc(title),
        tokens(look),
        style()
    );
    format!("data:text/html;charset=utf-8,{}", encode_uri_component(&html))
}

pub fn wordmark(text: &str) -> String {
    text.chars().enumerate().map(|(i, ch)| format!(r#"<span aria-hidden="true" style="--i:{i}">{}</span>"#, if ch == ' ' { " ".to_string() } else { esc(&ch.to_string()) })).collect()
}

pub fn loading_page(t: &Texts, look: Option<&Look>, version: Option<&str>) -> String {
    let n = t.app_name.chars().count();
    let ver = version.filter(|v| !v.is_empty()).map(|v| format!(r#"<p class="ver">v{}</p>"#, esc(v))).unwrap_or_default();
    let body = format!(
        r#"{}<h1 class="word" aria-label="{}">{}<i class="caret" aria-hidden="true"></i></h1><div class="bar" role="progressbar" aria-label="{}"></div><p class="status" role="status">{}</p><p class="hint">{}</p>{ver}"#,
        mark(),
        esc(&t.app_name),
        wordmark(&t.app_name),
        esc(&t.loading),
        esc(&t.loading),
        esc(&t.loading_hint)
    );
    page(&t.app_name, &body, look, n, true)
}

pub fn error_page(t: &Texts, message: &str, log: &[String], look: Option<&Look>) -> String {
    let logs = if log.is_empty() { String::new() } else { format!("<p>{}</p><pre>{}</pre>", esc(&t.log_hint), esc(&log.join("\n"))) };
    let body = format!(
        r#"{}<h1 class="title">{}</h1><p class="err" role="alert">{}</p><a class="btn" href="{RETRY_URL}">{}</a>{logs}"#,
        mark(),
        esc(&t.error_title),
        esc(message),
        esc(&t.retry)
    );
    page(&t.app_name, &body, look, 0, false)
}

pub fn register(desk: &mut Desk, _env: &DeskEnv) {
    desk.on("pages", "desk:pages-loading", |_, args| {
        let look = parse_look(arg(args, 1));
        Ok(Value::String(loading_page(&Texts::from_value(arg(args, 0)), look.as_ref(), arg(args, 2).as_str())))
    });
    desk.on("pages", "desk:pages-error", |_, args| {
        let log: Vec<String> = arg(args, 2).as_array().map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect()).unwrap_or_default();
        let look = parse_look(arg(args, 3));
        Ok(Value::String(error_page(&Texts::from_value(arg(args, 0)), arg(args, 1).as_str().unwrap_or(""), &log, look.as_ref())))
    });
}
