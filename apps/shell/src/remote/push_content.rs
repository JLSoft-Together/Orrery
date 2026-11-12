use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde_json::Value;

use super::pair::is_hidden;
use super::push::{test_text, title_body};

pub const TEXT_MAX: usize = 120;
pub const FACT_TTL: Duration = Duration::from_secs(600);
const FACTS_MAX: usize = 256;
const RAW_MAX: usize = 2000;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Fact {
    pub reason: &'static str,
    pub name: String,
    pub cwd: String,
    pub tool: String,
    pub question: String,
    pub worked: Option<u64>,
}

#[derive(Default)]
pub struct Facts {
    map: HashMap<String, (Fact, Instant)>,
}

fn reason_of(raw: &str) -> Option<&'static str> {
    ["done", "waiting", "question", "permission"].into_iter().find(|r| *r == raw)
}

fn short(v: &Value) -> String {
    v.as_str().unwrap_or("").chars().take(RAW_MAX).collect()
}

fn tool_ok(v: &Value) -> String {
    let t = v.as_str().unwrap_or("");
    let ok = (1..=80).contains(&t.len()) && t.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b':' | b'-'));
    if ok {
        t.to_owned()
    } else {
        String::new()
    }
}

impl Facts {
    pub fn note(&mut self, episode: &Value, now: Instant) -> bool {
        if episode["type"].as_str() != Some("telegram") {
            return false;
        }
        let (Some(terminal), Some(reason)) = (episode["terminalId"].as_str().filter(|t| !t.is_empty() && t.len() <= 200), episode["reason"].as_str().and_then(reason_of)) else { return false };
        let worked = episode["worked"].as_f64().filter(|w| w.is_finite() && *w >= 0.0).map(|w| w as u64);
        let fact = Fact { reason, name: short(&episode["name"]), cwd: short(&episode["cwd"]), tool: tool_ok(&episode["tool"]), question: short(&episode["ask"]["question"]), worked };
        if self.map.len() >= FACTS_MAX && !self.map.contains_key(terminal) {
            self.map.retain(|_, (_, at)| now.duration_since(*at) < FACT_TTL);
            if self.map.len() >= FACTS_MAX {
                let oldest = self.map.iter().min_by_key(|(_, (_, at))| *at).map(|(k, _)| k.clone());
                if let Some(k) = oldest {
                    self.map.remove(&k);
                }
            }
        }
        self.map.insert(terminal.to_owned(), (fact, now));
        true
    }

    pub fn get(&self, terminal: &str, now: Instant) -> Option<&Fact> {
        self.map.get(terminal).filter(|(_, at)| now.duration_since(*at) < FACT_TTL).map(|(f, _)| f)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Scene {
    pub agent: String,
    pub session: String,
    pub tab: String,
    pub project: String,
}

fn base_name(path: &str) -> &str {
    path.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().unwrap_or("")
}

pub fn scene_of(summary: &Value, pane: &str) -> Scene {
    let tabs = summary["tabs"].as_array().into_iter().flatten();
    for tab in tabs {
        let Some(p) = tab["panes"].as_array().into_iter().flatten().find(|p| p["key"].as_str() == Some(pane)) else { continue };
        let project = tab["project"].as_str().or(p["cwd"].as_str()).map(base_name).unwrap_or("");
        return Scene {
            agent: p["agent"].as_str().or(p["preset"].as_str()).unwrap_or("").to_owned(),
            session: p["title"].as_str().unwrap_or("").to_owned(),
            tab: tab["name"].as_str().unwrap_or("").to_owned(),
            project: project.to_owned(),
        };
    }
    Scene::default()
}

pub fn agent_label(agent: &str) -> &'static str {
    match agent {
        "claude" => "Claude",
        "codex" => "Codex",
        "deepseek" => "DeepSeek",
        "shell" => "Shell",
        _ => "Agent",
    }
}

fn is_extend(c: char) -> bool {
    matches!(c, '\u{0300}'..='\u{036F}' | '\u{1AB0}'..='\u{1AFF}' | '\u{1DC0}'..='\u{1DFF}' | '\u{20D0}'..='\u{20FF}' | '\u{FE00}'..='\u{FE0F}' | '\u{FE20}'..='\u{FE2F}' | '\u{1F3FB}'..='\u{1F3FF}' | '\u{E0100}'..='\u{E01EF}')
}

pub fn cut(text: &str, max: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() <= max {
        return text.to_owned();
    }
    let mut k = max.saturating_sub(1);
    while k > 0 && is_extend(chars[k]) {
        k -= 1;
    }
    let mut out: String = chars[..k].iter().collect();
    out = out.trim_end().to_owned();
    out.push('…');
    out
}

pub fn clean(text: &str, mask: &dyn Fn(&str) -> String, max: usize) -> String {
    let kept: String = text.chars().filter_map(|c| if c.is_whitespace() { Some(' ') } else if c.is_control() || is_hidden(c) { None } else { Some(c) }).collect();
    let flat = kept.split_whitespace().collect::<Vec<_>>().join(" ");
    cut(&mask(&flat), max)
}

fn session_name(raw: &str) -> &str {
    raw.trim_start_matches(|c: char| !c.is_alphanumeric()).trim()
}

pub fn title_of(scene: &Scene, fact: Option<&Fact>, mask: &dyn Fn(&str) -> String) -> String {
    let session = [session_name(&scene.session), scene.tab.trim(), fact.map_or("", |f| f.name.trim())].into_iter().find(|s| !s.is_empty()).unwrap_or("");
    let project = if scene.project.is_empty() { fact.map_or("", |f| base_name(&f.cwd)) } else { scene.project.as_str() };
    let mut parts: Vec<String> = Vec::new();
    for raw in [agent_label(&scene.agent), session, project] {
        let part = clean(raw, mask, TEXT_MAX);
        if !part.is_empty() && !parts.iter().any(|p| p.eq_ignore_ascii_case(&part)) {
            parts.push(part);
        }
    }
    cut(&parts.join(" · "), TEXT_MAX)
}

pub fn duration(ms: u64) -> String {
    let mins = (ms + 30_000) / 60_000;
    match mins {
        0 => "<1m".to_owned(),
        1..=59 => format!("{mins}m"),
        _ => format!("{}h {:02}m", mins / 60, mins % 60),
    }
}

pub fn body_of(kind: &str, vi: bool, fact: Option<&Fact>, worked: Option<u64>, mask: &dyn Fn(&str) -> String) -> String {
    let question = fact.map(|f| clean(&f.question, mask, TEXT_MAX)).filter(|q| !q.is_empty());
    let tool = fact.map(|f| f.tool.as_str()).filter(|t| !t.is_empty());
    let text = match (kind, vi) {
        ("question", _) if question.is_some() => question.unwrap_or_default(),
        ("question", true) => "Đang chờ bạn trả lời".to_owned(),
        ("question", false) => "Waiting for your answer".to_owned(),
        ("permission", true) => tool.map_or_else(|| "Cần bạn cho phép".to_owned(), |t| format!("Cần duyệt: {t}")),
        ("permission", false) => tool.map_or_else(|| "Needs your approval".to_owned(), |t| format!("Needs approval: {t}")),
        ("done", true) => worked.map_or_else(|| "Đã xong lượt".to_owned(), |w| format!("Xong sau {}", duration(w))),
        ("done", false) => worked.map_or_else(|| "Finished its turn".to_owned(), |w| format!("Done after {}", duration(w))),
        ("error", true) => "Dừng vì lỗi".to_owned(),
        ("error", false) => "Stopped with an error".to_owned(),
        (_, true) => "Đang chờ bạn".to_owned(),
        (_, false) => "Waiting for you".to_owned(),
    };
    clean(&text, mask, TEXT_MAX)
}

pub struct Ask<'a> {
    pub kind: &'a str,
    pub locale: &'a str,
    pub content: bool,
    pub scene: &'a Scene,
    pub fact: Option<&'a Fact>,
    pub worked: Option<u64>,
}

pub fn notice(a: &Ask, mask: &dyn Fn(&str) -> String) -> (String, String) {
    let fixed = |(t, b): (&str, &str)| (t.to_owned(), b.to_owned());
    match a.kind {
        "test" => fixed(test_text(a.locale)),
        "security" => fixed(title_body("security", a.locale)),
        _ if !a.content => fixed(title_body(a.kind, a.locale)),
        kind => (title_of(a.scene, a.fact, mask), body_of(kind, a.locale == "vi", a.fact, a.worked, mask)),
    }
}

pub fn refine(kind: &'static str, fact: Option<&Fact>) -> &'static str {
    match (kind, fact.map(|f| f.reason)) {
        ("attention", Some(r @ ("question" | "permission"))) => r,
        _ => kind,
    }
}

pub fn mask_text(text: &str) -> String {
    let home = std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).unwrap_or_default();
    let platform = if cfg!(windows) { "win32" } else if cfg!(target_os = "macos") { "darwin" } else { "linux" };
    core_desk::claude::mask::mask_hook_text(text, &home, platform)
}
