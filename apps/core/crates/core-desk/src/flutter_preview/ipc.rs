use super::host::read_package;
use super::service::{Deps, FlutterPreview};
use super::watcher::{watch_dir, SAVE_DEBOUNCE_MS};
use crate::preview::gate;
use crate::run::proc::process_env;
use crate::run::settings::SettingsStore;
use crate::{arg, js, Desk, DeskEnv};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

pub const MODULE: &str = "flutterPreview";
pub const EVENT: &str = "ccm:fwp-state";
const MAX_UP: usize = 8;

fn denied() -> Value {
    json!({"ok": false, "error": {"code": "denied"}})
}

fn none() -> Value {
    json!({"flutter": false, "root": null, "entries": []})
}

fn is_id(v: &Value) -> Option<&str> {
    v.as_str().filter(|s| (1..=128).contains(&js::len(s)) && s.chars().all(|c| c.is_ascii_alphanumeric() || "_.:@%-".contains(c)))
}

fn is_file(p: &Path) -> bool {
    std::fs::metadata(p).map(|m| m.is_file()).unwrap_or(false)
}

pub fn find_pubspec_root(start: &str, allowed: &dyn Fn(&str) -> Option<String>, exists: &dyn Fn(&Path) -> bool) -> Option<String> {
    let mut dir = if is_file(Path::new(start)) { Path::new(start).parent()?.to_string_lossy().to_string() } else { start.to_string() };
    for _ in 0..MAX_UP {
        if exists(&Path::new(&dir).join("pubspec.yaml")) {
            return Some(dir);
        }
        let parent = Path::new(&dir).parent()?.to_string_lossy().to_string();
        if parent == dir || parent.is_empty() {
            return None;
        }
        dir = allowed(&parent)?;
    }
    None
}

fn deps(settings: Arc<SettingsStore>, desk: &Desk) -> Deps {
    let emitter = desk.emitter();
    let debounce = Duration::from_millis(SAVE_DEBOUNCE_MS);
    Deps {
        env: process_env(),
        win: cfg!(windows),
        platform: crate::backup::service::platform_name(),
        home: crate::run::ipc::home_dir(),
        settings: Arc::new(move || settings.get()),
        emit: Arc::new(move |v| emitter.emit(EVENT, v)),
        watch: Arc::new(move |dir, cb| watch_dir(dir, true, None, debounce, cb)),
        watch_pubspec: Arc::new(move |root, cb| watch_dir(root, false, Some("pubspec.yaml"), debounce, Arc::new(move |_| cb()))),
        stop_timeout: Duration::from_millis(5_000),
        log_flush: Duration::from_millis(100),
        pub_get_timeout: Duration::from_millis(120_000),
    }
}

pub fn register(desk: &mut Desk, _env: &DeskEnv, settings: Arc<SettingsStore>) {
    let svc = FlutterPreview::new(deps(settings, desk));
    let s = svc.clone();
    desk.on(MODULE, "ccm:fwp-scan", move |ctx, args| {
        let real = gate::allowed(ctx, arg(args, 0), false).map(|a| a.real);
        let allowed = |p: &str| gate::allowed(ctx, &json!(p), false).map(|a| a.real);
        let Some(root) = real.and_then(|r| find_pubspec_root(&r, &allowed, &is_file)) else { return Ok(none()) };
        let pubspec = std::fs::read_to_string(Path::new(&root).join("pubspec.yaml")).unwrap_or_default();
        if !read_package(&pubspec).is_some_and(|p| p.flutter) {
            return Ok(json!({"flutter": false, "root": root, "entries": []}));
        }
        if arg(args, 1) == &json!(false) {
            return Ok(json!({"flutter": true, "root": root, "entries": []}));
        }
        let entries: Vec<Value> = s.scan(&root).iter().map(|e| e.to_value()).collect();
        Ok(json!({"flutter": true, "root": root, "entries": entries}))
    });
    let s = svc.clone();
    desk.on(MODULE, "ccm:fwp-start", move |ctx, args| {
        let root = gate::allowed(ctx, arg(args, 0), false).map(|a| a.real);
        let flavor = arg(args, 2);
        let flavor_ok = matches!(flavor, Value::Null | Value::String(_));
        let (Some(root), Some(device), true) = (root, is_id(arg(args, 1)), flavor_ok) else { return Ok(denied()) };
        Ok(s.start(&root, device, flavor.as_str().filter(|f| !f.is_empty())))
    });
    let s = svc.clone();
    desk.on(MODULE, "ccm:fwp-show", move |_, args| {
        let Some(id) = arg(args, 0).as_str().filter(|i| !i.is_empty() && js::len(i) <= 512) else { return Ok(denied()) };
        Ok(s.show(id))
    });
    let s = svc.clone();
    desk.on(MODULE, "ccm:fwp-stop", move |_, _| {
        s.stop();
        Ok(json!({"ok": true}))
    });
    let s = svc.clone();
    desk.on(MODULE, "ccm:fwp-get", move |_, _| Ok(s.get()));
    let closing = svc.clone();
    desk.on_close(Arc::new(move || closing.kill_now()));
    desk.on(MODULE, "desk:fwp-kill", move |_, _| {
        svc.kill_now();
        Ok(json!(true))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn climbs_to_the_pubspec_but_never_past_the_gate() {
        let root = std::env::temp_dir().join(format!("fwp-ipc-{}", std::process::id()));
        let lib = root.join("lib/deep");
        std::fs::create_dir_all(&lib).unwrap();
        std::fs::write(root.join("pubspec.yaml"), "name: x\n").unwrap();
        std::fs::write(lib.join("a.dart"), "").unwrap();
        let all = |p: &str| Some(p.to_string());
        let r = root.to_string_lossy().to_string();
        assert_eq!(find_pubspec_root(&lib.join("a.dart").to_string_lossy(), &all, &is_file).as_deref(), Some(r.as_str()));
        assert_eq!(find_pubspec_root(&r, &all, &is_file).as_deref(), Some(r.as_str()));
        let inside = |p: &str| p.starts_with(&r).then(|| p.to_string());
        assert_eq!(find_pubspec_root(&lib.to_string_lossy(), &inside, &is_file).as_deref(), Some(r.as_str()));
        std::fs::remove_file(root.join("pubspec.yaml")).unwrap();
        assert_eq!(find_pubspec_root(&lib.to_string_lossy(), &inside, &is_file), None);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn ids_are_validated() {
        assert_eq!(is_id(&json!("emulator-5554")), Some("emulator-5554"));
        assert_eq!(is_id(&json!("192.168.1.2:5555")), Some("192.168.1.2:5555"));
        assert!(is_id(&json!("bad id")).is_none() && is_id(&json!("")).is_none() && is_id(&json!(3)).is_none());
    }
}
