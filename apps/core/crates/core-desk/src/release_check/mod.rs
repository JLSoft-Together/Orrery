pub mod rules;
pub mod scan;
pub mod sign;
pub mod store;
pub mod xml;

use crate::js;
use crate::preview::paths::{is_absolute, is_within, real_or_null, root_key};
use crate::run::android::deps::FsEnvDeps;
use crate::run::proc::process_env;
use crate::run::settings::SettingsStore;
use crate::util::now_ms;
use crate::{arg, Ctx, Desk, DeskEnv};
use rules::cert_finding;
use scan::{newest_artifact, scan_release};
use serde_json::{json, Value};
use sign::{find_sign_tools, read_signer, Outcome, ToolsOpts};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use store::Store;

pub const MODULE: &str = "releaseCheck";
const VERIFIED_MAX: usize = 50;

struct Verified {
    key: String,
    sha256: String,
    debug_key: bool,
    file: String,
}

struct State {
    store: Store,
    data_dir: std::path::PathBuf,
    verified: Mutex<VecDeque<Verified>>,
}

fn fail(code: &str) -> Value {
    json!({"ok": false, "error": code})
}

fn with_verified(st: &State, mut app: Value) -> Value {
    let key = root_key(app["dir"].as_str().unwrap_or(""));
    let verified = st.verified.lock().unwrap();
    let Some(v) = verified.iter().find(|v| v.key == key) else { return app };
    let remembered = app["cert"]["sha256"].as_str().map(String::from);
    if let Some(list) = app["findings"].as_array_mut() {
        for f in list.iter_mut() {
            if f["rule"] == json!("cert") {
                *f = cert_finding(Some(&v.sha256), remembered.as_deref(), v.debug_key, Some(&v.file));
            }
        }
    }
    app
}

fn check(st: &State, ctx: &Ctx, root: &Value) -> Result<(String, Vec<Value>), Value> {
    let Some(r) = root.as_str().filter(|r| js::len(r) <= 4096 && is_absolute(r)) else { return Err(fail("invalid")) };
    let dir = ctx.host("gate.dir", json!({"dir": r})).ok().and_then(|v| v.as_str().map(String::from));
    let Some(dir) = dir else { return Err(fail("denied")) };
    let apps = scan_release(&dir, &st.store);
    if apps.is_empty() {
        return Err(fail("no-app"));
    }
    Ok((dir, apps.into_iter().map(|a| with_verified(st, a)).collect()))
}

fn check_value(st: &State, ctx: &Ctx, root: &Value) -> Value {
    match check(st, ctx, root) {
        Ok((dir, apps)) => json!({"ok": true, "root": dir, "apps": apps, "checkedAt": now_ms() as i64}),
        Err(v) => v,
    }
}

fn app_of(st: &State, ctx: &Ctx, req: &Value) -> Result<(String, Value), Value> {
    let app = req.get("app").unwrap_or(&Value::Null);
    let Some(app) = app.as_str().filter(|a| is_absolute(a)) else { return Err(fail("invalid")) };
    let (root, apps) = check(st, ctx, req.get("root").unwrap_or(&Value::Null))?;
    match apps.into_iter().find(|a| root_key(a["dir"].as_str().unwrap_or("")) == root_key(app)) {
        Some(hit) => Ok((root, hit)),
        None => Err(fail("invalid")),
    }
}

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    let st = Arc::new(State { store: Store::new(&env.data_dir), data_dir: env.data_dir.clone(), verified: Mutex::new(VecDeque::new()) });

    let s = st.clone();
    desk.on(MODULE, "ccm:release-check", move |ctx, args| Ok(check_value(&s, ctx, arg(args, 0))));

    let s = st.clone();
    desk.on(MODULE, "ccm:release-mark", move |ctx, args| {
        let (root, app) = match app_of(&s, ctx, arg(args, 0)) {
            Ok(x) => x,
            Err(v) => return Ok(v),
        };
        if !s.store.mark_released(app["dir"].as_str().unwrap_or(""), &app["version"]["code"], &app["version"]["name"], now_ms()) {
            return Ok(fail("read-failed"));
        }
        Ok(check_value(&s, ctx, &json!(root)))
    });

    let s = st.clone();
    desk.on(MODULE, "ccm:release-verify", move |ctx, args| {
        let req = arg(args, 0);
        let (root, app) = match app_of(&s, ctx, req) {
            Ok(x) => x,
            Err(v) => return Ok(fail(if v["error"] == json!("denied") { "denied" } else { "invalid" })),
        };
        let dir = app["dir"].as_str().unwrap_or("").to_string();
        let pick = req.get("pick") == Some(&json!(true));
        let file = if pick {
            ctx.host("release.pick", json!({"near": dir})).ok().and_then(|v| v.as_str().map(String::from))
        } else {
            newest_artifact(app["kind"].as_str().unwrap_or(""), &dir)
        };
        let Some(file) = file else { return Ok(fail(if pick { "cancelled" } else { "no-artifact" })) };
        let Some(file) = real_or_null(&file).filter(|f| {
            let l = f.to_ascii_lowercase();
            l.ends_with(".apk") || l.ends_with(".aab")
        }) else {
            return Ok(fail("invalid"));
        };
        let settings = SettingsStore::load(&s.data_dir, cfg!(windows)).get();
        let env = process_env();
        let home = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")).unwrap_or_default();
        let tools = find_sign_tools(&ToolsOpts { jdk_setting: settings.jdk.as_deref(), sdk_setting: settings.android_sdk.as_deref(), env: &env, win: cfg!(windows), home: &home }, &FsEnvDeps);
        for t in [&tools.java, &tools.keytool, &tools.apksigner_jar].into_iter().flatten() {
            if is_within(&root, t) {
                return Ok(fail("no-jdk"));
            }
        }
        let (sha256, subject, debug_key) = match read_signer(&file, &tools, &FsEnvDeps) {
            Outcome::Ok { sha256, subject, debug_key } => (sha256, subject, debug_key),
            Outcome::Err(e) => return Ok(fail(e)),
        };
        let key = root_key(&dir);
        {
            let mut v = s.verified.lock().unwrap();
            if v.len() >= VERIFIED_MAX && !v.iter().any(|x| x.key == key) {
                v.pop_front();
            }
            match v.iter_mut().find(|x| x.key == key) {
                Some(x) => {
                    x.sha256 = sha256.clone();
                    x.debug_key = debug_key;
                    x.file = file.clone();
                }
                None => v.push_back(Verified { key, sha256: sha256.clone(), debug_key, file: file.clone() }),
            }
        }
        let finding = cert_finding(Some(&sha256), app["cert"]["sha256"].as_str(), debug_key, Some(&file));
        Ok(json!({"ok": true, "file": file, "artifact": if file.to_ascii_lowercase().ends_with(".aab") { "aab" } else { "apk" }, "sha256": sha256, "subject": subject, "debugKey": debug_key, "finding": finding}))
    });

    let s = st;
    desk.on(MODULE, "ccm:release-remember", move |ctx, args| {
        let (root, app) = match app_of(&s, ctx, arg(args, 0)) {
            Ok(x) => x,
            Err(v) => return Ok(v),
        };
        let dir = app["dir"].as_str().unwrap_or("");
        let (sha, debug) = {
            let v = s.verified.lock().unwrap();
            let key = root_key(dir);
            match v.iter().find(|x| x.key == key) {
                Some(x) => (x.sha256.clone(), x.debug_key),
                None => return Ok(fail("invalid")),
            }
        };
        if debug {
            return Ok(fail("invalid"));
        }
        if !s.store.remember_cert(dir, &sha, now_ms()) {
            return Ok(fail("read-failed"));
        }
        Ok(check_value(&s, ctx, &json!(root)))
    });
}
