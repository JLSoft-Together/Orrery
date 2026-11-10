use super::model::{self, FlatHook};
use crate::claude::{self, diff, mask::mask_hook_text, norm_file, real, Obj, Sys};
use crate::js;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const HOOKS_STASH_FILE: &str = "hooks-disabled.json";
const JSON_MAX: u64 = 512 * 1024;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileState {
    Ok,
    None,
    Unreadable,
}

impl FileState {
    pub fn name(self) -> &'static str {
        match self {
            FileState::Ok => "ok",
            FileState::None => "none",
            FileState::Unreadable => "unreadable",
        }
    }
}

#[derive(Debug, Clone)]
pub struct JsonRead {
    pub state: FileState,
    pub text: String,
    pub data: Option<Obj>,
    pub mtime: f64,
}

pub fn read_json(file: &Path) -> JsonRead {
    let bad = |text: String, mtime: f64| JsonRead { state: FileState::Unreadable, text, data: None, mtime };
    let meta = match std::fs::metadata(file) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return JsonRead { state: FileState::None, text: String::new(), data: Some(Obj::new()), mtime: 0.0 },
        Err(_) => return bad(String::new(), 0.0),
    };
    if !meta.is_file() || meta.len() > JSON_MAX {
        return bad(String::new(), 0.0);
    }
    let mtime = crate::util::mtime_ms(&meta);
    let Ok(bytes) = std::fs::read(file) else { return bad(String::new(), 0.0) };
    let text = String::from_utf8_lossy(&bytes).into_owned();
    if claude::js_trim(&text).is_empty() {
        return JsonRead { state: FileState::Ok, text, data: Some(Obj::new()), mtime };
    }
    match claude::parse_text(text.strip_prefix('\u{feff}').unwrap_or(&text)) {
        Some(Value::Object(m)) if m.get("hooks").is_none_or(Value::is_object) => JsonRead { state: FileState::Ok, text, data: Some(m), mtime },
        _ => bad(text, mtime),
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Source {
    User,
    Project,
    Local,
    Plugin,
}

impl Source {
    pub fn name(self) -> &'static str {
        match self {
            Source::User => "user",
            Source::Project => "project",
            Source::Local => "local",
            Source::Plugin => "plugin",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Found {
    pub row: Value,
    pub flat: FlatHook,
    pub file: String,
    pub plugin_root: Option<String>,
}

impl Found {
    pub fn id(&self) -> &str {
        self.row["id"].as_str().unwrap_or("")
    }
    pub fn enabled(&self) -> bool {
        self.row["enabled"] == Value::Bool(true)
    }
}

pub struct Collected {
    pub found: Vec<Found>,
    pub files: Vec<(Source, FileState)>,
    pub disable_all: bool,
}

pub struct Ctx<'a> {
    pub sys: &'a Sys,
    pub data_dir: &'a Path,
}

impl Ctx<'_> {
    pub fn stash_file(&self) -> PathBuf {
        self.data_dir.join(HOOKS_STASH_FILE)
    }

    fn files(&self, dir: Option<&str>) -> Vec<(Source, String)> {
        let root = crate::util::path_str(&self.sys.claude_root());
        let mut out = vec![(Source::User, claude::join(&root, &["settings.json"]))];
        if let Some(d) = dir {
            out.push((Source::Project, claude::join(d, &[".claude", "settings.json"])));
            out.push((Source::Local, claude::join(d, &[".claude", "settings.local.json"])));
        }
        out
    }

    pub fn read_stash(&self) -> Vec<Value> {
        let r = read_json(&self.stash_file());
        let list = r.data.and_then(|d| d.get("entries").and_then(Value::as_array).cloned()).unwrap_or_default();
        list.into_iter()
            .filter(|e| {
                let Some(o) = e.as_object() else { return false };
                o.get("key").is_some_and(Value::is_string)
                    && o.get("file").is_some_and(Value::is_string)
                    && o.get("event").is_some_and(Value::is_string)
                    && o.get("matcher").is_some_and(|m| m.is_null() || m.is_string())
                    && o.get("hook").is_some_and(Value::is_object)
            })
            .collect()
    }

    pub fn write_stash(&self, entries: &[Value]) -> bool {
        let text = format!("{}\n", js::stringify_indent(&json!({"version": 1, "entries": entries}), "  "));
        claude::write_atomic(&self.stash_file(), &text, false)
    }

    pub fn row(&self, file: &str, flat: &FlatHook, source: Source, enabled: bool, plugin: Option<(&str, bool)>) -> Value {
        let orrery = model::is_orrery_hook(&flat.hook);
        let ty = flat.hook.get("type").and_then(Value::as_str).unwrap_or("unknown").to_string();
        let text = mask_hook_text(&model::hook_text(&flat.hook), &self.sys.home, self.sys.platform());
        let mut pairs = vec![
            ("id", Value::String(model::hook_key(file, flat, self.sys.win))),
            ("event", Value::String(flat.event.clone())),
            ("matcher", flat.matcher.clone().map(Value::String).unwrap_or(Value::Null)),
            ("type", Value::String(ty.clone())),
            ("text", Value::String(js::slice(&text, 2000))),
            ("timeout", flat.hook.get("timeout").filter(|t| t.is_number()).cloned().unwrap_or(Value::Null)),
            ("async", Value::Bool(flat.hook.get("async") == Some(&Value::Bool(true)))),
            ("source", Value::String(source.name().into())),
            ("orrery", Value::Bool(orrery)),
            ("enabled", Value::Bool(enabled)),
            ("toggleable", Value::Bool(source != Source::Plugin && !orrery)),
            ("testable", Value::Bool(!orrery && ty == "command" && flat.hook.get("command").is_some_and(Value::is_string))),
        ];
        if let Some((id, on)) = plugin {
            pairs.push(("plugin", Value::String(id.into())));
            pairs.push(("pluginEnabled", Value::Bool(on)));
        }
        model::obj(pairs)
    }

    fn plugin_hooks(&self, dir: Option<&str>, enabled: &[(String, bool)]) -> Vec<Found> {
        let win = self.sys.win;
        let plugins_dir = claude::join(&crate::util::path_str(&self.sys.claude_root()), &["plugins"]);
        let real_plugins = real(&plugins_dir);
        let installed = read_json(Path::new(&claude::join(&plugins_dir, &["installed_plugins.json"])));
        let plugins = installed.data.and_then(|d| d.get("plugins").and_then(Value::as_object).cloned()).unwrap_or_default();
        let mut out = Vec::new();
        for (id, entries) in js::entries(&plugins) {
            let Some(entries) = entries.as_array() else { continue };
            let entry = entries.iter().filter_map(Value::as_object).find(|e| {
                e.get("installPath").is_some_and(Value::is_string)
                    && (e.get("scope").and_then(Value::as_str) == Some("user")
                        || dir.is_some_and(|d| e.get("projectPath").and_then(Value::as_str).is_some_and(|p| claude::same_dir(p, d, win))))
            });
            let (Some(entry), Some(real_plugins)) = (entry, real_plugins.as_deref()) else { continue };
            let Some(real_dir) = real(entry["installPath"].as_str().unwrap_or("")) else { continue };
            if !claude::is_within(real_plugins, &real_dir, win) {
                continue;
            }
            let on = enabled.iter().rev().find(|(k, _)| k == id).map(|(_, v)| *v) != Some(false);
            let mut sources = vec![claude::join(&real_dir, &["hooks", "hooks.json"])];
            let manifest_file = claude::join(&real_dir, &[".claude-plugin", "plugin.json"]);
            let manifest = read_json(Path::new(&manifest_file));
            let inline = manifest.data.as_ref().and_then(|d| d.get("hooks")).cloned();
            if let Some(Value::String(r)) = &inline {
                sources.push(claude::join(&real_dir, &[r.as_str()]));
            }
            if let Some(Value::Array(a)) = &inline {
                for r in a.iter().filter_map(Value::as_str) {
                    sources.push(claude::join(&real_dir, &[r]));
                }
            }
            let mut add = |file: &str, hooks: Option<&Value>| {
                for flat in model::flatten_hooks(hooks) {
                    let row = self.row(file, &flat, Source::Plugin, on, Some((id, on)));
                    out.push(Found { row, flat, file: file.to_string(), plugin_root: Some(real_dir.clone()) });
                }
            };
            if let Some(Value::Object(inl)) = &inline {
                let hooks = match inl.get("hooks") {
                    Some(h @ Value::Object(_)) => h.clone(),
                    _ => Value::Object(inl.clone()),
                };
                add(&manifest_file, Some(&hooks));
            }
            let mut seen: Vec<String> = Vec::new();
            for file in sources {
                let key = norm_file(&file, win);
                if !claude::is_within(&real_dir, &file, win) || seen.contains(&key) {
                    continue;
                }
                seen.push(key);
                let r = read_json(Path::new(&file));
                if r.state == FileState::Ok {
                    add(&file, r.data.as_ref().and_then(|d| d.get("hooks")));
                }
            }
        }
        out
    }

    pub fn collect(&self, dir: Option<&str>) -> Collected {
        let win = self.sys.win;
        let files = self.files(dir);
        let reads: Vec<JsonRead> = files.iter().map(|(_, f)| read_json(Path::new(f))).collect();
        let mut found = Vec::new();
        let mut enabled: Vec<(String, bool)> = Vec::new();
        let mut disable_all = false;
        for ((source, file), r) in files.iter().zip(&reads) {
            let (FileState::Ok, Some(data)) = (r.state, &r.data) else { continue };
            if data.get("disableAllHooks") == Some(&Value::Bool(true)) {
                disable_all = true;
            }
            if let Some(Value::Object(ep)) = data.get("enabledPlugins") {
                for (id, on) in js::entries(ep) {
                    if let Value::Bool(b) = on {
                        enabled.push((id.clone(), *b));
                    }
                }
            }
            for flat in model::flatten_hooks(data.get("hooks")) {
                let row = self.row(file, &flat, *source, true, None);
                found.push(Found { row, flat, file: file.clone(), plugin_root: None });
            }
        }
        let stash = self.read_stash();
        for ((source, file), r) in files.iter().zip(&reads) {
            for e in &stash {
                if norm_file(e["file"].as_str().unwrap_or(""), win) != norm_file(file, win) {
                    continue;
                }
                let flat = FlatHook {
                    event: e["event"].as_str().unwrap_or("").to_string(),
                    matcher: e["matcher"].as_str().map(str::to_string),
                    hook: e["hook"].as_object().cloned().unwrap_or_default(),
                };
                if r.data.as_ref().is_some_and(|d| model::has_hook_entry(d, &flat)) {
                    continue;
                }
                let row = self.row(file, &flat, *source, false, None);
                found.push(Found { row, flat, file: file.clone(), plugin_root: None });
            }
        }
        found.extend(self.plugin_hooks(dir, &enabled));
        found.sort_by(|a, b| {
            let (ea, eb) = (a.row["event"].as_str().unwrap_or(""), b.row["event"].as_str().unwrap_or(""));
            model::event_rank(ea).cmp(&model::event_rank(eb)).then_with(|| core_transcript::collate::locale_cmp(ea, eb))
        });
        let states = files.iter().zip(&reads).map(|((s, _), r)| (*s, r.state)).collect();
        Collected { found, files: states, disable_all }
    }

    pub fn list(&self, dir: Option<&str>) -> Value {
        let c = self.collect(dir);
        let rows: Vec<Value> = c.found.into_iter().map(|f| f.row).collect();
        let files: Vec<Value> = c.files.iter().map(|(s, st)| json!({"source": s.name(), "state": st.name()})).collect();
        json!({"ok": true, "dir": dir, "rows": rows, "files": files, "disableAll": c.disable_all})
    }

    pub fn file_allowed(&self, file: &str, dir: Option<&str>) -> bool {
        let win = self.sys.win;
        let root = crate::util::path_str(&self.sys.claude_root());
        let base = match dir {
            Some(d) if claude::is_within(d, file, win) => d.to_string(),
            _ => root,
        };
        let real_base = real(&base).unwrap_or(base);
        for p in [claude::dirname(file), file.to_string()] {
            if let Some(r) = real(&p) {
                if !claude::is_within(&real_base, &r, win) {
                    return false;
                }
            }
        }
        true
    }

    pub fn toggle_diff(&self, before: &str, after: &str) -> String {
        diff::json_diff(before, after).split('\n').map(|l| mask_hook_text(l, &self.sys.home, self.sys.platform())).collect::<Vec<_>>().join("\n")
    }
}
