use super::args::quote_for_display;
use super::configs::{clean_run_config, drop_blocked_env, IMPORT_ID_PREFIX, MAX_ENV};
use crate::js;
use crate::preview::paths::{dirname, is_within, relative, resolve_str};
use regex::Regex;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::path::Path;
use std::sync::OnceLock;

const MAX_FILE: usize = 512 * 1024;

fn re(cell: &'static OnceLock<Regex>, src: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(src).expect("regex"))
}

fn read_small(file: &str) -> Option<String> {
    let s = String::from_utf8_lossy(&std::fs::read(file).ok()?).to_string();
    (js::len(&s) <= MAX_FILE).then_some(s)
}

fn list_files(dir: &str, suffix: &str) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut names: Vec<String> = rd.flatten().map(|e| e.file_name().to_string_lossy().to_string()).filter(|f| f.ends_with(suffix)).collect();
    names.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
    names.into_iter().map(|f| resolve_str(&format!("{dir}/{f}"))).collect()
}

fn pubspec_dir(start: &str, root: &str) -> String {
    let mut dir = start.to_string();
    while is_within(root, &dir) {
        if Path::new(&dir).join("pubspec.yaml").exists() {
            return dir;
        }
        let up = dirname(&dir);
        if up == dir {
            break;
        }
        dir = up;
    }
    root.to_string()
}

fn join_resolve(base: &str, p: &str) -> String {
    if crate::preview::paths::is_absolute(p) {
        resolve_str(p)
    } else {
        resolve_str(&format!("{base}/{p}"))
    }
}

fn rel_posix(from: &str, to: &str) -> String {
    relative(from, to).replace('\\', "/")
}

fn decode_xml(s: &str) -> String {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, r"&(lt|gt|quot|apos|amp|#([0-9]+)|#x([0-9a-fA-F]+));")
        .replace_all(s, |c: &regex::Captures| {
            if let Some(d) = c.get(2) {
                return d.as_str().parse::<u32>().ok().and_then(char::from_u32).map(String::from).unwrap_or_default();
            }
            if let Some(h) = c.get(3) {
                return u32::from_str_radix(h.as_str(), 16).ok().and_then(char::from_u32).map(String::from).unwrap_or_default();
            }
            match &c[1] {
                "lt" => "<",
                "gt" => ">",
                "quot" => "\"",
                "apos" => "'",
                "amp" => "&",
                _ => "",
            }
            .to_string()
        })
        .to_string()
}

fn attrs(tag: &str) -> Vec<(String, String)> {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, r#"([A-Za-z0-9_.:-]+)\s*=\s*"([^"]*)""#).captures_iter(tag).map(|m| (m[1].to_string(), decode_xml(&m[2]))).collect()
}

fn attr(list: &[(String, String)], key: &str) -> Option<String> {
    list.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v.clone())
}

fn gradle_module(name: &str) -> Option<String> {
    let mut parts: Vec<&str> = name.split('.').filter(|p| !p.is_empty()).collect();
    if parts.len() > 1 && ["main", "test", "androidTest", "unitTest"].contains(parts.last().unwrap()) {
        parts.pop();
    }
    if parts.len() > 1 {
        parts.remove(0);
    }
    (!parts.is_empty()).then(|| format!(":{}", parts.join(":")))
}

fn import_id(root: &str, file: &str, name: &str) -> String {
    let key = format!("{}\0{}", rel_posix(root, file), name);
    let digest = Sha256::digest(key.as_bytes());
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    format!("{IMPORT_ID_PREFIX}{}", &hex[..16])
}

fn base_name(p: &str) -> String {
    p.rsplit(['/', '\\']).next().unwrap_or(p).to_string()
}

fn parse_run_xml(xml: &str, root: &str, source: &str) -> Vec<Value> {
    static CONF: OnceLock<Regex> = OnceLock::new();
    static OPT: OnceLock<Regex> = OnceLock::new();
    static ENV: OnceLock<Regex> = OnceLock::new();
    static MODULE: OnceLock<Regex> = OnceLock::new();
    let mut out = Vec::new();
    let expand = |v: &str| v.replace("$PROJECT_DIR$", root);
    for m in re(&CONF, r"<configuration\b([^>]*?)(/>|>([\s\S]*?)</configuration>)").captures_iter(xml) {
        let head = attrs(&m[1]);
        let body = m.get(3).map(|b| b.as_str()).unwrap_or("");
        let mut opts: Vec<(String, String)> = Vec::new();
        for o in re(&OPT, r"<option\b([^>]*?)/?>").captures_iter(body) {
            let a = attrs(&o[1]);
            if let (Some(n), Some(v)) = (attr(&a, "name").filter(|n| !n.is_empty()), attr(&a, "value")) {
                opts.push((n, v));
            }
        }
        let opt = |k: &str| attr(&opts, k);
        let mut env = Map::new();
        for e in re(&ENV, r"<env\b([^>]*?)/?>").captures_iter(body) {
            let a = attrs(&e[1]);
            if let Some(n) = attr(&a, "name").filter(|n| !n.is_empty()) {
                if env.len() < MAX_ENV || env.contains_key(&n) {
                    env.insert(n, json!(attr(&a, "value").unwrap_or_default()));
                }
            }
        }
        let name = js::slice(attr(&head, "name").unwrap_or_default().trim(), 80);
        match attr(&head, "type").as_deref() {
            Some("FlutterRunConfigurationType") => {
                let file = match opt("filePath").filter(|p| !p.is_empty()) {
                    Some(p) => join_resolve(root, &expand(&p)),
                    None => resolve_str(&format!("{root}/lib/main.dart")),
                };
                let cwd = pubspec_dir(&dirname(&file), root);
                let shown = if name.is_empty() { base_name(&file) } else { name.clone() };
                out.push(json!({
                    "id": import_id(root, source, &shown), "type": "flutter", "name": shown, "cwd": cwd, "shared": false,
                    "env": env, "beforeLaunch": [], "entrypoint": rel_posix(&cwd, &file), "device": "selected", "mode": "debug",
                    "flavor": opt("buildFlavor").unwrap_or_default(), "args": expand(&opt("additionalArgs").unwrap_or_default()),
                    "attachArgs": expand(&opt("attachArgs").unwrap_or_default()),
                }));
            }
            Some("AndroidRunConfigurationType") => {
                let module = re(&MODULE, r"<module\b([^>]*?)/?>").captures(body).and_then(|mm| gradle_module(&attr(&attrs(&mm[1]), "name").unwrap_or_default()));
                let Some(module) = module else { continue };
                let shown = if name.is_empty() { module.clone() } else { name.clone() };
                out.push(json!({
                    "id": import_id(root, source, &shown), "type": "android", "name": if name.is_empty() { module[1..].to_string() } else { name },
                    "cwd": root, "shared": false, "env": env, "beforeLaunch": [], "module": module, "variant": "debug", "device": "selected", "args": "",
                }));
            }
            _ => {}
        }
    }
    out
}

fn finish(list: Vec<Value>, root: &str) -> Vec<Value> {
    let mut seen: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for mut raw in list {
        raw["env"] = drop_blocked_env(&raw["env"]);
        if let Some(c) = clean_run_config(&raw, root) {
            let id = c["id"].as_str().unwrap_or("").to_string();
            if !seen.contains(&id) {
                seen.push(id);
                out.push(c);
            }
        }
    }
    out
}

pub fn import_intellij(root: &str) -> Vec<Value> {
    let mut files = list_files(&format!("{root}/.run"), ".run.xml");
    files.extend(list_files(&format!("{root}/.idea/runConfigurations"), ".xml"));
    let mut out = Vec::new();
    for f in files {
        if let Some(xml) = read_small(&f) {
            out.extend(parse_run_xml(&xml, root, &f));
        }
    }
    finish(out, root)
}

pub fn strip_jsonc(s: &str) -> String {
    let c: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < c.len() {
        if c[i] == '"' {
            let mut j = i + 1;
            while j < c.len() && c[j] != '"' {
                j += if c[j] == '\\' { 2 } else { 1 };
            }
            let end = (j + 1).min(c.len());
            out.extend(&c[i..end]);
            i = j + 1;
        } else if c[i] == '/' && c.get(i + 1) == Some(&'/') {
            while i < c.len() && c[i] != '\n' {
                i += 1;
            }
        } else if c[i] == '/' && c.get(i + 1) == Some(&'*') {
            let mut k = i + 2;
            let mut found = None;
            while k + 1 < c.len() {
                if c[k] == '*' && c[k + 1] == '/' {
                    found = Some(k);
                    break;
                }
                k += 1;
            }
            i = found.map(|k| k + 2).unwrap_or(c.len());
        } else if c[i] == ',' {
            let mut j = i + 1;
            while j < c.len() && c[j].is_whitespace() {
                j += 1;
            }
            if c.get(j) != Some(&'}') && c.get(j) != Some(&']') {
                out.push(',');
            }
            i += 1;
        } else {
            out.push(c[i]);
            i += 1;
        }
    }
    out
}

fn str_array(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array).map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
}

pub fn import_vscode(root: &str) -> Vec<Value> {
    let source = resolve_str(&format!("{root}/.vscode/launch.json"));
    let Some(raw) = read_small(&source) else { return Vec::new() };
    let Ok(json) = serde_json::from_str::<Value>(&strip_jsonc(&raw)) else { return Vec::new() };
    let Some(list) = json.get("configurations").and_then(Value::as_array) else { return Vec::new() };
    let expand = |v: &str| v.replace("${workspaceFolder}", root).replace("${workspaceRoot}", root);
    let mut out = Vec::new();
    for item in list {
        let Some(e) = item.as_object() else { continue };
        if e.get("type").and_then(Value::as_str) != Some("dart") || e.get("request").is_some_and(|r| r.as_str() != Some("launch")) {
            continue;
        }
        let base = match e.get("cwd").and_then(Value::as_str) {
            Some(c) => join_resolve(root, &expand(c)),
            None => root.to_string(),
        };
        let program = e.get("program").and_then(Value::as_str).map(expand).unwrap_or_else(|| "lib/main.dart".to_string());
        let file = join_resolve(&base, &program);
        let cwd = if Path::new(&base).join("pubspec.yaml").exists() && is_within(&base, &file) { base.clone() } else { pubspec_dir(&dirname(&file), root) };
        let mut env = Map::new();
        if let Some(Value::Object(o)) = e.get("env") {
            for (k, v) in js::entries(o) {
                match v {
                    Value::String(s) => {
                        env.insert(k.clone(), json!(s));
                    }
                    Value::Number(n) => {
                        env.insert(k.clone(), json!(js::number(n.as_f64().unwrap_or(0.0))));
                    }
                    _ => {}
                }
            }
        }
        let mut args = str_array(e.get("toolArgs"));
        args.extend(str_array(e.get("args")));
        let name = e.get("name").and_then(Value::as_str).map(str::trim).filter(|n| !n.is_empty()).map(|n| js::slice(n, 80)).unwrap_or_else(|| base_name(&file));
        let expanded: Vec<String> = args.iter().map(|a| expand(a)).collect();
        out.push(json!({
            "id": import_id(root, &source, &name), "type": "flutter", "name": name, "cwd": cwd, "shared": false, "env": env, "beforeLaunch": [],
            "entrypoint": rel_posix(&cwd, &file),
            "device": e.get("deviceId").and_then(Value::as_str).filter(|d| !d.is_empty()).unwrap_or("selected"),
            "mode": match e.get("flutterMode").and_then(Value::as_str) { Some("profile") => "profile", Some("release") => "release", _ => "debug" },
            "flavor": "", "args": if args.is_empty() { String::new() } else { quote_for_display(&expanded) }, "attachArgs": "",
        }));
    }
    finish(out, root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_jsonc() {
        assert_eq!(strip_jsonc("{\"a\": \"x//y\", // c\n \"b\": [1, 2,],/* z */ }"), "{\"a\": \"x//y\", \n \"b\": [1, 2], }");
    }

    #[test]
    fn imports_both_sources() {
        let root = std::env::temp_dir().join(format!("ccm-d4-imp-{}", crate::util::uuid()));
        std::fs::create_dir_all(root.join(".run")).unwrap();
        std::fs::create_dir_all(root.join(".vscode")).unwrap();
        std::fs::create_dir_all(root.join("app/lib")).unwrap();
        std::fs::write(root.join("app/pubspec.yaml"), "name: app\n").unwrap();
        std::fs::write(root.join(".run/a.run.xml"), r#"<component><configuration name="Dev &amp; Co" type="FlutterRunConfigurationType"><option name="filePath" value="$PROJECT_DIR$/app/lib/main_dev.dart" /><option name="buildFlavor" value="dev" /><env name="A" value="1" /><env name="PATH" value="x" /></configuration><configuration name="m" type="AndroidRunConfigurationType"><module name="My.feature.login.main" /></configuration></component>"#).unwrap();
        std::fs::write(root.join(".vscode/launch.json"), r#"{"configurations": [{"type": "dart", "name": "go", "program": "app/lib/main.dart", "args": ["--a b"], "env": {"N": 3},}]}"#).unwrap();
        let r = root.to_string_lossy().to_string();
        let i = import_intellij(&r);
        assert_eq!(i.len(), 2);
        assert_eq!(i[0]["name"], "Dev & Co");
        assert_eq!(i[0]["entrypoint"], "lib/main_dev.dart");
        assert_eq!(i[0]["env"], json!({"A": "1"}));
        assert_eq!(i[1]["module"], ":feature:login");
        assert!(i[0]["id"].as_str().unwrap().starts_with("imp-"));
        let v = import_vscode(&r);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0]["args"], "\"--a b\"");
        assert_eq!(v[0]["env"], json!({"N": "3"}));
        assert_eq!(import_intellij(&r), i);
    }
}
