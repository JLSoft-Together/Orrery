use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use crate::disk_clean_text::{ask_buttons, format_ram, kill_box};
use crate::disk_guard::Plat;
use crate::state::lock;

const CHANNELS: [&str; 2] = ["ccm:diskram-snapshot", "ccm:diskram-kill"];
pub const KILL_MAX: usize = 64;

pub fn handles(channel: &str) -> bool {
    CHANNELS.contains(&channel)
}

#[derive(Clone, Debug, PartialEq)]
pub struct RawProc {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub start: Option<String>,
    pub ram: Option<u64>,
    pub cpu_ms: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub pid: u32,
    pub start: String,
    pub name: String,
    pub ram: Option<u64>,
    pub lock: Option<&'static str>,
    pub view: Value,
}

pub trait RamDeps: Send + Sync {
    fn read(&self) -> Option<Vec<RawProc>>;
    fn self_pid(&self) -> u32;
    fn plat(&self) -> Plat;
    fn cpus(&self) -> usize;
    fn total_mem(&self) -> u64;
    fn free_mem(&self) -> u64;
    fn now(&self) -> f64;
    fn live_starts(&self, pids: &[u32]) -> HashMap<u32, String>;
    fn kill_tree(&self, pid: u32) -> bool;
    fn confirm(&self, lines: &[String], n: usize, first_name: &str) -> bool;
}

const AGENT: [&str; 7] = ["claude", "codex", "codewhale", "deepseek", "gemini", "aider", "cursor-agent"];
const DEV: [&str; 43] = [
    "node", "java", "javaw", "gradle", "gradlew", "kotlin-daemon", "adb", "emulator", "docker", "dockerd", "com.docker.backend", "com.docker.build", "docker desktop", "vmmemwsl", "vmmem", "dart", "dartaotruntime", "flutter", "python", "python3", "pythonw", "cargo", "rustc",
    "rust-analyzer", "go", "gopls", "deno", "bun", "code", "studio64", "studio", "idea64", "idea", "xcode", "simulator", "tsserver", "esbuild", "vite", "npm", "pnpm", "yarn", "git", "qemu-system",
];
const SYSTEM_WIN: [&str; 26] = [
    "system", "registry", "smss", "csrss", "wininit", "winlogon", "services", "lsass", "lsaiso", "svchost", "fontdrvhost", "dwm", "memory compression", "secure system", "system idle process", "idle", "explorer", "sihost", "ctfmon", "audiodg", "spoolsv", "searchindexer", "msmpeng",
    "securityhealthservice", "wudfhost", "taskhostw",
];
const SYSTEM_MAC: [&str; 16] = ["kernel_task", "launchd", "windowserver", "loginwindow", "logd", "opendirectoryd", "coreaudiod", "systemstats", "securityd", "configd", "mds", "mds_stores", "fseventsd", "dock", "finder", "systemuiserver"];

pub fn base_name(name: &str) -> String {
    let n = name.trim().to_lowercase();
    n.strip_suffix(".exe").map(str::to_owned).unwrap_or(n)
}

pub fn tag_of(name: &str) -> Option<&'static str> {
    let b = base_name(name);
    if AGENT.contains(&b.as_str()) {
        return Some("agent");
    }
    if DEV.contains(&b.as_str()) || b.starts_with("qemu-system") || b.starts_with("com.docker.") {
        return Some("dev");
    }
    None
}

pub fn system_name(name: &str, plat: Plat) -> bool {
    let b = base_name(name);
    match plat {
        Plat::Mac => SYSTEM_MAC.contains(&b.as_str()),
        Plat::Win => SYSTEM_WIN.contains(&b.as_str()),
        Plat::Linux => b == "init" || b == "systemd",
    }
}

pub fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn same_start(a: Option<&str>, b: Option<&str>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => !a.trim().is_empty() && !b.trim().is_empty() && collapse(a) == collapse(b),
        _ => false,
    }
}

pub fn self_tree(rows: &[RawProc], self_pid: u32) -> (HashSet<u32>, HashSet<u32>) {
    let mut kids: HashMap<u32, Vec<u32>> = HashMap::new();
    let by: HashMap<u32, &RawProc> = rows.iter().map(|r| (r.pid, r)).collect();
    for r in rows {
        if r.pid != r.ppid {
            kids.entry(r.ppid).or_default().push(r.pid);
        }
    }
    let mut own: HashSet<u32> = HashSet::from([self_pid]);
    let mut stack = vec![self_pid];
    while let Some(p) = stack.pop() {
        for k in kids.get(&p).into_iter().flatten() {
            if own.insert(*k) {
                stack.push(*k);
            }
        }
    }
    let mut parents = HashSet::new();
    let mut cur = by.get(&self_pid).copied();
    while let Some(c) = cur {
        if c.ppid == 0 || own.contains(&c.ppid) || parents.contains(&c.ppid) {
            break;
        }
        parents.insert(c.ppid);
        cur = by.get(&c.ppid).copied();
    }
    (own, parents)
}

pub fn lock_of(row: &RawProc, tree: &(HashSet<u32>, HashSet<u32>), plat: Plat) -> Option<&'static str> {
    if tree.0.contains(&row.pid) {
        return Some("self");
    }
    if tree.1.contains(&row.pid) {
        return Some("parent");
    }
    if row.pid <= 4 || (plat != Plat::Win && row.pid <= 1) || system_name(&row.name, plat) {
        return Some("system");
    }
    None
}

pub fn parse_targets(raw: &Value) -> Option<Vec<(u32, String)>> {
    let list = raw.get("targets")?.as_array()?;
    if list.is_empty() || list.len() > KILL_MAX {
        return None;
    }
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for t in list {
        let o = t.as_object()?;
        let pid = o.get("pid")?.as_f64().filter(|f| f.fract() == 0.0 && *f > 0.0 && *f <= 2_147_483_647.0)? as u32;
        let start = o.get("start")?.as_str().filter(|s| !s.trim().is_empty() && s.encode_utf16().count() <= 64)?;
        if seen.insert(pid) {
            out.push((pid, start.to_owned()));
        }
    }
    Some(out)
}

#[derive(Default)]
struct State {
    prev: HashMap<String, (f64, f64)>,
    rows: HashMap<u32, Row>,
    last: Option<Value>,
}

pub struct Service<D: RamDeps> {
    pub deps: D,
    state: Mutex<State>,
    busy: AtomicBool,
}

fn empty_snapshot() -> Value {
    json!({ "at": 0, "ok": false, "totalMem": 0, "freeMem": 0, "cpus": 1, "procs": [] })
}

impl<D: RamDeps> Service<D> {
    pub fn new(deps: D) -> Self {
        Service { deps, state: Mutex::new(State::default()), busy: AtomicBool::new(false) }
    }

    pub fn get(&self, pid: u32) -> Option<Row> {
        lock(&self.state).rows.get(&pid).cloned()
    }

    pub fn snapshot(&self) -> Value {
        let mut st = lock(&self.state);
        let now = self.deps.now();
        if let Some(last) = &st.last {
            if now - last["at"].as_f64().unwrap_or(0.0) < 900.0 {
                return last.clone();
            }
        }
        let raw = self.deps.read();
        let at = self.deps.now();
        let cpus = self.deps.cpus().max(1);
        let mut base = json!({ "at": at, "totalMem": self.deps.total_mem(), "freeMem": self.deps.free_mem(), "cpus": cpus });
        let Some(raw) = raw else {
            base["ok"] = json!(false);
            base["procs"] = json!([]);
            return base;
        };
        let plat = self.deps.plat();
        let tree = self_tree(&raw, self.deps.self_pid());
        let mut next = HashMap::new();
        let mut rows = HashMap::new();
        let mut list: Vec<(Option<u64>, Value)> = Vec::new();
        for r in &raw {
            let key = format!("{}:{}", r.pid, r.start.as_deref().unwrap_or(""));
            let mut cpu: Option<f64> = None;
            if let (Some(ms), Some(_)) = (r.cpu_ms, r.start.as_ref()) {
                if let Some((pms, pat)) = st.prev.get(&key) {
                    if at > *pat {
                        cpu = Some(((ms - pms) / ((at - pat) * cpus as f64) * 100.0).clamp(0.0, 100.0));
                    }
                }
                next.insert(key, (ms, at));
            }
            let lock = if r.start.is_some() { lock_of(r, &tree, plat) } else { lock_of(r, &tree, plat).or(Some("system")) };
            let tag = if lock == Some("self") { Some("self") } else { tag_of(&r.name) };
            let view = json!({ "pid": r.pid, "ppid": r.ppid, "start": r.start.clone().unwrap_or_default(), "name": r.name, "ramBytes": r.ram, "cpu": cpu, "tag": tag, "lock": lock });
            rows.insert(r.pid, Row { pid: r.pid, start: r.start.clone().unwrap_or_default(), name: r.name.clone(), ram: r.ram, lock, view: view.clone() });
            list.push((r.ram, view));
        }
        list.sort_by(|a, b| b.0.map_or(-1i128, |v| v as i128).cmp(&a.0.map_or(-1i128, |v| v as i128)));
        st.prev = next;
        st.rows = rows;
        base["ok"] = json!(true);
        base["procs"] = Value::Array(list.into_iter().map(|(_, v)| v).collect());
        st.last = Some(base.clone());
        base
    }

    pub fn kill(&self, raw: &Value) -> Value {
        let Some(targets) = parse_targets(raw) else { return json!({ "ok": false, "killed": 0, "error": "invalid" }) };
        if self.busy.swap(true, Ordering::SeqCst) {
            return json!({ "ok": false, "killed": 0, "error": "busy" });
        }
        let out = self.kill_inner(&targets);
        self.busy.store(false, Ordering::SeqCst);
        out
    }

    fn kill_inner(&self, targets: &[(u32, String)]) -> Value {
        let mut items: Vec<Row> = Vec::new();
        for (pid, start) in targets {
            let Some(row) = self.get(*pid) else { return json!({ "ok": false, "killed": 0, "error": "gone" }) };
            if !same_start(Some(&row.start), Some(start)) {
                return json!({ "ok": false, "killed": 0, "error": "gone" });
            }
            if let Some(l) = row.lock {
                return json!({ "ok": false, "killed": 0, "error": "locked", "lock": l });
            }
            items.push(row);
        }
        let pids: Vec<u32> = items.iter().map(|i| i.pid).collect();
        let before = self.deps.live_starts(&pids);
        if !items.iter().all(|i| same_start(before.get(&i.pid).map(String::as_str), Some(&i.start))) {
            return json!({ "ok": false, "killed": 0, "error": "gone" });
        }
        let lines: Vec<String> = items.iter().map(|i| format!("{}, {}, pid {}", i.name, format_ram(i.ram), i.pid)).collect();
        if !self.deps.confirm(&lines, items.len(), &items[0].name) {
            return json!({ "ok": false, "killed": 0, "error": "cancelled" });
        }
        let live = self.deps.live_starts(&pids);
        let mut killed = 0;
        for i in &items {
            if !same_start(live.get(&i.pid).map(String::as_str), Some(&i.start)) {
                continue;
            }
            if self.deps.kill_tree(i.pid) {
                killed += 1;
            }
        }
        if killed > 0 {
            json!({ "ok": true, "killed": killed })
        } else {
            json!({ "ok": false, "killed": 0, "error": "failed" })
        }
    }

    pub fn handle(&self, channel: &str, a: &Value) -> Value {
        match channel {
            "ccm:diskram-snapshot" => self.snapshot(),
            "ccm:diskram-kill" => self.kill(a),
            _ => empty_snapshot(),
        }
    }
}

#[cfg(windows)]
pub fn memory() -> (u64, u64) {
    #[repr(C)]
    struct MemoryStatusEx {
        length: u32,
        load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page: u64,
        avail_page: u64,
        total_virtual: u64,
        avail_virtual: u64,
        avail_ext: u64,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalMemoryStatusEx(buffer: *mut MemoryStatusEx) -> i32;
    }
    let mut m = MemoryStatusEx { length: std::mem::size_of::<MemoryStatusEx>() as u32, load: 0, total_phys: 0, avail_phys: 0, total_page: 0, avail_page: 0, total_virtual: 0, avail_virtual: 0, avail_ext: 0 };
    if unsafe { GlobalMemoryStatusEx(&mut m) } == 0 {
        return (0, 0);
    }
    (m.total_phys, m.avail_phys)
}

#[cfg(target_os = "macos")]
pub fn memory() -> (u64, u64) {
    fn read(name: &str) -> u64 {
        let c = std::ffi::CString::new(name).unwrap_or_default();
        let mut v: u64 = 0;
        let mut len = std::mem::size_of::<u64>();
        let r = unsafe { libc::sysctlbyname(c.as_ptr(), &mut v as *mut u64 as *mut libc::c_void, &mut len, std::ptr::null_mut(), 0) };
        if r != 0 {
            return 0;
        }
        if len == 4 {
            v & 0xffff_ffff
        } else {
            v
        }
    }
    let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) }.max(0) as u64;
    (read("hw.memsize"), read("vm.page_free_count").saturating_mul(page))
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn memory() -> (u64, u64) {
    (0, 0)
}

pub fn read_procs() -> Option<Vec<RawProc>> {
    let rows = core_procs::table()?;
    let pids: Vec<u32> = rows.iter().map(|r| r.pid).collect();
    let usage: HashMap<u32, core_procs::Usage> = core_procs::usage(&pids).into_iter().map(|u| (u.pid, u)).collect();
    Some(
        rows.into_iter()
            .map(|r| {
                let u = usage.get(&r.pid);
                RawProc { pid: r.pid, ppid: r.ppid, name: r.name, start: r.start_raw.map(|s| if cfg!(windows) { s } else { collapse(&s) }), ram: u.and_then(|u| u.working_set), cpu_ms: u.and_then(|u| u.cpu_ms) }
            })
            .collect(),
    )
}

pub struct AppDeps {
    app: AppHandle,
}

fn auto_confirm() -> bool {
    crate::disk_clean::e2e_env("CCM_E2E_DISKRAM_CONFIRM").is_some_and(|v| v == "1")
}

impl RamDeps for AppDeps {
    fn read(&self) -> Option<Vec<RawProc>> {
        read_procs()
    }

    fn self_pid(&self) -> u32 {
        std::process::id()
    }

    fn plat(&self) -> Plat {
        Plat::host()
    }

    fn cpus(&self) -> usize {
        std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
    }

    fn total_mem(&self) -> u64 {
        memory().0
    }

    fn free_mem(&self) -> u64 {
        memory().1
    }

    fn now(&self) -> f64 {
        crate::disk_map::now_ms() as f64
    }

    fn live_starts(&self, pids: &[u32]) -> HashMap<u32, String> {
        let list: Vec<i64> = pids.iter().map(|p| *p as i64).collect();
        core_desk::resources::live_starts(&list).into_iter().map(|(k, v)| (k as u32, v)).collect()
    }

    fn kill_tree(&self, pid: u32) -> bool {
        core_desk::resources::kill_tree(pid as i64)
    }

    fn confirm(&self, lines: &[String], n: usize, first_name: &str) -> bool {
        if auto_confirm() {
            return true;
        }
        ask_buttons(&self.app, &kill_box(lines, n, first_name, &crate::hosts::locale(&self.app))) == 0
    }
}

pub struct DiskRamState(pub Arc<Service<AppDeps>>);

fn service(app: &AppHandle) -> Arc<Service<AppDeps>> {
    if let Some(s) = app.try_state::<DiskRamState>() {
        return s.0.clone();
    }
    let svc = Arc::new(Service::new(AppDeps { app: app.clone() }));
    app.manage(DiskRamState(svc.clone()));
    app.try_state::<DiskRamState>().map_or(svc, |s| s.0.clone())
}

pub fn dispatch(app: &AppHandle, channel: &str, args: &[Value]) -> Result<Value, String> {
    if !handles(channel) {
        return Err(format!("not ported: {channel}"));
    }
    let a = args.first().cloned().unwrap_or(Value::Null);
    Ok(service(app).handle(channel, &a))
}

#[cfg(test)]
#[path = "disk_ram_tests.rs"]
mod tests;
