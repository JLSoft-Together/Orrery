use super::manager::{new_manager, RunManager};
use super::proc::process_env;
use super::settings::SettingsStore;
use super::types::{RunDevice, RunEmulator, RunResult};
use crate::preview::gate;
use crate::preview::paths::is_absolute;
use crate::{arg, Ctx, Desk, DeskEnv};
use regex::Regex;
use serde_json::{json, Value};
use std::sync::{Arc, OnceLock};

pub const MODULE: &str = "run";

fn is_id(v: &Value) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    v.as_str().is_some_and(|s| R.get_or_init(|| Regex::new(r"^[A-Za-z0-9_.:@%-]{1,128}$").expect("regex")).is_match(s))
}

fn denied() -> Value {
    RunResult::err("denied").to_value()
}

fn root(ctx: &Ctx, dir: &Value) -> Option<String> {
    gate::allowed(ctx, dir, false).map(|a| a.real)
}

fn inside(ctx: &Ctx, dir: &Value) -> Option<String> {
    if !dir.as_str().is_some_and(is_absolute) {
        return None;
    }
    root(ctx, dir)
}

fn kind_ok(v: &Value) -> Option<&str> {
    v.as_str().filter(|k| *k == "flutter" || *k == "android")
}

pub fn home_dir() -> String {
    std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap_or_default()
}

pub fn make_manager(desk: &Desk, env: &DeskEnv, settings: Arc<SettingsStore>) -> RunManager {
    new_manager(env.data_dir.clone(), process_env(), cfg!(windows), home_dir(), settings, desk.emitter(), desk.host_link())
}

pub fn register(desk: &mut Desk, env: &DeskEnv) -> Arc<SettingsStore> {
    let settings = SettingsStore::load(&env.data_dir, cfg!(windows));
    let m = make_manager(desk, env, settings.clone());
    let closing = m.clone();
    desk.on_close(Arc::new(move || closing.kill_all()));
    on_config(desk, &m);
    on_env(desk, &m);
    on_process(desk, &m);
    settings
}

fn on_config(desk: &mut Desk, m: &RunManager) {
    let mm = m.clone();
    desk.on(MODULE, "ccm:run-hold", move |ctx, args| {
        let (Some(r), true) = (root(ctx, arg(args, 0)), is_id(arg(args, 1))) else { return Ok(json!(false)) };
        let owner = ctx.host("run.owner", json!({"key": arg(args, 1)})).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| arg(args, 1).as_str().unwrap_or("").to_string());
        mm.hold(&r, &owner, arg(args, 2) == &json!(true));
        Ok(json!(true))
    });
    let mm = m.clone();
    desk.on(MODULE, "desk:run-release-owner", move |_, args| {
        mm.release_owner(arg(args, 0).as_str().unwrap_or(""));
        Ok(json!(true))
    });
    let mm = m.clone();
    desk.on(MODULE, "ccm:run-config-auto-import", move |ctx, args| Ok(json!(root(ctx, arg(args, 0)).map(|r| mm.auto_import(&r)).unwrap_or_default())));
    let mm = m.clone();
    desk.on(MODULE, "ccm:run-templates", move |_, _| Ok(mm.get_templates()));
    let mm = m.clone();
    desk.on(MODULE, "ccm:run-templates-set", move |_, args| Ok(if arg(args, 0).is_object() { mm.set_templates(arg(args, 0)) } else { Value::Null }));
    let mm = m.clone();
    desk.on(MODULE, "ccm:run-project-sdk-set", move |ctx, args| {
        let Some(d) = inside(ctx, arg(args, 0)) else { return Ok(Value::Null) };
        let sdk = arg(args, 1);
        let next = match sdk {
            Value::Null => mm.settings.set_project_flutter_sdk(&d, None),
            Value::String(s) if is_absolute(s) => mm.settings.set_project_flutter_sdk(&d, Some(s)),
            _ => return Ok(Value::Null),
        };
        mm.settings_changed(&next);
        Ok(next.to_value())
    });
    let mm = m.clone();
    desk.on(MODULE, "ccm:run-configs", move |ctx, args| Ok(json!(root(ctx, arg(args, 0)).map(|r| mm.configs(&r)).unwrap_or_default())));
    let mm = m.clone();
    desk.on(MODULE, "ccm:run-config-save", move |ctx, args| Ok(root(ctx, arg(args, 0)).and_then(|r| mm.save_config(&r, arg(args, 1))).unwrap_or(Value::Null)));
    let mm = m.clone();
    desk.on(MODULE, "ccm:run-config-delete", move |ctx, args| {
        let id = arg(args, 1);
        Ok(json!(match (root(ctx, arg(args, 0)), is_id(id)) {
            (Some(r), true) => mm.delete_config(&r, id.as_str().unwrap_or("")),
            _ => false,
        }))
    });
    let mm = m.clone();
    desk.on(MODULE, "ccm:run-config-import", move |ctx, args| Ok(json!(root(ctx, arg(args, 0)).map(|r| mm.import_configs(&r)).unwrap_or_default())));
    let mm = m.clone();
    desk.on(MODULE, "ccm:run-preview", move |ctx, args| {
        let Some(r) = root(ctx, arg(args, 0)) else { return Ok(Value::Null) };
        let dev = arg(args, 2);
        Ok(mm.preview(&r, arg(args, 1), if is_id(dev) { dev.as_str() } else { None }).unwrap_or(Value::Null))
    });
    let mm = m.clone();
    desk.on(MODULE, "ccm:run-settings", move |_, _| Ok(mm.get_settings().to_value()));
    let mm = m.clone();
    desk.on(MODULE, "ccm:run-settings-set", move |_, args| Ok(if arg(args, 0).is_object() { mm.set_settings(arg(args, 0)).to_value() } else { Value::Null }));
}

fn on_env(desk: &mut Desk, m: &RunManager) {
    let mm = m.clone();
    desk.on(MODULE, "ccm:run-detect", move |ctx, args| Ok(root(ctx, arg(args, 0)).map(|r| mm.detect(&r, true)).unwrap_or(Value::Null)));
    let mm = m.clone();
    desk.on(MODULE, "ccm:run-env", move |ctx, args| {
        Ok(match (inside(ctx, arg(args, 0)), kind_ok(arg(args, 1))) {
            (Some(d), Some(k)) => mm.env_of(&d, k),
            _ => Value::Null,
        })
    });
    let mm = m.clone();
    desk.on(MODULE, "ccm:run-toolchain", move |ctx, args| {
        Ok(match (inside(ctx, arg(args, 0)), kind_ok(arg(args, 1))) {
            (Some(d), Some(k)) => mm.toolchain(&d, k),
            _ => Value::Null,
        })
    });
    let mm = m.clone();
    desk.on(MODULE, "ccm:run-devices", move |_, _| Ok(Value::Array(mm.devices().iter().map(RunDevice::to_value).collect())));
    let mm = m.clone();
    desk.on(MODULE, "ccm:run-emulators", move |_, _| Ok(Value::Array(mm.emulators().iter().map(RunEmulator::to_value).collect())));
    let mm = m.clone();
    desk.on(MODULE, "ccm:run-launch-emulator", move |_, args| {
        let source = arg(args, 1).as_str().filter(|s| *s == "flutter" || *s == "adb");
        match (is_id(arg(args, 0)), source) {
            (true, Some(s)) => Ok(mm.launch_emulator(arg(args, 0).as_str().unwrap_or(""), s).to_value()),
            _ => Ok(denied()),
        }
    });
}

fn on_process(desk: &mut Desk, m: &RunManager) {
    let mm = m.clone();
    desk.on(MODULE, "ccm:run-start", move |ctx, args| {
        let req = arg(args, 0);
        let field = |k: &str| req.get(k).cloned().unwrap_or(Value::Null);
        let Some(r) = root(ctx, &field("root")) else { return Ok(denied()) };
        if !is_id(&field("configId")) {
            return Ok(denied());
        }
        let device = field("deviceId");
        Ok(mm.start(&r, field("configId").as_str().unwrap_or(""), if is_id(&device) { device.as_str() } else { None }).to_value())
    });
    let mm = m.clone();
    desk.on(MODULE, "ccm:run-stop", move |_, args| Ok(if is_id(arg(args, 0)) { mm.stop(arg(args, 0).as_str().unwrap_or("")).to_value() } else { denied() }));
    let mm = m.clone();
    desk.on(MODULE, "ccm:run-reload", move |_, args| Ok(if is_id(arg(args, 0)) { mm.reload(arg(args, 0).as_str().unwrap_or(""), arg(args, 1) == &json!(true)).to_value() } else { denied() }));
    let mm = m.clone();
    desk.on(MODULE, "ccm:run-pub-get", move |ctx, args| Ok(inside(ctx, arg(args, 0)).map(|d| mm.pub_get(&d).to_value()).unwrap_or_else(denied)));
    let mm = m.clone();
    desk.on(MODULE, "ccm:run-sessions", move |_, _| Ok(Value::Array(mm.list())));
    let mm = m.clone();
    desk.on(MODULE, "desk:run-stop-all", move |_, _| {
        mm.stop_all();
        Ok(json!(true))
    });
    let mm = m.clone();
    desk.on(MODULE, "desk:run-kill-all", move |_, _| {
        mm.kill_all();
        Ok(json!(true))
    });
}
