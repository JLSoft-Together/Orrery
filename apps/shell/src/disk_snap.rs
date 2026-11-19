use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, Weak};
use std::time::Duration;

use serde_json::{json, Value};
use tauri::AppHandle;

use crate::disk_guard::{path_key, Plat};
use crate::disk_map::{now_ms, status_roots, to_status, DiskMap};
use crate::state::lock;

pub const SCHEDULE_EVENT: &str = "ccm:diskmap-schedule";
pub const SNAP_EVENT: &str = "ccm:diskmap-snap-new";
const DEPTH: usize = 4;
const KEEP: usize = 30;
const TOP: usize = 20;
const DIRS_MAX: usize = 60_000;
const ROOTS_MAX: usize = 64;
const PATH_MAX: usize = 2048;
const VOLUMES_MAX: usize = 64;
const FILE_MAX: u64 = 64 * 1024 * 1024;
const ROWS_MAX: usize = 2000;
const COMPARE_TTL_MS: u64 = 60_000;
const IDLE_START_S: f64 = 600.0;
const IDLE_PAUSE_S: f64 = 60.0;
const IDLE_RESUME_S: f64 = 300.0;
const RETRY_MS: u64 = 3_600_000;
const TICK_MS: u64 = 60_000;
const PREEMPT: Duration = Duration::from_secs(5);
const SKEW_MS: u64 = 60_000;

#[derive(Clone, Debug, PartialEq)]
pub struct Root {
    pub path: String,
    pub size: u64,
    pub dirs: Vec<(String, u64)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Snapshot {
    pub at: u64,
    pub roots: Vec<Root>,
    pub volumes: Vec<Value>,
}

fn size(v: &Value) -> u64 {
    v.as_f64().filter(|n| n.is_finite() && *n >= 0.0).map_or(0, |n| n.floor() as u64)
}

fn path_text(v: &Value) -> Option<String> {
    v.as_str().filter(|s| !s.is_empty() && s.chars().count() <= PATH_MAX && !s.contains('\0')).map(str::to_owned)
}

pub fn clean_roots(raw: &Value) -> Vec<Root> {
    let mut left = DIRS_MAX;
    let mut out = Vec::new();
    for r in raw.as_array().into_iter().flatten().take(ROOTS_MAX) {
        let Some(p) = path_text(&r["path"]) else { continue };
        let mut dirs = Vec::new();
        for d in r["dirs"].as_array().into_iter().flatten() {
            if left == 0 {
                break;
            }
            let Some(pair) = d.as_array() else { continue };
            let Some(dp) = pair.first().and_then(path_text) else { continue };
            dirs.push((dp, pair.get(1).map_or(0, size)));
            left -= 1;
        }
        out.push(Root { path: p, size: size(&r["size"]), dirs });
    }
    out
}

pub fn clean_volumes(raw: &Value) -> Vec<Value> {
    raw.as_array()
        .into_iter()
        .flatten()
        .take(VOLUMES_MAX)
        .filter_map(|v| {
            let mount = path_text(&v["mount"])?;
            let label: String = v["label"].as_str().unwrap_or("").chars().take(120).collect();
            Some(json!({ "mount": mount, "label": label, "free": size(&v["free"]), "total": size(&v["total"]) }))
        })
        .collect()
}

pub fn to_snapshot(summary: &Value, volumes: &Value, at: u64) -> Option<Snapshot> {
    let roots = clean_roots(&summary["roots"]);
    (!roots.is_empty()).then(|| Snapshot { at, roots, volumes: clean_volumes(volumes) })
}

pub fn parse_snapshot(raw: &Value) -> Option<Snapshot> {
    if raw["v"] != json!(1) {
        return None;
    }
    let at = raw["at"].as_f64().filter(|n| n.is_finite() && *n > 0.0)?.floor() as u64;
    let roots = clean_roots(&raw["roots"]);
    (!roots.is_empty()).then(|| Snapshot { at, roots, volumes: clean_volumes(&raw["volumes"]) })
}

fn snap_json(s: &Snapshot) -> Value {
    let roots: Vec<Value> = s.roots.iter().map(|r| json!({ "path": r.path, "size": r.size, "dirs": r.dirs.iter().map(|(p, n)| json!([p, n])).collect::<Vec<_>>() })).collect();
    json!({ "v": 1, "at": s.at, "roots": roots, "volumes": s.volumes })
}

fn bytes_of(s: &Snapshot) -> u64 {
    s.roots.iter().map(|r| r.size).sum()
}

fn meta_of(id: &str, s: &Snapshot) -> Value {
    json!({ "id": id, "at": s.at, "bytes": bytes_of(s), "roots": s.roots.iter().map(|r| r.path.clone()).collect::<Vec<_>>() })
}

pub fn is_snapshot_id(v: &str) -> bool {
    let Some((seq, at)) = v.split_once('-') else { return false };
    seq.len() == 8 && seq.bytes().all(|b| b.is_ascii_digit()) && (10..=15).contains(&at.len()) && at.bytes().all(|b| b.is_ascii_digit())
}

fn file_seq(name: &str) -> Option<u64> {
    let id = name.strip_suffix(".json.gz")?;
    is_snapshot_id(id).then(|| id[..8].parse().ok()).flatten()
}

pub struct Store {
    dir: PathBuf,
    metas: Mutex<HashMap<String, Option<Value>>>,
}

impl Store {
    pub fn new(dir: PathBuf) -> Self {
        Store { dir, metas: Mutex::new(HashMap::new()) }
    }

    fn names(&self) -> Vec<(String, u64)> {
        let mut out: Vec<(String, u64)> = std::fs::read_dir(&self.dir)
            .map(|rd| rd.flatten().filter_map(|e| e.file_name().to_str().and_then(|n| file_seq(n).map(|s| (n.to_owned(), s)))).collect())
            .unwrap_or_default();
        out.sort_by_key(|(_, s)| *s);
        out
    }

    pub fn write(&self, snap: &Snapshot) -> Option<Value> {
        std::fs::create_dir_all(&self.dir).ok()?;
        let names = self.names();
        let seq = names.last().map_or(0, |(_, s)| *s) + 1;
        let id = format!("{seq:08}-{}", snap.at);
        let file = self.dir.join(format!("{id}.json.gz"));
        let tmp = self.dir.join(format!("{id}.json.gz.tmp"));
        let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::new(6));
        enc.write_all(snap_json(snap).to_string().as_bytes()).ok()?;
        let body = enc.finish().ok()?;
        if std::fs::write(&tmp, body).and_then(|_| std::fs::rename(&tmp, &file)).is_err() {
            let _ = std::fs::remove_file(&tmp);
            return None;
        }
        let meta = meta_of(&id, snap);
        let mut metas = lock(&self.metas);
        metas.insert(format!("{id}.json.gz"), Some(meta.clone()));
        let mut all = names;
        all.push((format!("{id}.json.gz"), seq));
        let drop_n = all.len().saturating_sub(KEEP);
        for (old, _) in all.iter().take(drop_n) {
            let _ = std::fs::remove_file(self.dir.join(old));
            metas.remove(old);
        }
        Some(meta)
    }

    pub fn read(&self, id: &str) -> Option<Snapshot> {
        if !is_snapshot_id(id) {
            return None;
        }
        let file = self.dir.join(format!("{id}.json.gz"));
        let meta = std::fs::metadata(&file).ok()?;
        if !meta.is_file() || meta.len() > FILE_MAX {
            return None;
        }
        let raw = std::fs::read(&file).ok()?;
        let mut text = String::new();
        flate2::read::GzDecoder::new(&raw[..]).take(FILE_MAX).read_to_string(&mut text).ok()?;
        parse_snapshot(&serde_json::from_str(&text).ok()?)
    }

    pub fn list(&self) -> Vec<Value> {
        let names = self.names();
        let mut metas = lock(&self.metas);
        metas.retain(|k, _| names.iter().any(|(n, _)| n == k));
        let mut out = Vec::new();
        for (name, _) in names.iter().rev() {
            let id = name.trim_end_matches(".json.gz");
            if !metas.contains_key(name) {
                let m = self.read(id).map(|s| meta_of(id, &s));
                metas.insert(name.clone(), m);
            }
            if let Some(Some(m)) = metas.get(name) {
                out.push(m.clone());
            }
        }
        out
    }

    pub fn latest(&self) -> Option<Snapshot> {
        self.list().iter().find_map(|m| m["id"].as_str().and_then(|id| self.read(id)))
    }
}

fn parent_of(p: &str) -> String {
    let t = p.trim_end_matches(['\\', '/']);
    t.rfind(['\\', '/']).map_or(String::new(), |i| t[..i].to_owned())
}

pub fn dir_name(p: &str) -> String {
    let t = p.trim_end_matches(['\\', '/']);
    let n = t.rfind(['\\', '/']).map_or(t, |i| &t[i + 1..]);
    if n.is_empty() {
        p.to_owned()
    } else {
        n.to_owned()
    }
}

#[derive(Clone, Debug)]
struct Node {
    path: String,
    before: Option<u64>,
    after: Option<u64>,
    depth: usize,
    root: bool,
}

#[derive(Default)]
pub struct Compared {
    nodes: HashMap<String, Node>,
    children: HashMap<String, Vec<String>>,
    roots: Vec<String>,
}

fn put(c: &mut Compared, key: &str, p: &str, after: bool, size: u64, depth: usize, parent: &str, root: bool) {
    if let Some(n) = c.nodes.get_mut(key) {
        if after {
            n.after = Some(size);
        } else {
            n.before = Some(size);
        }
        return;
    }
    let (before, after_v) = if after { (None, Some(size)) } else { (Some(size), None) };
    c.nodes.insert(key.to_owned(), Node { path: p.to_owned(), before, after: after_v, depth, root });
    if !root {
        c.children.entry(parent.to_owned()).or_default().push(key.to_owned());
    }
}

pub fn build_compared(base: &Snapshot, cur: &Snapshot, plat: Plat) -> Compared {
    let mut c = Compared::default();
    for root in &cur.roots {
        let rk = path_key(&root.path, plat);
        let Some(b) = base.roots.iter().find(|r| path_key(&r.path, plat) == rk) else { continue };
        c.roots.push(rk.clone());
        put(&mut c, &rk, &root.path, true, root.size, 0, "", true);
        put(&mut c, &rk, &b.path, false, b.size, 0, "", true);
        let mut depth_of: HashMap<String, usize> = HashMap::from([(rk.clone(), 0)]);
        for (after, dirs) in [(true, &root.dirs), (false, &b.dirs)] {
            for (p, size) in dirs {
                let key = path_key(p, plat);
                let parent = path_key(&parent_of(p), plat);
                let depth = depth_of.get(&parent).copied().unwrap_or(0) + 1;
                depth_of.insert(key.clone(), depth);
                put(&mut c, &key, p, after, *size, depth, &parent, false);
            }
        }
    }
    c
}

fn row_of(n: &Node) -> Value {
    let before = n.before.unwrap_or(0) as i64;
    let after = n.after.unwrap_or(0) as i64;
    let state = if n.before.is_none() {
        "new"
    } else if n.after.is_none() {
        "gone"
    } else if after == before {
        "same"
    } else {
        "changed"
    };
    json!({ "path": n.path, "name": dir_name(&n.path), "before": before, "after": after, "delta": after - before, "state": state })
}

fn delta(n: &Node) -> i64 {
    n.after.unwrap_or(0) as i64 - n.before.unwrap_or(0) as i64
}

fn sort_rows(rows: &mut [Value]) {
    rows.sort_by(|a, b| b["delta"].as_i64().cmp(&a["delta"].as_i64()).then_with(|| a["path"].as_str().cmp(&b["path"].as_str())));
}

pub fn top_growth(c: &Compared) -> Vec<Value> {
    let mut out: Vec<Value> = c
        .nodes
        .iter()
        .filter(|(key, n)| {
            let d = delta(n);
            !n.root && d > 0 && !c.children.get(*key).into_iter().flatten().any(|k| c.nodes.get(k).is_some_and(|ch| delta(ch) as f64 >= d as f64 * 0.8))
        })
        .map(|(_, n)| row_of(n))
        .collect();
    sort_rows(&mut out);
    out.truncate(TOP);
    out
}

pub fn compare_at(c: &Compared, at: &str, plat: Plat, base: &Value) -> Value {
    let top = top_growth(c);
    if at.is_empty() {
        let mut rows: Vec<Value> = c.roots.iter().filter_map(|k| c.nodes.get(k)).map(row_of).collect();
        sort_rows(&mut rows);
        rows.truncate(ROWS_MAX);
        return json!({ "base": base, "path": "", "covered": true, "rows": rows, "top": top });
    }
    let key = path_key(at, plat);
    let covered = c.nodes.get(&key).is_some_and(|n| n.depth < DEPTH);
    let mut rows: Vec<Value> = if covered { c.children.get(&key).into_iter().flatten().filter_map(|k| c.nodes.get(k)).map(row_of).collect() } else { Vec::new() };
    sort_rows(&mut rows);
    rows.truncate(ROWS_MAX);
    json!({ "base": base, "path": at, "covered": covered, "rows": rows, "top": top })
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScheduleFile {
    pub freq: String,
    pub telegram: bool,
    pub last_at: Option<u64>,
    pub fail_at: Option<u64>,
    pub attempt_at: Option<u64>,
}

fn is_freq(v: &str) -> bool {
    matches!(v, "off" | "daily" | "weekly")
}

fn stamp(v: &Value) -> Option<u64> {
    v.as_f64().filter(|n| n.is_finite() && *n > 0.0).map(|n| n.floor() as u64)
}

pub fn read_schedule(file: &Path) -> ScheduleFile {
    let raw = std::fs::read_to_string(file).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).filter(Value::is_object).unwrap_or(json!({}));
    ScheduleFile {
        freq: raw["freq"].as_str().filter(|f| is_freq(f)).unwrap_or("daily").to_owned(),
        telegram: raw["telegram"] != json!(false),
        last_at: stamp(&raw["lastAt"]),
        fail_at: stamp(&raw["failAt"]),
        attempt_at: stamp(&raw["attemptAt"]),
    }
}

struct Sched {
    data: ScheduleFile,
    running: bool,
    started_at: u64,
    paused_by_us: bool,
}

pub struct DiskSnap {
    store: Store,
    file: PathBuf,
    sched: Mutex<Sched>,
    serial: Mutex<()>,
    ticking: AtomicBool,
    stopped: AtomicBool,
    cache: Mutex<Option<(String, u64, Value, Compared)>>,
    alert_at: Mutex<Option<u64>>,
}

fn freq_ms(freq: &str) -> Option<u64> {
    match freq {
        "daily" => Some(24 * 3_600_000),
        "weekly" => Some(7 * 24 * 3_600_000),
        _ => None,
    }
}

pub fn due(data: &ScheduleFile, now: u64) -> bool {
    let Some(every) = freq_ms(&data.freq) else { return false };
    let ok = |t: Option<u64>| t.filter(|t| *t <= now + SKEW_MS);
    let since = |t: Option<u64>| ok(t).map_or(u64::MAX, |t| now.saturating_sub(t));
    since(data.last_at) >= every && since(data.attempt_at) >= RETRY_MS
}

fn e2e_allowed() -> bool {
    cfg!(debug_assertions) || crate::config::e2e_isolated()
}

fn tick_ms() -> u64 {
    if !e2e_allowed() {
        return TICK_MS;
    }
    std::env::var("CCM_E2E_DISKSNAP_TICK").ok().and_then(|v| v.trim().parse::<u64>().ok()).filter(|t| *t >= 200).unwrap_or(TICK_MS)
}

fn fake() -> Value {
    if !e2e_allowed() {
        return Value::Null;
    }
    std::env::var("CCM_E2E_DISKSNAP_FAKE").ok().filter(|v| !v.is_empty()).and_then(|f| std::fs::read_to_string(f).ok()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Null)
}

fn idle_seconds() -> f64 {
    fake()["idle"].as_f64().unwrap_or_else(crate::host_extra::idle_seconds)
}

fn on_battery() -> bool {
    fake()["battery"].as_bool().unwrap_or_else(|| core_desk::power::native::power().on_battery())
}

impl DiskSnap {
    pub fn new(disk_dir: &Path) -> Self {
        let file = disk_dir.join("schedule.json");
        DiskSnap {
            store: Store::new(disk_dir.join("snapshots")),
            sched: Mutex::new(Sched { data: read_schedule(&file), running: false, started_at: 0, paused_by_us: false }),
            file,
            serial: Mutex::new(()),
            ticking: AtomicBool::new(false),
            stopped: AtomicBool::new(false),
            cache: Mutex::new(None),
            alert_at: Mutex::new(None),
        }
    }

    pub fn list(&self) -> Vec<Value> {
        self.store.list()
    }

    fn capture(&self, m: &DiskMap) -> Option<Snapshot> {
        let st = m.status().ok()?;
        let roots = m.scheduled_roots(&status_roots(&st));
        if st["indexed"] != json!(true) || roots.is_empty() {
            return None;
        }
        let summary = m.host.call("disk.summary", json!({ "roots": roots, "depth": DEPTH })).unwrap_or(Value::Null);
        let mut volumes = m.volumes();
        if let Some(free) = fake()["free"].as_f64() {
            for v in volumes.as_array_mut().into_iter().flatten() {
                v["free"] = json!(free);
            }
        }
        let vols: Vec<Value> = volumes.as_array().into_iter().flatten().filter(|v| v["network"] != json!(true)).map(|v| json!({ "mount": v["mount"], "label": v["label"], "free": v["free"], "total": v["total"] })).collect();
        to_snapshot(&summary, &Value::Array(vols), now_ms())
    }

    pub fn on_done(&self, app: &AppHandle, m: &DiskMap, done: &Value) {
        let bg = self.note_done(app, done);
        *lock(&self.cache) = None;
        if done["ok"] != json!(true) {
            return;
        }
        let _g = lock(&self.serial);
        let prev = if bg { self.store.latest() } else { None };
        let Some(cur) = self.capture(m) else { return };
        if let Some(meta) = self.store.write(&cur) {
            crate::desk::emit_app(app, SNAP_EVENT, meta);
        }
        if let Some(prev) = prev {
            self.report(app, &prev, &cur, false);
        }
    }

    pub fn on_alert(&self, app: &AppHandle, m: &DiskMap) -> bool {
        let _g = lock(&self.serial);
        let Some(base) = self.store.latest() else { return false };
        let Some(cur) = self.capture(m) else { return false };
        self.report(app, &base, &cur, true)
    }

    fn report(&self, app: &AppHandle, base: &Snapshot, cur: &Snapshot, alert: bool) -> bool {
        if !lock(&self.sched).data.telegram {
            return false;
        }
        let now = now_ms();
        if alert && lock(&self.alert_at).is_some_and(|at| now >= at && now - at < report::ALERT_GAP_MS) {
            return false;
        }
        let en = crate::hosts::locale(app).starts_with("en");
        let home = crate::disk_map::home_of(app);
        let Some(body) = report::compose_local(base, cur, alert, en, Plat::host(), &home) else { return false };
        if !report::telegram_on(app) {
            return false;
        }
        let ok = report::send(app, &body);
        if ok && alert {
            *lock(&self.alert_at) = Some(now);
        }
        ok
    }

    pub fn compare(&self, m: &DiskMap, raw: &Value) -> Value {
        let Some(id) = raw["id"].as_str().filter(|i| is_snapshot_id(i)).map(str::to_owned) else { return Value::Null };
        let path = match &raw["path"] {
            Value::Null => String::new(),
            Value::String(s) if s.chars().count() <= PATH_MAX && !s.contains('\0') => s.clone(),
            _ => return Value::Null,
        };
        let now = now_ms();
        let plat = Plat::host();
        let mut cache = lock(&self.cache);
        let fresh = cache.as_ref().is_some_and(|(cid, at, _, _)| *cid == id && now.saturating_sub(*at) <= COMPARE_TTL_MS);
        if path.is_empty() || !fresh {
            let Some(snap) = self.store.read(&id) else { return Value::Null };
            let Some(base) = self.store.list().into_iter().find(|meta| meta["id"] == json!(id)) else { return Value::Null };
            let Some(cur) = self.capture(m) else { return Value::Null };
            *cache = Some((id.clone(), now, base, build_compared(&snap, &cur, plat)));
        }
        let Some((_, _, base, compared)) = cache.as_ref() else { return Value::Null };
        compare_at(compared, &path, plat, base)
    }

    fn state_of(s: &Sched) -> Value {
        let last = match (s.data.last_at, s.data.fail_at) {
            (last, Some(fail)) if last.is_none_or(|l| fail > l) => json!({ "at": fail, "ok": false }),
            (Some(l), _) => json!({ "at": l, "ok": true }),
            _ => Value::Null,
        };
        json!({ "freq": s.data.freq, "telegram": s.data.telegram, "last": last, "running": s.running, "paused": s.running && s.paused_by_us })
    }

    pub fn schedule_state(&self) -> Value {
        Self::state_of(&lock(&self.sched))
    }

    fn save(&self, data: &ScheduleFile) {
        let mut o = json!({ "freq": data.freq, "telegram": data.telegram });
        for (k, v) in [("lastAt", data.last_at), ("failAt", data.fail_at), ("attemptAt", data.attempt_at)] {
            if let Some(v) = v {
                o[k] = json!(v);
            }
        }
        if let Some(dir) = self.file.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = self.file.with_extension("json.tmp");
        if std::fs::write(&tmp, o.to_string()).and_then(|_| std::fs::rename(&tmp, &self.file)).is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
    }

    fn emit(&self, app: &AppHandle) {
        crate::desk::emit_app(app, SCHEDULE_EVENT, self.schedule_state());
    }

    pub fn schedule_set(&self, app: &AppHandle, m: &DiskMap, raw: &Value) -> Value {
        let cancel = {
            let mut s = lock(&self.sched);
            if let Some(f) = raw["freq"].as_str().filter(|f| is_freq(f)) {
                s.data.freq = f.to_owned();
            }
            if let Some(t) = raw["telegram"].as_bool() {
                s.data.telegram = t;
            }
            self.save(&s.data);
            s.data.freq == "off" && s.running
        };
        if cancel {
            m.control("cancel");
        }
        self.emit(app);
        self.schedule_state()
    }

    fn note_done(&self, app: &AppHandle, done: &Value) -> bool {
        {
            let mut s = lock(&self.sched);
            if !s.running {
                return false;
            }
            s.running = false;
            s.paused_by_us = false;
            if done["ok"] == json!(true) {
                s.data.last_at = Some(now_ms());
            } else if done["cancelled"] != json!(true) {
                s.data.fail_at = Some(now_ms());
            }
            self.save(&s.data);
        }
        self.emit(app);
        true
    }

    pub fn before_scan(&self, m: &DiskMap) {
        if !lock(&self.sched).running {
            return;
        }
        m.control("cancel");
        let end = std::time::Instant::now() + PREEMPT;
        while lock(&self.sched).running && std::time::Instant::now() < end {
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    fn tick(&self, app: &AppHandle, m: &DiskMap) {
        if self.ticking.swap(true, Ordering::SeqCst) {
            return;
        }
        self.tick_inner(app, m);
        self.ticking.store(false, Ordering::SeqCst);
    }

    fn tick_inner(&self, app: &AppHandle, m: &DiskMap) {
        let now = now_ms();
        let (running, started_at, paused) = {
            let s = lock(&self.sched);
            (s.running, s.started_at, s.paused_by_us)
        };
        if running {
            let st = m.host.call("disk.status", json!({})).ok().map(|v| to_status(&v));
            if st.as_ref().is_some_and(|st| st["scanning"] != json!(true)) && now.saturating_sub(started_at) > 10_000 {
                {
                    let mut s = lock(&self.sched);
                    s.running = false;
                    s.paused_by_us = false;
                }
                self.emit(app);
                return;
            }
            let idle = idle_seconds();
            let busy = idle < IDLE_PAUSE_S || on_battery();
            if !paused && busy && m.control("pause") {
                lock(&self.sched).paused_by_us = true;
                self.emit(app);
            } else if paused && !busy && idle >= IDLE_RESUME_S && m.control("resume") {
                lock(&self.sched).paused_by_us = false;
                self.emit(app);
            }
            return;
        }
        if !due(&lock(&self.sched).data, now) {
            return;
        }
        if idle_seconds() < IDLE_START_S || on_battery() {
            return;
        }
        let Ok(st) = m.status() else { return };
        let roots = m.scheduled_roots(&status_roots(&st));
        if st["indexed"] != json!(true) || st["scanning"] == json!(true) || roots.is_empty() {
            return;
        }
        {
            let mut s = lock(&self.sched);
            s.data.attempt_at = Some(now);
            self.save(&s.data);
            s.running = true;
            s.started_at = now;
            s.paused_by_us = false;
        }
        let started = m.host.call("disk.scan", json!({ "roots": roots, "level": "light", "excludes": [], "save": true })).is_ok();
        if started {
            m.host.mark_scanning(true);
        } else {
            lock(&self.sched).running = false;
        }
        self.emit(app);
    }

    pub fn stop(&self) {
        self.stopped.store(true, Ordering::SeqCst);
    }
}

pub fn start(app: &AppHandle, map: Weak<DiskMap>) {
    let app = app.clone();
    let every = Duration::from_millis(tick_ms());
    let _ = std::thread::Builder::new().name("disk-schedule".into()).spawn(move || loop {
        std::thread::sleep(every);
        let Some(m) = map.upgrade() else { return };
        if m.snap.stopped.load(Ordering::SeqCst) {
            return;
        }
        m.snap.tick(&app, &m);
    });
}

#[path = "disk_snap_report.rs"]
mod report;

#[cfg(test)]
#[path = "disk_snap_tests.rs"]
mod tests;
