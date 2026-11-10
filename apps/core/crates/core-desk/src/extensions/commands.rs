use crate::js;
use regex::{Regex, RegexBuilder};
use serde_json::{json, Map, Value};
use std::sync::OnceLock;

pub struct CliStep {
    pub args: Vec<String>,
    pub secrets: Vec<String>,
}

fn step(args: &[&str], secrets: Vec<String>) -> CliStep {
    CliStep { args: args.iter().map(|s| s.to_string()).collect(), secrets }
}

pub fn catalog() -> &'static Vec<Value> {
    static C: OnceLock<Vec<Value>> = OnceLock::new();
    C.get_or_init(|| serde_json::from_str(include_str!("catalog.json")).unwrap_or_default())
}

pub fn catalog_entry(id: &Value) -> Option<&'static Value> {
    let id = id.as_str()?;
    catalog().iter().find(|e| e["id"] == id)
}

fn re(src: &str) -> Regex {
    RegexBuilder::new(src).size_limit(64 << 20).build().unwrap_or_else(|_| Regex::new("$^").unwrap())
}

pub fn marketplace_repo(s: &str) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| re(r"^[A-Za-z0-9][A-Za-z0-9-]{0,38}/[A-Za-z0-9._-]{1,100}$")).is_match(s)
}

pub fn mcp_name(s: &str) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| re(r"^[A-Za-z0-9][A-Za-z0-9_-]{0,39}$")).is_match(s)
}

pub fn plugin_name(s: &str) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| re(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,99}$")).is_match(s)
}

pub fn is_scope(v: &Value) -> bool {
    matches!(v.as_str(), Some("user" | "project" | "local"))
}

fn fields(entry: &Value) -> &[Value] {
    entry["fields"].as_array().map(Vec::as_slice).unwrap_or(&[])
}

fn field_default(entry: &Value, key: &str) -> String {
    fields(entry).iter().find(|f| f["key"] == key).and_then(|f| f["default"].as_str()).unwrap_or("").to_string()
}

pub fn check_fields(entry: &Value, values: &Map<String, Value>) -> Option<String> {
    for k in js::keys(values) {
        if !fields(entry).iter().any(|f| f["key"] == k.as_str()) {
            return Some(k.clone());
        }
    }
    for f in fields(entry) {
        let key = f["key"].as_str().unwrap_or("");
        let v = core_transcript::jsstr::js_trim(values.get(key).and_then(Value::as_str).unwrap_or(""));
        if v.is_empty() {
            if f["required"] == true && field_default(entry, key).is_empty() {
                return Some(key.into());
            }
            continue;
        }
        if !re(f["pattern"].as_str().unwrap_or("")).is_match(v) {
            return Some(key.into());
        }
    }
    None
}

fn fill(template: &str, entry: &Value, values: &Map<String, Value>) -> String {
    static R: OnceLock<Regex> = OnceLock::new();
    let r = R.get_or_init(|| re(r"\{([A-Z0-9_]+)\}"));
    r.replace_all(template, |c: &regex::Captures| {
        let key = &c[1];
        let v = core_transcript::jsstr::js_trim(values.get(key).and_then(Value::as_str).unwrap_or(""));
        if v.is_empty() {
            field_default(entry, key)
        } else {
            v.to_string()
        }
    })
    .into_owned()
}

fn secrets_of(entry: &Value, values: &Map<String, Value>) -> Vec<String> {
    fields(entry)
        .iter()
        .filter(|f| f["secret"] == true)
        .filter_map(|f| {
            let v = core_transcript::jsstr::js_trim(values.get(f["key"].as_str().unwrap_or("")).and_then(Value::as_str).unwrap_or(""));
            (!v.is_empty()).then(|| v.to_string())
        })
        .collect()
}

pub fn mcp_config(entry: &Value, values: &Map<String, Value>, win: bool) -> Option<Value> {
    let inst = &entry["install"];
    match inst["type"].as_str() {
        Some("http") => {
            let mut out = Map::new();
            out.insert("type".into(), json!("http"));
            out.insert("url".into(), Value::String(fill(inst["url"].as_str().unwrap_or(""), entry, values)));
            if let Some(h) = inst.get("header").filter(|h| h.is_object()) {
                let mut headers = Map::new();
                headers.insert(h["name"].as_str().unwrap_or("").into(), Value::String(fill(h["value"].as_str().unwrap_or(""), entry, values)));
                out.insert("headers".into(), Value::Object(headers));
            }
            Some(Value::Object(out))
        }
        Some("stdio") => {
            let mut env = Map::new();
            for k in inst["env"].as_array().into_iter().flatten().filter_map(Value::as_str) {
                let v = core_transcript::jsstr::js_trim(values.get(k).and_then(Value::as_str).unwrap_or(""));
                if !v.is_empty() {
                    env.insert(k.into(), Value::String(v.into()));
                }
            }
            let shim = win && inst["winShim"] == true;
            let command = inst["command"].clone();
            let args: Vec<Value> = inst["args"].as_array().cloned().unwrap_or_default();
            let mut out = Map::new();
            out.insert("type".into(), json!("stdio"));
            out.insert("command".into(), if shim { json!("cmd") } else { command.clone() });
            let mut all = if shim { vec![json!("/c"), command] } else { Vec::new() };
            all.extend(args);
            out.insert("args".into(), Value::Array(all));
            if !env.is_empty() {
                out.insert("env".into(), Value::Object(env));
            }
            Some(Value::Object(out))
        }
        _ => None,
    }
}

fn scope_args(scope: &str) -> [&str; 2] {
    ["-s", scope]
}

pub fn build_install(req: &Value, win: bool, marketplaces: &[String]) -> Option<Vec<CliStep>> {
    let entry = catalog_entry(req.get("id").unwrap_or(&Value::Null))?;
    let scope = req.get("scope").filter(|s| is_scope(s))?.as_str()?;
    let raw = req.get("values").filter(|v| v.is_object() || v.is_array())?;
    let mut values = Map::new();
    if let Value::Object(m) = raw {
        for (k, v) in js::entries(m) {
            let s = v.as_str().filter(|s| core_transcript::jsstr::units(s) <= 1024)?;
            values.insert(k.clone(), Value::String(s.into()));
        }
    } else if let Value::Array(a) = raw {
        for (i, v) in a.iter().enumerate() {
            let s = v.as_str().filter(|s| core_transcript::jsstr::units(s) <= 1024)?;
            values.insert(i.to_string(), Value::String(s.into()));
        }
    }
    if check_fields(entry, &values).is_some() {
        return None;
    }
    let inst = &entry["install"];
    if inst["type"] == "plugins" {
        let all: Vec<&str> = inst["plugins"].as_array().into_iter().flatten().filter_map(Value::as_str).collect();
        let wanted: Vec<String> = match req.get("plugins") {
            None => all.iter().map(|s| s.to_string()).collect(),
            Some(Value::Array(a)) => a.iter().map(|p| p.as_str().filter(|p| all.contains(p)).map(str::to_string)).collect::<Option<Vec<_>>>()?,
            Some(_) => return None,
        };
        if wanted.is_empty() {
            return None;
        }
        let market = inst["marketplace"].as_str().unwrap_or("");
        let mut steps = Vec::new();
        if !marketplaces.iter().any(|m| m == market) {
            let repo = inst["repo"].as_str().filter(|r| !r.is_empty())?;
            steps.push(step(&["plugin", "marketplace", "add", repo], Vec::new()));
        }
        for p in wanted {
            let id = format!("{p}@{market}");
            let [s, sc] = scope_args(scope);
            steps.push(step(&["plugin", "install", &id, s, sc], Vec::new()));
        }
        return Some(steps);
    }
    let config = mcp_config(entry, &values, win)?;
    let server = entry["server"].as_str().filter(|s| !s.is_empty())?;
    let [s, sc] = scope_args(scope);
    Some(vec![step(&["mcp", "add-json", s, sc, server, &js::stringify(&config)], secrets_of(entry, &values))])
}

pub fn is_plugin_id(id: &Value) -> bool {
    let Some(id) = id.as_str() else { return false };
    let parts: Vec<&str> = id.split('@').collect();
    parts.len() == 2 && plugin_name(parts[0]) && plugin_name(parts[1])
}

fn id_list(v: &Value) -> Option<Vec<String>> {
    let list: Vec<Value> = match v {
        Value::Array(a) => a.clone(),
        other => vec![other.clone()],
    };
    if list.is_empty() || list.len() > 20 || !list.iter().all(is_plugin_id) {
        return None;
    }
    let mut out: Vec<String> = Vec::new();
    for s in list.iter().filter_map(Value::as_str) {
        if !out.iter().any(|x| x == s) {
            out.push(s.into());
        }
    }
    Some(out)
}

fn remove_name_ok(s: &str) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| re(r"^[A-Za-z0-9][A-Za-z0-9 ._-]{0,79}$")).is_match(s)
}

pub fn build_remove(req: &Value) -> Option<Vec<CliStep>> {
    let scope = req.get("scope").filter(|s| is_scope(s))?.as_str()?;
    let [s, sc] = scope_args(scope);
    match req.get("kind").and_then(Value::as_str) {
        Some("mcp") => {
            let name = req.get("name").and_then(Value::as_str).filter(|n| remove_name_ok(n))?;
            Some(vec![step(&["mcp", "remove", name, s, sc], Vec::new())])
        }
        Some("plugin") => {
            let ids = id_list(req.get("name").unwrap_or(&Value::Null))?;
            Some(ids.iter().map(|id| step(&["plugin", "uninstall", id, s, sc], Vec::new())).collect())
        }
        _ => None,
    }
}

fn header_parts(h: &str) -> Option<(String, String)> {
    static R: OnceLock<Regex> = OnceLock::new();
    let r = R.get_or_init(|| re(r"^([A-Za-z0-9-]{1,64}):[\s\x{feff}]*([^\n\r\u{2028}\u{2029}]{1,2048})$"));
    let c = r.captures(h)?;
    Some((c[1].to_string(), c[2].to_string()))
}

pub fn build_source(req: &Value) -> Option<Vec<CliStep>> {
    if !req.is_object() && !req.is_array() {
        return None;
    }
    let ty = req.get("type").and_then(Value::as_str);
    if ty == Some("marketplace") {
        let repo = req.get("repo").and_then(Value::as_str).filter(|r| marketplace_repo(r))?;
        return Some(vec![step(&["plugin", "marketplace", "add", repo, "--json"], Vec::new())]);
    }
    let scope = req.get("scope").filter(|s| is_scope(s))?.as_str()?;
    let [s, sc] = scope_args(scope);
    match ty {
        Some("plugin") => {
            let ids = id_list(req.get("ids").unwrap_or(&Value::Null))?;
            Some(ids.iter().map(|id| step(&["plugin", "install", id, s, sc], Vec::new())).collect())
        }
        Some("mcp") => {
            let name = req.get("name").and_then(Value::as_str).filter(|n| mcp_name(n))?;
            let raw = req.get("url").and_then(Value::as_str).filter(|u| core_transcript::jsstr::units(u) <= 2048)?;
            let url = url::Url::parse(raw).ok()?;
            if url.scheme() != "https" && url.scheme() != "http" {
                return None;
            }
            let mut config = Map::new();
            config.insert("type".into(), json!("http"));
            config.insert("url".into(), Value::String(url.as_str().to_string()));
            let mut secrets = Vec::new();
            match req.get("header") {
                None | Some(Value::Null) => {}
                Some(Value::String(h)) if h.is_empty() => {}
                Some(Value::String(h)) => {
                    let (name, value) = header_parts(h).filter(|_| !h.contains(['\r', '\n']))?;
                    let mut headers = Map::new();
                    headers.insert(name, Value::String(value.clone()));
                    config.insert("headers".into(), Value::Object(headers));
                    secrets.push(value);
                }
                Some(_) => return None,
            }
            Some(vec![step(&["mcp", "add-json", s, sc, name, &js::stringify(&Value::Object(config))], secrets)])
        }
        _ => None,
    }
}

fn quote(arg: &str) -> String {
    if !arg.is_empty() && arg.chars().all(|c| c.is_ascii_alphanumeric() || "_@:./=-".contains(c)) {
        arg.to_string()
    } else {
        format!("'{}'", arg.replace('\'', "'\\''"))
    }
}

pub fn show_command(step: &CliStep) -> String {
    let masked = step.args.iter().map(|a| step.secrets.iter().fold(a.clone(), |s, secret| if secret.is_empty() { s } else { s.replace(secret.as_str(), "••••") }));
    std::iter::once("claude".to_string()).chain(masked.map(|a| quote(&a))).collect::<Vec<_>>().join(" ")
}
