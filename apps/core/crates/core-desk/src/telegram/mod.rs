pub mod api;
pub mod buttons;
pub mod callback;
pub mod clock;
pub mod commands;
pub mod base;
pub mod base_io;
pub mod base_more;
pub mod discover;
pub mod episodes;
pub mod files;
pub mod html;
pub mod inbound;
pub mod jsx;
pub mod message;
pub mod outbound;
pub mod parse;
pub mod poll;
pub mod report;
pub mod routes;
pub mod runs;
pub mod sanitize;
pub mod settings;
pub mod shape;
pub mod shape_out;
pub mod text;
pub mod text_act;
pub mod text_more;
pub mod ticks;

#[cfg(test)]
mod tests;

use self::base::{Deps, Tg, RECEIVE_COMMAND, SEND_COMMAND};
use crate::claude::Sys;
use crate::{arg, Desk, DeskEnv, DeskError};
use serde_json::{json, Value};
use std::sync::{Arc, OnceLock};

pub const MODULE: &str = "telegram";
pub const FILE: &str = "telegram.json";

pub fn deps_from(env: &DeskEnv, desk: &Desk) -> Deps {
    let sys = Sys::from_env();
    let base = crate::phone::send::e2e_override(std::env::var("CCM_TELEGRAM_URL_OVERRIDE").ok().as_deref()).unwrap_or_else(|| api::API_BASE.to_string());
    let host = desk.host_link();
    let emitter = desk.emitter();
    Deps {
        file: Some(env.data_dir.join(FILE)),
        base,
        masker: html::Masker { home: sys.home.clone(), platform: sys.platform() },
        win: sys.win,
        claude_root: sys.claude_root(),
        http: Arc::new(api::Ureq),
        host: Arc::new(move |m, p| host.call(m, p)),
        emit: Arc::new(move |e, p| emitter.emit(e, p)),
        clock: Arc::new(|| crate::util::now_ms() as i64),
        sleep: Arc::new(|ms| std::thread::sleep(std::time::Duration::from_millis(ms))),
        local: Arc::new(clock::local),
        album_ms: 1500,
    }
}

fn failed() -> Value {
    json!({"ok": false, "error": "failed"})
}

fn install(tg: &Arc<Tg>, kind: &str, name: &str) -> Value {
    let file = files::command_file(&tg.deps.claude_root, name);
    let ok = (tg.deps.host)("telegram.confirmCommand", json!({"kind": kind, "file": crate::util::path_str(&file)}));
    if !matches!(ok, Ok(Value::Bool(true))) {
        return json!({"ok": false, "error": "cancelled"});
    }
    if !files::install_command(&tg.deps.claude_root, name) {
        return failed();
    }
    let state = tg.state();
    (tg.deps.emit)(self::base::EVENT, json!({"state": state.clone()}));
    json!({"ok": true, "state": state})
}

pub fn handle(tg: &Arc<Tg>, channel: &str, args: &[Value]) -> Value {
    let a0 = arg(args, 0);
    match channel {
        "ccm:telegram-state" | "desk:tg-state" => tg.state(),
        "ccm:telegram-set" => sanitize::sanitize_patch(a0).map(|p| tg.set(&p)).unwrap_or_else(failed),
        "ccm:telegram-route-set" => sanitize::sanitize_route_input(a0).map(|p| tg.set_route(&p)).unwrap_or_else(failed),
        "ccm:telegram-route-remove" => match a0.as_str().filter(|s| jsx::len(s) < 4096) {
            Some(p) => tg.remove_route(p),
            None => failed(),
        },
        "ccm:telegram-session-route-set" => sanitize::sanitize_session_route_input(a0).map(|p| tg.set_session_route(&p)).unwrap_or_else(failed),
        "ccm:telegram-session-route-remove" => match sanitize::session_id(a0) {
            Some(s) => tg.remove_session_route(s),
            None => failed(),
        },
        "ccm:telegram-panes" => match sanitize::sanitize_pane_links(a0) {
            Some(l) => {
                tg.set_panes(&l);
                json!(true)
            }
            None => json!(false),
        },
        "ccm:telegram-test" => tg.test(a0.as_str().filter(|s| jsx::len(s) < 4096), sanitize::session_id(arg(args, 1))),
        "ccm:telegram-discover" => tg.discover(a0 == &Value::Bool(true)),
        "ccm:telegram-user" => sanitize::sanitize_user(a0).map(|u| tg.user(&u)).unwrap_or_else(failed),
        "ccm:telegram-create-topic" => match sanitize::sanitize_create_topic(a0) {
            Some((chat, project, name)) => tg.create_topic(&chat, &project, &name),
            None => json!({"ok": false, "error": "invalid-project"}),
        },
        "ccm:telegram-pair" => tg.pair(a0 != &Value::Bool(false)),
        "ccm:telegram-unpair" => tg.unpair(),
        "ccm:telegram-command" => install(tg, "send", SEND_COMMAND),
        "ccm:telegram-receive-command" => install(tg, "receive", RECEIVE_COMMAND),
        "desk:tg-set" => match a0.get("patch").and_then(sanitize::sanitize_patch) {
            Some(p) => tg.set(&p),
            None => failed(),
        },
        "desk:tg-route-for" => {
            let st = tg.lock();
            match Tg::route_for(&st, a0["cwd"].as_str().unwrap_or("")) {
                Some(r) => Value::Object(shape::route_value(&r)),
                None => Value::Null,
            }
        }
        "desk:tg-report" => {
            let buttons = match sanitize::parse_buttons(&a0["buttons"]) {
                Some(b) => b,
                None => return json!({"ok": false, "error": "failed"}),
            };
            let session = a0["from"].as_str().filter(|s| !s.is_empty() && jsx::len(s) < 200);
            tg.report(a0["cwd"].as_str().unwrap_or(""), a0["text"].as_str().unwrap_or(""), a0["file"].as_str().filter(|f| !f.is_empty()), session, buttons)
        }
        "desk:tg-handle" => match sanitize::parse_episode(&a0["episode"]) {
            Some(e) => json!(tg.handle(&e)),
            None => json!(false),
        },
        "desk:tg-alert" => json!(tg.alert(a0["kind"].as_str().unwrap_or(""), &a0["detail"])),
        "desk:tg-run-state" => {
            tg.run_state(&a0["session"]);
            Value::Null
        }
        "desk:tg-run-failed" => json!(tg.run_failed_report(&a0["session"])),
        "desk:tg-tick" => json!(tg.tick()),
        "desk:tg-stop" => {
            tg.stop();
            Value::Null
        }
        "desk:tg-upload-progress" => {
            if let (Some(id), Some(f)) = (a0["id"].as_u64(), a0["fraction"].as_f64()) {
                tg.upload_progress(id, f);
            }
            Value::Null
        }
        _ => Value::Null,
    }
}

pub const CHANNELS: [&str; 26] = [
    "ccm:telegram-state",
    "ccm:telegram-set",
    "ccm:telegram-route-set",
    "ccm:telegram-route-remove",
    "ccm:telegram-test",
    "ccm:telegram-command",
    "ccm:telegram-discover",
    "ccm:telegram-user",
    "ccm:telegram-create-topic",
    "ccm:telegram-pair",
    "ccm:telegram-unpair",
    "ccm:telegram-receive-command",
    "ccm:telegram-session-route-set",
    "ccm:telegram-session-route-remove",
    "ccm:telegram-panes",
    "desk:tg-state",
    "desk:tg-set",
    "desk:tg-route-for",
    "desk:tg-report",
    "desk:tg-handle",
    "desk:tg-alert",
    "desk:tg-run-state",
    "desk:tg-run-failed",
    "desk:tg-tick",
    "desk:tg-stop",
    "desk:tg-upload-progress",
];

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    let deps = deps_from(env, desk);
    let cell: Arc<OnceLock<Arc<Tg>>> = Arc::new(OnceLock::new());
    let closing = cell.clone();
    desk.on_close(Arc::new(move || {
        if let Some(tg) = closing.get() {
            tg.stop();
        }
    }));
    for channel in CHANNELS {
        let (cell, deps) = (cell.clone(), deps.clone());
        desk.on(MODULE, channel, move |_ctx, args| {
            let tg = cell.get_or_init(|| Tg::new(deps.clone()));
            Ok::<Value, DeskError>(handle(tg, channel, args))
        });
    }
}
