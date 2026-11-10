use crate::claude::{same_dir, Sys};
use crate::js;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn read(file: &Path) -> Value {
    std::fs::read(file).ok().and_then(|b| serde_json::from_str(&String::from_utf8_lossy(&b)).ok()).unwrap_or(Value::Null)
}

fn server_names(v: Option<&Value>) -> Vec<String> {
    match v.and_then(|v| v.get("mcpServers")) {
        Some(Value::Object(m)) => js::keys(m).into_iter().cloned().collect(),
        _ => Vec::new(),
    }
}

pub fn read_configured(claude_json: &Path, dir: Option<&str>, win: bool) -> Vec<(String, Vec<&'static str>)> {
    let mut out: Vec<(String, Vec<&'static str>)> = Vec::new();
    let mut add = |name: String, scope: &'static str| match out.iter_mut().find(|(n, _)| *n == name) {
        Some((_, list)) => {
            if !list.contains(&scope) {
                list.push(scope);
            }
        }
        None => out.push((name, vec![scope])),
    };
    let global = read(claude_json);
    for n in server_names(Some(&global)) {
        add(n, "user");
    }
    if let Some(dir) = dir {
        if let Some(Value::Object(projects)) = global.get("projects") {
            for (key, value) in js::entries(projects) {
                if same_dir(key, dir, win) {
                    for n in server_names(Some(value)) {
                        add(n, "local");
                    }
                }
            }
        }
        for n in server_names(Some(&read(&Path::new(dir).join(".mcp.json")))) {
            add(n, "project");
        }
    }
    out
}

pub fn configured_json(list: &[(String, Vec<&'static str>)]) -> Value {
    Value::Array(list.iter().map(|(n, s)| json!({"name": n, "scopes": s})).collect())
}

pub fn list_skills(dir: &str) -> Vec<String> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut names: Vec<String> = rd
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter_map(|e| e.file_name().into_string().ok())
        .filter(|n| !n.starts_with('.'))
        .collect();
    names.sort_by(|a, b| core_transcript::jsstr::utf16_cmp(a, b));
    names.truncate(200);
    names
}

pub fn path_dirs(env: &BTreeMap<String, String>, win: bool) -> Vec<String> {
    let path = env.iter().find(|(k, _)| if win { k.eq_ignore_ascii_case("PATH") } else { k.as_str() == "PATH" }).map(|(_, v)| v.clone()).unwrap_or_default();
    path.split(if win { ';' } else { ':' }).map(str::to_string).collect()
}

pub fn find_on_path(name: &str, env: &BTreeMap<String, String>, extra: &[PathBuf], win: bool) -> Option<PathBuf> {
    let dirs = path_dirs(env, win).into_iter().filter(|d| !d.is_empty() && crate::claude::is_absolute(d)).map(PathBuf::from);
    dirs.chain(extra.iter().cloned()).map(|d| d.join(name)).find(|f| f.is_file())
}

pub fn probe_prereqs(env: &BTreeMap<String, String>, sys: &Sys) -> Value {
    let home = Path::new(&sys.home);
    let extra = [home.join(".local").join("bin"), home.join(".cargo").join("bin")];
    let pick = |w: &str, p: &str| find_on_path(if sys.win { w } else { p }, env, &extra, sys.win).is_some();
    json!({"node": pick("npx.cmd", "npx"), "uv": pick("uvx.exe", "uvx"), "docker": pick("docker.exe", "docker")})
}

pub fn find_claude_cli(env: &BTreeMap<String, String>, sys: &Sys) -> Option<String> {
    let exe = if sys.win { "claude.exe" } else { "claude" };
    let mut out: Vec<PathBuf> = Vec::new();
    for dir in path_dirs(env, sys.win).into_iter().filter(|d| !d.is_empty()) {
        let d = Path::new(&dir);
        out.push(d.join(exe));
        out.push(d.join("node_modules").join("@anthropic-ai").join("claude-code").join("bin").join(exe));
    }
    let home = env.get("USERPROFILE").or_else(|| env.get("HOME")).filter(|h| !h.is_empty());
    if let Some(h) = home {
        out.push(Path::new(h).join(".local").join("bin").join(exe));
    }
    out.into_iter().find(|p| p.is_file()).map(|p| crate::claude::resolve(&crate::util::path_str(&p)))
}

const AGENT_SESSION_VARS: [&str; 14] = [
    "CLAUDECODE",
    "CLAUDE_PID",
    "CLAUDE_EFFORT",
    "CLAUDE_CODE_CHILD_SESSION",
    "CLAUDE_CODE_SESSION_ID",
    "CLAUDE_CODE_SESSION_ATTENDED",
    "CLAUDE_CODE_ENTRYPOINT",
    "CLAUDE_CODE_EXECPATH",
    "CLAUDE_CODE_MESSAGING_SOCKET",
    "CLAUDE_CODE_MESSAGING_TOKEN",
    "CLAUDE_CODE_SSE_PORT",
    "CODEX_THREAD_ID",
    "CODEX_SANDBOX",
    "CODEX_SANDBOX_NETWORK_DISABLED",
];

pub fn child_env(env: &[(String, String)]) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (key, value) in env {
        let k = key.to_uppercase();
        if k.starts_with("CCM_") || k.starts_with("ORRERY_") || k.starts_with("GOLAIM_") || k.starts_with("ELECTRON_") || k == "NODE_OPTIONS" || AGENT_SESSION_VARS.contains(&k.as_str()) {
            continue;
        }
        out.insert(key.clone(), value.clone());
    }
    out.insert("TERM".into(), "xterm-256color".into());
    out.insert("COLORTERM".into(), "truecolor".into());
    out.insert("FORCE_HYPERLINK".into(), "1".into());
    out.insert("NO_COLOR".into(), "1".into());
    out
}
