use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::Serialize;
use serde_json::{json, Map, Value};
use tauri::{AppHandle, Manager};

use crate::disk_clean_dev::{self as dev, DockerDeps, JunkHit, SdkDeps};
use crate::disk_clean_sys::{self as sys, SysDeps};
use crate::disk_clean_text::{bin_box, command_box, one_tap_box, BoxN};
use crate::disk_guard::{self as guard, GuardContext, Plat, RealIo};
use crate::disk_text::{delete_box, Ask};
use crate::state::{lock, Shell};

pub const PROGRESS: &str = "ccm:diskclean-progress";

const CHANNELS: [&str; 6] = [
    "ccm:diskclean-categories",
    "ccm:diskclean-scan",
    "ccm:diskclean-cancel",
    "ccm:diskclean-delete",
    "ccm:diskclean-empty-bin",
    "ccm:diskclean-command",
];

pub const PERMANENT_OK: [&str; 2] = ["temp", "browser"];
pub const ONE_TAP: [&str; 6] = ["temp", "browser", "crash", "logs", "node_modules", "build"];
const ROOTS_MAX: usize = 200;
const OPEN_MAX: usize = 500;
const DELETE_MAX: usize = 5000;
const IDS_MAX: usize = 32;
const PRUNE_TIMEOUT: Duration = Duration::from_secs(30 * 60);

pub fn handles(channel: &str) -> bool {
    CHANNELS.contains(&channel)
}

fn no(b: &bool) -> bool {
    !*b
}

#[derive(Clone, Debug, Default, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub path: String,
    pub size: u64,
    pub group: &'static str,
    pub category: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_touched: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restore: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<&'static str>,
    #[serde(skip_serializing_if = "no")]
    pub file: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(skip_serializing_if = "no")]
    pub unknown: bool,
    #[serde(skip_serializing_if = "no")]
    pub hold: bool,
    #[serde(skip_serializing_if = "no")]
    pub warn: bool,
}

impl Item {
    pub fn new(path: &str, size: u64, group: &'static str, category: &str) -> Item {
        Item { path: path.to_owned(), size, group, category: category.to_owned(), ..Default::default() }
    }

    pub fn kind(mut self, k: &str) -> Item {
        self.kind = Some(k.to_owned());
        self
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub path: String,
    pub size: u64,
    pub mtime: f64,
    pub file: bool,
}

#[derive(Clone, Debug)]
pub struct Holder {
    pub cwd: String,
    pub lock: &'static str,
}

pub struct Ctx<'a> {
    pub roots: Vec<String>,
    pub home: String,
    pub plat: Plat,
    pub env: &'a dyn Fn(&str) -> Option<String>,
    pub abort: &'a AtomicBool,
    pub keep: Vec<String>,
    pub now: f64,
    pub system: Option<&'a SysDeps>,
    pub sdk: Option<&'a SdkDeps>,
    pub docker: Option<&'a DockerDeps>,
    pub note: RefCell<Option<&'static str>>,
    pub attached: RefCell<Vec<(String, Vec<Member>)>>,
    junk: OnceCell<Vec<JunkHit>>,
    sizer: &'a dyn Fn(&str) -> Option<u64>,
    emitter: &'a dyn Fn(Item),
    keeper: &'a dyn Fn(&str) -> bool,
    tracked: &'a dyn Fn(&str, &str) -> Option<bool>,
}

impl Ctx<'_> {
    pub fn aborted(&self) -> bool {
        self.abort.load(Ordering::SeqCst)
    }

    pub fn size(&self, p: &str) -> u64 {
        (self.sizer)(p).unwrap_or(0)
    }

    pub fn emit(&self, item: Item) {
        (self.emitter)(item)
    }

    pub fn keep_member(&self, p: &str) -> bool {
        (self.keeper)(p)
    }

    pub fn set_note(&self, n: &'static str) {
        *self.note.borrow_mut() = Some(n);
    }

    pub fn attach(&self, p: &str, list: Vec<Member>) {
        self.attached.borrow_mut().push((p.to_owned(), list));
    }

    pub fn junk(&self) -> &[JunkHit] {
        self.junk.get_or_init(|| {
            let mut out = Vec::new();
            for r in &self.roots {
                match dev::find_junk(r, self.abort, dev::WALK_DEPTH, self.tracked) {
                    Some(hits) => out.extend(hits),
                    None => break,
                }
            }
            out
        })
    }
}

pub struct Provider {
    pub id: &'static str,
    pub group: &'static str,
    pub preselect: bool,
    pub platforms: &'static [Plat],
    pub scan: fn(&Ctx),
}

const ALL: &[Plat] = &[Plat::Win, Plat::Mac, Plat::Linux];
const MAC: &[Plat] = &[Plat::Mac];
const WIN: &[Plat] = &[Plat::Win];
const WIN_MAC: &[Plat] = &[Plat::Win, Plat::Mac];

pub fn dev_providers() -> Vec<Provider> {
    vec![
        Provider { id: "node_modules", group: "dev", preselect: true, platforms: ALL, scan: dev::scan_node_modules },
        Provider { id: "build", group: "dev", preselect: true, platforms: ALL, scan: dev::scan_build },
        Provider { id: "xcode", group: "dev", preselect: true, platforms: MAC, scan: dev::scan_xcode },
        Provider { id: "cache", group: "dev", preselect: false, platforms: ALL, scan: dev::scan_cache },
    ]
}

pub fn sdk_providers() -> Vec<Provider> {
    vec![
        Provider { id: "sdk", group: "dev", preselect: true, platforms: ALL, scan: dev::scan_sdk },
        Provider { id: "avd", group: "dev", preselect: false, platforms: ALL, scan: dev::scan_avd },
        Provider { id: "gradle_dists", group: "dev", preselect: true, platforms: ALL, scan: dev::scan_gradle_dists },
        Provider { id: "ios", group: "dev", preselect: true, platforms: MAC, scan: dev::scan_ios },
    ]
}

pub fn docker_provider() -> Provider {
    Provider { id: "docker", group: "dev", preselect: false, platforms: ALL, scan: dev::scan_docker }
}

pub fn system_providers() -> Vec<Provider> {
    vec![
        Provider { id: "temp", group: "system", preselect: true, platforms: WIN_MAC, scan: sys::scan_temp },
        Provider { id: "browser", group: "system", preselect: true, platforms: WIN_MAC, scan: sys::scan_browser },
        Provider { id: "crash", group: "system", preselect: true, platforms: WIN_MAC, scan: sys::scan_crash },
        Provider { id: "logs", group: "system", preselect: true, platforms: WIN_MAC, scan: sys::scan_logs },
        Provider { id: "thumbs", group: "system", preselect: false, platforms: WIN, scan: sys::scan_thumbs },
        Provider { id: "installers", group: "system", preselect: false, platforms: WIN_MAC, scan: sys::scan_installers },
        Provider { id: "bin", group: "system", preselect: false, platforms: WIN_MAC, scan: sys::scan_bin },
        Provider { id: "update", group: "system", preselect: false, platforms: WIN, scan: sys::scan_update },
        Provider { id: "appcache", group: "system", preselect: false, platforms: MAC, scan: sys::scan_appcache },
    ]
}

pub enum OneTap {
    Clean,
    Trash,
    Cancel,
}

pub struct OneTapAsk {
    pub bytes: u64,
    pub perm_bytes: u64,
    pub perm_count: usize,
    pub trash_bytes: u64,
    pub trash_count: usize,
}

pub struct CommandAsk {
    pub kind: String,
    pub bytes: u64,
    pub count: u64,
    pub command: String,
    pub warn: bool,
}

pub trait Deps: Send + Sync {
    fn plat(&self) -> Plat;
    fn home(&self) -> String;
    fn env(&self, k: &str) -> Option<String>;
    fn keep(&self) -> Vec<String>;
    fn self_dirs(&self) -> Vec<String>;
    fn running(&self) -> Vec<Holder>;
    fn open_now(&self) -> Vec<String> {
        Vec::new()
    }
    fn emit(&self, p: Value);
    fn confirm(&self, ask: &Ask, second: bool) -> bool;
    fn ask_one_tap(&self, ask: &OneTapAsk) -> OneTap;
    fn ask_bin(&self, bytes: u64, count: u64) -> bool;
    fn ask_command(&self, ask: &CommandAsk) -> bool;
    fn trash(&self, p: &str) -> Result<(), String>;
    fn remove(&self, p: &str) -> Result<(), String>;
    fn measure(&self, p: &str) -> Option<u64>;
    fn record(&self, e: Value);
    fn forget(&self, _paths: &[String]) {}
    fn index_size(&self, _p: &str) -> Option<u64> {
        None
    }
    fn tracked(&self, dir: &str, name: &str) -> Option<bool> {
        dev::git_tracked(dir, name)
    }
    fn now(&self) -> f64;
}

pub struct ScanReq {
    pub ids: Vec<String>,
    pub roots: Vec<String>,
    pub open: Vec<String>,
}

pub struct DeleteReq {
    pub paths: Vec<String>,
    pub permanent: bool,
    pub open: Vec<String>,
    pub one_tap: bool,
}

fn text_ok(v: &Value) -> Option<String> {
    let s = v.as_str()?;
    let n = s.encode_utf16().count();
    (n > 0 && n <= 4096 && !s.chars().any(|c| (c as u32) < 0x20)).then(|| s.to_owned())
}

fn strings(raw: Option<&Value>, max: usize) -> Option<Vec<String>> {
    let list = match raw {
        None => return Some(Vec::new()),
        Some(v) => v.as_array()?,
    };
    if list.len() > max {
        return None;
    }
    let mut out: Vec<String> = Vec::new();
    for r in list {
        let s = text_ok(r)?;
        if !out.contains(&s) {
            out.push(s);
        }
    }
    Some(out)
}

fn obj(raw: &Value) -> Option<&Map<String, Value>> {
    raw.as_object()
}

pub fn parse_scan(raw: &Value) -> Option<ScanReq> {
    let o = obj(raw)?;
    let ids = strings(Some(o.get("ids")?), IDS_MAX)?;
    Some(ScanReq { ids, roots: strings(o.get("roots"), ROOTS_MAX)?, open: strings(o.get("open"), OPEN_MAX)? })
}

pub fn parse_delete(raw: &Value) -> Option<DeleteReq> {
    let o = obj(raw)?;
    let paths = strings(Some(o.get("paths")?), DELETE_MAX)?;
    if paths.is_empty() {
        return None;
    }
    Some(DeleteReq { paths, permanent: o.get("permanent") == Some(&Value::Bool(true)), open: strings(o.get("open"), OPEN_MAX)?, one_tap: o.get("oneTap") == Some(&Value::Bool(true)) })
}

pub fn parse_command(raw: &Value) -> Option<String> {
    text_ok(obj(raw)?.get("path")?)
}

pub fn lock_of(path: &str, project: Option<&str>, open: &[String], running: &[Holder], self_dirs: &[String], plat: Plat) -> Option<&'static str> {
    let inside = |cwd: &str| project.is_some_and(|p| guard::is_under(cwd, p, plat)) || guard::is_under(cwd, path, plat);
    if self_dirs.iter().any(|s| !s.is_empty() && (guard::is_under(path, s, plat) || guard::is_under(s, path, plat))) {
        return Some("self");
    }
    if open.iter().any(|o| !o.is_empty() && inside(o)) {
        return Some("open");
    }
    running.iter().find(|h| !h.cwd.is_empty() && inside(&h.cwd)).map(|h| h.lock)
}

pub fn refused_root(abs: &str, home: &str, plat: Plat) -> bool {
    let n = guard::normalize(abs, plat);
    let fold = |s: &str| {
        let t = s.trim_end_matches(['\\', '/']);
        if plat == Plat::Win {
            t.to_lowercase()
        } else {
            t.to_owned()
        }
    };
    n == guard::root_of(&n, plat) || (!home.is_empty() && fold(&n) == fold(&guard::normalize(home, plat)))
}

pub fn lstat(p: &str) -> Option<std::fs::Metadata> {
    std::fs::symlink_metadata(p).ok()
}

pub fn is_link(m: &std::fs::Metadata) -> bool {
    m.file_type().is_symlink()
}

pub fn mtime_ms(m: &std::fs::Metadata) -> f64 {
    m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0.0, |d| d.as_secs_f64() * 1000.0)
}

pub fn dir_size(root: &str, abort: &AtomicBool) -> Option<u64> {
    let mut bytes = 0u64;
    let mut stack = vec![std::path::PathBuf::from(root)];
    while let Some(dir) = stack.pop() {
        if abort.load(Ordering::SeqCst) {
            return None;
        }
        let Ok(rd) = std::fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let Ok(ft) = e.file_type() else { continue };
            if ft.is_symlink() {
                continue;
            }
            if ft.is_dir() {
                stack.push(e.path());
            } else if ft.is_file() {
                bytes += e.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
    }
    Some(bytes)
}

struct Target {
    path: String,
    item: Item,
    member: Option<Member>,
}

pub struct Service<D: Deps> {
    pub deps: D,
    providers: Vec<Provider>,
    system: Option<SysDeps>,
    sdk: Option<SdkDeps>,
    docker: Option<DockerDeps>,
    abort: Mutex<Option<Arc<AtomicBool>>>,
    deleting: AtomicBool,
    last: Mutex<HashMap<String, Item>>,
    members: Mutex<HashMap<String, Vec<Member>>>,
}

fn reason_of(code: &str) -> String {
    code.to_owned()
}

impl<D: Deps> Service<D> {
    pub fn new(deps: D, system: Option<SysDeps>, sdk: Option<SdkDeps>, docker: Option<DockerDeps>) -> Self {
        let plat = deps.plat();
        let mut base = dev_providers();
        if sdk.is_some() {
            base.extend(sdk_providers());
        }
        if docker.is_some() {
            base.push(docker_provider());
        }
        if system.is_some() {
            base.extend(system_providers());
        }
        let providers = base.into_iter().filter(|p| p.platforms.contains(&plat)).collect();
        Service { deps, providers, system, sdk, docker, abort: Mutex::new(None), deleting: AtomicBool::new(false), last: Mutex::new(HashMap::new()), members: Mutex::new(HashMap::new()) }
    }

    fn plat(&self) -> Plat {
        self.deps.plat()
    }

    fn key(&self, p: &str) -> String {
        let n = guard::normalize(p, self.plat());
        if self.plat() == Plat::Linux {
            n
        } else {
            n.to_lowercase()
        }
    }

    pub fn categories(&self) -> Value {
        json!(self
            .providers
            .iter()
            .map(|p| json!({ "id": p.id, "group": p.group, "preselect": p.preselect, "titleKey": format!("diskClean.cat.{}.title", p.id), "hintKey": format!("diskClean.cat.{}.hint", p.id) }))
            .collect::<Vec<_>>())
    }

    pub fn scanning(&self) -> bool {
        lock(&self.abort).is_some()
    }

    pub fn cancel(&self) -> bool {
        match lock(&self.abort).as_ref() {
            Some(a) => {
                a.store(true, Ordering::SeqCst);
                true
            }
            None => false,
        }
    }

    pub fn clean_roots(&self, raw: &[String]) -> Vec<String> {
        let plat = self.plat();
        let home = self.deps.home();
        let mut ok = Vec::new();
        for r in raw {
            if !guard::is_absolute(r, plat) {
                continue;
            }
            let abs = guard::normalize(r, plat);
            if refused_root(&abs, &home, plat) {
                continue;
            }
            match lstat(&abs) {
                Some(m) if m.is_dir() && !is_link(&m) => ok.push(abs),
                _ => {}
            }
        }
        guard::top_level(&ok, plat)
    }

    fn open_list(&self, from_page: &[String]) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for p in self.deps.open_now().into_iter().chain(from_page.iter().cloned()) {
            if !p.is_empty() && !out.iter().any(|o| self.key(o) == self.key(&p)) {
                out.push(p);
            }
        }
        out
    }

    fn begin_scan(&self) -> Option<Arc<AtomicBool>> {
        let mut a = lock(&self.abort);
        if a.is_some() || self.deleting.load(Ordering::SeqCst) {
            return None;
        }
        let flag = Arc::new(AtomicBool::new(false));
        *a = Some(flag.clone());
        Some(flag)
    }

    pub fn scan(&self, req: &ScanReq) -> Value {
        let Some(abort) = self.begin_scan() else { return json!({ "items": [], "busy": true }) };
        let out = self.scan_inner(req, &abort);
        *lock(&self.abort) = None;
        out
    }

    fn scan_inner(&self, req: &ScanReq, abort: &AtomicBool) -> Value {
        let plat = self.plat();
        let items: RefCell<Vec<Item>> = RefCell::new(Vec::new());
        let mut notes = Map::new();
        let mut attached: Vec<(String, Vec<Member>)> = Vec::new();
        let roots = self.clean_roots(&req.roots);
        let running = self.deps.running();
        let self_dirs = self.deps.self_dirs();
        let open = self.open_list(&req.open);
        let keep = self.deps.keep();
        let home = self.deps.home();
        let env = |k: &str| self.deps.env(k);
        let seen: RefCell<std::collections::HashSet<String>> = RefCell::new(Default::default());
        let lock_item = |mut it: Item| -> Item {
            let l = if matches!(it.action, Some("files" | "bin" | "command")) { None } else { lock_of(&it.path, it.project.as_deref(), &open, &running, &self_dirs, plat) };
            if l.is_some() {
                it.locked = l;
            }
            it
        };
        let sizer = |p: &str| -> Option<u64> {
            if let Some(h) = self.deps.index_size(p) {
                return Some(h);
            }
            dir_size(p, abort)
        };
        let keeper = |p: &str| lock_of(p, None, &open, &running, &self_dirs, plat).is_none();
        let tracked = |d: &str, n: &str| self.deps.tracked(d, n);
        let mut cancelled = false;
        for id in &req.ids {
            let Some(provider) = self.providers.iter().find(|p| p.id == id) else { continue };
            if abort.load(Ordering::SeqCst) {
                cancelled = true;
                break;
            }
            let count = std::cell::Cell::new(0u64);
            let bytes = std::cell::Cell::new(0u64);
            self.deps.emit(json!({ "category": id, "state": "start", "count": 0, "bytes": 0 }));
            let emitter = |item: Item| {
                let k = self.key(&item.path);
                if !seen.borrow_mut().insert(k) {
                    return;
                }
                let it = lock_item(item);
                count.set(count.get() + 1);
                bytes.set(bytes.get() + it.size);
                self.deps.emit(json!({ "category": id, "state": "items", "items": [it.clone()], "count": count.get(), "bytes": bytes.get() }));
                items.borrow_mut().push(it);
            };
            let ctx = Ctx {
                roots: roots.clone(),
                home: home.clone(),
                plat,
                env: &env,
                abort,
                keep: keep.clone(),
                now: self.deps.now(),
                system: self.system.as_ref(),
                sdk: self.sdk.as_ref(),
                docker: self.docker.as_ref(),
                note: RefCell::new(None),
                attached: RefCell::new(Vec::new()),
                junk: OnceCell::new(),
                sizer: &sizer,
                emitter: &emitter,
                keeper: &keeper,
                tracked: &tracked,
            };
            (provider.scan)(&ctx);
            attached.extend(ctx.attached.into_inner());
            if abort.load(Ordering::SeqCst) {
                cancelled = true;
                break;
            }
            let note = ctx.note.into_inner();
            let mut done = json!({ "category": id, "state": "done", "count": count.get(), "bytes": bytes.get() });
            if let Some(n) = note {
                done["note"] = json!(n);
                notes.insert(id.clone(), json!(n));
            }
            self.deps.emit(done);
        }
        let items = items.into_inner();
        if cancelled {
            return json!({ "items": items, "cancelled": true });
        }
        {
            let mut last = lock(&self.last);
            let mut members = lock(&self.members);
            last.retain(|k, v| {
                let drop = req.ids.contains(&v.category);
                if drop {
                    members.remove(k);
                }
                !drop
            });
            for it in &items {
                let k = self.key(&it.path);
                last.insert(k.clone(), it.clone());
                if it.action == Some("files") {
                    if let Some((_, m)) = attached.iter().find(|(p, _)| self.key(p) == k) {
                        members.insert(k, m.clone());
                    }
                }
            }
        }
        let mut out = json!({ "items": items });
        if !notes.is_empty() {
            out["notes"] = Value::Object(notes);
        }
        out
    }

    pub fn known(&self, p: &str) -> Option<Item> {
        lock(&self.last).get(&self.key(p)).cloned()
    }

    fn members_of(&self, p: &str) -> Vec<Member> {
        lock(&self.members).get(&self.key(p)).cloned().unwrap_or_default()
    }

    fn live_lock(&self, item: &Item) -> Option<&'static str> {
        if item.category == "avd" {
            if let Some(sdk) = &self.sdk {
                return dev::avd_busy(item, sdk).then_some("app");
            }
        }
        let s = self.system.as_ref()?;
        if item.category == "browser" {
            let running = sys::running_browsers(s);
            let spot = sys::browser_spots(&s.roots).into_iter().find(|b| Some(b.id) == item.kind.as_deref())?;
            return running.contains(spot.id).then_some("browser");
        }
        if item.category == "appcache" {
            let name = guard::base_name(&item.path, self.plat());
            return sys::app_cache_lock(&name, &sys::running_apps(s)).then_some("app");
        }
        None
    }

    fn system_safe(&self, p: &str) -> bool {
        match &self.system {
            None => true,
            Some(s) => !sys::is_forbidden(p, &s.roots, &self.deps.keep()),
        }
    }

    fn real_parent(&self, p: &str) -> Option<String> {
        let up = guard::parent(&guard::normalize(p, self.plat()), self.plat());
        let real = std::fs::canonicalize(&up).ok()?;
        let s = guard::plain_canonical(&real.to_string_lossy(), self.plat())?;
        Some(self.key(&s))
    }

    fn kind_ok(m: &std::fs::Metadata, want_file: bool) -> bool {
        !is_link(m) && if want_file { m.is_file() } else { m.is_dir() }
    }

    pub fn remove(&self, req: &DeleteReq) -> Value {
        let permanent = req.permanent;
        if self.deleting.swap(true, Ordering::SeqCst) {
            let failed: Vec<Value> = req.paths.iter().map(|p| json!({ "path": p, "reason": "busy" })).collect();
            return json!({ "done": [], "failed": failed, "freed": 0, "permanent": permanent });
        }
        if self.scanning() {
            self.deleting.store(false, Ordering::SeqCst);
            let failed: Vec<Value> = req.paths.iter().map(|p| json!({ "path": p, "reason": "busy" })).collect();
            return json!({ "done": [], "failed": failed, "freed": 0, "permanent": permanent });
        }
        let out = self.remove_inner(req);
        self.deleting.store(false, Ordering::SeqCst);
        out
    }

    fn remove_inner(&self, req: &DeleteReq) -> Value {
        let plat = self.plat();
        let permanent = req.permanent;
        let mut failed: Vec<Value> = Vec::new();
        let mut targets: Vec<Target> = Vec::new();
        let running = self.deps.running();
        let self_dirs = self.deps.self_dirs();
        let open = self.open_list(&req.open);
        for p in &req.paths {
            let item = match self.known(p) {
                Some(it) if !matches!(it.action, Some("bin" | "command")) && (!req.one_tap || ONE_TAP.contains(&it.category.as_str())) => it,
                _ => {
                    failed.push(json!({ "path": p, "reason": "invalid" }));
                    continue;
                }
            };
            if let Some(l) = item.locked {
                failed.push(json!({ "path": item.path, "reason": format!("locked:{l}") }));
                continue;
            }
            let Some(st) = lstat(&item.path) else {
                failed.push(json!({ "path": item.path, "reason": "ENOENT" }));
                continue;
            };
            if !Self::kind_ok(&st, item.file) {
                failed.push(json!({ "path": item.path, "reason": "changed" }));
                continue;
            }
            let files = item.action == Some("files");
            let live = (if files { None } else { lock_of(&item.path, item.project.as_deref(), &open, &running, &self_dirs, plat) }).or_else(|| self.live_lock(&item));
            if let Some(l) = live {
                failed.push(json!({ "path": item.path, "reason": format!("locked:{l}") }));
                continue;
            }
            if item.group == "system" && !files && !self.system_safe(&item.path) {
                failed.push(json!({ "path": item.path, "reason": "protected" }));
                continue;
            }
            if files {
                let mut skipped = 0;
                for m in self.members_of(&item.path) {
                    let ok = lstat(&m.path).is_some_and(|ms| Self::kind_ok(&ms, m.file) && mtime_ms(&ms) <= m.mtime + 2000.0)
                        && guard::is_under(&m.path, &item.path, plat)
                        && self.system_safe(&m.path)
                        && lock_of(&m.path, None, &open, &running, &self_dirs, plat).is_none()
                        && !(m.file && self.system.as_ref().is_some_and(|s| s.io.busy(&m.path)));
                    if ok {
                        targets.push(Target { path: m.path.clone(), item: item.clone(), member: Some(m) });
                    } else {
                        skipped += 1;
                    }
                }
                if skipped > 0 {
                    failed.push(json!({ "path": item.path, "reason": format!("skipped:{skipped}") }));
                }
                continue;
            }
            targets.push(Target { path: item.path.clone(), item, member: None });
        }
        if targets.is_empty() {
            let mut out = json!({ "done": [], "failed": failed, "freed": 0, "permanent": permanent });
            if failed.is_empty() {
                out["cancelled"] = json!(true);
            }
            return out;
        }
        let mut parents: HashMap<String, Option<String>> = HashMap::new();
        for t in &targets {
            parents.insert(self.key(&t.path), self.real_parent(&t.path));
        }
        let mut by_key: HashMap<String, usize> = targets.iter().enumerate().map(|(i, t)| (self.key(&t.path), i)).collect();
        for (i, t) in targets.iter().enumerate() {
            let Some(c) = guard::canonical_of(&t.path, plat, &RealIo) else { continue };
            if self.key(&c.path) == self.key(&t.path) {
                continue;
            }
            by_key.insert(self.key(&c.path), i);
            let up = parents.get(&self.key(&t.path)).cloned().flatten();
            parents.insert(self.key(&c.path), up);
        }
        let fresh: RefCell<(Vec<String>, Vec<Holder>, Vec<String>)> = RefCell::new((open, running, self_dirs));
        let refresh = || {
            *fresh.borrow_mut() = (self.open_list(&req.open), self.deps.running(), self.deps.self_dirs());
        };
        let precheck = |p: &str| -> Result<(), String> {
            let i = *by_key.get(&self.key(p)).ok_or("invalid")?;
            let t = &targets[i];
            let st = lstat(p).ok_or("ENOENT")?;
            let want_file = t.member.as_ref().map_or(t.item.file, |m| m.file);
            if !Self::kind_ok(&st, want_file) {
                return Err("changed".into());
            }
            let before = parents.get(&self.key(p)).cloned().flatten();
            if before.is_none() || self.real_parent(p) != before {
                return Err("changed".into());
            }
            let f = fresh.borrow();
            let l = if t.member.is_some() { lock_of(p, None, &f.0, &f.1, &f.2, plat) } else { lock_of(&t.item.path, t.item.project.as_deref(), &f.0, &f.1, &f.2, plat) };
            match l {
                Some(l) => Err(format!("locked:{l}")),
                None => Ok(()),
            }
        };
        let size_of = |p: &str| -> Option<u64> {
            let i = by_key.get(&self.key(p))?;
            let t = &targets[*i];
            Some(t.member.as_ref().map_or(t.item.size, |m| m.size))
        };
        let forget = |gone: &[String]| {
            let mut items = Vec::new();
            for g in gone {
                let Some(i) = by_key.get(&self.key(g)) else { continue };
                let t = &targets[*i];
                if t.member.is_some() {
                    let k = self.key(&t.item.path);
                    let mut members = lock(&self.members);
                    let rest: Vec<Member> = members.get(&k).cloned().unwrap_or_default().into_iter().filter(|m| self.key(&m.path) != self.key(g)).collect();
                    if rest.is_empty() {
                        members.remove(&k);
                        lock(&self.last).remove(&k);
                    } else {
                        members.insert(k, rest);
                    }
                } else {
                    lock(&self.last).remove(&self.key(g));
                    items.push(g.clone());
                }
            }
            if !items.is_empty() {
                self.deps.forget(&items);
            }
        };
        let gctx = |paths: &[String]| GuardContext { home: self.deps.home(), keep: self.deps.keep(), roots: paths.to_vec(), volumes: Vec::new() };
        let trash = |p: &str| precheck(p).and_then(|_| self.deps.trash(p));
        let hard = |p: &str| precheck(p).and_then(|_| self.deps.remove(p));
        let run = |paths: Vec<String>, perm: bool, confirm: &dyn Fn(&Ask) -> bool, confirm_perm: &dyn Fn(&Ask) -> bool| {
            run_delete(
                &paths,
                perm,
                &DeleteHooks { plat, ctx: gctx(&paths), first_count: 3, size_of: &size_of, confirm, confirm_perm, trash: &trash, remove: &hard, measure: &|p| self.deps.measure(p), forget: &forget, record: &|e| self.deps.record(e), now: self.deps.now() },
            )
        };
        let results: Vec<Value> = if req.one_tap {
            let (perm, soft): (Vec<&Target>, Vec<&Target>) = targets.iter().partition(|t| PERMANENT_OK.contains(&t.item.category.as_str()));
            let sizes = |list: &[&Target]| list.iter().map(|t| t.member.as_ref().map_or(t.item.size, |m| m.size)).sum::<u64>();
            let ask = OneTapAsk { bytes: sizes(&perm) + sizes(&soft), perm_bytes: sizes(&perm), perm_count: perm.len(), trash_bytes: sizes(&soft), trash_count: soft.len() };
            let choice = self.deps.ask_one_tap(&ask);
            if matches!(choice, OneTap::Cancel) {
                return json!({ "done": [], "failed": failed, "freed": 0, "permanent": permanent, "cancelled": true });
            }
            refresh();
            let yes = |_: &Ask| true;
            let (hard_list, rest): (Vec<String>, Vec<String>) = match choice {
                OneTap::Clean => (perm.iter().map(|t| t.path.clone()).collect(), soft.iter().map(|t| t.path.clone()).collect()),
                _ => (Vec::new(), targets.iter().map(|t| t.path.clone()).collect()),
            };
            let mut r = Vec::new();
            if !hard_list.is_empty() {
                r.push(run(hard_list, true, &yes, &yes));
            }
            if !rest.is_empty() {
                r.push(run(rest, false, &yes, &yes));
            }
            r
        } else {
            let all: Vec<String> = targets.iter().map(|t| t.path.clone()).collect();
            let first = |a: &Ask| {
                let ok = self.deps.confirm(a, false);
                if ok {
                    refresh();
                }
                ok
            };
            let second = |a: &Ask| {
                let ok = self.deps.confirm(a, true);
                if ok {
                    refresh();
                }
                ok
            };
            vec![run(all, permanent, &first, &second)]
        };
        self.fold(&results, failed, permanent, &targets, &by_key)
    }

    fn fold(&self, results: &[Value], failed: Vec<Value>, permanent: bool, targets: &[Target], by_key: &HashMap<String, usize>) -> Value {
        let mut done: Vec<(String, String, u64)> = Vec::new();
        let mut fails: Vec<(String, String, String, u64)> = Vec::new();
        let mut freed = 0u64;
        let mut cancelled = !results.is_empty();
        let mut any_perm = false;
        for r in results {
            if r["cancelled"] != json!(true) {
                cancelled = false;
            }
            let rdone = r["done"].as_array().cloned().unwrap_or_default();
            if r["permanent"] == json!(true) && !rdone.is_empty() {
                any_perm = true;
            }
            freed += r["freed"].as_u64().unwrap_or(0);
            for d in rdone {
                let p = d["path"].as_str().unwrap_or("").to_owned();
                let t = by_key.get(&self.key(&p)).map(|i| &targets[*i]);
                let owner = match t {
                    Some(t) if t.member.is_some() => t.item.path.clone(),
                    _ => p.clone(),
                };
                let k = self.key(&owner);
                match done.iter_mut().find(|e| e.0 == k) {
                    Some(e) => e.2 += d["size"].as_u64().unwrap_or(0),
                    None => done.push((k, owner, d["size"].as_u64().unwrap_or(0))),
                }
            }
            for f in r["failed"].as_array().cloned().unwrap_or_default() {
                let p = f["path"].as_str().unwrap_or("").to_owned();
                let reason = f["reason"].as_str().unwrap_or("failed").to_owned();
                let t = by_key.get(&self.key(&p)).map(|i| &targets[*i]);
                match t {
                    Some(t) if t.member.is_some() => {
                        let k = self.key(&t.item.path);
                        match fails.iter_mut().find(|e| e.0 == k) {
                            Some(e) => e.3 += 1,
                            None => fails.push((k, t.item.path.clone(), reason, 1)),
                        }
                    }
                    _ => {
                        let k = self.key(&p);
                        fails.retain(|e| e.0 != k);
                        fails.push((k, p, reason, 0));
                    }
                }
            }
        }
        let mut all_failed = failed;
        all_failed.extend(fails.into_iter().map(|(_, p, reason, n)| json!({ "path": p, "reason": if n > 0 { format!("some:{n}:{reason}") } else { reason } })));
        let done_v: Vec<Value> = done.into_iter().map(|(_, p, s)| json!({ "path": p, "size": s })).collect();
        let empty = done_v.is_empty();
        let mut out = json!({ "done": done_v, "failed": all_failed, "freed": freed, "permanent": permanent || any_perm });
        if cancelled && empty {
            out["cancelled"] = json!(true);
        }
        out
    }

    fn command_of(item: &Item) -> Option<(&'static str, Vec<String>)> {
        if item.action != Some("command") {
            return None;
        }
        let kind = item.kind.as_deref().unwrap_or("");
        if item.category == "docker" {
            return dev::docker_prune(kind).map(|a| ("docker", a.iter().map(|s| s.to_string()).collect()));
        }
        if item.category == "ios" && kind == "simUnavailable" {
            return Some(("xcrun", ["simctl", "delete", "unavailable"].iter().map(|s| s.to_string()).collect()));
        }
        None
    }

    pub fn run_command(&self, p: &str) -> Value {
        if self.deleting.load(Ordering::SeqCst) || self.scanning() {
            return json!({ "done": false, "freed": 0, "error": "busy" });
        }
        let item = self.known(p);
        let cmd = item.as_ref().and_then(Self::command_of);
        let (Some(item), Some((tool, args))) = (item, cmd) else { return json!({ "done": false, "freed": 0, "error": "invalid" }) };
        let available = if tool == "docker" { self.docker.is_some() } else { self.sdk.as_ref().is_some_and(|s| s.simctl.is_some()) };
        if !available {
            return json!({ "done": false, "freed": 0, "error": "invalid" });
        }
        if self.deleting.swap(true, Ordering::SeqCst) {
            return json!({ "done": false, "freed": 0, "error": "busy" });
        }
        let out = self.run_command_inner(&item, tool, &args);
        self.deleting.store(false, Ordering::SeqCst);
        out
    }

    fn run_command_inner(&self, item: &Item, tool: &str, args: &[String]) -> Value {
        let ask = CommandAsk { kind: item.kind.clone().unwrap_or_default(), bytes: item.size, count: item.files.unwrap_or(0), command: format!("{tool} {}", args.join(" ")), warn: item.warn };
        if !self.deps.ask_command(&ask) {
            return json!({ "done": false, "freed": 0, "cancelled": true });
        }
        let ok = if tool == "docker" {
            let Some(d) = &self.docker else { return json!({ "done": false, "freed": 0, "error": "invalid" }) };
            let r = (d.run)(args, PRUNE_TIMEOUT);
            if !r.ok {
                return json!({ "done": false, "freed": 0, "error": r.code.unwrap_or_else(|| "failed".into()) });
            }
            true
        } else {
            let Some(sim) = self.sdk.as_ref().and_then(|s| s.simctl.as_ref()) else { return json!({ "done": false, "freed": 0, "error": "invalid" }) };
            sim(args).is_some()
        };
        if !ok {
            return json!({ "done": false, "freed": 0, "error": "failed" });
        }
        let mut freed = item.size;
        let k = self.key(&item.path);
        if tool == "docker" {
            if let Some(after) = self.docker.as_ref().and_then(dev::docker_rows).and_then(|rows| rows.into_iter().find(|r| Some(r.kind) == item.kind.as_deref())) {
                freed = item.size.saturating_sub(after.reclaimable);
                let mut next = item.clone();
                next.size = after.reclaimable;
                next.files = Some(after.total.saturating_sub(after.active));
                next.detail = Some(format!("{}/{}", after.total, after.active));
                lock(&self.last).insert(k, next);
            }
        } else {
            let mut next = item.clone();
            next.size = 0;
            next.files = Some(0);
            lock(&self.last).insert(k, next);
        }
        self.deps.record(json!({ "at": self.deps.now(), "action": "delete", "count": item.files.unwrap_or(1), "freed": freed, "paths": [item.path], "failed": 0 }));
        json!({ "done": true, "freed": freed })
    }

    pub fn empty_bin(&self) -> Value {
        let Some(s) = &self.system else { return json!({ "done": false, "freed": 0, "error": "busy" }) };
        if self.deleting.load(Ordering::SeqCst) || self.scanning() {
            return json!({ "done": false, "freed": 0, "error": "busy" });
        }
        let item = lock(&self.last).values().find(|i| i.action == Some("bin")).cloned();
        let Some(item) = item else { return json!({ "done": false, "freed": 0, "error": "invalid" }) };
        if self.deleting.swap(true, Ordering::SeqCst) {
            return json!({ "done": false, "freed": 0, "error": "busy" });
        }
        let out = (|| {
            let info = s.io.bin_size();
            let bytes = info.map_or(item.size, |i| i.0);
            let count = info.map_or(item.files.unwrap_or(0), |i| i.1);
            if !self.deps.ask_bin(bytes, count) {
                return json!({ "done": false, "freed": 0, "cancelled": true });
            }
            if let Err(e) = s.io.bin_empty() {
                return json!({ "done": false, "freed": 0, "error": reason_of(&e) });
            }
            let mut next = item.clone();
            next.size = 0;
            next.files = Some(0);
            lock(&self.last).insert(self.key(&item.path), next);
            self.deps.record(json!({ "at": self.deps.now(), "action": "delete", "count": count, "freed": bytes, "paths": [item.path], "failed": 0 }));
            json!({ "done": true, "freed": bytes })
        })();
        self.deleting.store(false, Ordering::SeqCst);
        out
    }

    pub fn handle(&self, channel: &str, a: &Value) -> Value {
        let nothing = json!({ "done": [], "failed": [], "freed": 0, "permanent": false, "cancelled": true });
        match channel {
            "ccm:diskclean-categories" => self.categories(),
            "ccm:diskclean-scan" => match parse_scan(a) {
                Some(req) => self.scan(&req),
                None => json!({ "items": [] }),
            },
            "ccm:diskclean-cancel" => json!(self.cancel()),
            "ccm:diskclean-delete" => match parse_delete(a) {
                Some(req) => self.remove(&req),
                None => nothing,
            },
            "ccm:diskclean-empty-bin" => self.empty_bin(),
            "ccm:diskclean-command" => match parse_command(a) {
                Some(p) => self.run_command(&p),
                None => json!({ "done": false, "freed": 0, "error": "invalid" }),
            },
            _ => Value::Null,
        }
    }
}

pub struct DeleteHooks<'a> {
    pub plat: Plat,
    pub ctx: GuardContext,
    pub first_count: usize,
    pub size_of: &'a dyn Fn(&str) -> Option<u64>,
    pub confirm: &'a dyn Fn(&Ask) -> bool,
    pub confirm_perm: &'a dyn Fn(&Ask) -> bool,
    pub trash: &'a dyn Fn(&str) -> Result<(), String>,
    pub remove: &'a dyn Fn(&str) -> Result<(), String>,
    pub measure: &'a dyn Fn(&str) -> Option<u64>,
    pub forget: &'a dyn Fn(&[String]),
    pub record: &'a dyn Fn(Value),
    pub now: f64,
}

pub fn run_delete(paths: &[String], permanent: bool, h: &DeleteHooks) -> Value {
    let plat = h.plat;
    let key = |p: &str| guard::normalize(p, plat).to_lowercase();
    let mut failed: Vec<Value> = Vec::new();
    let mut allowed: Vec<String> = Vec::new();
    let mut canon: Vec<(String, guard::Canonical)> = Vec::new();
    for p in paths {
        match guard::resolve_deletable(p, &h.ctx, plat, &RealIo) {
            Err(reason) => failed.push(json!({ "path": p, "reason": reason })),
            Ok(c) => {
                allowed.push(p.clone());
                canon.push((key(p), c));
            }
        }
    }
    let targets = guard::top_level(&allowed, plat);
    if targets.is_empty() {
        return json!({ "done": [], "failed": failed, "freed": 0, "permanent": permanent });
    }
    let sizes: Vec<Option<u64>> = targets.iter().map(|p| (h.size_of)(p)).collect();
    let first: Vec<String> = targets.iter().take(h.first_count).cloned().collect();
    let ask = Ask { count: targets.len(), bytes: sizes.iter().map(|s| s.unwrap_or(0)).sum(), first: &first, permanent };
    if !(h.confirm)(&ask) || (permanent && !(h.confirm_perm)(&ask)) {
        return json!({ "done": [], "failed": failed, "freed": 0, "permanent": permanent, "cancelled": true });
    }
    let mut done: Vec<Value> = Vec::new();
    let mut freed = 0u64;
    let mut gone: Vec<String> = Vec::new();
    let mut partial: Vec<String> = Vec::new();
    for (i, p) in targets.iter().enumerate() {
        let before = canon.iter().find(|(k, _)| *k == key(p)).map(|(_, c)| c.clone());
        let target = match (before, guard::resolve_deletable(p, &h.ctx, plat, &RealIo)) {
            (_, Err(reason)) => {
                failed.push(json!({ "path": p, "reason": reason }));
                continue;
            }
            (Some(b), Ok(a)) if guard::same_canonical(&b, &a, plat) => a.path,
            _ => {
                failed.push(json!({ "path": p, "reason": "changed" }));
                continue;
            }
        };
        let size = sizes[i].unwrap_or(0);
        match if permanent { (h.remove)(&target) } else { (h.trash)(&target) } {
            Ok(()) => {
                done.push(json!({ "path": p, "size": size }));
                freed += size;
                gone.push(p.clone());
            }
            Err(reason) => {
                failed.push(json!({ "path": p, "reason": reason }));
                if reason == "ENOENT" {
                    gone.push(p.clone());
                } else if permanent && reason != "EACCES" && reason != "EPERM" {
                    gone.push(p.clone());
                    if let Some(rest) = (h.measure)(&target) {
                        if size > rest {
                            freed += size - rest;
                            partial.push(p.clone());
                        }
                    }
                }
            }
        }
    }
    if !gone.is_empty() {
        (h.forget)(&gone);
    }
    if !done.is_empty() || !failed.is_empty() {
        let mut list: Vec<Value> = done.iter().map(|d| d["path"].clone()).collect();
        list.extend(partial.iter().map(|p| json!(p)));
        (h.record)(json!({ "at": h.now, "action": if permanent { "delete" } else { "trash" }, "count": done.len(), "freed": freed, "paths": list, "failed": failed.len() }));
    }
    json!({ "done": done, "failed": failed, "freed": freed, "permanent": permanent })
}

fn e2e_on() -> bool {
    cfg!(debug_assertions) || crate::config::e2e_isolated()
}

pub fn e2e_env(name: &str) -> Option<String> {
    std::env::var(name).ok().map(|v| v.trim().to_owned()).filter(|v| !v.is_empty() && e2e_on())
}

fn auto_confirm() -> bool {
    e2e_env("CCM_E2E_DISKMAP_CONFIRM").is_some_and(|v| v == "1")
}

pub struct AppDeps {
    app: AppHandle,
}

impl AppDeps {
    fn disk(&self) -> Option<Arc<crate::disk_map::DiskMap>> {
        self.app.try_state::<crate::disk_map::DiskState>().map(|s| s.0.clone())
    }

    fn ask_n(&self, b: &BoxN) -> usize {
        crate::disk_clean_text::ask_buttons(&self.app, b)
    }
}

fn desk_runs(app: &AppHandle) -> Vec<Value> {
    match app.try_state::<crate::desk::DeskState>() {
        Some(d) => d.desk.dispatch("desk:run-sessions", Value::Null).ok().and_then(|v| v.as_array().cloned()).unwrap_or_default(),
        None => Vec::new(),
    }
}

impl Deps for AppDeps {
    fn plat(&self) -> Plat {
        Plat::host()
    }

    fn home(&self) -> String {
        crate::disk_map::home_of(&self.app)
    }

    fn env(&self, k: &str) -> Option<String> {
        std::env::var(k).ok()
    }

    fn keep(&self) -> Vec<String> {
        crate::disk_map::keep_of(&self.app)
    }

    fn self_dirs(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
            out.push(dir.to_string_lossy().into_owned());
        }
        out.push(self.app.state::<Shell>().data_dir.to_string_lossy().into_owned());
        if cfg!(debug_assertions) {
            if let Some(repo) = Path::new(env!("CARGO_MANIFEST_DIR")).parent().and_then(Path::parent) {
                out.push(repo.to_string_lossy().into_owned());
            }
        }
        out
    }

    fn running(&self) -> Vec<Holder> {
        let mut out = Vec::new();
        for r in desk_runs(&self.app) {
            let live = core_desk::run::sessions::is_live(r["state"].as_str().unwrap_or(""));
            if let Some(cwd) = r["cwd"].as_str().filter(|c| live && !c.is_empty()) {
                out.push(Holder { cwd: cwd.to_owned(), lock: "run" });
            }
        }
        let report = core_serve::sampler::latest_resources().unwrap_or(Value::Null);
        for s in report["servers"].as_array().into_iter().flatten() {
            if let Some(cwd) = s["cwd"].as_str().filter(|c| !c.is_empty() && s["kind"] != json!("daemon")) {
                out.push(Holder { cwd: cwd.to_owned(), lock: "server" });
            }
        }
        out
    }

    fn emit(&self, p: Value) {
        crate::desk::emit_app(&self.app, PROGRESS, p);
    }

    fn confirm(&self, ask: &Ask, second: bool) -> bool {
        if auto_confirm() {
            return true;
        }
        let b = delete_box(ask, second, &crate::hosts::locale(&self.app), cfg!(target_os = "macos"));
        crate::hosts::confirm(&self.app, &b.message, &b.detail, &b.ok, &b.cancel)
    }

    fn ask_one_tap(&self, ask: &OneTapAsk) -> OneTap {
        if auto_confirm() {
            return OneTap::Clean;
        }
        let b = one_tap_box(ask, &crate::hosts::locale(&self.app), cfg!(target_os = "macos"));
        match self.ask_n(&b) {
            0 => OneTap::Clean,
            1 if ask.perm_count > 0 => OneTap::Trash,
            _ => OneTap::Cancel,
        }
    }

    fn ask_bin(&self, bytes: u64, count: u64) -> bool {
        if auto_confirm() {
            return true;
        }
        self.ask_n(&bin_box(bytes, count, &crate::hosts::locale(&self.app), cfg!(target_os = "macos"))) == 0
    }

    fn ask_command(&self, ask: &CommandAsk) -> bool {
        if auto_confirm() {
            return true;
        }
        self.ask_n(&command_box(ask, &crate::hosts::locale(&self.app))) == 0
    }

    fn trash(&self, p: &str) -> Result<(), String> {
        crate::disk_map::trash_path(p)
    }

    fn remove(&self, p: &str) -> Result<(), String> {
        crate::disk_map::remove_path(p)
    }

    fn measure(&self, p: &str) -> Option<u64> {
        crate::disk_map::measure_rest(p)
    }

    fn record(&self, e: Value) {
        if let Some(m) = self.disk() {
            m.history.append(&e);
        }
    }

    fn forget(&self, paths: &[String]) {
        let Some(m) = self.disk() else { return };
        if !m.host.running() {
            return;
        }
        let _ = m.host.call("disk.forget", json!({ "paths": paths }));
        let host = m.host.clone();
        std::thread::spawn(move || {
            let _ = host.call("disk.save", json!({}));
        });
    }

    fn index_size(&self, p: &str) -> Option<u64> {
        let m = self.disk()?;
        if !m.host.running() {
            return None;
        }
        let st = crate::disk_map::to_status(&m.host.call("disk.status", json!({})).ok()?);
        if st["indexed"] != json!(true) || st["scanning"] == json!(true) {
            return None;
        }
        if !crate::disk_map::status_roots(&st).iter().any(|r| guard::is_under(p, r, Plat::host())) {
            return None;
        }
        m.size_of(p)
    }

    fn now(&self) -> f64 {
        crate::disk_map::now_ms() as f64
    }
}

pub struct DiskCleanState(pub Arc<Service<AppDeps>>);

fn build(app: &AppHandle) -> Service<AppDeps> {
    let plat = Plat::host();
    let home = crate::disk_map::home_of(app);
    let env = |k: &str| std::env::var(k).ok();
    let download = app.path().download_dir().ok().map(|d| d.to_string_lossy().into_owned());
    let system = sys::system_deps(plat, &home, &env, download, e2e_env("CCM_E2E_DISKCLEAN_SYS"));
    let sdk = dev::sdk_deps(plat, &home, &env, e2e_env("CCM_E2E_DISKCLEAN_SDK"));
    let docker = dev::docker_deps(e2e_env("CCM_E2E_DOCKER"));
    Service::new(AppDeps { app: app.clone() }, Some(system), Some(sdk), Some(docker))
}

pub fn service(app: &AppHandle) -> Arc<Service<AppDeps>> {
    if let Some(s) = app.try_state::<DiskCleanState>() {
        return s.0.clone();
    }
    let svc = Arc::new(build(app));
    app.manage(DiskCleanState(svc.clone()));
    app.try_state::<DiskCleanState>().map_or(svc, |s| s.0.clone())
}

pub fn known_plain(app: &AppHandle, p: &str) -> Option<String> {
    let s = app.try_state::<DiskCleanState>()?;
    let it = s.0.known(p)?;
    (it.action.is_none() && !it.file).then_some(it.path)
}

pub fn stop(app: &AppHandle) {
    if let Some(s) = app.try_state::<DiskCleanState>() {
        s.0.cancel();
    }
}

pub fn dispatch(app: &AppHandle, channel: &str, args: &[Value]) -> Result<Value, String> {
    if !handles(channel) {
        return Err(format!("not ported: {channel}"));
    }
    let a = args.first().cloned().unwrap_or(Value::Null);
    Ok(service(app).handle(channel, &a))
}

#[cfg(test)]
#[path = "disk_clean_tests.rs"]
mod tests;
