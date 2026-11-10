use std::path::PathBuf;
use std::process::ExitCode;

use core_store::extra::{PlanLimits, SessionRecord};
use core_store::model::{TerminalSession, TerminalStatus};
use core_store::persist::aliases::{sanitize_alias_str, AliasStore, MAX_ALIAS_LENGTH, MAX_NOTE_LENGTH};
use core_store::persist::datadir::{default_data_dir, Platform};
use core_store::persist::daystart::first_start_today;
use core_store::persist::favorites::FavoriteStore;
use core_store::persist::forecast::LimitHistory;
use core_store::persist::history::SessionHistory;
use core_store::persist::localtime::Zone;
use core_store::persist::notify_file::{sanitize_idle, sanitize_kinds, sanitize_quiet, sanitize_stuck, NotifyFile, ToastLocale};
use core_store::persist::response::ResponseTracker;
use core_store::persist::snooze::SnoozeStore;
use core_store::persist::timeline::StatusTimeline;
use serde_json::{json, Value};

fn s(v: &Value) -> &str {
    v.as_str().unwrap_or("")
}

fn clock(now: i64) -> Box<dyn Fn() -> i64 + Send> {
    Box::new(move || now)
}

fn terminal(v: &Value) -> Result<TerminalSession, String> {
    serde_json::from_value(v.clone()).map_err(|e| format!("terminal: {e}"))
}

fn aliases(file: PathBuf, max: usize, ops: &[Value]) -> Value {
    let mut store = AliasStore::new(Some(file), max);
    store.load();
    for op in ops {
        match s(&op[0]) {
            "set" => {
                store.set(s(&op[1]), op[2].as_str().and_then(|v| sanitize_alias_str(v, max)).as_deref());
            }
            "delete" => store.delete(s(&op[1])),
            _ => {}
        }
    }
    Value::Object(store.entries().into_iter().map(|(k, v)| (k, json!(v))).collect())
}

fn favorites(file: PathBuf, now: i64, ops: &[Value]) -> Value {
    let mut store = FavoriteStore::with_clock(Some(file), Platform::HOST, clock(now));
    store.load();
    for op in ops {
        match s(&op[0]) {
            "toggle" => {
                store.toggle(s(&op[1]), s(&op[2]));
            }
            "add" => {
                store.add(s(&op[1]), &op[2]);
            }
            "rename" => {
                store.rename(s(&op[1]), &op[2]);
            }
            "remove" => {
                store.remove(s(&op[1]));
            }
            "reorder" => {
                let dirs: Vec<String> = op[1].as_array().map(|a| a.iter().map(|d| s(d).to_string()).collect()).unwrap_or_default();
                store.reorder(&dirs);
            }
            _ => {}
        }
    }
    serde_json::to_value(store.list()).unwrap_or(Value::Null)
}

fn history(file: PathBuf, now: i64, ops: &[Value]) -> Result<Value, String> {
    let mut store = SessionHistory::with_clock(Some(file), Zone::Local, clock(now));
    store.load();
    for op in ops {
        if s(&op[0]) == "add" {
            let record: SessionRecord = serde_json::from_value(op[1].clone()).map_err(|e| format!("record: {e}"))?;
            store.add(record);
        }
    }
    if !ops.is_empty() {
        store.save();
    }
    serde_json::to_value(store.list()).map_err(|e| e.to_string())
}

fn snooze(file: PathBuf, now: i64, ops: &[Value]) -> Result<Value, String> {
    let mut store = SnoozeStore::with_clock(Some(file), clock(now));
    store.load();
    for op in ops {
        match s(&op[0]) {
            "set" => {
                store.set(&terminal(&op[1])?, op[2].as_f64());
            }
            "clear" => {
                store.clear(s(&op[1]));
            }
            _ => {}
        }
    }
    store.flush();
    serde_json::to_value(store.all()).map_err(|e| e.to_string())
}

fn response(file: PathBuf, now: i64, ops: &[Value]) -> Value {
    let mut store = ResponseTracker::with_clock(Some(file), Zone::Local, clock(now));
    store.load();
    for op in ops {
        if s(&op[0]) == "record" {
            store.record(op[1].as_f64().unwrap_or(f64::NAN));
        }
    }
    if !ops.is_empty() {
        store.save();
    }
    serde_json::to_value(store.stats()).unwrap_or(Value::Null)
}

fn timeline(file: PathBuf, now: i64, ops: &[Value]) -> Result<Value, String> {
    let mut store = StatusTimeline::with_clock(Some(file), Zone::Local, clock(now));
    store.load();
    for op in ops {
        if s(&op[0]) == "observe" {
            let status: TerminalStatus = serde_json::from_value(op[3].clone()).map_err(|e| format!("status: {e}"))?;
            store.observe(s(&op[1]), s(&op[2]), status, op[4].as_str());
        }
    }
    if !ops.is_empty() {
        store.save();
    }
    serde_json::to_value(store.data()).map_err(|e| e.to_string())
}

fn limits(file: PathBuf, now: i64, ops: &[Value]) -> Result<Value, String> {
    let mut store = LimitHistory::with_clock(Some(file), Zone::Local, clock(now));
    store.load();
    let report = |v: &Value| serde_json::from_value::<PlanLimits>(v.clone()).map_err(|e| format!("limits: {e}"));
    let mut probe = Value::Null;
    let mut wrote = false;
    for op in ops {
        match s(&op[0]) {
            "apply" => {
                store.apply(Some(&report(&op[1])?));
                wrote = true;
            }
            "probe" => {
                if wrote {
                    store.save();
                    wrote = false;
                }
                probe = serde_json::to_value(store.apply(Some(&report(&op[1])?))).map_err(|e| e.to_string())?;
            }
            _ => {}
        }
    }
    if wrote {
        store.save();
    }
    Ok(probe)
}

fn notify(file: PathBuf, now: i64, ops: &[Value]) -> Value {
    let mut store = NotifyFile::with_clock(Some(file), || "0123456789abcdef0123456789abcdef".to_string(), clock(now));
    store.load();
    for op in ops {
        if s(&op[0]) != "patch" {
            continue;
        }
        let p = &op[1];
        let prefs = store.prefs_mut();
        if let Some(v) = p.get("toast").and_then(Value::as_bool) {
            prefs.enabled = v;
        }
        match p.get("locale").and_then(Value::as_str) {
            Some("en") => prefs.locale = ToastLocale::En,
            Some("vi") => prefs.locale = ToastLocale::Vi,
            _ => {}
        }
        if let Some(kinds) = p.get("kinds").and_then(sanitize_kinds) {
            prefs.kinds.extend(kinds);
        }
        if let Some(q) = p.get("quiet").and_then(sanitize_quiet) {
            prefs.quiet = q;
        }
        if let Some(n) = p.get("stuckMinutes").and_then(sanitize_stuck) {
            prefs.stuck_minutes = n;
        }
        if let Some(n) = p.get("idleMinutes").and_then(sanitize_idle) {
            prefs.idle_minutes = n;
        }
        store.save();
    }
    store.flush();
    store.prefs().to_value()
}

fn run(args: &[String]) -> Result<Value, String> {
    let kind = args.first().ok_or("kind")?.as_str();
    if kind == "datadir" {
        let platform = if args.get(1).map(String::as_str) == Some("win32") { Platform::Win32 } else { Platform::Darwin };
        let local = args.get(2).map(String::as_str).filter(|v| *v != "-");
        return Ok(json!(default_data_dir(local, platform, args.get(3).map_or("", String::as_str))));
    }
    let file = PathBuf::from(args.get(1).ok_or("file")?);
    let now: i64 = args.get(2).ok_or("now")?.parse().map_err(|_| "now")?;
    let ops: Vec<Value> = match args.get(3) {
        Some(text) => match serde_json::from_str::<Value>(text).map_err(|e| format!("ops: {e}"))? {
            Value::Array(a) => a,
            _ => return Err("ops must be an array".into()),
        },
        None => Vec::new(),
    };
    match kind {
        "aliases" => Ok(aliases(file, MAX_ALIAS_LENGTH, &ops)),
        "notes" => Ok(aliases(file, MAX_NOTE_LENGTH, &ops)),
        "favorites" => Ok(favorites(file, now, &ops)),
        "history" => history(file, now, &ops),
        "snooze" => snooze(file, now, &ops),
        "response" => Ok(response(file, now, &ops)),
        "timeline" => timeline(file, now, &ops),
        "limits" => limits(file, now, &ops),
        "notify" => Ok(notify(file, now, &ops)),
        "daystart" => Ok(json!(first_start_today(&file, now, Zone::Local))),
        other => Err(format!("unknown kind {other}")),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match run(&args) {
        Ok(view) => {
            println!("{view}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("persist-cli: {err}");
            ExitCode::FAILURE
        }
    }
}
