use crate::js;
use crate::preview::paths::{is_absolute, is_within, resolve_str};
use regex::Regex;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::sync::OnceLock;

pub const MAX_ENV: usize = 50;
pub const MAX_BEFORE: usize = 10;
pub const IMPORT_ID_PREFIX: &str = "imp-";
const MAX_NAME: usize = 80;
const MAX_CMD: usize = 500;
const MAX_ARGS: usize = 2000;
const MAX_VALUE: usize = 4096;
const BLOCKED_ENV: [&str; 9] = ["PATH", "PATHEXT", "SYSTEMROOT", "COMSPEC", "NODE_OPTIONS", "JAVA_TOOL_OPTIONS", "_JAVA_OPTIONS", "GRADLE_OPTS", "JAVA_OPTS"];

fn re(cell: &'static OnceLock<Regex>, src: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(src).expect("regex"))
}

pub fn is_run_config_id(v: &Value) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    v.as_str().is_some_and(|s| re(&R, r"^[A-Za-z0-9_-]{1,64}$").is_match(s))
}

pub fn is_blocked_shared_env(key: &str) -> bool {
    let k = key.to_uppercase();
    BLOCKED_ENV.contains(&k.as_str()) || k == "LD_PRELOAD" || k.starts_with("DYLD_")
}

pub fn drop_blocked_env(env: &Value) -> Value {
    let mut out = Map::new();
    if let Some(o) = env.as_object() {
        for (k, v) in js::entries(o) {
            if !is_blocked_shared_env(k) {
                out.insert(k.clone(), v.clone());
            }
        }
    }
    Value::Object(out)
}

pub fn needs_trust(cfg: &Value) -> bool {
    cfg.get("shared") == Some(&Value::Bool(true)) || cfg.get("id").and_then(Value::as_str).is_some_and(|i| i.starts_with(IMPORT_ID_PREFIX))
}

fn text(v: Option<&Value>, max: usize) -> Option<String> {
    match v {
        None => Some(String::new()),
        Some(Value::String(s)) if js::len(s) <= max => Some(s.clone()),
        _ => None,
    }
}

fn clean_entrypoint(v: Option<&Value>) -> Option<String> {
    let s = v?.as_str()?;
    let drive = s.len() >= 2 && s.as_bytes()[0].is_ascii_alphabetic() && s.as_bytes()[1] == b':';
    if s.is_empty() || js::len(s) > 260 || is_absolute(s) || drive {
        return None;
    }
    let norm = s.replace('\\', "/");
    let parts: Vec<&str> = norm.split('/').filter(|p| !p.is_empty() && *p != ".").collect();
    (!parts.is_empty() && !parts.contains(&"..")).then(|| parts.join("/"))
}

fn clean_env(v: Option<&Value>) -> Option<Value> {
    static KEY: OnceLock<Regex> = OnceLock::new();
    let Some(v) = v else { return Some(json!({})) };
    let o = v.as_object()?;
    if o.len() > MAX_ENV {
        return None;
    }
    let mut env = Map::new();
    for (k, val) in js::entries(o) {
        let s = val.as_str()?;
        if !re(&KEY, r"^[A-Za-z_][A-Za-z0-9_]*$").is_match(k) || js::len(s) > MAX_VALUE {
            return None;
        }
        env.insert(k.clone(), json!(s));
    }
    Some(Value::Object(env))
}

fn clean_before(v: Option<&Value>) -> Option<Vec<Value>> {
    let Some(v) = v else { return Some(Vec::new()) };
    let a = v.as_array()?;
    if a.len() > MAX_BEFORE {
        return None;
    }
    let mut out = Vec::new();
    for c in a {
        let s = c.as_str().filter(|s| js::len(s) <= MAX_CMD)?;
        let t = s.trim();
        if !t.is_empty() {
            out.push(json!(t));
        }
    }
    Some(out)
}

fn clean_device(v: Option<&Value>) -> Option<String> {
    static R: OnceLock<Regex> = OnceLock::new();
    match v {
        None => Some("selected".into()),
        Some(Value::String(s)) if s == "selected" || re(&R, r"^[A-Za-z0-9_.:@%-]{1,128}$").is_match(s) => Some(s.clone()),
        _ => None,
    }
}

pub fn clean_run_config(raw: &Value, root: &str) -> Option<Value> {
    static FLAVOR: OnceLock<Regex> = OnceLock::new();
    static VARIANT: OnceLock<Regex> = OnceLock::new();
    static MODULE: OnceLock<Regex> = OnceLock::new();
    let r = raw.as_object()?;
    if !is_absolute(root) {
        return None;
    }
    let id = r.get("id").filter(|v| is_run_config_id(v))?;
    let name = r.get("name")?.as_str()?.trim();
    if name.is_empty() || js::len(name) > MAX_NAME {
        return None;
    }
    let cwd_raw = r.get("cwd")?.as_str()?;
    if !is_absolute(cwd_raw) || js::len(cwd_raw) > MAX_VALUE {
        return None;
    }
    let cwd = resolve_str(cwd_raw);
    if !is_within(root, &cwd) {
        return None;
    }
    let env = clean_env(r.get("env"))?;
    let before = clean_before(r.get("beforeLaunch"))?;
    let mut out = Map::new();
    out.insert("id".into(), id.clone());
    out.insert("name".into(), json!(name));
    out.insert("cwd".into(), json!(cwd));
    out.insert("shared".into(), json!(r.get("shared") == Some(&Value::Bool(true))));
    out.insert("env".into(), env);
    out.insert("beforeLaunch".into(), Value::Array(before));
    match r.get("type").and_then(Value::as_str)? {
        "flutter" => {
            let entry = clean_entrypoint(r.get("entrypoint"))?;
            let device = clean_device(r.get("device"))?;
            let mode = match r.get("mode") {
                None => "debug".to_string(),
                Some(Value::String(m)) if matches!(m.as_str(), "debug" | "profile" | "release") => m.clone(),
                _ => return None,
            };
            let flavor = text(r.get("flavor"), 64)?;
            let args = text(r.get("args"), MAX_ARGS)?;
            let attach = text(r.get("attachArgs"), MAX_ARGS)?;
            if !re(&FLAVOR, r"^[A-Za-z0-9_]{0,64}$").is_match(&flavor) {
                return None;
            }
            for (k, v) in [("type", "flutter"), ("entrypoint", &entry), ("device", &device), ("mode", &mode), ("flavor", &flavor), ("args", &args), ("attachArgs", &attach)] {
                out.insert(k.into(), json!(v));
            }
        }
        "android" => {
            let device = clean_device(r.get("device"))?;
            let args = text(r.get("args"), MAX_ARGS)?;
            let variant = match r.get("variant") {
                None => "debug".to_string(),
                Some(Value::String(v)) => v.clone(),
                _ => return None,
            };
            let module = r.get("module")?.as_str()?;
            if js::len(module) > 200 || !re(&MODULE, r"^(:[A-Za-z0-9_.-]+)+$").is_match(module) || !re(&VARIANT, r"^[A-Za-z0-9_]{1,64}$").is_match(&variant) {
                return None;
            }
            for (k, v) in [("type", "android"), ("module", module), ("variant", &variant), ("device", &device), ("args", &args)] {
                out.insert(k.into(), json!(v));
            }
        }
        "custom" => {
            let command = text(r.get("command"), MAX_ARGS)?;
            out.insert("type".into(), json!("custom"));
            out.insert("command".into(), json!(command));
        }
        _ => return None,
    }
    Some(Value::Object(out))
}

pub fn default_templates() -> Value {
    json!({"flutter": {"mode": "debug", "flavor": "", "args": ""}, "android": {"variant": "debug", "args": ""}})
}

pub fn clean_templates(raw: Option<&Value>) -> Value {
    static FLAVOR: OnceLock<Regex> = OnceLock::new();
    static VARIANT: OnceLock<Regex> = OnceLock::new();
    let empty = Value::Object(Map::new());
    let raw = raw.filter(|v| v.is_object()).unwrap_or(&empty);
    let sect = |k: &str| raw.get(k).filter(|v| v.is_object()).cloned().unwrap_or_else(|| json!({}));
    let (f, a) = (sect("flutter"), sect("android"));
    let args = |v: Option<&Value>| v.and_then(Value::as_str).filter(|s| js::len(s) <= MAX_ARGS).unwrap_or("").to_string();
    let mode = f.get("mode").and_then(Value::as_str).filter(|m| matches!(*m, "debug" | "profile" | "release")).unwrap_or("debug");
    let flavor = f.get("flavor").and_then(Value::as_str).filter(|s| re(&FLAVOR, r"^[A-Za-z0-9_]{0,64}$").is_match(s)).unwrap_or("");
    let variant = a.get("variant").and_then(Value::as_str).filter(|s| re(&VARIANT, r"^[A-Za-z0-9_]{1,64}$").is_match(s)).unwrap_or("debug");
    json!({"flutter": {"mode": mode, "flavor": flavor, "args": args(f.get("args"))}, "android": {"variant": variant, "args": args(a.get("args"))}})
}

fn relative_posix(root: &str, dir: &str) -> String {
    crate::preview::paths::relative(root, dir).replace('\\', "/")
}

fn stable_id(root: &str, kind: &str, dir: &str, rest: &[&str]) -> String {
    let rel = relative_posix(root, dir);
    let mut parts = vec![kind.to_string(), if cfg!(windows) { rel.to_lowercase() } else { rel }];
    parts.extend(rest.iter().map(|s| s.to_string()));
    let digest = Sha256::digest(parts.join("\0").as_bytes());
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    format!("auto-{}", &hex[..16])
}

fn base_name(p: &str) -> String {
    p.rsplit(['/', '\\']).next().unwrap_or(p).to_string()
}

fn strs(v: &Value, key: &str) -> Vec<String> {
    v.get(key).and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
}

pub fn default_configs(projects: &[Value], root: &str, templates: &Value) -> Vec<Value> {
    let flutter: Vec<&Value> = projects.iter().filter(|p| p["kind"] == "flutter" && p["type"] == "app").collect();
    let android: Vec<&Value> = projects.iter().filter(|p| p["kind"] == "android" && p["framework"] != "flutter").collect();
    let mut out: Vec<Value> = Vec::new();
    let tf = &templates["flutter"];
    let ta = &templates["android"];
    for p in &flutter {
        let dir = p["dir"].as_str().unwrap_or("");
        let tag = if flutter.len() > 1 { base_name(dir) } else { String::new() };
        let entrypoints = strs(p, "entrypoints");
        let flavors = strs(p, "flavors");
        let label = |file: &str, flavor: &str| {
            let extra: Vec<&str> = [tag.as_str(), flavor].into_iter().filter(|s| !s.is_empty()).collect();
            let b = file.rsplit('/').next().unwrap_or(file);
            if extra.is_empty() {
                b.to_string()
            } else {
                format!("{b} ({})", extra.join(", "))
            }
        };
        let mk = |id: String, name: String, entry: &str, flavor: &str| {
            json!({"shared": false, "env": {}, "beforeLaunch": [], "type": "flutter", "cwd": dir, "device": "selected", "mode": tf["mode"], "args": tf["args"], "attachArgs": "", "id": id, "name": name, "entrypoint": entry, "flavor": flavor})
        };
        let tflavor = tf["flavor"].as_str().unwrap_or("");
        let flavor = if flavors.iter().any(|f| f == tflavor) { tflavor } else { "" };
        for e in &entrypoints {
            out.push(mk(stable_id(root, "flutter", dir, &[e, ""]), label(e, ""), e, flavor));
        }
        let main = entrypoints.iter().find(|e| *e == "lib/main.dart").or_else(|| entrypoints.first());
        if let Some(main) = main {
            for f in &flavors {
                out.push(mk(stable_id(root, "flutter", dir, &[main, f]), label(main, f), main, f));
            }
        }
    }
    for p in &android {
        let dir = p["dir"].as_str().unwrap_or("");
        let tag = if android.len() > 1 { base_name(dir) } else { String::new() };
        for m in p["modules"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
            if m["type"] != "application" && m["guessed"] != true {
                continue;
            }
            let mpath = m["path"].as_str().unwrap_or("");
            let short = mpath.strip_prefix(':').unwrap_or(mpath);
            out.push(json!({
                "shared": false, "env": {}, "beforeLaunch": [], "type": "android",
                "id": stable_id(root, "android", dir, &[mpath]),
                "name": if tag.is_empty() { short.to_string() } else { format!("{short} ({tag})") },
                "cwd": dir, "module": mpath, "variant": ta["variant"], "device": "selected", "args": ta["args"],
            }));
        }
    }
    out.iter().filter_map(|c| clean_run_config(c, root)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raw(extra: Value) -> Value {
        let mut base = json!({"id": "c1", "name": " Run ", "cwd": "/tmp/p/app", "type": "flutter", "entrypoint": "./lib\\main.dart", "env": {"A": "1"}, "beforeLaunch": [" pub get ", ""]});
        for (k, v) in extra.as_object().unwrap() {
            base[k] = v.clone();
        }
        base
    }

    #[test]
    fn cleans_flutter_config() {
        let c = clean_run_config(&raw(json!({})), "/tmp/p").unwrap();
        assert_eq!(c["name"], "Run");
        assert_eq!(c["entrypoint"], "lib/main.dart");
        assert_eq!(c["beforeLaunch"], json!(["pub get"]));
        assert_eq!(c["mode"], "debug");
        assert_eq!(c["device"], "selected");
        let keys: Vec<&String> = c.as_object().unwrap().keys().collect();
        assert_eq!(keys, ["id", "name", "cwd", "shared", "env", "beforeLaunch", "type", "entrypoint", "device", "mode", "flavor", "args", "attachArgs"]);
    }

    #[test]
    fn rejects_bad_input() {
        assert!(clean_run_config(&raw(json!({"cwd": "/tmp/other"})), "/tmp/p").is_none());
        assert!(clean_run_config(&raw(json!({"entrypoint": "../x.dart"})), "/tmp/p").is_none());
        assert!(clean_run_config(&raw(json!({"id": "bad id"})), "/tmp/p").is_none());
        assert!(clean_run_config(&raw(json!({"env": {"1X": "a"}})), "/tmp/p").is_none());
        assert!(clean_run_config(&raw(json!({"mode": "fast"})), "/tmp/p").is_none());
        assert!(clean_run_config(&raw(json!({"type": "x"})), "/tmp/p").is_none());
        assert!(clean_run_config(&raw(json!({})), "rel").is_none());
    }

    #[test]
    fn blocked_env_and_templates() {
        assert!(is_blocked_shared_env("path") && is_blocked_shared_env("DYLD_X") && !is_blocked_shared_env("FOO"));
        assert_eq!(drop_blocked_env(&json!({"PATH": "x", "A": "1"})), json!({"A": "1"}));
        assert_eq!(clean_templates(Some(&json!({"flutter": {"mode": "bad", "flavor": "x y"}}))), default_templates());
        assert!(needs_trust(&json!({"id": "imp-1", "shared": false})) && !needs_trust(&json!({"id": "a", "shared": false})));
    }

    #[test]
    fn default_ids_are_stable() {
        let p = json!({"kind": "flutter", "type": "app", "dir": "/tmp/p/app", "entrypoints": ["lib/main.dart", "lib/main_dev.dart"], "flavors": ["dev"]});
        let a = default_configs(std::slice::from_ref(&p), "/tmp/p", &default_templates());
        let b = default_configs(&[p], "/tmp/p", &default_templates());
        assert_eq!(a.len(), 3);
        assert_eq!(a, b);
        assert!(a[0]["id"].as_str().unwrap().starts_with("auto-"));
        assert_eq!(a[2]["name"], "main.dart (dev)");
    }
}
