use super::{fail, status, ExtService};
use crate::claude::{self, check_dir, same_dir, DirArg};
use crate::js;
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};

const LOCKED: &str = "orrery";
const NAME_MAX: usize = 200;

enum Load {
    Missing,
    Bad,
    Ok(Map<String, Value>, bool),
}

fn load(file: &Path) -> Load {
    let bytes = match std::fs::read(file) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Load::Missing,
        Err(_) => return Load::Bad,
    };
    let text = String::from_utf8_lossy(&bytes);
    match claude::parse_text(&text) {
        Some(Value::Object(m)) => Load::Ok(m, text.ends_with('\n')),
        _ => Load::Bad,
    }
}

fn read_map(file: &Path) -> Map<String, Value> {
    match load(file) {
        Load::Ok(m, _) => m,
        _ => Map::new(),
    }
}

fn render(root: Map<String, Value>, newline: bool) -> String {
    let mut text = js::stringify_indent(&Value::Object(root), "  ");
    if newline {
        text.push('\n');
    }
    text
}

fn project_keys(root: &Map<String, Value>, dir: &str, win: bool) -> Vec<String> {
    match root.get("projects") {
        Some(Value::Object(p)) => js::keys(p).into_iter().filter(|k| same_dir(k, dir, win)).cloned().collect(),
        _ => Vec::new(),
    }
}

fn disabled_names(root: &Map<String, Value>, dir: &str, win: bool) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Some(Value::Object(projects)) = root.get("projects") {
        for key in project_keys(root, dir, win) {
            if let Some(Value::Array(list)) = projects.get(&key).and_then(|p| p.get("disabledMcpServers")) {
                for n in list.iter().filter_map(Value::as_str) {
                    if !out.iter().any(|o| o == n) {
                        out.push(n.to_string());
                    }
                }
            }
        }
    }
    out
}

fn plugin_map(root: &Map<String, Value>) -> Vec<(String, bool)> {
    match root.get("enabledPlugins") {
        Some(Value::Object(m)) => js::entries(m).into_iter().filter_map(|(k, v)| v.as_bool().map(|b| (k.clone(), b))).collect(),
        _ => Vec::new(),
    }
}

fn scope_of(name: &str, configured: &[(String, Vec<&'static str>)]) -> &'static str {
    if let Some((_, scopes)) = configured.iter().find(|(n, _)| n == name) {
        return if scopes.contains(&"local") {
            "local"
        } else if scopes.contains(&"project") {
            "project"
        } else {
            "user"
        };
    }
    if name.starts_with("plugin:") {
        "plugin"
    } else if name.starts_with("claude.ai ") {
        "connector"
    } else {
        "user"
    }
}

fn valid_name(name: &str) -> bool {
    !name.is_empty() && name.chars().count() <= NAME_MAX && !name.starts_with('-') && !name.chars().any(char::is_control)
}

fn edit_mcp(root: &mut Map<String, Value>, dir: &str, name: &str, enabled: bool, win: bool) -> Result<bool, ()> {
    let keys = project_keys(root, dir, win);
    if enabled {
        let Some(Value::Object(projects)) = root.get_mut("projects") else { return Ok(false) };
        let mut changed = false;
        for key in keys {
            let Some(Value::Object(p)) = projects.get_mut(&key) else { continue };
            let Some(entry) = p.get_mut("disabledMcpServers") else { continue };
            let Value::Array(list) = entry else { return Err(()) };
            let before = list.len();
            list.retain(|v| v.as_str() != Some(name));
            if list.len() != before {
                changed = true;
                if list.is_empty() {
                    p.shift_remove("disabledMcpServers");
                }
            }
        }
        return Ok(changed);
    }
    if !root.contains_key("projects") {
        root.insert("projects".into(), Value::Object(Map::new()));
    }
    let Some(Value::Object(projects)) = root.get_mut("projects") else { return Err(()) };
    for key in &keys {
        if let Some(Value::Object(p)) = projects.get(key) {
            match p.get("disabledMcpServers") {
                Some(Value::Array(list)) if list.iter().any(|v| v.as_str() == Some(name)) => return Ok(false),
                Some(Value::Array(_)) | None => {}
                Some(_) => return Err(()),
            }
        }
    }
    let target = keys.first().cloned().unwrap_or_else(|| dir.replace('\\', "/"));
    let slot = projects.entry(target).or_insert_with(|| Value::Object(Map::new()));
    let Value::Object(p) = slot else { return Err(()) };
    let list = p.entry("disabledMcpServers").or_insert_with(|| Value::Array(Vec::new()));
    let Value::Array(list) = list else { return Err(()) };
    list.push(Value::String(name.to_string()));
    Ok(true)
}

fn mcp_state_holds(root: &Map<String, Value>, dir: &str, name: &str, enabled: bool, win: bool) -> bool {
    let off = disabled_names(root, dir, win).iter().any(|n| n == name);
    off != enabled
}

impl ExtService {
    pub fn project(&self, dir_arg: &Value, refresh: bool) -> Value {
        let DirArg::Dir(dir) = check_dir(dir_arg) else { return fail("badArg") };
        let win = self.sys.win;
        let claude_json = self.sys.claude_json();
        let configured = status::read_configured(&claude_json, Some(&dir), win);
        let root = read_map(&claude_json);
        let disabled = disabled_names(&root, &dir, win);
        if refresh {
            let _ = self.mcp(Some(&dir), true);
        }
        let mut names: Vec<String> = configured.iter().map(|(n, _)| n.clone()).collect();
        let cached: Vec<String> = self
            .mcp
            .lock()
            .unwrap()
            .get(&dir)
            .and_then(|v| v.get("servers").and_then(Value::as_array).cloned())
            .unwrap_or_default()
            .iter()
            .filter_map(|r| r.get("name").and_then(Value::as_str).map(str::to_string))
            .collect();
        for n in cached.into_iter().chain(disabled.iter().cloned()) {
            if !names.contains(&n) {
                names.push(n);
            }
        }
        let mcp: Vec<Value> = names
            .iter()
            .map(|n| {
                let mut row = Map::new();
                row.insert("name".into(), Value::String(n.clone()));
                row.insert("scope".into(), Value::String(scope_of(n, &configured).into()));
                row.insert("enabled".into(), Value::Bool(!disabled.contains(n)));
                if n == LOCKED {
                    row.insert("locked".into(), Value::Bool(true));
                }
                Value::Object(row)
            })
            .collect();
        let user = plugin_map(&read_map(&self.sys.settings_file()));
        let local = plugin_map(&read_map(&PathBuf::from(&dir).join(".claude").join("settings.local.json")));
        let mut plugins: Vec<Value> = Vec::new();
        for (name, on) in &user {
            let over = local.iter().find(|(n, _)| n == name).map(|(_, b)| *b);
            if *on || over.is_some() {
                plugins.push(json!({"name": name, "enabled": over.unwrap_or(*on), "scope": if over.is_some() { "project" } else { "user" }}));
            }
        }
        for (name, on) in &local {
            if !user.iter().any(|(n, _)| n == name) {
                plugins.push(json!({"name": name, "enabled": on, "scope": "project"}));
            }
        }
        json!({"ok": true, "dir": dir, "mcp": mcp, "plugins": plugins})
    }

    pub fn project_toggle(&self, req: &Value) -> Value {
        let Some(_busy) = self.take() else { return fail("busy") };
        let Some(o) = req.as_object() else { return fail("badArg") };
        let DirArg::Dir(dir) = check_dir(o.get("dir").unwrap_or(&Value::Null)) else { return fail("badArg") };
        let kind = o.get("kind").and_then(Value::as_str);
        let Some(name) = o.get("name").and_then(Value::as_str).filter(|n| valid_name(n)) else { return fail("badArg") };
        let Some(enabled) = o.get("enabled").and_then(Value::as_bool) else { return fail("badArg") };
        match kind {
            Some("mcp") => {
                if name == LOCKED {
                    return fail("locked");
                }
                self.toggle_mcp(&dir, name, enabled)
            }
            Some("plugin") => self.toggle_plugin(&dir, name, enabled),
            _ => fail("badArg"),
        }
    }

    fn toggle_mcp(&self, dir: &str, name: &str, enabled: bool) -> Value {
        let file = self.sys.claude_json();
        let win = self.sys.win;
        for attempt in 0..2u32 {
            let Load::Ok(mut root, newline) = load(&file) else { return fail("io") };
            match edit_mcp(&mut root, dir, name, enabled, win) {
                Err(()) => return fail("io"),
                Ok(false) => {}
                Ok(true) => {
                    if !claude::write_atomic(&file, &render(root, newline), true) {
                        return fail("io");
                    }
                }
            }
            (self.settle)(attempt);
            if let Load::Ok(root, _) = load(&file) {
                if mcp_state_holds(&root, dir, name, enabled, win) {
                    return json!({"ok": true});
                }
            }
        }
        fail("conflict")
    }

    fn toggle_plugin(&self, dir: &str, name: &str, enabled: bool) -> Value {
        let file = PathBuf::from(dir).join(".claude").join("settings.local.json");
        let (mut root, newline) = match load(&file) {
            Load::Missing => (Map::new(), true),
            Load::Bad => return fail("io"),
            Load::Ok(m, nl) => (m, nl),
        };
        if enabled {
            let mut changed = false;
            match root.get_mut("enabledPlugins") {
                Some(Value::Object(ep)) => {
                    changed = ep.shift_remove(name).is_some();
                    if changed && ep.is_empty() {
                        root.shift_remove("enabledPlugins");
                    }
                }
                Some(_) => return fail("io"),
                None => {}
            }
            if !changed {
                return json!({"ok": true});
            }
        } else {
            if !root.contains_key("enabledPlugins") {
                root.insert("enabledPlugins".into(), Value::Object(Map::new()));
            }
            let Some(Value::Object(ep)) = root.get_mut("enabledPlugins") else { return fail("io") };
            if ep.get(name) == Some(&Value::Bool(false)) {
                return json!({"ok": true});
            }
            ep.insert(name.to_string(), Value::Bool(false));
        }
        if claude::write_with_backup(&file, &render(root, newline)) {
            json!({"ok": true})
        } else {
            fail("io")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::claude::Sys;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    struct Env {
        root: PathBuf,
        cfg: PathBuf,
        proj: String,
        svc: ExtService,
    }

    fn env(name: &str, win: bool) -> Env {
        let root = std::env::temp_dir().join(format!("ccm-ext-proj-{name}-{}", crate::util::uuid()));
        let cfg = root.join("cfg");
        let proj = root.join("proj");
        std::fs::create_dir_all(&cfg).unwrap();
        std::fs::create_dir_all(&proj).unwrap();
        let DirArg::Dir(real) = check_dir(&json!(proj.to_string_lossy())) else { panic!("dir") };
        let sys = Sys { home: root.join("home").to_string_lossy().to_string(), config_dir: Some(cfg.to_string_lossy().to_string()), win };
        let mut svc = ExtService::new(sys, Vec::new(), None);
        svc.settle = Box::new(|_| {});
        Env { root, cfg, proj: real, svc }
    }

    impl Env {
        fn key(&self) -> String {
            self.proj.replace('\\', "/")
        }
        fn cj(&self) -> PathBuf {
            self.cfg.join(".claude.json")
        }
        fn local(&self) -> PathBuf {
            PathBuf::from(&self.proj).join(".claude").join("settings.local.json")
        }
        fn put(&self, file: &Path, v: Value, nl: bool) -> String {
            let text = render(v.as_object().unwrap().clone(), nl);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, &text).unwrap();
            text
        }
        fn toggle(&self, kind: &str, name: &str, enabled: bool) -> Value {
            self.svc.project_toggle(&json!({"dir": self.proj, "kind": kind, "name": name, "enabled": enabled}))
        }
        fn read(&self, file: &Path) -> String {
            std::fs::read_to_string(file).unwrap()
        }
    }

    impl Drop for Env {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn base(key: &str, project: Value) -> Value {
        json!({"numStartups": 7, "mcpServers": {"figma": {"command": "x", "env": {"K": "secret"}}}, "projects": {"/other": {"disabledMcpServers": ["z"], "x": 1}, key: project}, "tail": [1, 2, {"a": null}]})
    }

    #[test]
    fn reads_names_scopes_and_plugins_without_secrets() {
        let e = env("read", false);
        e.put(&e.cj(), base(&e.key(), json!({"mcpServers": {"loc": {"command": "c"}}, "disabledMcpServers": ["figma", "claude.ai Google Drive"]})), true);
        e.put(&e.svc.sys.settings_file(), json!({"enabledPlugins": {"a@m": true, "b@m": false, "c@m": true}}), true);
        e.put(&e.local(), json!({"enabledPlugins": {"c@m": false, "d@m": true}}), true);
        e.svc.mcp.lock().unwrap().insert(e.proj.clone(), json!({"at": 1, "servers": [{"name": "figma"}, {"name": "plugin:github:github"}, {"name": "claude.ai Gmail"}, {"name": "orrery"}]}));
        let out = e.svc.project(&json!(e.proj), false);
        assert_eq!(out["ok"], true);
        assert_eq!(out["dir"], json!(e.proj));
        assert_eq!(
            out["mcp"],
            json!([
                {"name": "figma", "scope": "user", "enabled": false},
                {"name": "loc", "scope": "local", "enabled": true},
                {"name": "plugin:github:github", "scope": "plugin", "enabled": true},
                {"name": "claude.ai Gmail", "scope": "connector", "enabled": true},
                {"name": "orrery", "scope": "user", "enabled": true, "locked": true},
                {"name": "claude.ai Google Drive", "scope": "connector", "enabled": false},
            ])
        );
        assert_eq!(out["plugins"], json!([{"name": "a@m", "enabled": true, "scope": "user"}, {"name": "c@m", "enabled": false, "scope": "project"}, {"name": "d@m", "enabled": true, "scope": "project"}]));
        let text = out.to_string();
        assert!(!text.contains("secret") && !text.contains("command"));
        assert_eq!(e.svc.project(&json!("rel"), false), json!({"ok": false, "code": "badArg"}));
        assert_eq!(e.svc.project(&Value::Null, false)["code"], "badArg");
    }

    #[test]
    fn disable_and_enable_roundtrip_keeps_everything_else() {
        let e = env("rt", false);
        let original = e.put(&e.cj(), base(&e.key(), json!({"history": [], "allowedTools": ["a"]})), true);
        assert_eq!(e.toggle("mcp", "figma", false), json!({"ok": true}));
        let after: Value = serde_json::from_str(&e.read(&e.cj())).unwrap();
        assert_eq!(after["projects"][e.key().as_str()]["disabledMcpServers"], json!(["figma"]));
        assert_eq!(after["projects"]["/other"], json!({"disabledMcpServers": ["z"], "x": 1}));
        assert_eq!(after["tail"], json!([1, 2, {"a": null}]));
        assert!(e.read(&e.cj()).ends_with('\n'));
        assert!(e.cfg.join(".claude.json.ccm-backup").exists());
        assert_eq!(e.toggle("mcp", "figma", false), json!({"ok": true}));
        assert_eq!(e.toggle("mcp", "claude.ai Google Drive", false), json!({"ok": true}));
        assert_eq!(e.toggle("mcp", "figma", true), json!({"ok": true}));
        assert_eq!(e.toggle("mcp", "claude.ai Google Drive", true), json!({"ok": true}));
        assert_eq!(e.read(&e.cj()), original);
    }

    #[test]
    fn missing_project_entry_is_created_with_forward_slashes_and_no_trailing_newline_is_kept() {
        let e = env("new", false);
        let text = render(json!({"projects": {}}).as_object().unwrap().clone(), false);
        std::fs::write(e.cj(), &text).unwrap();
        assert_eq!(e.toggle("mcp", "grapuco", false), json!({"ok": true}));
        let raw = e.read(&e.cj());
        assert!(!raw.ends_with('\n'));
        let v: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(v["projects"][e.key().as_str()], json!({"disabledMcpServers": ["grapuco"]}));
        assert_eq!(e.toggle("mcp", "grapuco", true), json!({"ok": true}));
        let v: Value = serde_json::from_str(&e.read(&e.cj())).unwrap();
        assert_eq!(v["projects"][e.key().as_str()], json!({}));
        let none = env("none", false);
        std::fs::write(none.cj(), "{}").unwrap();
        assert_eq!(none.toggle("mcp", "x", true), json!({"ok": true}));
        assert_eq!(none.read(&none.cj()), "{}");
    }

    #[test]
    fn windows_key_matches_case_and_slashes() {
        let e = env("win", true);
        let odd = e.proj.replace('/', "\\").to_uppercase();
        e.put(&e.cj(), json!({"projects": {odd.clone(): {"disabledMcpServers": ["a"]}}}), true);
        let out = e.svc.project(&json!(e.proj), false);
        assert_eq!(out["mcp"], json!([{"name": "a", "scope": "user", "enabled": false}]));
        assert_eq!(e.toggle("mcp", "b", false), json!({"ok": true}));
        let v: Value = serde_json::from_str(&e.read(&e.cj())).unwrap();
        assert_eq!(v["projects"].as_object().unwrap().len(), 1);
        assert_eq!(v["projects"][odd.as_str()]["disabledMcpServers"], json!(["a", "b"]));
        assert_eq!(e.toggle("mcp", "a", true), json!({"ok": true}));
        let v: Value = serde_json::from_str(&e.read(&e.cj())).unwrap();
        assert_eq!(v["projects"][odd.as_str()]["disabledMcpServers"], json!(["b"]));
    }

    #[test]
    fn plugin_override_set_and_clear() {
        let e = env("plug", false);
        assert_eq!(e.toggle("plugin", "a@m", false), json!({"ok": true}));
        assert_eq!(e.read(&e.local()), "{\n  \"enabledPlugins\": {\n    \"a@m\": false\n  }\n}\n");
        assert_eq!(e.toggle("plugin", "a@m", false), json!({"ok": true}));
        e.put(&e.local(), json!({"model": "x", "enabledPlugins": {"a@m": false, "b@m": false}}), true);
        assert_eq!(e.toggle("plugin", "a@m", true), json!({"ok": true}));
        let v: Value = serde_json::from_str(&e.read(&e.local())).unwrap();
        assert_eq!(v, json!({"model": "x", "enabledPlugins": {"b@m": false}}));
        assert_eq!(e.toggle("plugin", "b@m", true), json!({"ok": true}));
        let v: Value = serde_json::from_str(&e.read(&e.local())).unwrap();
        assert_eq!(v, json!({"model": "x"}));
        assert_eq!(e.toggle("plugin", "never@m", true), json!({"ok": true}));
        std::fs::write(e.local(), "{ nope").unwrap();
        assert_eq!(e.toggle("plugin", "a@m", false), json!({"ok": false, "code": "io"}));
        assert_eq!(e.read(&e.local()), "{ nope");
    }

    #[test]
    fn rejects_locked_bad_args_busy_and_unparsable_file() {
        let e = env("bad", false);
        let text = e.put(&e.cj(), json!({"projects": {}}), true);
        assert_eq!(e.toggle("mcp", "orrery", false), json!({"ok": false, "code": "locked"}));
        let bad = |req: Value| e.svc.project_toggle(&req);
        assert_eq!(bad(json!(null))["code"], "badArg");
        assert_eq!(bad(json!({"dir": "rel", "kind": "mcp", "name": "a", "enabled": false}))["code"], "badArg");
        assert_eq!(bad(json!({"kind": "mcp", "name": "a", "enabled": false}))["code"], "badArg");
        assert_eq!(bad(json!({"dir": e.proj, "kind": "x", "name": "a", "enabled": false}))["code"], "badArg");
        assert_eq!(bad(json!({"dir": e.proj, "kind": "mcp", "name": "", "enabled": false}))["code"], "badArg");
        assert_eq!(bad(json!({"dir": e.proj, "kind": "mcp", "name": "-x", "enabled": false}))["code"], "badArg");
        assert_eq!(bad(json!({"dir": e.proj, "kind": "mcp", "name": "a\nb", "enabled": false}))["code"], "badArg");
        assert_eq!(bad(json!({"dir": e.proj, "kind": "mcp", "name": "a".repeat(201), "enabled": false}))["code"], "badArg");
        assert_eq!(bad(json!({"dir": e.proj, "kind": "mcp", "name": "a", "enabled": "no"}))["code"], "badArg");
        assert_eq!(e.read(&e.cj()), text);
        let held = e.svc.take().unwrap();
        assert_eq!(e.toggle("mcp", "a", false), json!({"ok": false, "code": "busy"}));
        drop(held);
        std::fs::write(e.cj(), "{ broken").unwrap();
        assert_eq!(e.toggle("mcp", "a", false), json!({"ok": false, "code": "io"}));
        assert_eq!(e.read(&e.cj()), "{ broken");
        std::fs::remove_file(e.cj()).unwrap();
        assert_eq!(e.toggle("mcp", "a", false), json!({"ok": false, "code": "io"}));
        assert!(!e.cj().exists());
    }

    #[test]
    fn race_with_foreign_write_is_retried_once_then_reported() {
        let mut e = env("race", false);
        let original = e.put(&e.cj(), base(&e.key(), json!({})), true);
        let file = e.cj();
        let calls = Arc::new(AtomicU32::new(0));
        let (c, f, o) = (calls.clone(), file.clone(), original.clone());
        e.svc.settle = Box::new(move |attempt| {
            c.fetch_add(1, Ordering::SeqCst);
            if attempt == 0 {
                std::fs::write(&f, &o).unwrap();
            }
        });
        assert_eq!(e.toggle("mcp", "figma", false), json!({"ok": true}));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        assert!(e.read(&file).contains("disabledMcpServers"));
        std::fs::write(&file, &original).unwrap();
        let (f, o) = (file.clone(), original.clone());
        e.svc.settle = Box::new(move |_| std::fs::write(&f, &o).unwrap());
        assert_eq!(e.toggle("mcp", "figma", false), json!({"ok": false, "code": "conflict"}));
    }
}
