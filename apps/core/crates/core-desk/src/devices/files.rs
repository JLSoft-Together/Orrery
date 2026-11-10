use super::appdata::run_as_failure;
use super::devfs::*;
use super::manager::DeviceManager;
use super::parse::{err, err_detail, is_ok};
use super::toggles::sh_quote;
use crate::js;
use crate::run::android::jsre::Js;
use crate::run::proc::kill_pid_only;
use crate::util::{path_str, resolve};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const CMD_MS: u64 = 20_000;
const RM_MS: u64 = 120_000;
const PROGRESS_MS: u128 = 120;
pub const PUSH_MAX: usize = 20;

struct Active {
    transfer: String,
    pid: Mutex<Option<u32>>,
    cancelled: AtomicBool,
}

pub type Progress<'a> = &'a dyn Fn(Value);

pub struct DeviceFiles {
    m: Arc<DeviceManager>,
    active: Mutex<Option<Arc<Active>>>,
    pulled: Mutex<HashSet<String>>,
}

fn fail(code: &str, detail: &str) -> Value {
    static WS: Js = Js::new(r"\s+", "g");
    err_detail(code, &js::slice(WS.replace_all(detail, " ").trim(), 300))
}

fn error_of(text: &str) -> Value {
    static DENIED: Js = Js::new(r"Permission denied", "i");
    static GONE: Js = Js::new(r"No such file", "i");
    static NOTDIR: Js = Js::new(r"Not a directory", "i");
    static EXISTS: Js = Js::new(r"File exists", "i");
    static RO: Js = Js::new(r"Read-only file system", "i");
    if let Some(v) = run_as_failure(text) {
        return v;
    }
    if DENIED.test(text) {
        return err("permission");
    }
    if GONE.test(text) {
        return err("not-found");
    }
    if NOTDIR.test(text) {
        return err("not-dir");
    }
    if EXISTS.test(text) {
        return err("exists");
    }
    if RO.test(text) {
        return fail("permission", text);
    }
    fail("failed", text)
}

fn where_of(scope: &Scope, p: &str) -> String {
    match scope {
        Scope::App(pkg) => format!("{pkg}:{p}"),
        Scope::Sdcard => p.to_string(),
    }
}

fn drain_tail(mut r: Box<dyn Read + Send>, cap: usize) -> (Arc<Mutex<String>>, std::thread::JoinHandle<()>) {
    let out = Arc::new(Mutex::new(String::new()));
    let o = out.clone();
    let h = std::thread::spawn(move || {
        let mut buf = [0u8; 8192];
        while let Ok(n) = r.read(&mut buf) {
            if n == 0 {
                break;
            }
            let mut s = o.lock().unwrap();
            s.push_str(&String::from_utf8_lossy(&buf[..n]));
            let chars: Vec<char> = s.chars().collect();
            if chars.len() > cap {
                *s = chars[chars.len() - cap..].iter().collect();
            }
        }
    });
    (out, h)
}

struct Tick<'a> {
    cb: Progress<'a>,
    base: Value,
    last: Mutex<Option<Instant>>,
}

impl Tick<'_> {
    fn at(&self, done: u64, force: bool) {
        let mut last = self.last.lock().unwrap();
        if !force && last.is_some_and(|l| l.elapsed().as_millis() < PROGRESS_MS) {
            return;
        }
        *last = Some(Instant::now());
        let mut v = self.base.clone();
        v["done"] = json!(done);
        (self.cb)(v);
    }
}

impl DeviceFiles {
    pub fn new(m: Arc<DeviceManager>) -> Self {
        DeviceFiles { m, active: Mutex::new(None), pulled: Mutex::new(HashSet::new()) }
    }

    pub fn was_pulled(&self, local: &str) -> bool {
        self.pulled.lock().unwrap().contains(&path_str(&resolve(local)))
    }

    fn scope_check(&self, id: &str, scope: &Scope) -> Option<Value> {
        let Scope::App(pkg) = scope else { return None };
        let Some(r) = self.m.adb.shell(id, &["run-as".into(), sh_quote(pkg), "true".into()], Some(CMD_MS)) else { return Some(err("adb-missing")) };
        if r.timed_out {
            return Some(err("timeout"));
        }
        run_as_failure(&r.out).or_else(|| (r.code != Some(0)).then(|| fail("denied", &r.out)))
    }

    fn run(&self, id: &str, scope: &Scope, f: impl FnOnce() -> Value) -> Value {
        let ready = self.m.ready_check(id);
        if !is_ok(&ready) {
            return ready;
        }
        self.scope_check(id, scope).unwrap_or_else(f)
    }

    fn sh(&self, id: &str, args: Vec<String>, ms: u64) -> Result<crate::run::collect::Collected, Value> {
        match self.m.adb.shell(id, &args, Some(ms)) {
            None => Err(err("adb-missing")),
            Some(r) if r.timed_out => Err(err("timeout")),
            Some(r) => Ok(r),
        }
    }

    pub fn list(&self, id: &str, scope: &Scope, dir: &str) -> Value {
        self.run(id, scope, || {
            let r = match self.sh(id, ls_args(scope, dir), CMD_MS) {
                Ok(r) => r,
                Err(e) => return e,
            };
            let parsed = parse_ls_la(&r.out);
            if let Some(e) = parsed.error {
                return err(e);
            }
            if parsed.entries.is_empty() && r.code != Some(0) {
                return error_of(&r.out);
            }
            json!({"ok": true, "path": dir, "entries": parsed.entries, "skipped": parsed.skipped})
        })
    }

    pub fn mkdir(&self, id: &str, scope: &Scope, dir: &str, name: &str) -> Value {
        if !is_entry_name(name) {
            return err("invalid");
        }
        self.run(id, scope, || match self.sh(id, mkdir_args(scope, &join_path(dir, name)), CMD_MS) {
            Ok(r) if r.code == Some(0) && r.out.trim().is_empty() => json!({"ok": true}),
            Ok(r) => error_of(&r.out),
            Err(e) => e,
        })
    }

    pub fn remove(&self, id: &str, scope: &Scope, p: &str, dir: bool) -> Value {
        if !deletable(scope, p) {
            return err("outside");
        }
        self.run(id, scope, || {
            let confirmed = self.m.adb.host.call("devices.confirm", json!({"kind": "fs-delete", "name": base_name(p), "where": where_of(scope, p), "dir": dir}));
            if !matches!(confirmed, Ok(Value::Bool(true))) {
                return err("cancelled");
            }
            match self.sh(id, rm_args(scope, p, dir), RM_MS) {
                Ok(r) if r.code == Some(0) && r.out.trim().is_empty() => json!({"ok": true}),
                Ok(r) => error_of(&r.out),
                Err(e) => e,
            }
        })
    }

    fn begin(&self, transfer: &str) -> Option<Arc<Active>> {
        let mut a = self.active.lock().unwrap();
        if a.is_some() {
            return None;
        }
        let act = Arc::new(Active { transfer: transfer.to_string(), pid: Mutex::new(None), cancelled: AtomicBool::new(false) });
        *a = Some(act.clone());
        Some(act)
    }

    pub fn cancel(&self, transfer: &str) -> bool {
        let Some(a) = self.active.lock().unwrap().clone() else { return false };
        if a.transfer != transfer {
            return false;
        }
        a.cancelled.store(true, Ordering::SeqCst);
        if let Some(pid) = *a.pid.lock().unwrap() {
            kill_pid_only(pid);
        }
        true
    }

    pub fn dispose(&self) {
        if let Some(a) = self.active.lock().unwrap().clone() {
            a.cancelled.store(true, Ordering::SeqCst);
            if let Some(pid) = *a.pid.lock().unwrap() {
                kill_pid_only(pid);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn pull(&self, id: &str, scope: &Scope, p: &str, kind: &str, size: Option<u64>, dest: &Path, transfer: &str, progress: Progress) -> Value {
        let Some(a) = self.begin(transfer) else { return err("busy") };
        let r = self.run(id, scope, || self.pull_inner(id, scope, p, kind, size, dest, transfer, progress, &a));
        *self.active.lock().unwrap() = None;
        r
    }

    #[allow(clippy::too_many_arguments)]
    fn pull_inner(&self, id: &str, scope: &Scope, p: &str, kind: &str, size: Option<u64>, dest: &Path, transfer: &str, progress: Progress, a: &Arc<Active>) -> Value {
        let taken = |n: &str| dest.join(n).exists() || dest.join(format!("{n}.orrery-part")).exists();
        let Some(name) = unique_name(&local_safe_name(&base_name(p)), taken) else { return err("exists") };
        let final_path = dest.join(&name);
        let base = json!({"transfer": transfer, "name": base_name(p), "total": if kind == "dir" { Value::Null } else { size.map(|s| json!(s)).unwrap_or(Value::Null) }, "index": 0, "count": 1});
        let tick = Tick { cb: progress, base, last: Mutex::new(None) };
        if kind == "dir" {
            if matches!(scope, Scope::App(_)) {
                return err("unsupported");
            }
            return self.pull_dir(id, p, &final_path, &name, dest, a, &tick);
        }
        let part = PathBuf::from(format!("{}.orrery-part", path_str(&final_path)));
        let mut args = vec!["exec-out".to_string()];
        args.extend(cat_args(scope, p));
        let Some(mut child) = self.m.adb.spawn_adb(id, &args, false) else { return err("adb-missing") };
        let Some(mut stdout) = child.stdout.take() else { return err("adb-missing") };
        *a.pid.lock().unwrap() = Some(child.id());
        let (errs, errh) = drain_tail(Box::new(child.stderr.take().unwrap()), 2048);
        tick.at(0, true);
        let (mut done, mut head, mut write_ok) = (0u64, Vec::<u8>::new(), true);
        let file = std::fs::File::create(&part);
        if let Ok(mut out) = file {
            let mut buf = vec![0u8; 64 * 1024];
            while let Ok(n) = stdout.read(&mut buf) {
                if n == 0 {
                    break;
                }
                if head.len() < 512 {
                    head.extend_from_slice(&buf[..n.min(512 - head.len())]);
                }
                done += n as u64;
                if out.write_all(&buf[..n]).is_err() {
                    write_ok = false;
                    break;
                }
                tick.at(done, false);
            }
        } else {
            write_ok = false;
        }
        if !write_ok {
            kill_pid_only(child.id());
        }
        let code = child.wait().ok().and_then(|s| s.code());
        let _ = errh.join();
        let cancelled = a.cancelled.load(Ordering::SeqCst);
        tick.at(done, true);
        let drop_part = || {
            let _ = std::fs::remove_file(&part);
        };
        if cancelled || !write_ok {
            drop_part();
            return if cancelled { err("cancelled") } else { fail("failed", "write") };
        }
        let text = format!("{} {}", String::from_utf8_lossy(&head), errs.lock().unwrap());
        if code != Some(0) || size.is_some_and(|s| kind == "file" && done != s) {
            drop_part();
            let e = if done < 512 || code != Some(0) { error_of(&text) } else { err("failed") };
            return match size {
                Some(s) if e["error"] == json!("failed") => fail("failed", &format!("{done} / {s} B")),
                _ => e,
            };
        }
        static CAT_ERR: Js = Js::new(r"^cat: .*(denied|No such file)", "m");
        if size.is_none() && done < 512 && (run_as_failure(&text).is_some() || CAT_ERR.test(&text)) {
            drop_part();
            return error_of(&text);
        }
        let _ = std::fs::rename(&part, &final_path);
        self.pulled.lock().unwrap().insert(path_str(&resolve(&path_str(&final_path))));
        json!({"ok": true, "local": path_str(&final_path), "name": name, "inDir": path_str(dest)})
    }

    #[allow(clippy::too_many_arguments)]
    fn pull_dir(&self, id: &str, p: &str, final_path: &Path, name: &str, dest: &Path, a: &Arc<Active>, tick: &Tick) -> Value {
        let Some(mut child) = self.m.adb.spawn_adb(id, &["pull".to_string(), p.to_string(), path_str(final_path)], false) else { return err("adb-missing") };
        *a.pid.lock().unwrap() = Some(child.id());
        tick.at(0, true);
        let (outs, outh) = drain_tail(Box::new(child.stdout.take().unwrap()), 2048);
        let (errs, errh) = drain_tail(Box::new(child.stderr.take().unwrap()), 2048);
        let code = child.wait().ok().and_then(|s| s.code());
        let _ = (outh.join(), errh.join());
        let cancelled = a.cancelled.load(Ordering::SeqCst);
        if cancelled || code != Some(0) {
            let _ = std::fs::remove_dir_all(final_path);
            return if cancelled { err("cancelled") } else { error_of(&format!("{} {}", outs.lock().unwrap(), errs.lock().unwrap())) };
        }
        self.pulled.lock().unwrap().insert(path_str(&resolve(&path_str(final_path))));
        json!({"ok": true, "local": path_str(final_path), "name": name, "inDir": path_str(dest)})
    }

    pub fn push(&self, id: &str, scope: &Scope, dir: &str, locals: &[String], transfer: &str, progress: Progress) -> Value {
        let Some(a) = self.begin(transfer) else { return err("busy") };
        let r = self.run(id, scope, || self.push_inner(id, scope, dir, locals, transfer, progress, &a));
        *self.active.lock().unwrap() = None;
        r
    }

    #[allow(clippy::too_many_arguments)]
    fn push_inner(&self, id: &str, scope: &Scope, dir: &str, locals: &[String], transfer: &str, progress: Progress, a: &Arc<Active>) -> Value {
        let (mut pushed, mut skipped) = (Vec::<String>::new(), Vec::<String>::new());
        let mut files: Vec<(String, String, u64)> = Vec::new();
        for local in locals.iter().take(PUSH_MAX) {
            let name = Path::new(local).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            match std::fs::metadata(local) {
                Ok(m) if m.is_file() && is_entry_name(&name) => files.push((local.clone(), name, m.len())),
                _ => skipped.push(name),
            }
        }
        let count = files.len();
        for (index, (local, name, size)) in files.iter().enumerate() {
            if a.cancelled.load(Ordering::SeqCst) {
                return err("cancelled");
            }
            let remote = join_path(dir, name);
            let Some(exists) = self.m.adb.shell(id, &exists_args(scope, &remote), Some(CMD_MS)) else { return err("adb-missing") };
            if exists.code == Some(0) && !matches!(self.m.adb.host.call("devices.confirm", json!({"kind": "fs-replace", "name": name, "where": where_of(scope, &remote)})), Ok(Value::Bool(true))) {
                skipped.push(name.clone());
                continue;
            }
            let tick = Tick { cb: progress, base: json!({"transfer": transfer, "name": name, "total": size, "index": index, "count": count}), last: Mutex::new(None) };
            if let Some(r) = self.push_one(id, scope, &remote, local, *size, a, &tick) {
                return r;
            }
            pushed.push(name.clone());
        }
        json!({"ok": true, "pushed": pushed, "skipped": skipped})
    }

    #[allow(clippy::too_many_arguments)]
    fn push_one(&self, id: &str, scope: &Scope, remote: &str, local: &str, size: u64, a: &Arc<Active>, tick: &Tick) -> Option<Value> {
        let mut args = vec!["shell".to_string()];
        args.extend(write_args(scope, remote));
        let Some(mut child) = self.m.adb.spawn_adb(id, &args, true) else { return Some(err("adb-missing")) };
        let Some(mut stdin) = child.stdin.take() else { return Some(err("adb-missing")) };
        let pid = child.id();
        *a.pid.lock().unwrap() = Some(pid);
        let (outs, outh) = drain_tail(Box::new(child.stdout.take().unwrap()), 2048);
        let (errs, errh) = drain_tail(Box::new(child.stderr.take().unwrap()), 2048);
        let (tx, rx) = mpsc::channel::<Option<i32>>();
        std::thread::spawn(move || {
            let _ = tx.send(child.wait().ok().and_then(|s| s.code()));
        });
        let mut done = 0u64;
        tick.at(0, true);
        match std::fs::File::open(local) {
            Ok(mut src) => {
                let mut buf = vec![0u8; 64 * 1024];
                while let Ok(n) = src.read(&mut buf) {
                    if n == 0 || a.cancelled.load(Ordering::SeqCst) {
                        break;
                    }
                    if stdin.write_all(&buf[..n]).is_err() {
                        break;
                    }
                    done += n as u64;
                    tick.at(done, false);
                }
            }
            Err(_) => kill_pid_only(pid),
        }
        drop(stdin);
        let code = rx.recv_timeout(Duration::from_secs(3600)).unwrap_or(None);
        let _ = (outh.join(), errh.join());
        tick.at(done, true);
        let cleanup = || {
            self.m.adb.shell(id, &part_rm_args(scope, remote), Some(CMD_MS));
        };
        if a.cancelled.load(Ordering::SeqCst) {
            cleanup();
            return Some(err("cancelled"));
        }
        if code != Some(0) || done != size {
            cleanup();
            return Some(error_of(&format!("{} {}", outs.lock().unwrap(), errs.lock().unwrap())));
        }
        let st = self.m.adb.shell(id, &stat_args(scope, remote), Some(CMD_MS));
        let got = st.and_then(|s| parse_ls_la(&s.out).entries.first().and_then(|e| e["size"].as_u64()));
        if got == Some(size) {
            None
        } else {
            Some(fail("failed", &format!("{} / {size} B", got.map(|g| g.to_string()).unwrap_or_else(|| "?".into()))))
        }
    }
}
