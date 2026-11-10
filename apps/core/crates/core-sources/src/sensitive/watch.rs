use std::collections::HashMap;
use std::path::{Path, PathBuf};

use core_store::extra::{Dict, SensitiveSettings};
use core_store::persist::json_file::JsonFile;
use core_store::time::now_ms;
use core_transcript::jsstr::units;
use core_transcript::rules::Touch;
use serde_json::Value;

use super::classify::{classify_touch, ClassifyContext, SensitiveHit};
use super::settings::{default_settings, is_sensitive_rule, sanitize_globs, sanitize_groups, sanitize_ignore, sanitize_sensitive, MAX_IGNORE_PROJECTS};

pub const SENSITIVE_FILE: &str = "sensitive.json";
const REPEAT_MS: f64 = 10.0 * 60_000.0;
const MAX_SEEN: usize = 2000;

pub type RootOf = Box<dyn Fn(&str) -> Option<String> + Send>;

pub struct WatchDeps {
    pub claude_root: PathBuf,
    pub home: String,
    pub tmp: PathBuf,
    pub win: bool,
    pub now: Option<Box<dyn Fn() -> f64 + Send>>,
    pub root_of: Option<RootOf>,
}

impl WatchDeps {
    pub fn host(claude_root: impl Into<PathBuf>) -> Self {
        Self {
            claude_root: claude_root.into(),
            home: crate::resources::sampler::home_dir(),
            tmp: std::env::temp_dir(),
            win: cfg!(windows),
            now: None,
            root_of: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WatchHit {
    pub hit: SensitiveHit,
    pub project: Option<String>,
}

pub struct SensitiveWatch {
    value: SensitiveSettings,
    seen: HashMap<String, f64>,
    file: Option<JsonFile>,
    deps: WatchDeps,
    pub on_change: Box<dyn FnMut(&SensitiveSettings) + Send>,
}

fn join(base: &Path, part: &str) -> String {
    base.join(part).to_string_lossy().into_owned()
}

impl SensitiveWatch {
    pub fn new(deps: WatchDeps, file: Option<PathBuf>) -> Self {
        Self { value: default_settings(), seen: HashMap::new(), file: file.map(|f| JsonFile::new(f, 300)), deps, on_change: Box::new(|_| {}) }
    }

    pub fn load(&mut self) {
        let raw = self.file.as_ref().and_then(JsonFile::read);
        self.value = sanitize_sensitive(raw.as_ref());
    }

    pub fn settings(&self) -> &SensitiveSettings {
        &self.value
    }

    fn now(&self) -> f64 {
        self.deps.now.as_ref().map_or_else(|| now_ms() as f64, |f| f())
    }

    pub fn update(&mut self, groups: Option<&Value>, globs: Option<&Value>, ignore: Option<&Value>) {
        let groups = groups.and_then(sanitize_groups);
        let globs = globs.and_then(sanitize_globs);
        let ignore = ignore.and_then(sanitize_ignore);
        if let Some(groups) = groups {
            for (g, b) in groups {
                self.value.groups.insert(g, b);
            }
        }
        if let Some(globs) = globs {
            self.value.globs = globs;
        }
        if let Some(ignore) = ignore {
            self.value.ignore = ignore;
        }
        self.save();
    }

    pub fn ignore(&mut self, project: &str, rule: &str, on: bool) -> bool {
        if project.is_empty() || units(project) > 1024 || !is_sensitive_rule(rule) {
            return false;
        }
        let mut ignore: Dict<Vec<String>> = self.value.ignore.clone();
        let mut list: Vec<String> = ignore.get(project).cloned().unwrap_or_default();
        if on {
            if !list.iter().any(|r| r == rule) {
                list.push(rule.to_owned());
            }
        } else {
            list.retain(|r| r != rule);
        }
        if list.is_empty() {
            ignore.shift_remove(project);
        } else {
            ignore.insert(project.to_owned(), list);
        }
        if ignore.len() > MAX_IGNORE_PROJECTS {
            return false;
        }
        self.value.ignore = ignore;
        self.save();
        true
    }

    pub fn next_due(&self) -> Option<i64> {
        self.file.as_ref().and_then(JsonFile::next_due)
    }

    pub fn poll(&mut self, now: i64) -> bool {
        let value = &self.value;
        match self.file.as_mut() {
            Some(f) => f.poll(now, || serde_json::to_value(value).unwrap_or(Value::Null)),
            None => false,
        }
    }

    pub fn flush(&mut self) -> bool {
        let value = &self.value;
        match self.file.as_mut() {
            Some(f) => f.flush_pending(|| serde_json::to_value(value).unwrap_or(Value::Null)),
            None => false,
        }
    }

    pub fn check(&mut self, terminal_id: &str, touch: &Touch, cwd: Option<&str>, at_ms: Option<f64>) -> Vec<WatchHit> {
        let cwd = cwd.filter(|c| !c.is_empty());
        let project = cwd.map(|c| self.deps.root_of.as_ref().and_then(|f| f(c)).unwrap_or_else(|| c.to_owned()));
        let safe = self.safe_dirs(cwd);
        let hits = classify_touch(
            touch,
            &ClassifyContext { project: project.as_deref(), home: &self.deps.home, win: self.deps.win, safe_dirs: &safe, settings: &self.value, project_key: cwd },
        );
        if hits.is_empty() {
            return Vec::new();
        }
        let now = at_ms.unwrap_or_else(|| self.now());
        self.prune(now);
        let mut out = Vec::new();
        for h in hits {
            let key = format!("{terminal_id}|{}|{}", h.rule, h.path.as_deref().unwrap_or(""));
            let last = self.seen.insert(key, now);
            if last.is_none_or(|l| now - l >= REPEAT_MS) {
                out.push(WatchHit { hit: h, project: cwd.map(str::to_owned) });
            }
        }
        out
    }

    fn safe_dirs(&self, cwd: Option<&str>) -> Vec<String> {
        let root = &self.deps.claude_root;
        let mut dirs = vec![join(root, "plans"), join(root, "todos"), join(&self.deps.tmp, "claude")];
        if let Some(cwd) = cwd {
            let name = crate::prompt_search::text::project_dir_name(cwd);
            dirs.push(join(&root.join("projects"), &name));
        }
        dirs
    }

    fn prune(&mut self, now: f64) {
        if self.seen.len() < MAX_SEEN {
            return;
        }
        self.seen.retain(|_, at| now - *at < REPEAT_MS);
    }

    fn save(&mut self) {
        (self.on_change)(&self.value);
        if let Some(f) = self.file.as_mut() {
            f.schedule(now_ms());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    fn watch(now: Arc<Mutex<f64>>) -> SensitiveWatch {
        let deps = WatchDeps {
            claude_root: PathBuf::from("/home/me/.claude"),
            home: "/home/me".into(),
            tmp: PathBuf::from("/tmp"),
            win: false,
            now: Some(Box::new(move || *now.lock().unwrap())),
            root_of: None,
        };
        SensitiveWatch::new(deps, None)
    }

    #[test]
    fn drops_repeats_within_ten_minutes_and_keeps_the_project() {
        let now = Arc::new(Mutex::new(0.0));
        let mut w = watch(now.clone());
        let touch = Touch::File("/home/me/app/.env".into());
        let first = w.check("t1", &touch, Some("/home/me/app"), None);
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].hit, SensitiveHit { rule: ".env*".into(), path: Some(".env".into()), file: Some("/home/me/app/.env".into()) });
        assert_eq!(first[0].project.as_deref(), Some("/home/me/app"));
        assert!(w.check("t1", &touch, Some("/home/me/app"), None).is_empty());
        *now.lock().unwrap() = 11.0 * 60_000.0;
        assert_eq!(w.check("t1", &touch, Some("/home/me/app"), None).len(), 1);
        assert!(w.ignore("/home/me/app", ".env*", true));
        *now.lock().unwrap() = 30.0 * 60_000.0;
        assert!(w.check("t1", &touch, Some("/home/me/app"), None).is_empty());
        assert!(!w.ignore("/home/me/app", "bogus", true));
        assert!(w.ignore("/home/me/app", ".env*", false));
        assert!(w.settings().ignore.is_empty());
    }

    #[test]
    fn the_session_plan_and_memory_folders_are_not_outside() {
        let mut w = watch(Arc::new(Mutex::new(0.0)));
        let plan = Touch::File("/home/me/.claude/plans/p.md".into());
        assert!(w.check("t", &plan, Some("/home/me/app"), None).is_empty());
        let mem = Touch::File("/home/me/.claude/projects/-home-me-app/memory/a.md".into());
        assert!(w.check("t", &mem, Some("/home/me/app"), None).is_empty());
        let other = Touch::File("/etc/hosts".into());
        assert_eq!(w.check("t", &other, Some("/home/me/app"), None)[0].hit.rule, "outside-project");
    }

    #[test]
    fn settings_persist_with_a_debounce_and_load_sanitized() {
        let dir = std::env::temp_dir().join(format!("ccm-sens-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let file = dir.join(SENSITIVE_FILE);
        let mut w = SensitiveWatch::new(WatchDeps::host(dir.join("claude")), Some(file.clone()));
        w.update(Some(&serde_json::json!({"git": false, "x": true})), Some(&serde_json::json!(["*.k", "*.k"])), None);
        assert!(w.next_due().is_some());
        assert!(w.flush());
        let text = std::fs::read_to_string(&file).unwrap();
        assert_eq!(text, r#"{"groups":{"secrets":true,"outside":true,"git":false,"delete":true,"data":true,"custom":true},"globs":["*.k"],"ignore":{}}"#);
        let mut w2 = SensitiveWatch::new(WatchDeps::host(dir.join("claude")), Some(file));
        w2.load();
        assert_eq!(w2.settings().globs, vec!["*.k".to_owned()]);
        assert_eq!(w2.settings().groups.get("git"), Some(&false));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
