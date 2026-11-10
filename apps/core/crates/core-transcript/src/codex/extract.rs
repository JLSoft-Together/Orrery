use super::value::J;
use crate::jsstr::{iso_of_ms, js_slice, num, put, put_num, put_opt};
use crate::time::parse_iso;
use crate::toolsum::{codex_tool_summary_of, host_home, host_platform};
use serde_json::{Map, Value};

pub const MAX_ENUM: usize = 80;
const MAX_CWD: usize = 1024;
const DAY_MINUTES: f64 = 24.0 * 60.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Usage {
    pub input: f64,
    pub cached: f64,
    pub output: f64,
    pub total: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Meta {
    pub id: String,
    pub created_at: i64,
    pub cwd: Option<String>,
    pub originator: Option<String>,
    pub source: Option<String>,
    pub parent_thread_id: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Window {
    pub used_pct: f64,
    pub resets_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Limits {
    pub five_hour: Option<Window>,
    pub seven_day: Option<Window>,
    pub plan: Option<String>,
    pub at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CodexSignal {
    Meta(Meta),
    Context { model: Option<String>, effort: Option<String> },
    TurnStart { at: Option<String>, context_window: Option<f64> },
    TurnEnd { at: Option<String>, aborted: bool },
    ToolCall { id: String, name: String, summary: Option<String>, at: Option<String> },
    ToolOutput { id: String },
    Tokens { total: Usage, last: Option<Usage>, context_window: Option<f64> },
    Limits(Limits),
    Compact,
    Seen { at: String },
}

fn s(v: Option<&J>) -> Option<&str> {
    v.and_then(J::as_str).filter(|x| !x.is_empty())
}

fn int(v: Option<&J>) -> f64 {
    match v.and_then(J::as_num) {
        Some(n) if n.is_finite() && n >= 0.0 => n.round(),
        _ => 0.0,
    }
}

fn nonzero(n: f64) -> Option<f64> {
    (n != 0.0).then_some(n)
}

fn is_enum_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b':' | b'/' | b'@' | b'+' | b'-')
}

fn enum_of(v: Option<&J>) -> Option<String> {
    s(v).filter(|x| x.len() <= MAX_ENUM && x.bytes().all(is_enum_char)).map(str::to_owned)
}

fn obj(v: Option<&J>) -> Option<&J> {
    v.filter(|x| x.is_obj())
}

fn usage_of(v: Option<&J>) -> Option<Usage> {
    let v = obj(v)?;
    let input = int(v.get("input_tokens"));
    let cached = int(v.get("cached_input_tokens")).min(input);
    let output = int(v.get("output_tokens"));
    let total = nonzero(int(v.get("total_tokens"))).unwrap_or(input + output);
    Some(Usage { input, cached, output, total })
}

fn find_parent_thread(v: &J, depth: u32) -> Option<String> {
    if !v.is_obj() || depth > 4 {
        return None;
    }
    if let Some(direct) = enum_of(v.get("parent_thread_id")) {
        return Some(direct);
    }
    v.values_js_order().into_iter().find_map(|c| find_parent_thread(c, depth + 1))
}

pub fn meta_of(payload: Option<&J>) -> Option<Meta> {
    let p = obj(payload)?;
    let id = enum_of(p.get("id")).or_else(|| enum_of(p.get("session_id")))?;
    let created_at = parse_iso(s(p.get("timestamp"))?)?;
    let source = p.get("source");
    Some(Meta {
        id,
        created_at,
        cwd: s(p.get("cwd")).map(|x| js_slice(x, MAX_CWD).to_owned()),
        originator: s(p.get("originator")).map(|x| js_slice(x, MAX_ENUM).to_owned()),
        source: match source {
            Some(J::Str(x)) => Some(js_slice(x, MAX_ENUM).to_owned()),
            Some(J::Obj(_)) => Some("subagent".to_owned()),
            _ => None,
        },
        parent_thread_id: source.and_then(|x| find_parent_thread(x, 0)),
        version: enum_of(p.get("cli_version")),
    })
}

fn window_of(v: Option<&J>) -> Option<(f64, Window)> {
    let v = obj(v)?;
    let used = v.get("used_percent").and_then(J::as_num).filter(|n| n.is_finite())?;
    let resets_at = v.get("resets_at").and_then(J::as_num).filter(|n| *n > 0.0).and_then(|n| iso_of_ms(n * 1000.0));
    Some((int(v.get("window_minutes")), Window { used_pct: used.clamp(0.0, 100.0), resets_at }))
}

fn limits_of(v: Option<&J>, at: Option<&str>) -> Option<Limits> {
    let v = obj(v)?;
    let at = at?;
    match v.get("limit_id") {
        None | Some(J::Null) => {}
        Some(J::Str(x)) if x == "codex" => {}
        Some(_) => return None,
    }
    let mut out = Limits { five_hour: None, seven_day: None, plan: enum_of(v.get("plan_type")), at: at.to_owned() };
    for (minutes, w) in [window_of(v.get("primary")), window_of(v.get("secondary"))].into_iter().flatten() {
        if minutes > 0.0 && minutes <= DAY_MINUTES {
            out.five_hour = Some(w);
        } else if minutes > DAY_MINUTES {
            out.seven_day = Some(w);
        }
    }
    (out.five_hour.is_some() || out.seven_day.is_some()).then_some(out)
}

fn tool_call(p: &J, item_type: &str, id: String, at: Option<String>) -> Option<CodexSignal> {
    let (name, bare, namespaced) = match (enum_of(p.get("namespace")), enum_of(p.get("name"))) {
        (Some(ns), Some(n)) => (format!("{ns}.{n}"), n, true),
        (_, n) => {
            let n = n?;
            (n.clone(), n, false)
        }
    };
    let summary = codex_tool_summary_of(item_type, &bare, namespaced, p, host_home(), host_platform());
    Some(CodexSignal::ToolCall { id, name: js_slice(&name, MAX_ENUM).to_owned(), summary, at })
}

pub fn compacted_signals(at: Option<String>) -> Vec<CodexSignal> {
    let mut out: Vec<CodexSignal> = at.filter(|a| !a.is_empty()).map(|at| CodexSignal::Seen { at }).into_iter().collect();
    out.push(CodexSignal::Compact);
    out
}

pub fn extract_codex_signals(rec: &J) -> Vec<CodexSignal> {
    let Some(p) = obj(rec.get("payload")) else { return Vec::new() };
    let at = s(rec.get("timestamp")).map(str::to_owned);
    let mut out = Vec::new();
    if let Some(at) = &at {
        out.push(CodexSignal::Seen { at: at.clone() });
    }
    let kind = rec.get("type").and_then(J::as_str);
    let sub = p.get("type").and_then(J::as_str);
    match kind {
        Some("session_meta") => out.extend(meta_of(Some(p)).map(CodexSignal::Meta)),
        Some("turn_context") => {
            let settings = obj(p.get("collaboration_mode")).and_then(|c| obj(c.get("settings")));
            let from = |own: &str, nested: &str| enum_of(p.get(own)).or_else(|| enum_of(settings.and_then(|x| x.get(nested))));
            out.push(CodexSignal::Context { model: from("model", "model"), effort: from("effort", "reasoning_effort") });
        }
        Some("compacted") => out.push(CodexSignal::Compact),
        Some("event_msg") => match sub {
            Some("task_started") => out.push(CodexSignal::TurnStart { at, context_window: nonzero(int(p.get("model_context_window"))) }),
            Some("task_complete") => out.push(CodexSignal::TurnEnd { at, aborted: false }),
            Some("turn_aborted") => out.push(CodexSignal::TurnEnd { at, aborted: true }),
            Some("context_compacted") => out.push(CodexSignal::Compact),
            Some("token_count") => {
                let info = obj(p.get("info"));
                if let Some(total) = usage_of(info.and_then(|i| i.get("total_token_usage"))) {
                    let last = usage_of(info.and_then(|i| i.get("last_token_usage")));
                    let context_window = nonzero(int(info.and_then(|i| i.get("model_context_window"))));
                    out.push(CodexSignal::Tokens { total, last, context_window });
                }
                out.extend(limits_of(p.get("rate_limits"), at.as_deref()).map(CodexSignal::Limits));
            }
            _ => {}
        },
        Some("response_item") => {
            let Some(id) = s(p.get("call_id")).map(str::to_owned) else { return out };
            match sub {
                Some("function_call_output" | "custom_tool_call_output") => out.push(CodexSignal::ToolOutput { id }),
                Some("local_shell_call") => out.push(CodexSignal::ToolCall { id, name: "shell".to_owned(), summary: None, at }),
                Some(t @ ("function_call" | "custom_tool_call")) => out.extend(tool_call(p, t, id, at)),
                _ => {}
            }
        }
        _ => {}
    }
    out
}

fn usage_json(u: &Usage) -> Value {
    let mut m = Map::new();
    put(&mut m, "input", num(u.input));
    put(&mut m, "cached", num(u.cached));
    put(&mut m, "output", num(u.output));
    put(&mut m, "total", num(u.total));
    Value::Object(m)
}

fn window_json(w: &Window) -> Value {
    let mut m = Map::new();
    put(&mut m, "usedPct", num(w.used_pct));
    put_opt(&mut m, "resetsAt", &w.resets_at);
    Value::Object(m)
}

impl Meta {
    pub fn to_json(&self) -> Value {
        let mut m = Map::new();
        put(&mut m, "id", self.id.as_str());
        put(&mut m, "createdAt", self.created_at);
        put_opt(&mut m, "cwd", &self.cwd);
        put_opt(&mut m, "originator", &self.originator);
        put_opt(&mut m, "source", &self.source);
        put_opt(&mut m, "parentThreadId", &self.parent_thread_id);
        put_opt(&mut m, "version", &self.version);
        Value::Object(m)
    }
}

impl CodexSignal {
    pub fn kind(&self) -> &'static str {
        match self {
            CodexSignal::Meta(_) => "meta",
            CodexSignal::Context { .. } => "context",
            CodexSignal::TurnStart { .. } => "turn_start",
            CodexSignal::TurnEnd { .. } => "turn_end",
            CodexSignal::ToolCall { .. } => "tool_call",
            CodexSignal::ToolOutput { .. } => "tool_output",
            CodexSignal::Tokens { .. } => "tokens",
            CodexSignal::Limits(_) => "limits",
            CodexSignal::Compact => "compact",
            CodexSignal::Seen { .. } => "seen",
        }
    }

    pub fn to_json(&self) -> Value {
        let mut m = Map::new();
        put(&mut m, "k", self.kind());
        match self {
            CodexSignal::Meta(meta) => put(&mut m, "meta", meta.to_json()),
            CodexSignal::Context { model, effort } => {
                put_opt(&mut m, "model", model);
                put_opt(&mut m, "effort", effort);
            }
            CodexSignal::TurnStart { at, context_window } => {
                put_opt(&mut m, "at", at);
                put_num(&mut m, "contextWindow", *context_window);
            }
            CodexSignal::TurnEnd { at, aborted } => {
                put_opt(&mut m, "at", at);
                put(&mut m, "aborted", *aborted);
            }
            CodexSignal::ToolCall { id, name, summary, at } => {
                put(&mut m, "id", id.as_str());
                put(&mut m, "name", name.as_str());
                put_opt(&mut m, "summary", summary);
                put_opt(&mut m, "at", at);
            }
            CodexSignal::ToolOutput { id } => put(&mut m, "id", id.as_str()),
            CodexSignal::Tokens { total, last, context_window } => {
                put(&mut m, "total", usage_json(total));
                if let Some(last) = last {
                    put(&mut m, "last", usage_json(last));
                }
                put_num(&mut m, "contextWindow", *context_window);
            }
            CodexSignal::Limits(l) => {
                let mut x = Map::new();
                if let Some(w) = &l.five_hour {
                    put(&mut x, "fiveHour", window_json(w));
                }
                if let Some(w) = &l.seven_day {
                    put(&mut x, "sevenDay", window_json(w));
                }
                put_opt(&mut x, "plan", &l.plan);
                put(&mut x, "at", l.at.as_str());
                put(&mut m, "limits", Value::Object(x));
            }
            CodexSignal::Compact => {}
            CodexSignal::Seen { at } => put(&mut m, "at", at.as_str()),
        }
        Value::Object(m)
    }
}
