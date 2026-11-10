use crate::claude::{norm_file, Obj};
use crate::js;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

pub const HOOK_MARK: &str = "hook-bridge";

pub const HOOK_EVENT_ORDER: [&str; 21] = [
    "Setup",
    "SessionStart",
    "UserPromptSubmit",
    "UserPromptExpansion",
    "PreToolUse",
    "PermissionRequest",
    "PermissionDenied",
    "PostToolUse",
    "PostToolUseFailure",
    "PostToolBatch",
    "Notification",
    "SubagentStart",
    "SubagentStop",
    "Stop",
    "StopFailure",
    "PreCompact",
    "PostCompact",
    "PreModelSwitch",
    "PostModelSwitch",
    "TeammateIdle",
    "SessionEnd",
];

pub fn event_rank(event: &str) -> usize {
    HOOK_EVENT_ORDER.iter().position(|e| *e == event).unwrap_or(HOOK_EVENT_ORDER.len())
}

#[derive(Debug, Clone, PartialEq)]
pub struct FlatHook {
    pub event: String,
    pub matcher: Option<String>,
    pub hook: Obj,
}

fn norm_matcher(m: Option<&Value>) -> Option<String> {
    m.and_then(Value::as_str).filter(|s| !s.is_empty()).map(str::to_string)
}

pub fn flatten_hooks(hooks: Option<&Value>) -> Vec<FlatHook> {
    let Some(Value::Object(hooks)) = hooks else { return Vec::new() };
    let mut out = Vec::new();
    for (event, groups) in js::entries(hooks) {
        let Some(groups) = groups.as_array() else { continue };
        for g in groups {
            let Some(list) = g.as_object().and_then(|g| g.get("hooks")).and_then(Value::as_array) else { continue };
            for h in list {
                if let Some(h) = h.as_object() {
                    out.push(FlatHook { event: event.clone(), matcher: norm_matcher(g.get("matcher")), hook: h.clone() });
                }
            }
        }
    }
    out
}

pub fn canonical(v: &Value) -> String {
    match v {
        Value::Array(a) => format!("[{}]", a.iter().map(canonical).collect::<Vec<_>>().join(",")),
        Value::Object(m) => {
            let mut keys: Vec<&String> = m.keys().collect();
            keys.sort_by(|a, b| core_transcript::jsstr::utf16_cmp(a, b));
            let parts: Vec<String> = keys.iter().map(|k| format!("{}:{}", js::stringify(&Value::String((*k).clone())), canonical(&m[*k]))).collect();
            format!("{{{}}}", parts.join(","))
        }
        other => js::stringify(other),
    }
}

pub fn hook_key(file: &str, flat: &FlatHook, win: bool) -> String {
    let raw = [norm_file(file, win), flat.event.clone(), flat.matcher.clone().unwrap_or_default(), canonical(&Value::Object(flat.hook.clone()))].join("\0");
    let digest = Sha256::digest(raw.as_bytes());
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    hex[..24].to_string()
}

pub fn is_orrery_hook(hook: &Obj) -> bool {
    hook.get("command").and_then(Value::as_str).is_some_and(|c| c.contains(HOOK_MARK))
}

fn same_entry(a: &Value, b: &Obj) -> bool {
    canonical(a) == canonical(&Value::Object(b.clone()))
}

pub fn remove_hook_entry(settings: &Obj, flat: &FlatHook) -> Option<Obj> {
    let hooks = settings.get("hooks")?.as_object()?;
    let groups = hooks.get(&flat.event)?.as_array()?;
    for (gi, g) in groups.iter().enumerate() {
        let Some(go) = g.as_object() else { continue };
        let Some(list) = go.get("hooks").and_then(Value::as_array) else { continue };
        if norm_matcher(go.get("matcher")) != flat.matcher {
            continue;
        }
        let Some(hi) = list.iter().position(|h| same_entry(h, &flat.hook)) else { continue };
        let left: Vec<Value> = list.iter().enumerate().filter(|(i, _)| *i != hi).map(|(_, h)| h.clone()).collect();
        let next_groups: Vec<Value> = if left.is_empty() {
            groups.iter().enumerate().filter(|(i, _)| *i != gi).map(|(_, x)| x.clone()).collect()
        } else {
            groups
                .iter()
                .enumerate()
                .map(|(i, x)| {
                    if i == gi {
                        let mut ng = go.clone();
                        ng.insert("hooks".into(), Value::Array(left.clone()));
                        Value::Object(ng)
                    } else {
                        x.clone()
                    }
                })
                .collect()
        };
        let mut hooks = hooks.clone();
        if next_groups.is_empty() {
            hooks.shift_remove(&flat.event);
        } else {
            hooks.insert(flat.event.clone(), Value::Array(next_groups));
        }
        let mut next = settings.clone();
        let empty = hooks.is_empty();
        next.insert("hooks".into(), Value::Object(hooks));
        if empty {
            next.shift_remove("hooks");
        }
        return Some(next);
    }
    None
}

pub fn has_hook_entry(settings: &Obj, flat: &FlatHook) -> bool {
    flatten_hooks(settings.get("hooks")).iter().any(|f| f.event == flat.event && f.matcher == flat.matcher && canonical(&Value::Object(f.hook.clone())) == canonical(&Value::Object(flat.hook.clone())))
}

pub fn restore_hook_entry(settings: &Obj, flat: &FlatHook) -> Option<Obj> {
    if has_hook_entry(settings, flat) {
        return None;
    }
    let mut hooks: Obj = settings.get("hooks").and_then(Value::as_object).cloned().unwrap_or_default();
    let groups: Vec<Value> = hooks.get(&flat.event).and_then(Value::as_array).cloned().unwrap_or_default();
    let gi = groups.iter().position(|g| g.as_object().is_some_and(|g| g.get("hooks").is_some_and(Value::is_array) && norm_matcher(g.get("matcher")) == flat.matcher));
    let next_groups = match gi {
        Some(gi) => groups
            .iter()
            .enumerate()
            .map(|(i, x)| {
                if i != gi {
                    return x.clone();
                }
                let mut g = x.as_object().cloned().unwrap_or_default();
                let mut list = g.get("hooks").and_then(Value::as_array).cloned().unwrap_or_default();
                list.push(Value::Object(flat.hook.clone()));
                g.insert("hooks".into(), Value::Array(list));
                Value::Object(g)
            })
            .collect(),
        None => {
            let mut v = groups.clone();
            v.push(match &flat.matcher {
                None => json!({"hooks": [flat.hook]}),
                Some(m) => json!({"matcher": m, "hooks": [flat.hook]}),
            });
            v
        }
    };
    hooks.insert(flat.event.clone(), Value::Array(next_groups));
    let mut next = settings.clone();
    next.insert("hooks".into(), Value::Object(hooks));
    Some(next)
}

pub fn hook_text(hook: &Obj) -> String {
    let s = |k: &str| hook.get(k).and_then(Value::as_str);
    if let Some(c) = s("command") {
        return match hook.get("args").and_then(Value::as_array) {
            Some(args) => std::iter::once(c).chain(args.iter().filter_map(Value::as_str)).collect::<Vec<_>>().join(" "),
            None => c.to_string(),
        };
    }
    s("url").or_else(|| s("tool")).or_else(|| s("prompt")).unwrap_or("").to_string()
}

pub fn clip_output(text: &str, max: usize) -> (String, bool) {
    if core_transcript::jsstr::units(text) > max {
        (format!("…{}", crate::claude::tail(text, max)), true)
    } else {
        (text.to_string(), false)
    }
}

pub fn obj(pairs: Vec<(&str, Value)>) -> Value {
    let mut m = Map::new();
    for (k, v) in pairs {
        m.insert(k.to_string(), v);
    }
    Value::Object(m)
}
