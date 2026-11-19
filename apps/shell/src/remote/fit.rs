use std::io;

use core_remote_proto::frame::MAX_MESSAGE;
use serde_json::{json, Value};

pub const SNAPSHOT_BUDGET: usize = 900_000;
pub const ACTIVITY_KEEP: usize = 100;
pub const TEXT_MAX: usize = 1_000;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Trimmed {
    pub activity: usize,
    pub agents: usize,
    pub text: bool,
}

impl Trimmed {
    pub fn any(&self) -> bool {
        self.activity > 0 || self.agents > 0 || self.text
    }

    pub fn json(&self) -> Value {
        json!({ "activity": self.activity, "agents": self.agents, "text": self.text })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Fit {
    Whole,
    Trimmed(Trimmed),
    Failed(usize),
}

struct Count(usize);

impl io::Write for Count {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0 += buf.len();
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn size(v: &Value) -> usize {
    let mut c = Count(0);
    match serde_json::to_writer(&mut c, v) {
        Ok(()) => c.0,
        Err(_) => usize::MAX,
    }
}

pub fn fits_message(v: &Value) -> bool {
    size(v) <= MAX_MESSAGE
}

fn keep_tail(data: &mut Value, key: &str, keep: usize) -> usize {
    let Some(list) = data.get_mut(key).and_then(Value::as_array_mut) else { return 0 };
    let cut = list.len().saturating_sub(keep);
    list.drain(..cut);
    cut
}

fn finished(agent: &Value) -> bool {
    matches!(agent.get("status").and_then(Value::as_str), Some("completed" | "error" | "cancelled"))
}

fn is_main(agent: &Value) -> bool {
    agent.get("role").and_then(Value::as_str) == Some("main")
}

fn age(agent: &Value) -> &str {
    ["completedAt", "updatedAt", "startedAt"].iter().find_map(|k| agent.get(*k).and_then(Value::as_str)).unwrap_or("")
}

fn drop_agents(data: &mut Value, budget: usize, finished_only: bool) -> usize {
    let mut total = size(data);
    let Some(list) = data.get_mut("agents").and_then(Value::as_array_mut) else { return 0 };
    let mut order: Vec<usize> = (0..list.len()).filter(|&i| !is_main(&list[i]) && (!finished_only || finished(&list[i]))).collect();
    order.sort_by(|&a, &b| age(&list[a]).cmp(age(&list[b])));
    let mut gone = vec![false; list.len()];
    let mut dropped = 0;
    for i in order {
        if total <= budget {
            break;
        }
        total = total.saturating_sub(size(&list[i]) + 1);
        gone[i] = true;
        dropped += 1;
    }
    let mut at = 0;
    list.retain(|_| {
        let keep = !gone[at];
        at += 1;
        keep
    });
    dropped
}

fn shorten(v: &mut Value, max: usize) -> bool {
    match v {
        Value::String(s) if s.chars().count() > max => {
            let cut: String = s.chars().take(max).collect();
            *s = format!("{cut}…");
            true
        }
        Value::Array(items) => items.iter_mut().fold(false, |acc, i| shorten(i, max) | acc),
        Value::Object(map) => map.values_mut().fold(false, |acc, i| shorten(i, max) | acc),
        _ => false,
    }
}

pub fn fit_data(data: &mut Value, budget: usize) -> Fit {
    if size(data) <= budget {
        return Fit::Whole;
    }
    let mut t = Trimmed { activity: keep_tail(data, "activity", ACTIVITY_KEEP), ..Trimmed::default() };
    let steps: [fn(&mut Value, usize, &mut Trimmed); 4] = [
        |d, b, t| t.agents += drop_agents(d, b, true),
        |d, _, t| t.activity += keep_tail(d, "activity", 0),
        |d, _, t| t.text = shorten(d, TEXT_MAX),
        |d, b, t| t.agents += drop_agents(d, b, false),
    ];
    for step in steps {
        if size(data) <= budget {
            break;
        }
        step(data, budget, &mut t);
    }
    match size(data) {
        n if n <= budget && t.any() => Fit::Trimmed(t),
        n if n <= budget => Fit::Whole,
        n => Fit::Failed(n),
    }
}

pub fn fit_snapshot(event: &mut Value, budget: usize) -> Fit {
    let before = event.get("data").map_or(0, size);
    let Some(data) = event.get_mut("data") else { return Fit::Whole };
    let fit = fit_data(data, budget);
    match &fit {
        Fit::Trimmed(t) => {
            event["trimmed"] = t.json();
            eprintln!("[remote] snapshot {before} B over {budget} B, trimmed: {} activity, {} agents, text {}", t.activity, t.agents, t.text);
        }
        Fit::Failed(n) => eprintln!("[remote] snapshot still {n} B after trimming (limit {budget} B), sent limit event"),
        Fit::Whole => {}
    }
    fit
}

pub fn limit_event(what: &str, bytes: usize) -> Value {
    json!({ "t": "limit", "what": what, "bytes": bytes, "max": MAX_MESSAGE })
}

pub fn oversized(value: &Value, bytes: usize) -> Value {
    let what = value.get("t").and_then(Value::as_str).unwrap_or("reply").to_owned();
    eprintln!("[remote] message {what} is {bytes} B, over {MAX_MESSAGE} B, not sent");
    match value.get("re") {
        Some(re) if value.get("t").is_none() => json!({ "re": re, "ok": false, "error": "limit" }),
        _ => limit_event(&what, bytes),
    }
}
