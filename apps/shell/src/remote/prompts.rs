use serde_json::{json, Map, Value};

use super::super::ctx::Host;

pub const LIST: &str = "ccm:remote-prompts-list";
pub const SAVE: &str = "ccm:remote-prompts-save";
pub const DELETE: &str = "ccm:remote-prompts-delete";
pub const EVENT: &str = "prompts";
pub const BUDGET: usize = 700_000;
const GET: &str = "ccm:prompts-get";
const SET: &str = "ccm:prompts-set";
const MAX_KEY: usize = 128;
const MAX_ID: usize = 64;
const MAX_FIELD: usize = 16_000;
const MAX_SCOPE: usize = 4096;
const NAMES: [&str; 3] = ["branch", "project", "cwd"];

fn fail(code: &str) -> Value {
    json!({ "raw": { "ok": false, "error": code } })
}

fn id_ok(s: &str) -> bool {
    !s.is_empty() && s.len() <= MAX_ID && s.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn text<'a>(cmd: &'a Value, key: &str, max: usize) -> Option<&'a str> {
    cmd.get(key).and_then(Value::as_str).filter(|s| s.len() <= max)
}

fn opt_id(cmd: &Value) -> Option<Option<&str>> {
    match cmd.get("promptId") {
        None | Some(Value::Null) => Some(None),
        Some(Value::String(s)) if id_ok(s) => Some(Some(s)),
        Some(_) => None,
    }
}

pub fn args(t: &str, cmd: &Value) -> Option<Value> {
    Some(match t {
        "prompts.list" => match cmd.get("paneKey") {
            None | Some(Value::Null) => json!([{}]),
            Some(Value::String(k)) if !k.is_empty() && k.len() <= MAX_KEY => json!([{ "paneKey": k }]),
            Some(_) => return None,
        },
        "prompts.save" => {
            let mut req = json!({ "title": text(cmd, "title", MAX_FIELD)?, "body": text(cmd, "body", MAX_FIELD)? });
            req["scope"] = match cmd.get("scope") {
                None | Some(Value::Null) => json!("all"),
                Some(Value::String(s)) if s.len() <= MAX_SCOPE => json!(s),
                Some(_) => return None,
            };
            if let Some(id) = opt_id(cmd)? {
                req["id"] = json!(id);
            }
            json!([req])
        }
        "prompts.delete" => json!([{ "id": cmd.get("promptId").and_then(Value::as_str).filter(|i| id_ok(i))? }]),
        _ => return None,
    })
}

pub fn looks_secret(s: &str) -> bool {
    core_desk::claude::mask::mask_hook_text(s, "", "linux") != s
}

fn hidden(item: &Value) -> bool {
    let field = |k: &str| item.get(k).and_then(Value::as_str).unwrap_or("");
    looks_secret(field("title")) || looks_secret(field("body"))
}

fn fetch(host: &dyn Host, owner: &str) -> Result<Vec<Value>, Value> {
    let env = host.viewer(owner, GET, json!([]), &[]);
    let Some(raw) = env.get("raw") else { return Err(env) };
    if raw.get("ok") != Some(&Value::Bool(true)) {
        return Err(env);
    }
    match raw.get("items").and_then(Value::as_array) {
        Some(items) => Ok(items.clone()),
        None => Err(fail("io")),
    }
}

#[derive(Default)]
pub struct Vars {
    branch: String,
    project: String,
    cwd: String,
}

impl Vars {
    fn get(&self, name: &str) -> &str {
        match name {
            "branch" => &self.branch,
            "project" => &self.project,
            _ => &self.cwd,
        }
    }
}

pub struct PaneCtx {
    pub vars: Vars,
    pub dir: String,
}

fn base_name(path: &str) -> &str {
    path.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().unwrap_or("")
}

pub fn pane_ctx(summary: &Value, key: &str) -> Option<PaneCtx> {
    let panes = summary["tabs"].as_array()?.iter().flat_map(|t| t["panes"].as_array().into_iter().flatten());
    let pane = panes.into_iter().find(|p| p["key"].as_str() == Some(key))?;
    let cwd = pane["cwd"].as_str().unwrap_or("").to_owned();
    let branch = pane["branch"].as_str().unwrap_or("").to_owned();
    Some(PaneCtx { vars: Vars { project: base_name(&cwd).to_owned(), branch, cwd: cwd.clone() }, dir: cwd })
}

pub fn expand(body: &str, vars: &Vars) -> (String, Vec<&'static str>) {
    let mut out = String::with_capacity(body.len());
    let mut missing: Vec<&'static str> = Vec::new();
    let mut rest = body;
    while let Some(at) = rest.find('{') {
        out.push_str(&rest[..at]);
        let tail = &rest[at..];
        let hit = NAMES.iter().copied().find(|n| tail[1..].starts_with(n) && tail[1 + n.len()..].starts_with('}'));
        match hit {
            Some(name) => {
                let whole = name.len() + 2;
                let value = vars.get(name);
                if value.is_empty() {
                    out.push_str(&tail[..whole]);
                    if !missing.contains(&name) {
                        missing.push(name);
                    }
                } else {
                    out.push_str(value);
                }
                rest = &tail[whole..];
            }
            None => {
                out.push('{');
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    (out, missing)
}

fn norm(p: &str) -> String {
    let slashed = p.replace('\\', "/");
    let slashed = slashed.trim_end_matches('/');
    let drive = slashed.len() >= 2 && slashed.as_bytes()[0].is_ascii_alphabetic() && slashed.as_bytes()[1] == b':';
    if drive { slashed.to_lowercase() } else { slashed.to_owned() }
}

pub fn scope_matches(scope: &str, dir: &str) -> bool {
    if scope == "all" {
        return true;
    }
    if dir.is_empty() {
        return false;
    }
    let (a, b) = (norm(scope), norm(dir));
    b == a || b.starts_with(&format!("{a}/"))
}

fn shaped(item: &Value, pane: Option<&PaneCtx>) -> (Value, bool) {
    let mut out = Map::new();
    for k in ["id", "title", "body", "scope", "updatedAt"] {
        if let Some(v) = item.get(k) {
            out.insert(k.into(), v.clone());
        }
    }
    let scope = item.get("scope").and_then(Value::as_str).unwrap_or("all");
    let Some(p) = pane else { return (Value::Object(out), false) };
    let local = scope != "all" && scope_matches(scope, &p.dir);
    let body = item.get("body").and_then(Value::as_str).unwrap_or("");
    let (filled, missing) = expand(body, &p.vars);
    if filled != body && !looks_secret(&filled) {
        out.insert("text".into(), json!(filled));
    }
    if !missing.is_empty() {
        out.insert("missing".into(), json!(missing));
    }
    out.insert("local".into(), json!(local));
    (Value::Object(out), local)
}

pub fn list_reply(items: &[Value], pane: Option<&PaneCtx>, budget: usize) -> Value {
    let mut hidden_count = 0u64;
    let (mut local, mut rest) = (Vec::new(), Vec::new());
    for item in items {
        if hidden(item) {
            hidden_count += 1;
            continue;
        }
        let (shape, is_local) = shaped(item, pane);
        if is_local { local.push(shape) } else { rest.push(shape) }
    }
    let mut kept = Vec::new();
    let (mut bytes, mut more) = (0usize, 0u64);
    for shape in local.into_iter().chain(rest) {
        let cost = serde_json::to_vec(&shape).map_or(budget, |b| b.len() + 1);
        if bytes + cost > budget {
            more += 1;
            continue;
        }
        bytes += cost;
        kept.push(shape);
    }
    let mut out = json!({ "ok": true, "items": kept, "hidden": hidden_count });
    if more > 0 {
        out["more"] = json!(more);
    }
    out
}

pub fn list(host: &dyn Host, owner: &str, summary: Option<&Value>, args: &Value) -> Value {
    let items = match fetch(host, owner) {
        Ok(items) => items,
        Err(env) => return env,
    };
    let key = args.get(0).and_then(|a| a.get("paneKey")).and_then(Value::as_str);
    let pane = key.and_then(|k| summary.and_then(|s| pane_ctx(s, k)));
    json!({ "raw": list_reply(&items, pane.as_ref(), BUDGET) })
}

fn set(host: &dyn Host, owner: &str, req: Value) -> Result<Value, Value> {
    let env = host.viewer(owner, SET, json!([req]), &[]);
    let Some(raw) = env.get("raw") else { return Err(env) };
    if raw.get("ok") == Some(&Value::Bool(true)) { Ok(raw.clone()) } else { Err(json!({ "raw": raw })) }
}

fn editable(items: &[Value], id: &str) -> bool {
    items.iter().any(|i| i.get("id").and_then(Value::as_str) == Some(id) && !hidden(i))
}

fn exists_hidden(items: &[Value], id: &str) -> bool {
    items.iter().any(|i| i.get("id").and_then(Value::as_str) == Some(id) && hidden(i))
}

pub fn save(host: &dyn Host, owner: &str, args: &Value) -> Value {
    if !host.control() {
        return fail("denied");
    }
    let req = args.get(0).cloned().unwrap_or(Value::Null);
    let field = |k: &str| req.get(k).and_then(Value::as_str).unwrap_or("");
    if looks_secret(field("title")) || looks_secret(field("body")) {
        return fail("secret");
    }
    if let Some(id) = req.get("id").and_then(Value::as_str) {
        match fetch(host, owner) {
            Ok(items) if !editable(&items, id) => return fail("not-found"),
            Ok(_) => {}
            Err(env) => return env,
        }
    }
    match set(host, owner, json!({ "op": "upsert", "item": req })) {
        Ok(raw) => json!({ "raw": { "ok": true, "id": raw.get("id") } }),
        Err(env) => env,
    }
}

pub fn delete(host: &dyn Host, owner: &str, args: &Value) -> Value {
    if !host.control() {
        return fail("denied");
    }
    let id = args.get(0).and_then(|a| a.get("id")).and_then(Value::as_str).unwrap_or("");
    match fetch(host, owner) {
        Ok(items) if exists_hidden(&items, id) => return fail("not-found"),
        Ok(_) => {}
        Err(env) => return env,
    }
    match set(host, owner, json!({ "op": "delete", "id": id })) {
        Ok(_) => json!({ "raw": { "ok": true } }),
        Err(env) => env,
    }
}

pub fn changed_event(count: u64) -> Value {
    json!({ "t": EVENT, "count": count })
}

#[cfg(test)]
#[path = "prompts_tests.rs"]
mod tests;
