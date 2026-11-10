use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use regex::Regex;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

const MAX_DIRS: usize = 64;
const DEBOUNCE_MS: u64 = 500;

pub fn detect_file() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^(settings\.gradle(\.kts)?|build\.gradle(\.kts)?|libs\.versions\.toml|AndroidManifest\.xml|pubspec\.(yaml|lock)|\.metadata|\.fvmrc|fvm_config\.json|package_config\.json|main[A-Za-z0-9_.-]*\.dart)$").expect("regex"))
}

fn join(a: &str, b: &str) -> String {
    if b.is_empty() {
        a.to_string()
    } else {
        crate::preview::paths::resolve_str(&format!("{a}/{b}"))
    }
}

pub fn watch_dirs(result: &Value) -> Vec<String> {
    let root = result["root"].as_str().unwrap_or("").to_string();
    let mut out: Vec<String> = vec![root];
    let mut add = |d: String| {
        if !out.contains(&d) {
            out.push(d);
        }
    };
    for p in result["projects"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
        let dir = p["dir"].as_str().unwrap_or("");
        if p["kind"] == "flutter" {
            for sub in ["", "lib", ".dart_tool", ".fvm", "android/app"] {
                add(join(dir, sub));
            }
        } else {
            add(dir.to_string());
            add(join(dir, "gradle"));
            for m in p["modules"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
                if let Some(d) = m["dir"].as_str() {
                    add(d.to_string());
                }
                if let Some(man) = m["manifest"].as_str() {
                    add(crate::preview::paths::dirname(man));
                }
            }
        }
    }
    out.truncate(MAX_DIRS);
    out
}

struct Watched {
    watchers: HashMap<String, RecommendedWatcher>,
    due: Arc<Mutex<Option<Instant>>>,
}

pub struct ProjectWatchers {
    roots: Mutex<HashMap<String, Watched>>,
    on_change: Arc<dyn Fn(String) + Send + Sync>,
    debounce: Duration,
}

impl ProjectWatchers {
    pub fn new(on_change: Arc<dyn Fn(String) + Send + Sync>) -> Self {
        ProjectWatchers { roots: Mutex::new(HashMap::new()), on_change, debounce: Duration::from_millis(DEBOUNCE_MS) }
    }

    pub fn has(&self, root: &str) -> bool {
        self.roots.lock().unwrap().contains_key(root)
    }

    pub fn sync(&self, root: &str, dirs: &[String]) {
        let mut g = self.roots.lock().unwrap();
        let w = g.entry(root.to_string()).or_insert_with(|| Watched { watchers: HashMap::new(), due: Arc::new(Mutex::new(None)) });
        let want: HashSet<&String> = dirs.iter().collect();
        w.watchers.retain(|d, _| want.contains(d));
        for dir in dirs {
            if w.watchers.contains_key(dir) {
                continue;
            }
            let (due, cb, root_s, debounce) = (w.due.clone(), self.on_change.clone(), root.to_string(), self.debounce);
            let made = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
                let Ok(ev) = res else { return };
                let hit = ev.paths.is_empty() || ev.paths.iter().any(|p| p.file_name().is_some_and(|n| detect_file().is_match(&n.to_string_lossy())));
                if !hit {
                    return;
                }
                let first = {
                    let mut d = due.lock().unwrap();
                    let first = d.is_none();
                    *d = Some(Instant::now() + debounce);
                    first
                };
                if first {
                    let (due, cb, root_s) = (due.clone(), cb.clone(), root_s.clone());
                    std::thread::spawn(move || loop {
                        let at = match *due.lock().unwrap() {
                            Some(t) => t,
                            None => return,
                        };
                        let now = Instant::now();
                        if now >= at {
                            *due.lock().unwrap() = None;
                            cb(root_s);
                            return;
                        }
                        std::thread::sleep(at - now);
                    });
                }
            });
            if let Ok(mut watcher) = made {
                if watcher.watch(Path::new(dir), RecursiveMode::NonRecursive).is_ok() {
                    w.watchers.insert(dir.clone(), watcher);
                }
            }
        }
    }

    pub fn close(&self, root: &str) {
        if let Some(w) = self.roots.lock().unwrap().remove(root) {
            *w.due.lock().unwrap() = None;
        }
    }

    pub fn close_all(&self) {
        let roots: Vec<String> = self.roots.lock().unwrap().keys().cloned().collect();
        for r in roots {
            self.close(&r);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn dirs_and_file_filter() {
        assert!(detect_file().is_match("pubspec.yaml") && detect_file().is_match("main_dev.dart") && !detect_file().is_match("foo.dart"));
        let r = json!({"root": "/r", "projects": [{"kind": "flutter", "dir": "/r/app"}, {"kind": "android", "dir": "/r/and", "modules": [{"dir": "/r/and/app", "manifest": "/r/and/app/src/main/AndroidManifest.xml"}]}]});
        let d = watch_dirs(&r);
        assert_eq!(d[0], "/r");
        assert!(d.contains(&"/r/app/android/app".to_string()) && d.contains(&"/r/and/gradle".to_string()) && d.contains(&"/r/and/app/src/main".to_string()));
    }

    #[test]
    fn fires_after_pubspec_change() {
        let dir = std::env::temp_dir().join(format!("ccm-d4-w-{}", crate::util::uuid()));
        std::fs::create_dir_all(&dir).unwrap();
        let (tx, rx) = std::sync::mpsc::channel::<String>();
        let tx = Mutex::new(tx);
        let w = ProjectWatchers::new(Arc::new(move |r| {
            let _ = tx.lock().unwrap().send(r);
        }));
        w.sync("k", &[dir.to_string_lossy().to_string()]);
        assert!(w.has("k"));
        std::thread::sleep(Duration::from_millis(300));
        std::fs::write(dir.join("pubspec.yaml"), "name: x\n").unwrap();
        assert_eq!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), "k");
        w.close("k");
        assert!(!w.has("k"));
    }
}
