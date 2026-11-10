use super::read::draft;
use super::{tailer, Meta, Out, Sub, Tracker};
use crate::jsstr::{iso_of_ms, js_slice, put_opt};
use crate::spans::SPAN_RETENTION_MS;
use crate::state::TranscriptState;
use crate::time::parse_iso;
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_DESCRIPTION: usize = 160;

pub fn cwd_slug(cwd: &str) -> String {
    let mut out = String::with_capacity(cwd.len());
    for c in cwd.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else {
            for _ in 0..c.len_utf16() {
                out.push('-');
            }
        }
    }
    out
}

pub fn find_transcript(projects: &Path, session_id: &str, cwd: Option<&str>) -> Option<PathBuf> {
    let name = format!("{session_id}.jsonl");
    if let Some(cwd) = cwd.filter(|c| !c.is_empty()) {
        let guess = projects.join(cwd_slug(cwd)).join(&name);
        if guess.exists() {
            return Some(guess);
        }
    }
    let dirs = crate::jobs::read_dir_sorted(projects)?;
    for d in dirs {
        let candidate = projects.join(d.file_name()).join(&name);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

fn id_chars(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

pub fn meta_id(name: &str) -> Option<&str> {
    name.strip_prefix("agent-")?.strip_suffix(".meta.json").filter(|id| id_chars(id))
}

fn text(o: &Map<String, Value>, k: &str) -> Option<String> {
    o.get(k).and_then(Value::as_str).map(str::to_owned)
}

pub fn parse_meta(raw: &str) -> Meta {
    let Ok(Value::Object(o)) = serde_json::from_str::<Value>(raw) else { return Meta::default() };
    Meta {
        agent_type: text(&o, "agentType"),
        description: o.get("description").and_then(Value::as_str).map(|d| js_slice(d, MAX_DESCRIPTION).to_owned()),
        tool_use_id: text(&o, "toolUseId"),
        spawn_depth: o.get("spawnDepth").and_then(Value::as_f64),
        parent_agent_id: text(&o, "parentAgentId").filter(|p| id_chars(p)),
        model: text(&o, "model"),
    }
}

fn js_ms(d: std::time::Duration) -> f64 {
    d.as_secs() as f64 * 1000.0 + f64::from(d.subsec_nanos()) / 1e6
}

fn ms_of(t: std::io::Result<SystemTime>) -> f64 {
    match t.ok().map(|t| t.duration_since(UNIX_EPOCH)) {
        Some(Ok(d)) => js_ms(d),
        Some(Err(e)) => -js_ms(e.duration()),
        None => 0.0,
    }
}

fn started_at(file: &Path) -> Option<String> {
    let md = std::fs::metadata(file).ok()?;
    let birth = ms_of(md.created());
    let ms = if birth != 0.0 { birth } else { ms_of(md.modified()) };
    iso_of_ms(ms)
}

fn mtime_ms(file: &Path) -> f64 {
    std::fs::metadata(file).map_or(f64::INFINITY, |m| ms_of(m.modified()))
}

pub fn task_status(raw: &str) -> &'static str {
    match raw {
        "completed" => "completed",
        "failed" => "error",
        "stopped" => "cancelled",
        _ => "unknown",
    }
}

pub fn finished(status: &str) -> bool {
    matches!(status, "completed" | "error" | "cancelled")
}

fn agent_data(meta: &Meta) -> Map<String, Value> {
    let mut d = Map::new();
    put_opt(&mut d, "agentType", &meta.agent_type);
    put_opt(&mut d, "description", &meta.description);
    d
}

impl Tracker {
    pub(super) fn discover(&mut self, catch_up: bool, out: &mut Out) -> HashSet<String> {
        let mut fresh = HashSet::new();
        let Some(transcript) = &self.transcript else { return fresh };
        let dir = transcript.with_extension("").join("subagents");
        let Some(entries) = crate::jobs::read_dir_sorted(&dir) else { return fresh };
        for e in entries {
            let name = e.file_name();
            let Some(name) = name.to_str() else { continue };
            let Some(agent_id) = meta_id(name) else { continue };
            if self.subs.contains(agent_id) {
                continue;
            }
            let meta_file = dir.join(name);
            let Ok(raw) = std::fs::read(&meta_file) else { continue };
            let meta = parse_meta(&String::from_utf8_lossy(&raw));
            let Some(started) = started_at(&meta_file) else { continue };
            let sub = Sub {
                agent_id: agent_id.to_owned(),
                meta,
                tailer: tailer(dir.join(format!("agent-{agent_id}.jsonl"))),
                state: TranscriptState::default(),
                started_at: Some(started),
                status: "working",
                resolved: false,
                completed_at: None,
            };
            if !catch_up {
                out.activity.push(Value::Object(draft("agent.started", &self.terminal_id, Some(agent_id), &sub.started_at, Some(agent_data(&sub.meta)))));
            }
            self.subs.insert(agent_id, sub);
            self.dirty.insert(agent_id.to_owned());
            fresh.insert(agent_id.to_owned());
        }
        fresh
    }

    fn linked_outcome(&self, sub: &Sub) -> Option<(&'static str, Option<String>)> {
        let id = sub.meta.tool_use_id.as_deref().filter(|t| !t.is_empty())?;
        if let Some((status, at)) = self.links.notifications.get(id) {
            return Some((task_status(status), at.clone()));
        }
        self.links.foreground.get(id).map(|(err, at)| (if *err { "error" } else { "completed" }, at.clone()))
    }

    pub(super) fn about_to_resolve(&self, sub: &Sub) -> bool {
        !sub.resolved && self.linked_outcome(sub).is_some_and(|(status, _)| finished(status) || status == "unknown")
    }

    pub(super) fn is_lite_catch_up(&self, id: &str, now: i64) -> bool {
        let Some(sub) = self.subs.get(id) else { return false };
        let cutoff = now - SPAN_RETENTION_MS;
        let Some((status, at)) = self.linked_outcome(sub) else { return false };
        if !finished(status) && status != "unknown" {
            return false;
        }
        if !at.as_deref().and_then(parse_iso).is_some_and(|t| t < cutoff) {
            return false;
        }
        mtime_ms(&sub.tailer.file) < cutoff as f64
    }

    pub(super) fn resolve(&mut self, id: &str, catch_up: bool, out: &mut Out) {
        let Some(sub) = self.subs.get(id) else { return };
        if sub.resolved {
            return;
        }
        let (status, at) = match self.linked_outcome(sub) {
            Some(l) => l,
            None if sub.state.ended && sub.state.open_tools.is_empty() => ("completed", sub.state.last_end_at.clone()),
            None => ("working", None),
        };
        if !finished(status) && status != "unknown" {
            return;
        }
        let Some(sub) = self.subs.get_mut(id) else { return };
        sub.status = status;
        sub.resolved = true;
        sub.completed_at = at.or_else(|| sub.state.last_at.clone());
        let completed = sub.completed_at.clone();
        let data = agent_data(&sub.meta);
        self.dirty.insert(id.to_owned());
        let closed = match completed.as_deref().filter(|c| !c.is_empty()) {
            Some(c) => self.spans.close_open(c, Some(id)),
            None => Vec::new(),
        };
        for s in closed {
            out.spans.push(s.to_json());
        }
        if catch_up {
            return;
        }
        let kind = match status {
            "completed" => "agent.completed",
            "cancelled" => "agent.cancelled",
            _ => "agent.failed",
        };
        out.activity.push(Value::Object(draft(kind, &self.terminal_id, Some(id), &completed, Some(data))));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_and_meta_like_node() {
        assert_eq!(cwd_slug("D:\\work\\demo"), "D--work-demo");
        assert_eq!(cwd_slug("/Users/me/my.app"), "-Users-me-my-app");
        assert_eq!(cwd_slug("a😀"), "a--");
        assert_eq!(meta_id("agent-a1_B-2.meta.json"), Some("a1_B-2"));
        assert_eq!(meta_id("agent-.meta.json"), None);
        assert_eq!(meta_id("agent-a.b.meta.json"), None);
        let m = parse_meta(&format!(r#"{{"agentType":"Explore","description":"{}","parentAgentId":"../evil","spawnDepth":2.5}}"#, "x".repeat(300)));
        assert_eq!(m.description.map(|d| d.len()), Some(160));
        assert_eq!(m.parent_agent_id, None);
        assert_eq!(m.spawn_depth, Some(2.5));
        assert!(parse_meta("[1]").agent_type.is_none());
        assert!(parse_meta("{bad").agent_type.is_none());
    }
}
