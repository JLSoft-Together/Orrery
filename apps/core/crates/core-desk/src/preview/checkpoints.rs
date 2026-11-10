use super::changes::{hunk_patch_for, MAX_DIFF_BYTES};
use super::exec::{exec_git, ExecOpts, ExecResult};
use super::paths::{root_key, safe_repo_path};
use regex::Regex;
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

const KEEP: usize = 30;

fn id_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^[0-9a-f]{40,64}$").expect("regex"))
}

fn detail(s: &str) -> String {
    crate::js::slice(s.trim(), 300)
}

fn err(e: &str) -> Value {
    json!({"ok": false, "error": e})
}

pub struct CheckpointStore {
    data_dir: PathBuf,
    busy: Mutex<HashSet<String>>,
    enabled: Mutex<Option<Vec<String>>>,
}

struct BusyKey<'a> {
    store: &'a CheckpointStore,
    key: String,
}

impl Drop for BusyKey<'_> {
    fn drop(&mut self) {
        self.store.busy.lock().unwrap().remove(&self.key);
    }
}

impl CheckpointStore {
    pub fn new(data_dir: PathBuf) -> Self {
        CheckpointStore { data_dir, busy: Mutex::new(HashSet::new()), enabled: Mutex::new(None) }
    }

    fn base(&self) -> PathBuf {
        self.data_dir.join("checkpoints")
    }

    fn dir(&self, root: &str) -> PathBuf {
        let hash = Sha1::digest(root_key(root).as_bytes());
        let hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();
        self.base().join(format!("{}.git", &hex[..16]))
    }

    fn opts(&self, root: &str, timeout: u64, input: Option<&str>) -> ExecOpts {
        let dir = self.dir(root);
        let mut o = ExecOpts::timeout(timeout).with_max(16 * 1024 * 1024);
        o.env = vec![
            ("GIT_DIR".into(), dir.to_string_lossy().to_string()),
            ("GIT_WORK_TREE".into(), root.to_string()),
            ("GIT_INDEX_FILE".into(), dir.join("orrery-index").to_string_lossy().to_string()),
            ("GIT_AUTHOR_NAME".into(), "Orrery".into()),
            ("GIT_AUTHOR_EMAIL".into(), "orrery@localhost".into()),
            ("GIT_COMMITTER_NAME".into(), "Orrery".into()),
            ("GIT_COMMITTER_EMAIL".into(), "orrery@localhost".into()),
        ];
        o.input = input.map(String::from);
        o
    }

    fn git(&self, root: &str, args: &[&str], timeout: u64) -> ExecResult {
        exec_git(Path::new(root), args, &self.opts(root, timeout, None))
    }

    fn with_enabled<T>(&self, f: impl FnOnce(&mut Vec<String>) -> T) -> T {
        let mut guard = self.enabled.lock().unwrap();
        if guard.is_none() {
            let text = std::fs::read_to_string(self.base().join("enabled.json")).unwrap_or_else(|_| "[]".into());
            let list = crate::js::parse(&text).and_then(|v| v.as_array().cloned()).unwrap_or_default();
            let mut out: Vec<String> = Vec::new();
            for s in list.iter().filter_map(Value::as_str) {
                if !out.iter().any(|x| x == s) {
                    out.push(s.to_string());
                }
            }
            *guard = Some(out);
        }
        f(guard.as_mut().expect("enabled"))
    }

    pub fn is_on(&self, root: &str) -> bool {
        let k = root_key(root);
        self.with_enabled(|l| l.contains(&k))
    }

    pub fn set_on(&self, root: &str, on: bool) {
        let k = root_key(root);
        let list = self.with_enabled(|l| {
            if on {
                if !l.contains(&k) {
                    l.push(k.clone());
                }
            } else {
                l.retain(|x| x != &k);
            }
            l.clone()
        });
        let _ = std::fs::create_dir_all(self.base());
        let _ = std::fs::write(self.base().join("enabled.json"), crate::js::stringify(&json!(list)));
        if !on {
            let _ = std::fs::remove_dir_all(self.dir(root));
        }
    }

    fn enter(&self, root: &str) -> Option<BusyKey<'_>> {
        let key = root_key(root);
        if !self.busy.lock().unwrap().insert(key.clone()) {
            return None;
        }
        Some(BusyKey { store: self, key })
    }

    fn ensure(&self, root: &str) -> bool {
        let dir = self.dir(root);
        if std::fs::metadata(dir.join("HEAD")).map(|m| m.is_file()).unwrap_or(false) {
            return true;
        }
        let _ = std::fs::create_dir_all(&dir);
        let d = dir.to_string_lossy().to_string();
        if !exec_git(&self.data_dir, &["init", "-q", "--bare", &d], &ExecOpts::timeout(20_000)).ok() {
            return false;
        }
        for (k, v) in [("core.bare", "false"), ("core.autocrlf", "false"), ("core.safecrlf", "false"), ("gc.auto", "0")] {
            exec_git(&self.data_dir, &["--git-dir", &d, "config", k, v], &ExecOpts::timeout(10_000));
        }
        true
    }

    fn tree(&self, root: &str) -> Option<String> {
        if !self.git(root, &["add", "-A", "--", "."], 120_000).ok() {
            return None;
        }
        let t = self.git(root, &["write-tree"], 60_000);
        if t.ok() {
            Some(t.stdout.trim().to_string())
        } else {
            None
        }
    }

    pub fn list(&self, root: &str) -> Vec<Value> {
        if !self.is_on(root) {
            return Vec::new();
        }
        let count = format!("--count={KEEP}");
        let res = self.git(root, &["for-each-ref", "--sort=-creatordate", &count, "--format=%(objectname)%00%(creatordate:unix)%00%(subject)", "refs/cp/"], 15_000);
        if !res.ok() {
            return Vec::new();
        }
        res.stdout
            .split('\n')
            .filter(|l| !l.is_empty())
            .map(|l| {
                let mut p = l.split('\0');
                let id = p.next().unwrap_or("");
                let at: f64 = p.next().map(|s| if s.trim().is_empty() { 0.0 } else { s.trim().parse().unwrap_or(f64::NAN) }).unwrap_or(0.0);
                let label = p.next().unwrap_or("");
                json!({"id": id, "at": if at.is_finite() { json!((at * 1000.0) as i64) } else { Value::Null }, "label": label})
            })
            .collect()
    }

    pub fn take(&self, root: &str, label: &str) -> Value {
        if !self.is_on(root) {
            return err("off");
        }
        let Some(_busy) = self.enter(root) else { return err("busy") };
        if !self.ensure(root) {
            return err("failed");
        }
        let Some(tree) = self.tree(root) else { return err("failed") };
        if let Some(last) = self.list(root).first() {
            let spec = format!("{}^{{tree}}", last["id"].as_str().unwrap_or(""));
            if self.git(root, &["rev-parse", &spec], 60_000).stdout.trim() == tree {
                return json!({"ok": true, "id": null});
            }
        }
        let clean: String = label.chars().map(|c| if c == '\r' || c == '\n' || c == '\0' { ' ' } else { c }).collect();
        let mut msg = crate::js::slice(&clean, 200);
        if msg.is_empty() {
            msg = "turn".into();
        }
        let commit = self.git(root, &["commit-tree", &tree, "-m", &msg], 60_000);
        let id = commit.stdout.trim().to_string();
        if !commit.ok() || !id_re().is_match(&id) {
            return json!({"ok": false, "error": "failed", "detail": detail(&commit.stderr)});
        }
        let r = format!("refs/cp/{}", crate::util::now_ms() as u64);
        self.git(root, &["update-ref", &r, &id], 60_000);
        self.prune(root);
        json!({"ok": true, "id": id})
    }

    fn prune(&self, root: &str) {
        let res = self.git(root, &["for-each-ref", "--sort=-creatordate", "--format=%(refname)", "refs/cp/"], 15_000);
        let old: Vec<String> = res.stdout.split('\n').filter(|l| !l.is_empty()).skip(KEEP).map(String::from).collect();
        if old.is_empty() {
            return;
        }
        for r in &old {
            self.git(root, &["update-ref", "-d", r], 10_000);
        }
        self.git(root, &["gc", "-q", "--prune=now"], 120_000);
    }

    fn known(&self, root: &str, id: &Value) -> Option<String> {
        let s = id.as_str().filter(|s| id_re().is_match(s))?;
        if self.list(root).iter().any(|c| c["id"] == json!(s)) {
            Some(s.to_string())
        } else {
            None
        }
    }

    pub fn diff(&self, root: &str, id: &Value, to: &Value) -> Value {
        let id = self.known(root, id);
        let to_k = if to.is_null() { None } else { self.known(root, to) };
        let Some(id) = id.filter(|_| to.is_null() || to_k.is_some()) else { return err("unknown") };
        let Some(_busy) = self.enter(root) else { return err("busy") };
        let Some(now) = to_k.or_else(|| self.tree(root)) else { return err("failed") };
        let names = self.git(root, &["diff-tree", "-r", "-z", "--no-renames", "--name-status", &id, &now], 60_000);
        let nums = self.git(root, &["diff-tree", "-r", "-z", "--no-renames", "--numstat", &id, &now], 60_000);
        if !names.ok() {
            return json!({"ok": false, "error": "failed", "detail": detail(&names.stderr)});
        }
        let re = Regex::new(r"(?s)^(-|[0-9]+)\t(-|[0-9]+)\t(.+)$").expect("regex");
        let mut counts: Vec<(String, Option<(i64, i64)>)> = Vec::new();
        for rec in nums.stdout.split('\0') {
            if let Some(m) = re.captures(rec) {
                let v = if &m[1] == "-" { None } else { Some((m[1].parse().unwrap_or(0), m[2].parse().unwrap_or(0))) };
                counts.retain(|(k, _)| k != &m[3]);
                counts.push((m[3].to_string(), v));
            }
        }
        let parts: Vec<&str> = names.stdout.split('\0').filter(|p| !p.is_empty()).collect();
        let mut files = Vec::new();
        let mut i = 0;
        while i + 1 < parts.len() && files.len() < 1000 {
            let status = match parts[i].chars().next() {
                Some('A') => "A",
                Some('D') => "D",
                _ => "M",
            };
            let p = parts[i + 1];
            let mut f = json!({"path": p, "status": status});
            if let Some((_, Some((a, r)))) = counts.iter().find(|(k, _)| k == p) {
                f["added"] = json!(a);
                f["removed"] = json!(r);
            }
            files.push(f);
            i += 2;
        }
        json!({"ok": true, "files": files})
    }

    pub fn file_diff(&self, root: &str, from: &Value, to: &Value, rel: &Value) -> Value {
        let from = self.known(root, from);
        let to = self.known(root, to);
        let safe = safe_repo_path(root, rel);
        let (Some(from), Some(to), Some(safe)) = (from, to, safe) else { return err("unknown") };
        let o = self.opts(root, 20_000, None).with_max(MAX_DIFF_BYTES);
        let res = exec_git(Path::new(root), &["diff", "--no-ext-diff", "--no-textconv", "--no-renames", &from, &to, "--", &safe.rel], &o);
        if res.truncated {
            return json!({"ok": true, "text": "", "tooLarge": true});
        }
        if !res.ok() {
            return json!({"ok": false, "error": "failed", "detail": detail(&res.stderr)});
        }
        json!({"ok": true, "text": res.stdout, "tooLarge": false})
    }

    pub fn revert_hunk(&self, root: &str, rel: &Value, patch: &Value) -> Value {
        let Some(safe) = safe_repo_path(root, rel) else { return err("denied") };
        let Some(text) = hunk_patch_for(&safe.rel, patch) else { return err("denied") };
        let Some(_busy) = self.enter(root) else { return err("busy") };
        let r = Path::new(root);
        let check = exec_git(r, &["apply", "-R", "--whitespace=nowarn", "--check", "-"], &self.opts(root, 20_000, Some(&text)));
        if !check.ok() {
            return json!({"ok": false, "error": "stale", "detail": detail(&check.stderr)});
        }
        let res = exec_git(r, &["apply", "-R", "--whitespace=nowarn", "-"], &self.opts(root, 20_000, Some(&text)));
        if res.ok() {
            json!({"ok": true})
        } else {
            json!({"ok": false, "error": "failed", "detail": detail(&res.stderr)})
        }
    }

    pub fn restore(&self, root: &str, id: &Value, paths: &Value, trash: &dyn Fn(&str) -> Result<(), String>) -> Value {
        let id = self.known(root, id);
        let list: Vec<Value> = paths.as_array().map(|a| a.iter().take(500).cloned().collect()).unwrap_or_default();
        let safe: Vec<_> = list.iter().map(|p| safe_repo_path(root, p)).collect();
        let Some(id) = id.filter(|_| !safe.is_empty() && safe.iter().all(Option::is_some)) else { return err("denied") };
        let Some(_busy) = self.enter(root) else { return err("busy") };
        let mut present: Vec<String> = Vec::new();
        for s in safe.into_iter().flatten() {
            let spec = format!("{id}:{}", s.rel);
            if self.git(root, &["cat-file", "-e", &spec], 10_000).ok() {
                present.push(s.rel);
            } else if std::fs::symlink_metadata(&s.abs).is_ok() {
                if let Err(e) = trash(&s.abs) {
                    return json!({"ok": false, "error": "failed", "detail": detail(&e)});
                }
            }
        }
        if !present.is_empty() {
            let mut args: Vec<&str> = vec!["checkout", &id, "--"];
            args.extend(present.iter().map(String::as_str));
            let res = self.git(root, &args, 60_000);
            if !res.ok() {
                return json!({"ok": false, "error": "failed", "detail": detail(&res.stderr)});
            }
        }
        json!({"ok": true})
    }
}
