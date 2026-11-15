use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use crate::disk_apps_left::{self as left, Hit, LeftApp, Query, Root};
use crate::disk_apps_reg::{self as reg, App};
use crate::disk_guard::{self as guard, GuardContext, Plat, RealIo};
use crate::disk_text::{delete_box, Ask};
use crate::state::lock;

const CHANNELS: [&str; 5] = ["ccm:diskapps-list", "ccm:diskapps-uninstall", "ccm:diskapps-leftovers", "ccm:diskapps-delete", "ccm:diskapps-settings"];
const REMOVED_MAX: usize = 20;
const MAX_LEFTOVER_DELETE: usize = 200;
const MAX_INDEX_PATHS: usize = 2000;

pub fn handles(channel: &str) -> bool {
    CHANNELS.contains(&channel)
}

pub struct Text {
    pub uninstall_title: String,
    pub uninstall_detail: String,
    pub uninstall_confirm: &'static str,
    pub cancel: &'static str,
}

pub fn uninstall_text(locale: &str, name: &str, cmd: &str) -> Text {
    if locale == "en" {
        Text { uninstall_title: format!("Uninstall {name}?"), uninstall_detail: format!("Orrery will start the app's own uninstaller with this command:\n\n{cmd}\n\nThe uninstaller may ask for administrator rights. Nothing else is run."), uninstall_confirm: "Start uninstaller", cancel: "Cancel" }
    } else {
        Text { uninstall_title: format!("Gỡ {name}?"), uninstall_detail: format!("Orrery sẽ mở trình gỡ của chính app này bằng lệnh:\n\n{cmd}\n\nTrình gỡ có thể xin quyền quản trị. Không chạy lệnh nào khác."), uninstall_confirm: "Mở trình gỡ", cancel: "Huỷ" }
    }
}

pub fn trash_text(locale: &str, name: &str, path: &str) -> Text {
    if locale == "en" {
        Text { uninstall_title: format!("Move {name} to the Trash?"), uninstall_detail: format!("{path}\n\nApp data in ~/Library stays until you clean leftovers."), uninstall_confirm: "Move to Trash", cancel: "Cancel" }
    } else {
        Text { uninstall_title: format!("Chuyển {name} vào Thùng rác?"), uninstall_detail: format!("{path}\n\nDữ liệu app trong ~/Library vẫn còn cho tới khi bạn dọn sót."), uninstall_confirm: "Chuyển vào Thùng rác", cancel: "Huỷ" }
    }
}

#[derive(Default)]
struct Store {
    apps: Vec<App>,
    seen: Vec<App>,
    leftovers: HashMap<String, Vec<Hit>>,
}

pub struct DiskApps {
    app: AppHandle,
    plat: Plat,
    env: HashMap<String, String>,
    reg_fixture: Option<PathBuf>,
    roots: Vec<Root>,
    mac_roots: Vec<String>,
    self_bundle: Option<String>,
    auto_confirm: bool,
    store: Mutex<Store>,
    listing: Mutex<()>,
    busy: AtomicBool,
}

fn e2e_on() -> bool {
    cfg!(debug_assertions) || crate::config::e2e_isolated()
}

fn e2e_var(name: &str) -> Option<String> {
    if !e2e_on() {
        return None;
    }
    std::env::var(format!("CCM_E2E_{name}")).ok().map(|v| v.trim().to_owned()).filter(|v| !v.is_empty())
}

fn json_list(list: &[App]) -> Value {
    json!(list.iter().map(App::to_json).collect::<Vec<_>>())
}

fn delete_fail(permanent: bool, reason: &str) -> Value {
    json!({ "done": [], "failed": [{ "path": "", "reason": reason }], "freed": 0, "permanent": permanent })
}

impl DiskApps {
    fn supported(&self) -> bool {
        matches!(self.plat, Plat::Win | Plat::Mac)
    }

    fn index_sizes(&self, paths: &[String]) -> Option<HashMap<String, Option<f64>>> {
        let state = self.app.try_state::<crate::disk_map::DiskState>()?;
        let m = state.0.clone();
        let data_dir = self.app.state::<crate::state::Shell>().data_dir.clone();
        if !m.host.running() && !data_dir.join("disk").join("index.bin").exists() {
            return None;
        }
        m.status().ok()?;
        let take: Vec<&String> = paths.iter().take(MAX_INDEX_PATHS).collect();
        let v = m.host.call("disk.sizes", json!({ "paths": take })).ok()?;
        let sizes = crate::disk_map::to_sizes(&v)?;
        Some(sizes["sizes"].as_object()?.iter().map(|(k, n)| (k.clone(), n.as_f64())).collect())
    }

    fn read_raw(&self) -> Option<Vec<App>> {
        match self.plat {
            Plat::Win => {
                let texts = match &self.reg_fixture {
                    Some(dir) => reg::read_fixture_text(dir),
                    None => reg::read_registry_text(&self.env),
                };
                reg::read_windows_apps(texts, &self.env)
            }
            Plat::Mac => Some(left::read_mac_apps(&self.mac_roots, self.self_bundle.as_deref(), &|p| self.index_sizes(p))),
            Plat::Linux => None,
        }
    }

    fn with_index_sizes(&self, apps: Vec<App>) -> Vec<App> {
        if self.plat != Plat::Win {
            return apps;
        }
        let mut locs: Vec<String> = Vec::new();
        for a in &apps {
            if let Some(l) = &a.location {
                if !locs.contains(l) {
                    locs.push(l.clone());
                }
            }
        }
        if locs.is_empty() {
            return apps;
        }
        let Some(sizes) = self.index_sizes(&locs) else { return apps };
        apps.into_iter()
            .map(|mut a| {
                if let Some(s) = a.location.as_ref().and_then(|l| sizes.get(l).copied().flatten()).filter(|s| *s > 0.0) {
                    a.size = Some(s);
                    a.size_from = Some("index");
                }
                a
            })
            .collect()
    }

    fn removed(st: &mut Store) -> Vec<App> {
        let out: Vec<App> = st.seen.iter().filter(|a| !st.apps.iter().any(|x| x.id == a.id)).cloned().collect();
        let excess = out.len().saturating_sub(REMOVED_MAX);
        for a in &out[..excess] {
            st.seen.retain(|s| s.id != a.id);
        }
        out[excess..].iter().rev().cloned().collect()
    }

    pub fn list(&self) -> Value {
        let _one = lock(&self.listing);
        let t0 = Instant::now();
        let raw = if self.supported() { self.read_raw() } else { None };
        let Some(raw) = raw else {
            let mut st = lock(&self.store);
            let removed = Self::removed(&mut st);
            return json!({ "apps": json_list(&st.apps), "ms": t0.elapsed().as_millis() as u64, "ok": false, "storeApps": "none", "removed": json_list(&removed) });
        };
        let apps = self.with_index_sizes(raw);
        let mut st = lock(&self.store);
        st.apps = apps.clone();
        for a in &apps {
            st.seen.retain(|s| s.id != a.id);
            st.seen.push(a.clone());
        }
        let removed = Self::removed(&mut st);
        json!({ "apps": json_list(&apps), "ms": t0.elapsed().as_millis() as u64, "ok": true, "storeApps": if self.plat == Plat::Win { "settings" } else { "none" }, "removed": json_list(&removed) })
    }

    fn confirm(&self, t: &Text) -> bool {
        self.auto_confirm || crate::hosts::confirm(&self.app, &t.uninstall_title, &t.uninstall_detail, t.uninstall_confirm, t.cancel)
    }

    pub fn uninstall(&self, id: &Value) -> Value {
        let Some(id) = id.as_str() else { return json!({ "ok": false, "error": "unknown" }) };
        let Some(app) = lock(&self.store).apps.iter().find(|a| a.id == id).cloned() else { return json!({ "ok": false, "error": "unknown" }) };
        if app.blocked.is_some() {
            return json!({ "ok": false, "error": "blocked" });
        }
        if self.busy.swap(true, Ordering::SeqCst) {
            return json!({ "ok": false, "error": "busy" });
        }
        let r = match self.plat {
            Plat::Mac => self.trash_app(&app),
            Plat::Win => self.run_uninstaller(&app),
            Plat::Linux => json!({ "ok": false, "error": "unsupported" }),
        };
        self.busy.store(false, Ordering::SeqCst);
        r
    }

    fn run_uninstaller(&self, app: &App) -> Value {
        let fresh = self.read_raw().and_then(|l| l.into_iter().find(|a| a.id == app.id));
        let Some(fresh) = fresh.filter(|f| f.uninstall == app.uninstall) else { return json!({ "ok": false, "error": "changed" }) };
        let exists = |p: &str| std::fs::metadata(p).is_ok_and(|m| m.is_file());
        let plan = match reg::parse_uninstall(fresh.uninstall.as_deref(), &self.env, &exists) {
            Ok(p) => p,
            Err(e) => return json!({ "ok": false, "error": if e == "missing" { "missing" } else { "unparsable" } }),
        };
        let text = uninstall_text(&crate::hosts::locale(&self.app), &app.name, &plan.display);
        if !self.confirm(&text) {
            return json!({ "ok": false, "error": "cancelled" });
        }
        match reg::parse_uninstall(fresh.uninstall.as_deref(), &self.env, &exists) {
            Ok(again) if again.file == plan.file && again.args == plan.args => {}
            _ => return json!({ "ok": false, "error": "changed" }),
        }
        if reg::launch(&plan, &self.env) {
            json!({ "ok": true, "mode": "started" })
        } else {
            json!({ "ok": false, "error": "failed" })
        }
    }

    fn check_bundle(&self, p: &str) -> bool {
        let parent = guard::parent(p, Plat::Mac);
        if !p.ends_with(".app") || !self.mac_roots.iter().any(|r| guard::normalize(r, Plat::Mac) == parent) {
            return false;
        }
        std::fs::symlink_metadata(p).is_ok_and(|m| m.is_dir() && !m.file_type().is_symlink())
    }

    fn trash_app(&self, app: &App) -> Value {
        if !self.check_bundle(&app.id) {
            return json!({ "ok": false, "error": "blocked" });
        }
        let text = trash_text(&crate::hosts::locale(&self.app), &app.name, &app.id);
        if !self.confirm(&text) {
            return json!({ "ok": false, "error": "cancelled" });
        }
        if !self.check_bundle(&app.id) {
            return json!({ "ok": false, "error": "changed" });
        }
        match crate::disk_map::trash_path(&app.id) {
            Ok(()) => json!({ "ok": true, "mode": "trashed" }),
            Err(_) => json!({ "ok": false, "error": "failed" }),
        }
    }

    pub fn scan_leftovers(&self, id: &Value) -> Value {
        let Some(id) = id.as_str() else { return Value::Null };
        let (app, installed, others) = {
            let st = lock(&self.store);
            let installed = st.apps.iter().any(|a| a.id == id);
            let Some(app) = st.apps.iter().find(|a| a.id == id).or_else(|| st.seen.iter().find(|a| a.id == id)).cloned() else { return Value::Null };
            let others: Vec<LeftApp> = st.apps.iter().filter(|a| a.id != id).map(|a| LeftApp { name: a.name.clone(), publisher: a.publisher.clone(), bundle_id: a.bundle_id.clone(), location: a.location.clone() }).collect();
            (app, installed, others)
        };
        if app.blocked.is_some() {
            return Value::Null;
        }
        let me = LeftApp { name: app.name.clone(), publisher: app.publisher.clone(), bundle_id: app.bundle_id.clone(), location: app.location.clone() };
        let q = Query { app: &me, installed, others: &others };
        let mut hits = left::find_leftovers(&q, &self.roots, self.plat, &left::read_entries, &|p| std::fs::symlink_metadata(p).is_ok());
        for h in &mut hits {
            h.size = if h.file { std::fs::symlink_metadata(&h.path).ok().map(|m| m.len() as f64) } else { left::dir_size(Path::new(&h.path), Instant::now() + Duration::from_secs(30)).map(|n| n as f64) };
        }
        hits.sort_by(|a, b| match (a.exact, b.exact) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => b.size.unwrap_or(0.0).total_cmp(&a.size.unwrap_or(0.0)),
        });
        let items: Vec<Value> = hits.iter().map(Hit::to_json).collect();
        lock(&self.store).leftovers.insert(id.to_owned(), hits);
        json!({ "id": id, "installed": installed, "items": items })
    }

    fn in_tree(&self, p: &str, tree: &[String]) -> bool {
        let n = guard::normalize(p, self.plat);
        tree.iter().filter(|d| !d.is_empty()).any(|d| guard::is_under(&n, d, self.plat) || guard::is_under(d, &n, self.plat))
    }

    fn resolve(&self, p: &str, ctx: &GuardContext, tree: &[String]) -> Result<guard::Canonical, String> {
        if self.in_tree(p, tree) {
            return Err("protected".into());
        }
        let c = guard::resolve_deletable(p, ctx, self.plat, &RealIo).map_err(str::to_owned)?;
        if self.in_tree(&c.path, tree) {
            return Err("protected".into());
        }
        Ok(c)
    }

    fn confirm_delete(&self, ask: &Ask, second: bool) -> bool {
        if self.auto_confirm {
            return true;
        }
        let b = delete_box(ask, second, &crate::hosts::locale(&self.app), cfg!(target_os = "macos"));
        crate::hosts::confirm(&self.app, &b.message, &b.detail, &b.ok, &b.cancel)
    }

    pub fn delete_leftovers(&self, req: &Value) -> Value {
        let permanent = req.get("permanent") == Some(&Value::Bool(true));
        let id = req.get("id").and_then(Value::as_str);
        let paths = req.get("paths").and_then(Value::as_array);
        let (Some(id), Some(paths)) = (id, paths) else { return delete_fail(permanent, "invalid") };
        if paths.is_empty() || paths.len() > MAX_LEFTOVER_DELETE {
            return delete_fail(permanent, "invalid");
        }
        let Some(list) = lock(&self.store).leftovers.get(id).cloned() else { return delete_fail(permanent, "expired") };
        let mut picked: Vec<Hit> = Vec::new();
        for p in paths {
            let Some(hit) = p.as_str().and_then(|p| list.iter().find(|l| l.path.to_lowercase() == p.to_lowercase())) else { return delete_fail(permanent, "unknown") };
            picked.push(hit.clone());
        }
        if self.busy.swap(true, Ordering::SeqCst) {
            return delete_fail(permanent, "busy");
        }
        let result = self.run_delete(&picked, permanent);
        self.busy.store(false, Ordering::SeqCst);
        let gone: Vec<String> = result["done"].as_array().map(|a| a.iter().filter_map(|d| d["path"].as_str()).map(str::to_lowercase).collect()).unwrap_or_default();
        if let Some(l) = lock(&self.store).leftovers.get_mut(id) {
            l.retain(|h| !gone.contains(&h.path.to_lowercase()));
        }
        result
    }

    fn run_delete(&self, picked: &[Hit], permanent: bool) -> Value {
        let plat = self.plat;
        let paths: Vec<String> = picked.iter().map(|h| h.path.clone()).collect();
        let ctx = GuardContext { home: crate::disk_map::home_of(&self.app), keep: crate::disk_map::keep_of(&self.app), roots: paths.clone(), volumes: Vec::new() };
        let tree = crate::disk_organize::app_dirs(&self.app);
        let mut failed: Vec<Value> = Vec::new();
        let mut done: Vec<Value> = Vec::new();
        let mut freed: f64 = 0.0;
        let mut allowed: Vec<String> = Vec::new();
        let mut canon: Vec<(String, guard::Canonical)> = Vec::new();
        let key = |p: &str| guard::normalize(p, plat).to_lowercase();
        for p in &paths {
            match self.resolve(p, &ctx, &tree) {
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
        let size_of = |p: &str| picked.iter().find(|h| h.path.to_lowercase() == p.to_lowercase()).and_then(|h| h.size);
        let sizes: Vec<Option<f64>> = targets.iter().map(|p| size_of(p)).collect();
        let first: Vec<String> = targets.iter().take(3).cloned().collect();
        let ask = Ask { count: targets.len(), bytes: sizes.iter().map(|s| s.unwrap_or(0.0) as u64).sum(), first: &first, permanent };
        if !self.confirm_delete(&ask, false) || (permanent && !self.confirm_delete(&ask, true)) {
            return json!({ "done": [], "failed": failed, "freed": 0, "permanent": permanent, "cancelled": true });
        }
        let mut partial: Vec<String> = Vec::new();
        for (i, p) in targets.iter().enumerate() {
            let before = canon.iter().find(|(k, _)| *k == key(p)).map(|(_, c)| c.clone());
            let target = match (before, self.resolve(p, &ctx, &tree)) {
                (Some(b), Ok(a)) if guard::same_canonical(&b, &a, plat) => a.path,
                (_, Err(reason)) => {
                    failed.push(json!({ "path": p, "reason": reason }));
                    continue;
                }
                _ => {
                    failed.push(json!({ "path": p, "reason": "changed" }));
                    continue;
                }
            };
            let size = sizes[i].unwrap_or(0.0);
            let r = if permanent { crate::disk_map::remove_path(&target) } else { crate::disk_map::trash_path(&target) };
            match r {
                Ok(()) => {
                    done.push(json!({ "path": p, "size": size }));
                    freed += size;
                }
                Err(reason) => {
                    if permanent && reason != "ENOENT" && reason != "EACCES" && reason != "EPERM" {
                        if let Some(rest) = crate::disk_map::measure_rest(&target) {
                            if size > rest as f64 {
                                freed += size - rest as f64;
                                partial.push(p.clone());
                            }
                        }
                    }
                    failed.push(json!({ "path": p, "reason": reason }));
                }
            }
        }
        if !done.is_empty() || !failed.is_empty() {
            let mut list: Vec<Value> = done.iter().map(|d| d["path"].clone()).collect();
            list.extend(partial.iter().map(|p| json!(p)));
            let entry = json!({ "at": crate::disk_map::now_ms(), "action": if permanent { "delete" } else { "trash" }, "count": done.len(), "freed": freed, "paths": list, "failed": failed.len() });
            match self.app.try_state::<crate::disk_map::DiskState>() {
                Some(s) => s.0.history.append(&entry),
                None => crate::disk_map::History::new(&self.app.state::<crate::state::Shell>().data_dir.join("disk")).append(&entry),
            }
        }
        json!({ "done": done, "failed": failed, "freed": freed, "permanent": permanent })
    }

    fn open_settings(&self) -> bool {
        if self.plat != Plat::Win {
            return false;
        }
        use tauri_plugin_opener::OpenerExt;
        self.app.opener().open_url("ms-settings:appsfeatures", None::<&str>).is_ok()
    }
}

pub struct DiskAppsState(pub Arc<DiskApps>);

fn self_bundle() -> Option<String> {
    let exe = std::env::current_exe().ok()?.to_string_lossy().into_owned();
    let i = exe.find(".app/")?;
    (i > 0).then(|| exe[..i + 4].to_owned())
}

fn service(app: &AppHandle) -> Arc<DiskApps> {
    if let Some(s) = app.try_state::<DiskAppsState>() {
        return s.0.clone();
    }
    let plat = Plat::host();
    let env = reg::env_map();
    let home = crate::disk_map::home_of(app);
    let roots = match (e2e_var("DISKAPPS_ROOTS"), plat) {
        (Some(fake), Plat::Mac) => left::mac_roots(&fake),
        (Some(fake), _) => left::windows_roots(Some(guard::join(&fake, "Roaming", plat)), Some(guard::join(&fake, "Local", plat)), Some(guard::join(&fake, "ProgramData", plat))),
        (None, Plat::Mac) => left::mac_roots(&home),
        (None, Plat::Win) => left::windows_roots(env.get("appdata").cloned(), env.get("localappdata").cloned(), env.get("programdata").cloned()),
        (None, Plat::Linux) => Vec::new(),
    };
    let svc = Arc::new(DiskApps {
        app: app.clone(),
        plat,
        reg_fixture: if plat == Plat::Win { e2e_var("DISKAPPS_REG").map(PathBuf::from) } else { None },
        roots,
        mac_roots: vec!["/Applications".into(), format!("{}/Applications", home.trim_end_matches('/'))],
        self_bundle: if plat == Plat::Mac { self_bundle() } else { None },
        auto_confirm: e2e_var("DISKAPPS_CONFIRM").as_deref() == Some("1"),
        env,
        store: Mutex::new(Store::default()),
        listing: Mutex::new(()),
        busy: AtomicBool::new(false),
    });
    app.manage(DiskAppsState(svc.clone()));
    app.try_state::<DiskAppsState>().map_or(svc, |s| s.0.clone())
}

pub fn dispatch(app: &AppHandle, channel: &str, args: &[Value]) -> Result<Value, String> {
    if !handles(channel) {
        return Err(format!("not ported: {channel}"));
    }
    let s = service(app);
    let a = args.first().cloned().unwrap_or(Value::Null);
    let id = a.get("id").cloned().unwrap_or(Value::Null);
    Ok(match channel {
        "ccm:diskapps-list" => s.list(),
        "ccm:diskapps-uninstall" => s.uninstall(&id),
        "ccm:diskapps-leftovers" => s.scan_leftovers(&id),
        "ccm:diskapps-delete" => s.delete_leftovers(&a),
        _ => json!(s.open_settings()),
    })
}

#[cfg(test)]
#[path = "disk_apps_tests.rs"]
mod tests;
