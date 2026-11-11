use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use core_desk::preview::changes::toplevel;
use core_desk::preview::files::real_roots;
use core_desk::preview::paths::{is_absolute, real_or_null, root_key, root_of};
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use crate::hosts;
use crate::preview_files::FileListCache;
use crate::preview_grants::{is_fs_root, FullAccess, GrantStore, FULL_ACCESS_FILE, GRANTS_FILE};
use crate::preview_text::confirm_spec;
use crate::state::{lock, Shell};

const MAX_FILE_LEN: usize = 4096;
const SENDER: u64 = 1;
const DRIVES_TTL: Duration = Duration::from_secs(30);

pub type Roots<'a> = &'a dyn Fn(bool) -> Vec<String>;

#[derive(Clone)]
struct Repo {
    root: String,
    from: String,
}

pub struct Gate {
    repos: Mutex<HashMap<String, Repo>>,
    pub grants: GrantStore,
    pub full: FullAccess,
    pub files: FileListCache,
    pub asking: Mutex<()>,
    drives: Mutex<Option<(Instant, Vec<String>)>>,
}

pub fn field<'a>(p: &'a Value, key: &str) -> &'a Value {
    p.get(key).unwrap_or(&Value::Null)
}

pub fn text(p: &Value, key: &str) -> String {
    match field(p, key) {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

pub fn is_dir(p: &str) -> bool {
    std::fs::metadata(p).is_ok_and(|m| m.is_dir())
}

fn is_file(p: &str) -> bool {
    std::fs::metadata(p).is_ok_and(|m| m.is_file())
}

pub fn claude_config_dir() -> String {
    if let Some(dir) = std::env::var("CLAUDE_CONFIG_DIR").ok().filter(|v| !v.is_empty()) {
        return dir;
    }
    let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).map_or_else(PathBuf::new, PathBuf::from);
    home.join(".claude").to_string_lossy().into_owned()
}

pub fn caps(claude_dir: &str) -> Value {
    json!({
        "trash": true,
        "confirmGit": true,
        "confirmRun": true,
        "confirmReplace": true,
        "confirmPermRules": true,
        "confirmDenyRules": true,
        "github": true,
        "history": true,
        "claudeConfigDir": claude_dir,
    })
}

impl Gate {
    pub fn new(grants: GrantStore, full: FullAccess) -> Gate {
        Gate { repos: Mutex::new(HashMap::new()), grants, full, files: FileListCache::default(), asking: Mutex::new(()), drives: Mutex::new(None) }
    }

    pub fn in_memory() -> Gate {
        Gate::new(GrantStore::load(None), FullAccess::load(None))
    }

    pub fn load(data_dir: &Path) -> Gate {
        Gate::new(GrantStore::load(Some(data_dir.join(GRANTS_FILE))), FullAccess::load(Some(data_dir.join(FULL_ACCESS_FILE))))
    }

    fn filesystem_roots(&self) -> Vec<String> {
        if !cfg!(windows) {
            return vec!["/".into()];
        }
        let mut cache = lock(&self.drives);
        if let Some((_, list)) = cache.as_ref().filter(|(at, _)| at.elapsed() < DRIVES_TTL) {
            return list.clone();
        }
        let list: Vec<String> = (b'A'..=b'Z').map(|l| format!("{}:\\", l as char)).filter(|r| Path::new(r).exists()).collect();
        *cache = Some((Instant::now(), list.clone()));
        list
    }

    pub fn allowed_roots(&self, roots: Roots, fresh: bool) -> Vec<String> {
        let mut all = roots(fresh);
        all.extend(self.grants.list());
        if self.full.get() {
            all.extend(self.filesystem_roots());
        }
        real_roots(&all)
    }

    pub fn allowed(&self, roots: Roots, path: &Value, fresh: bool) -> Option<(String, Vec<String>)> {
        let p = path.as_str().filter(|p| is_absolute(p))?;
        let all = self.allowed_roots(roots, fresh);
        let real = real_or_null(p)?;
        root_of(&real, &all).map(|_| (real, all))
    }

    pub fn repo_of(&self, roots: Roots, root: &Value) -> Option<String> {
        let real = real_or_null(root.as_str().filter(|r| is_absolute(r))?)?;
        let repo = lock(&self.repos).get(&root_key(&real)).cloned()?;
        self.allowed(roots, &json!(repo.from), false).map(|_| repo.root)
    }

    pub fn register_repo(&self, roots: Roots, dir: &Value) -> Value {
        if let Some(known) = self.repo_of(roots, dir) {
            return json!(known);
        }
        let Some((real, _)) = self.allowed(roots, dir, false).or_else(|| self.allowed(roots, dir, true)) else { return json!({"error": "denied"}) };
        match toplevel(&real) {
            Ok(top) => {
                self.remember_repo(&top, &real);
                json!(top)
            }
            Err(e) => json!({"error": e}),
        }
    }

    pub fn remember_repo(&self, top: &str, from: &str) {
        lock(&self.repos).insert(root_key(top), Repo { root: top.to_owned(), from: from.to_owned() });
    }

    pub fn add_repo(&self, root: &str, from: &str) {
        if is_absolute(root) && is_absolute(from) {
            lock(&self.repos).insert(root_key(root), Repo { root: root.to_owned(), from: from.to_owned() });
        }
    }

    pub fn check_dir(&self, roots: Roots, dir: &Value) -> Option<String> {
        let real = self.repo_of(roots, dir).or_else(|| self.allowed(roots, dir, false).map(|a| a.0))?;
        is_dir(&real).then_some(real)
    }

    pub fn explorer_root(&self, roots: Roots, root: &Value) -> Option<String> {
        self.repo_of(roots, root).or_else(|| self.allowed(roots, root, false).or_else(|| self.allowed(roots, root, true)).map(|a| a.0))
    }

    pub fn write_roots(&self, roots: Roots) -> Vec<String> {
        let mut all = self.allowed_roots(roots, false);
        let extra: Vec<String> = lock(&self.repos).values().filter(|r| root_of(&r.from, &all).is_some()).map(|r| r.root.clone()).collect();
        all.extend(extra);
        all
    }

    pub fn editable_file(&self, roots: Roots, file: &Value) -> Option<(String, String)> {
        let f = file.as_str().filter(|f| f.len() <= MAX_FILE_LEN && is_absolute(f))?;
        let real = real_or_null(f)?;
        let all = self.write_roots(roots);
        let deepest = root_of(&real, &all)?;
        if !is_file(&real) {
            return None;
        }
        let wide = all.iter().filter(|r| !is_fs_root(r) && root_of(&real, std::slice::from_ref(*r)).is_some()).min_by_key(|r| r.len()).cloned();
        Some((real, wide.unwrap_or(deepest)))
    }

    pub fn answer(&self, roots: Roots, method: &str, p: &Value) -> Option<Value> {
        let or_null = |v: Option<String>| v.map_or(Value::Null, Value::String);
        Some(match method {
            "preview.repoOf" => or_null(self.repo_of(roots, field(p, "root"))),
            "preview.registerRepo" => self.register_repo(roots, field(p, "dir")),
            "preview.addRepo" => {
                self.add_repo(&text(p, "root"), &text(p, "from"));
                json!(true)
            }
            "preview.allowed" => self.allowed(roots, field(p, "path"), field(p, "fresh") == &Value::Bool(true)).map_or(Value::Null, |(real, roots)| json!({"real": real, "roots": roots})),
            "preview.checkDir" => or_null(self.check_dir(roots, field(p, "dir"))),
            "preview.explorerRoot" => or_null(self.explorer_root(roots, field(p, "root"))),
            "preview.writeRoots" => json!(self.write_roots(roots)),
            "preview.roots" => json!(self.allowed_roots(roots, false)),
            "preview.editableFile" => self.editable_file(roots, field(p, "file")).map_or(Value::Null, |(real, root)| json!({"real": real, "root": root})),
            "preview.sender" => json!(SENDER),
            "preview.invalidate" => {
                self.files.invalidate(&text(p, "root"));
                json!(true)
            }
            _ => return None,
        })
    }
}

fn trash(p: &Value) -> Value {
    let path = text(p, "path");
    if !is_absolute(&path) {
        return json!({"error": "denied"});
    }
    match trash::delete(&path) {
        Ok(()) => json!(true),
        Err(e) => json!({"error": e.to_string()}),
    }
}

fn confirm(app: &AppHandle, p: &Value) -> bool {
    let kind = field(p, "kind").as_str().unwrap_or("");
    let Some(c) = confirm_spec(&hosts::locale(app), kind, field(p, "req")) else { return false };
    hosts::confirm(app, &c.message, &c.detail, c.ok, c.cancel)
}

fn github(app: &AppHandle, channel: &str, args: Value) -> Value {
    crate::desk::call_inside(app, channel, json!([args])).unwrap_or(Value::Null)
}

pub fn pane_roots(app: &AppHandle) -> Vec<String> {
    app.state::<Shell>().pty.pane_roots()
}

pub fn call(app: &AppHandle, method: &str, p: &Value) -> Result<Value, String> {
    let gate = app.try_state::<Gate>().ok_or("preview gate not ready")?;
    let roots = |_fresh: bool| pane_roots(app);
    if let Some(v) = gate.answer(&roots, method, p) {
        return Ok(v);
    }
    let repo_url = || json!({"repo": text(p, "repo"), "url": text(p, "url")});
    match method {
        "preview.caps" => Ok(caps(&claude_config_dir())),
        "preview.trash" => Ok(trash(p)),
        "preview.confirm" => Ok(json!(confirm(app, p))),
        "preview.ghEnv" => Ok(github(app, "desk:gh-env", repo_url())),
        "preview.ghResolve" => Ok(match github(app, "desk:gh-resolve", repo_url()) {
            Value::Null => json!({"github": false}),
            v => v,
        }),
        "preview.ghSetChoice" => Ok(json!(github(app, "desk:gh-set-choice", json!({"repo": text(p, "repo"), "choice": field(p, "choice")})) == json!(true))),
        _ => Err(format!("no host for {method}")),
    }
}

#[cfg(test)]
#[path = "preview_host_tests.rs"]
mod tests;
