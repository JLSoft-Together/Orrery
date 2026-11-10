use super::deps::Deps;
use super::scope::{plan_free_port, pane_scope, servers_for, PaneScope};
use crate::capture::files::SHOT_MAX_BYTES;
use crate::{js, util};
use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

pub const DIAG_MAX: usize = 12;
const FALLBACK_KEEP: usize = 20;
const ERRORS: [&str; 29] = [
    "unavailable", "failed", "starting", "not-layout", "missing", "too-large", "jar-missing", "bundle-missing", "jdk-missing", "jdk-old", "crash-loop", "render-failed", "timeout", "no-report", "not-found", "not-mine", "outside",
    "emulator", "self", "busy", "gone", "cancelled", "confirm-open", "telegram-off", "telegram-config", "no-topic", "not-paired", "rate", "buttons-file",
];

static CONFIRMING: AtomicBool = AtomicBool::new(false);

struct ConfirmLock;

impl ConfirmLock {
    fn take() -> Option<ConfirmLock> {
        (!CONFIRMING.swap(true, Ordering::SeqCst)).then_some(ConfirmLock)
    }
}

impl Drop for ConfirmLock {
    fn drop(&mut self) {
        CONFIRMING.store(false, Ordering::SeqCst);
    }
}

#[derive(Debug, Clone)]
pub struct Opts {
    pub win: bool,
    pub home: String,
    pub self_pids: Vec<i64>,
    pub fallback_dir: PathBuf,
    pub tmp_root: PathBuf,
    pub deadline_ms: u64,
}

pub fn fail(code: &str) -> Value {
    json!({"ok": false, "error": code})
}

fn desk_error(code: &str) -> &str {
    if ERRORS.contains(&code) {
        code
    } else {
        "failed"
    }
}

fn real_or(p: &str) -> String {
    match std::fs::canonicalize(p) {
        Ok(r) => util::path_str(&crate::preview::paths::strip_verbatim(r)),
        Err(_) => p.to_string(),
    }
}

fn text<'a>(call: &'a Value, key: &str) -> &'a str {
    call.get(key).and_then(Value::as_str).unwrap_or("")
}

pub fn pane_name(call: &Value) -> String {
    let clean: String = text(call, "name").chars().map(|c| if (c as u32) < 0x20 || c as u32 == 0x7f { ' ' } else { c }).collect();
    js::slice(&clean, 80)
}

fn extname(base: &str) -> &str {
    match base.rfind('.') {
        Some(i) if i > 0 => &base[i..],
        _ => "",
    }
}

fn layout_name(path: &str, night: bool) -> String {
    let base = Path::new(path).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
    let ext = extname(&base);
    let stem = if !ext.is_empty() && base != ext { &base[..base.len() - ext.len()] } else { &base[..] };
    format!("layout-{stem}{}", if night { "-dark" } else { "" })
}

fn keep_newest(dir: &Path, keep: usize) {
    let Ok(read) = std::fs::read_dir(dir) else { return };
    let mut files: Vec<(PathBuf, f64)> = read
        .flatten()
        .filter(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            n.starts_with("layout-") && n.ends_with(".png")
        })
        .map(|e| {
            let at = e.metadata().and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs_f64() * 1000.0).unwrap_or(0.0);
            (e.path(), at)
        })
        .collect();
    files.sort_by(|a, b| b.1.total_cmp(&a.1));
    for (f, _) in files.into_iter().skip(keep) {
        let _ = std::fs::remove_file(f);
    }
}

fn fallback_name(name: &str) -> String {
    let mut out = String::new();
    let mut run = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
            out.push(c);
            run = false;
        } else if !run {
            out.push('-');
            run = true;
        }
    }
    js::slice(&out, 60)
}

fn preview_layout(d: &dyn Deps, o: &Opts, call: &Value) -> Value {
    let path = text(call, "path");
    if !util::is_inside_or_same(&real_or(text(call, "cwd")), &real_or(path)) {
        return fail("outside");
    }
    let tablet = text(call, "device") == "tablet";
    let device = if tablet { json!({"width": 1600, "height": 2560, "density": 320}) } else { json!({"width": 1080, "height": 2340, "density": 440}) };
    let night = call.get("night") == Some(&json!(true));
    let r = d.render(&real_or(path), &device, night, text(call, "from"));
    if r["ok"] != json!(true) {
        let mut m = Map::new();
        m.insert("ok".into(), json!(false));
        m.insert("error".into(), json!(desk_error(r["code"].as_str().unwrap_or(""))));
        if r["retryInSec"].as_f64().is_some_and(|n| n != 0.0) {
            m.insert("retryInSec".into(), r["retryInSec"].clone());
        }
        if r["code"] == "render-failed" && r["detail"].as_str().is_some_and(|s| !s.is_empty()) {
            m.insert("detail".into(), r["detail"].clone());
        }
        return Value::Object(m);
    }
    let Some(buf) = r["png"].as_str().and_then(|p| std::fs::read(p).ok()).filter(|b| b.len() <= SHOT_MAX_BYTES) else { return fail("failed") };
    let name = layout_name(path, call.get("night").is_some_and(|n| n == &json!(true)));
    let mut saved = d.write_shot(&buf, &name);
    if saved.is_none() {
        let _ = std::fs::create_dir_all(&o.fallback_dir);
        let file = o.fallback_dir.join(format!("{}-{}.png", fallback_name(&name), util::now_ms() as i64));
        if std::fs::write(&file, &buf).is_ok() {
            saved = Some(util::path_str(&file));
            keep_newest(&o.fallback_dir, FALLBACK_KEEP);
        }
    }
    let Some(saved) = saved else { return fail("failed") };
    let diagnostics: Vec<Value> = r["diagnostics"].as_array().map(|a| a.iter().take(DIAG_MAX).cloned().collect()).unwrap_or_default();
    json!({"ok": true, "tool": "preview_layout", "png": saved, "width": r["width"], "height": r["height"], "renderMs": r["renderMs"], "frameworkOnly": r["frameworkOnly"], "diagnostics": diagnostics})
}

fn scope_for(d: &dyn Deps, o: &Opts, call: &Value) -> PaneScope {
    let pid = call.get("pid").and_then(Value::as_i64);
    let rows = if pid.is_some_and(|p| p != 0) { d.proc_rows() } else { None };
    pane_scope(&real_or(text(call, "cwd")), pid, rows.as_deref(), &o.home, o.win)
}

fn free_port(d: &dyn Deps, o: &Opts, call: &Value) -> Value {
    if CONFIRMING.load(Ordering::SeqCst) {
        return fail("confirm-open");
    }
    let port = call.get("port").and_then(Value::as_i64).unwrap_or(0);
    let report = d.report();
    let targets = match plan_free_port(report.as_ref(), &scope_for(d, o, call), port, &o.self_pids) {
        Ok(t) => t,
        Err(e) => return e,
    };
    let res = {
        let Some(_lock) = ConfirmLock::take() else { return fail("confirm-open") };
        d.kill(&targets, &pane_name(call), port)
    };
    if res["ok"] != json!(true) {
        let e = res["error"].as_str().unwrap_or("failed");
        return fail(if e == "invalid" { "failed" } else { desk_error(e) });
    }
    let stopped: Vec<Value> = targets
        .iter()
        .map(|t| {
            let label = t.label.clone().unwrap_or_else(|| t.pid.to_string());
            let name = match label.rfind(" :") {
                Some(i) if !label[i + 2..].is_empty() && label[i + 2..].bytes().all(|b| b.is_ascii_digit()) => label[..i].to_string(),
                _ => label,
            };
            json!({"pid": t.pid, "name": name})
        })
        .collect();
    json!({"ok": true, "tool": "free_port", "port": port, "stopped": stopped})
}

fn flag(v: &Value) -> bool {
    v.as_bool().unwrap_or(false)
}

fn telegram(d: &dyn Deps, call: &Value) -> Value {
    if !d.tg_available() {
        return fail("unavailable");
    }
    let cwd = real_or(text(call, "cwd"));
    match text(call, "tool") {
        "telegram_send" => telegram_send(d, call, &cwd),
        "telegram_receive" => telegram_receive(d, call),
        _ => telegram_notify(d, call, &cwd),
    }
}

fn telegram_send(d: &dyn Deps, call: &Value, cwd: &str) -> Value {
    let file = text(call, "file");
    let buttons = call.get("buttons").filter(|b| !b.is_null());
    if buttons.is_some() && !file.is_empty() {
        return fail("buttons-file");
    }
    let mut body = Map::new();
    body.insert("cwd".into(), json!(cwd));
    body.insert("text".into(), call.get("text").cloned().unwrap_or(Value::Null));
    if !file.is_empty() {
        body.insert("file".into(), json!(real_or(file)));
    }
    let from = text(call, "from");
    if !from.is_empty() {
        body.insert("from".into(), json!(from));
    }
    if let Some(b) = buttons {
        body.insert("buttons".into(), b.clone());
    }
    let res = d.tg_report(Value::Object(body));
    if res["ok"] != json!(true) {
        let e = res["error"].as_str().unwrap_or("");
        return json!({"ok": false, "error": desk_error(e), "detail": res["error"]});
    }
    let mut m = Map::new();
    m.insert("ok".into(), json!(true));
    m.insert("tool".into(), json!("telegram_send"));
    m.insert("topic".into(), res["topic"].clone());
    if res["file"].as_str().is_some_and(|f| !f.is_empty()) {
        m.insert("file".into(), res["file"].clone());
    }
    if flag(&res["linkOnly"]) {
        m.insert("linkOnly".into(), json!(true));
    }
    if flag(&res["uploading"]) {
        m.insert("uploading".into(), json!(true));
    }
    if res["buttons"].as_f64().is_some_and(|n| n != 0.0) {
        m.insert("buttons".into(), res["buttons"].clone());
    }
    Value::Object(m)
}

fn telegram_receive(d: &dyn Deps, call: &Value) -> Value {
    let state = text(call, "state");
    let before = d.tg_state();
    let owner = &before["owner"];
    if state == "on" && !flag(&before["configured"]) {
        return fail("telegram-config");
    }
    if state == "on" && owner.is_null() {
        return fail("not-paired");
    }
    if state == "on" && !flag(&before["receiving"]) {
        let Some(_lock) = ConfirmLock::take() else { return fail("confirm-open") };
        let response = d.confirm_receive(&pane_name(call), owner["name"].as_str().unwrap_or(""));
        drop(_lock);
        if response != 0 {
            return fail("cancelled");
        }
        if d.tg_state()["owner"].is_null() {
            return fail("not-paired");
        }
    }
    if state != "status" && d.tg_set(json!({"receiving": state == "on"}))["ok"] != json!(true) {
        return fail("failed");
    }
    let s = d.tg_state();
    let mut m = Map::new();
    m.insert("ok".into(), json!(true));
    m.insert("tool".into(), json!("telegram_receive"));
    m.insert("receiving".into(), json!(flag(&s["receiving"])));
    m.insert("paired".into(), json!(!s["owner"].is_null()));
    if !s["owner"].is_null() {
        m.insert("owner".into(), s["owner"]["name"].clone());
    }
    Value::Object(m)
}

fn telegram_notify(d: &dyn Deps, call: &Value, cwd: &str) -> Value {
    let state = text(call, "state");
    if state != "status" {
        if state == "on" && !flag(&d.tg_state()["configured"]) {
            return fail("telegram-config");
        }
        if d.tg_set(json!({"enabled": state == "on"}))["ok"] != json!(true) {
            return fail("failed");
        }
    }
    let s = d.tg_state();
    let projects: Vec<Value> = s["routes"].as_array().map(|r| r.iter().map(|x| x["label"].clone()).collect()).unwrap_or_default();
    json!({"ok": true, "tool": "telegram_notify", "enabled": flag(&s["enabled"]), "configured": flag(&s["configured"]), "projects": projects, "routed": !d.tg_route_for(cwd).is_null()})
}

pub fn run(d: &dyn Deps, o: &Opts, call: &Value) -> Value {
    match text(call, "tool") {
        "preview_layout" => preview_layout(d, o, call),
        "free_port" => free_port(d, o, call),
        "telegram_notify" | "telegram_send" | "telegram_receive" => telegram(d, call),
        "list_servers" => {
            let Some(report) = d.report() else { return fail("no-report") };
            let (servers, outside) = servers_for(&report, &scope_for(d, o, call), util::now_ms());
            let mut m = Map::new();
            m.insert("ok".into(), json!(true));
            m.insert("tool".into(), json!("list_servers"));
            if let Some(at) = report.get("at") {
                m.insert("at".into(), at.clone());
            }
            m.insert("servers".into(), Value::Array(servers));
            m.insert("outside".into(), Value::Array(outside));
            Value::Object(m)
        }
        _ => fail("unavailable"),
    }
}
