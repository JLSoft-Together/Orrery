use super::avd::is_avd_name;
use super::manager::DeviceManager;
use super::manifest::read_project_manifest;
use super::parse::err;
use super::toggles::{clean_change, clean_launch, is_package};
use crate::run::android::jsre::Js;
use crate::{arg, js, Ctx, Desk, DeskEnv, DeskError};
use serde_json::{json, Value};
use std::sync::Arc;

pub const MODULE: &str = "devices";

pub fn denied() -> Value {
    err("denied")
}

pub fn invalid() -> Value {
    err("invalid")
}

pub fn is_id(v: &Value) -> bool {
    static RE: Js = Js::new(r"^(?!-)[A-Za-z0-9_.:@%[\]-]{1,128}$", "");
    v.as_str().is_some_and(|s| RE.test(s))
}

fn ids(v: &Value) -> Option<Vec<String>> {
    let a = v.as_array().filter(|a| !a.is_empty() && a.len() <= 50)?;
    a.iter().all(is_id).then(|| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
}

fn is_address(v: &Value) -> Option<&str> {
    v.as_str().filter(|s| !s.is_empty() && js::len(s) <= 300)
}

fn pkg_of(v: &Value) -> Option<Option<&str>> {
    match v {
        Value::Null => Some(None),
        Value::String(p) if is_package(p) => Some(Some(p.as_str())),
        _ => None,
    }
}

fn owner(ctx: &Ctx, key: &str) -> String {
    ctx.host("devices.owner", json!({"key": key})).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_else(|| key.to_string())
}

fn on<F>(desk: &mut Desk, m: &Arc<DeviceManager>, channel: &str, f: F)
where
    F: Fn(&DeviceManager, &Ctx, &[Value]) -> Value + Send + Sync + 'static,
{
    let m = m.clone();
    desk.on(MODULE, channel, move |ctx, args| Ok::<Value, DeskError>(f(&m, ctx, args)));
}

pub fn register(desk: &mut Desk, env: &DeskEnv, settings: Arc<crate::run::settings::SettingsStore>) {
    let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap_or_default();
    let m = DeviceManager::new(&env.data_dir, crate::run::proc::process_env(), cfg!(windows), home, settings, desk.emitter(), desk.host_link());
    let closing = m.clone();
    desk.on_close(Arc::new(move || closing.stop()));

    on(desk, &m, "ccm:dev-snapshot", |m, _, _| m.snapshot());
    on(desk, &m, "ccm:dev-hold", |m, ctx, args| match arg(args, 0).as_bool() {
        Some(on) => {
            m.hold(&owner(ctx, "view"), on);
            json!(true)
        }
        None => json!(false),
    });
    on(desk, &m, "ccm:dev-connect", |m, _, args| is_address(arg(args, 0)).map(|a| m.connect(a)).unwrap_or_else(denied));
    on(desk, &m, "ccm:dev-pair", |m, _, args| {
        let Some(a) = is_address(arg(args, 0)) else { return denied() };
        match arg(args, 1).as_str().filter(|c| c.len() == 6 && c.bytes().all(|b| b.is_ascii_digit())) {
            Some(c) => m.pair(a, c),
            None => invalid(),
        }
    });
    on(desk, &m, "ccm:dev-disconnect", |m, _, args| ids(arg(args, 0)).map(|i| Value::Array(m.disconnect(&i))).unwrap_or_else(|| json!([])));
    on(desk, &m, "ccm:dev-connect-many", |m, _, args| ids(arg(args, 0)).map(|i| Value::Array(m.connect_many(&i))).unwrap_or_else(|| json!([])));
    on(desk, &m, "ccm:dev-to-wifi", |m, _, args| if is_id(arg(args, 0)) { m.to_wifi(arg(args, 0).as_str().unwrap_or("")) } else { denied() });
    on(desk, &m, "ccm:dev-remove", |m, _, args| ids(arg(args, 0)).map(|i| m.remove(&i)).unwrap_or_else(denied));
    on(desk, &m, "ccm:dev-alias", |m, _, args| {
        if !is_id(arg(args, 0)) {
            return denied();
        }
        let id = arg(args, 0).as_str().unwrap_or("");
        match arg(args, 1) {
            Value::Null => m.set_alias(id, None),
            Value::String(a) if js::len(a.trim()) <= 64 => m.set_alias(id, Some(a.trim())),
            _ => denied(),
        }
    });
    on(desk, &m, "ccm:dev-auto-connect", |m, _, args| arg(args, 0).as_bool().map(|b| m.set_auto_connect(b)).unwrap_or_else(denied));
    on(desk, &m, "ccm:dev-discover", |m, _, _| m.discover());
    on(desk, &m, "ccm:dev-qr-start", |m, ctx, _| m.qr_start(&owner(ctx, "qr")));
    on(desk, &m, "ccm:dev-qr-cancel", |m, _, args| if is_id(arg(args, 0)) { m.qr_cancel(arg(args, 0).as_str().unwrap_or("")) } else { denied() });
    on(desk, &m, "ccm:dev-screencap", |m, _, args| if is_id(arg(args, 0)) { m.screencap(arg(args, 0).as_str().unwrap_or("")) } else { denied() });
    on(desk, &m, "ccm:dev-uidump", |m, _, args| if is_id(arg(args, 0)) { m.ui_dump(arg(args, 0).as_str().unwrap_or("")) } else { denied() });
    on(desk, &m, "ccm:dev-ui-read", |m, _, args| match (is_id(arg(args, 0)), pkg_of(arg(args, 1))) {
        (true, Some(p)) => m.ui_read(arg(args, 0).as_str().unwrap_or(""), p),
        _ => denied(),
    });
    on(desk, &m, "ccm:dev-ui-set", |m, _, args| {
        if !is_id(arg(args, 0)) {
            return denied();
        }
        match (clean_change(arg(args, 1)), pkg_of(arg(args, 2))) {
            (Some(c), Some(p)) => m.ui_apply(arg(args, 0).as_str().unwrap_or(""), &c, p),
            _ => invalid(),
        }
    });
    on(desk, &m, "ccm:dev-ui-restore", |m, _, args| match (is_id(arg(args, 0)), pkg_of(arg(args, 1))) {
        (true, Some(p)) => m.ui_restore(arg(args, 0).as_str().unwrap_or(""), p),
        _ => denied(),
    });
    on(desk, &m, "ccm:dev-manifest", |_, ctx, args| match ctx.host("gate.dir", json!({"dir": arg(args, 0)})).ok().and_then(|v| v.as_str().map(String::from)) {
        Some(root) => read_project_manifest(&root),
        None => denied(),
    });
    on(desk, &m, "ccm:dev-launch", |m, _, args| {
        if !is_id(arg(args, 0)) {
            return denied();
        }
        match clean_launch(arg(args, 1)) {
            Some(l) => m.launch(arg(args, 0).as_str().unwrap_or(""), &l),
            None => invalid(),
        }
    });
    on(desk, &m, "ccm:avd-list", |m, _, _| m.avd_list());
    on(desk, &m, "ccm:avd-start", |m, _, args| match (arg(args, 0).as_str().filter(|n| is_avd_name(n)), arg(args, 1).as_str().filter(|x| matches!(*x, "normal" | "cold" | "wipe"))) {
        (Some(n), Some(mode)) => m.avd_start(n, mode),
        _ => invalid(),
    });
    on(desk, &m, "ccm:avd-stop", |m, _, args| match arg(args, 0).as_str().filter(|n| is_avd_name(n)) {
        Some(n) => m.avd_stop(n),
        None => invalid(),
    });
    on(desk, &m, "desk:dev-release-owner", |m, _, args| {
        m.release_owner(arg(args, 0).as_str().unwrap_or(""));
        json!(true)
    });
    on(desk, &m, "desk:dev-auto-connect-once", |m, _, _| {
        m.auto_connect_once();
        json!(true)
    });
    super::ipc_ext::register(desk, &m, &env.data_dir);
}
