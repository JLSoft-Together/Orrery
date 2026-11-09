use crate::conflict::port_conflict_in;
use crate::jsstr::{iso_of_ms, js_slice, js_trim, word};
use crate::origin::{ask_of, effort_ok, origin_signals, peer_sent, record_id, task_notifications, window_ok};
use crate::rules::{touch_of, SHELL_TOOLS};
use crate::signal::Signal;
use serde_json::{Map, Value};

pub const MAX_ENUM: usize = 120;
pub const REPLY_MAX: usize = 3000;

type Obj = Map<String, Value>;

fn obj(v: Option<&Value>) -> Option<&Obj> {
    v.and_then(Value::as_object)
}

fn s(v: Option<&Value>) -> Option<&str> {
    v.and_then(Value::as_str)
}

fn int(v: Option<&Value>) -> f64 {
    v.and_then(Value::as_f64).filter(|n| n.is_finite() && *n >= 0.0).unwrap_or(0.0)
}

fn word_runs(s: &str) -> Vec<(usize, &str)> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in s.char_indices() {
        match (word(c), start) {
            (true, None) => start = Some(i),
            (false, Some(st)) => {
                out.push((st, &s[st..i]));
                start = None;
            }
            _ => {}
        }
    }
    if let Some(st) = start {
        out.push((st, &s[st..]));
    }
    out
}

fn shell_command<'a>(name: &str, input: &'a Value) -> Option<&'a str> {
    if !SHELL_TOOLS.contains(&name) {
        return None;
    }
    input.as_object()?.get("command")?.as_str()
}

pub fn mentions_codex(cmd: &str) -> bool {
    word_runs(cmd).iter().any(|(_, w)| w.eq_ignore_ascii_case("codex"))
}

pub fn mentions_device(cmd: &str) -> bool {
    let runs = word_runs(cmd);
    for (k, (at, w)) in runs.iter().enumerate() {
        if *w == "adb" || (w.len() >= 12 && w.starts_with("install") && w.ends_with("Debug")) {
            return true;
        }
        if *w == "flutter" {
            let gap_start = at + w.len();
            if let Some((next_at, next)) = runs.get(k + 1) {
                let gap = &cmd[gap_start..*next_at];
                if *next == "run" && !gap.is_empty() && gap.chars().all(crate::jsstr::js_space) {
                    return true;
                }
            }
        }
    }
    false
}

fn texts_of(content: Option<&Value>) -> Vec<&str> {
    match content {
        Some(Value::String(t)) => vec![t.as_str()],
        Some(Value::Array(a)) => a.iter().filter_map(|b| obj(Some(b)).filter(|b| s(b.get("type")) == Some("text")).and_then(|b| s(b.get("text")))).collect(),
        _ => Vec::new(),
    }
}

fn limit_hit_of(rec: &Obj, at: &Option<String>) -> Option<Signal> {
    if rec.get("isApiErrorMessage") != Some(&Value::Bool(true)) || s(rec.get("apiError")) != Some("usage_limit_reached") {
        return None;
    }
    let q = obj(rec.get("quotaLimits"));
    let secs = q.and_then(|q| q.get("resetsAt")).and_then(Value::as_f64).filter(|v| v.is_finite() && *v > 0.0);
    let window = s(q.and_then(|q| q.get("rateLimitType"))).filter(|w| window_ok(w)).map(str::to_owned);
    Some(Signal::LimitHit { resets_at: secs.and_then(|v| iso_of_ms(v * 1000.0)), window, at: at.clone() })
}

const RESUME_NOTICES: [(&str, &str); 4] = [
    ("Usage limit reached · continuing automatically", "armed"),
    ("Usage limit reached again · continuing automatically", "armed"),
    ("Automatic continue cancelled", "cancelled"),
    ("Usage limit reset · continuing automatically", "reset"),
];

fn assistant(rec: &Obj, msg: &Obj, at: &Option<String>, out: &mut Vec<Signal>) {
    if let Some(a) = at.as_ref().filter(|a| !a.is_empty()) {
        out.push(Signal::Seen { at: a.clone() });
    }
    let model = s(msg.get("model"));
    let effort = s(rec.get("effort")).or_else(|| s(rec.get("perTurnEffort")));
    if let Some(m) = model.filter(|m| !m.is_empty() && *m != "<synthetic>") {
        out.push(Signal::Model { model: m.to_owned(), effort: effort.filter(|e| effort_ok(e)).map(str::to_owned) });
    }
    let id = s(msg.get("id")).filter(|i| !i.is_empty());
    if let (Some(id), Some(u)) = (id, obj(msg.get("usage"))) {
        let creation = obj(u.get("cache_creation"));
        let tier = |k: &str| int(creation.and_then(|c| c.get(k))) > 0.0;
        let ttl = if tier("ephemeral_1h_input_tokens") {
            Some("1h")
        } else if tier("ephemeral_5m_input_tokens") {
            Some("5m")
        } else {
            None
        };
        out.push(Signal::Usage {
            message_id: id.to_owned(),
            input: int(u.get("input_tokens")) + int(u.get("cache_creation_input_tokens")),
            output: int(u.get("output_tokens")),
            cache_read: int(u.get("cache_read_input_tokens")),
            ttl,
            at: at.clone(),
        });
    }
    if let Some(blocks) = msg.get("content").and_then(Value::as_array) {
        for b in blocks.iter().filter_map(Value::as_object) {
            let (Some("tool_use"), Some(bid), Some(name)) = (s(b.get("type")), s(b.get("id")), s(b.get("name"))) else { continue };
            let input = b.get("input").unwrap_or(&Value::Null);
            let cmd = shell_command(name, input);
            out.push(Signal::ToolUse {
                id: bid.to_owned(),
                name: name.to_owned(),
                at: at.clone(),
                codex: cmd.is_some_and(mentions_codex),
                device: cmd.is_some_and(mentions_device),
                touch: touch_of(name, input),
                ask: ask_of(name, input),
            });
            if name == "SendMessage" && input.as_object().and_then(|i| s(i.get("to"))) == Some("main") {
                out.push(Signal::SendMain { key: bid.to_owned(), at: at.clone() });
            }
        }
    }
    let said = texts_of(msg.get("content")).join("\n");
    let said = js_trim(&said);
    if let (Some(id), false) = (id, said.is_empty()) {
        out.push(Signal::ReplyText { message_id: id.to_owned(), text: js_slice(said, REPLY_MAX).to_owned() });
    }
    if let Some(l) = limit_hit_of(rec, at) {
        out.push(l);
    }
    if s(msg.get("stop_reason")) == Some("end_turn") {
        out.push(Signal::AssistantEnd { message_id: id.map(str::to_owned), at: at.clone() });
    }
}

fn user(rec: &Obj, msg: &Obj, at: &Option<String>, out: &mut Vec<Signal>) {
    if let Some(a) = at.as_ref().filter(|a| !a.is_empty()) {
        out.push(Signal::Seen { at: a.clone() });
    }
    let content = msg.get("content");
    let tur = obj(rec.get("toolUseResult"));
    let async_launched = tur.is_some_and(|t| s(t.get("status")) == Some("async_launched"));
    let blocks = content.and_then(Value::as_array);
    let has_result = blocks.is_some_and(|a| a.iter().any(|b| obj(Some(b)).is_some_and(|b| s(b.get("type")) == Some("tool_result"))));
    if rec.get("isMeta") != Some(&Value::Bool(true)) && !has_result {
        out.push(Signal::Prompt { at: at.clone() });
    }
    for b in blocks.into_iter().flatten().filter_map(Value::as_object) {
        let (Some("tool_result"), Some(tid)) = (s(b.get("type")), s(b.get("tool_use_id"))) else { continue };
        let is_error = b.get("is_error") == Some(&Value::Bool(true));
        let port_conflict = if is_error { port_conflict_in(&texts_of(b.get("content")).join("\n")) } else { None };
        out.push(Signal::ToolResult { tool_use_id: tid.to_owned(), is_error, async_launched, at: at.clone(), port_conflict });
    }
    for t in texts_of(content) {
        out.extend(task_notifications(t, at.as_ref()));
    }
    out.extend(origin_signals(rec.get("origin"), record_id(rec.get("uuid")), at.as_ref()));
    out.extend(peer_sent(rec.get("toolUseResult"), at.as_ref()));
}

pub fn extract_signals(rec: &Value) -> Vec<Signal> {
    let Some(rec) = rec.as_object() else { return Vec::new() };
    let ty = s(rec.get("type"));
    let at = s(rec.get("timestamp")).map(str::to_owned);
    let mut out = Vec::new();
    if let Some(b) = s(rec.get("gitBranch")).filter(|b| !b.is_empty()) {
        out.push(Signal::Branch { branch: js_slice(b, MAX_ENUM).to_owned() });
    }
    match ty {
        Some("permission-mode") => {
            if let Some(mode) = s(rec.get("permissionMode")) {
                out.push(Signal::PermissionMode { mode: js_slice(mode, MAX_ENUM).to_owned() });
            }
        }
        Some("assistant") => {
            if let Some(msg) = obj(rec.get("message")) {
                assistant(rec, msg, &at, &mut out);
            }
        }
        Some("user") => {
            if let Some(msg) = obj(rec.get("message")) {
                user(rec, msg, &at, &mut out);
            }
        }
        Some("attachment") => {
            if let Some(a) = obj(rec.get("attachment")).filter(|a| s(a.get("type")) == Some("queued_command")) {
                let id = record_id(a.get("source_uuid")).or_else(|| record_id(rec.get("uuid")));
                let when = at.clone().or_else(|| s(a.get("timestamp")).map(str::to_owned));
                out.extend(origin_signals(a.get("origin"), id, when.as_ref()));
            }
        }
        Some("queue-operation") => {
            if let Some(c) = s(rec.get("content")) {
                return task_notifications(c, at.as_ref());
            }
        }
        Some("system") => match s(rec.get("subtype")) {
            Some("turn_duration") => out.push(Signal::TurnEnd { at }),
            Some("informational") => {
                if let Some(c) = s(rec.get("content")) {
                    if let Some((_, state)) = RESUME_NOTICES.iter().find(|(p, _)| c.starts_with(p)) {
                        out.push(Signal::LimitResume { state });
                    }
                }
            }
            Some("compact_boundary") => {
                let meta = obj(rec.get("compactMetadata"));
                let tokens = |k: &str| meta.and_then(|m| m.get(k)).filter(|v| v.is_number()).map(|v| int(Some(v)));
                out.push(Signal::Compact {
                    trigger: s(meta.and_then(|m| m.get("trigger"))).map(|t| js_slice(t, MAX_ENUM).to_owned()),
                    pre_tokens: tokens("preTokens"),
                    post_tokens: tokens("postTokens"),
                    at,
                });
            }
            _ => {}
        },
        _ => {}
    }
    out
}
