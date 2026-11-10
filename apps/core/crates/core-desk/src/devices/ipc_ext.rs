use super::appdata::{is_data_file_name, AppData};
use super::devfs::{clean_scope, safe_to_open, valid_path};
use super::files::{DeviceFiles, PUSH_MAX};
use super::ipc::{denied, invalid, is_id, MODULE};
use super::manager::DeviceManager;
use super::perf::{device_label, Perf};
use super::prefs::clean_pref_edit;
use super::recorder::{is_rec_token, Recorder, TIMING};
use super::toggles::is_package;
use crate::run::android::jsre::Js;
use crate::{arg, js, Ctx, Desk, DeskError};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::Arc;

fn is_transfer(v: &Value) -> bool {
    v.as_str().is_some_and(|s| s.len() == 16 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
}

fn is_handle(v: &Value) -> bool {
    is_transfer(v)
}

fn id_of(v: &Value) -> Option<&str> {
    is_id(v).then(|| v.as_str().unwrap_or(""))
}

fn pkg_of(v: &Value) -> Option<&str> {
    v.as_str().filter(|p| is_package(p))
}

fn clean_local_paths(v: &Value) -> Option<Vec<String>> {
    let a = v.as_array().filter(|a| !a.is_empty() && a.len() <= PUSH_MAX)?;
    let ok = a.iter().all(|p| p.as_str().is_some_and(|s| js::len(s) <= 4096 && !s.contains('\0') && Path::new(s).is_absolute()));
    ok.then(|| a.iter().filter_map(|p| p.as_str().map(String::from)).collect())
}

fn save_name(name: &Value) -> String {
    static BAD: Js = Js::new(r"[^\w.-]+", "g");
    static LEAD: Js = Js::new(r"^[.-]+", "");
    match name.as_str() {
        Some(n) => js::slice(&LEAD.replace_all(&BAD.replace_all(n, "-"), ""), 80),
        None => String::new(),
    }
}

fn host_str(ctx: &Ctx, method: &str, params: Value) -> Option<String> {
    ctx.host(method, params).ok().and_then(|v| v.as_str().map(String::from))
}

pub fn register(desk: &mut Desk, m: &Arc<DeviceManager>, data_dir: &Path) {
    let rec = Recorder::new(m.clone(), TIMING);
    let files = Arc::new(DeviceFiles::new(m.clone()));
    let perf = Arc::new(Perf::new(m.clone(), data_dir));
    let appdata = Arc::new(AppData::new(m.clone(), data_dir));
    let (r, f, a) = (rec.clone(), files.clone(), appdata.clone());
    desk.on_close(Arc::new(move || {
        r.dispose();
        f.dispose();
        a.close_all();
    }));

    macro_rules! on {
        ($ch:expr, $body:expr) => {
            desk.on(MODULE, $ch, $body)
        };
    }

    let r = rec.clone();
    on!("ccm:dev-rec-start", move |_, args| Ok::<Value, DeskError>(id_of(arg(args, 0)).map(|i| r.start(i)).unwrap_or_else(denied)));
    let r = rec.clone();
    on!("ccm:dev-rec-stop", move |_, args| Ok::<Value, DeskError>(id_of(arg(args, 0)).map(|i| r.stop(i)).unwrap_or_else(denied)));
    let r = rec.clone();
    on!("ccm:dev-rec-cancel", move |_, args| Ok::<Value, DeskError>(id_of(arg(args, 0)).map(|i| r.cancel(i)).unwrap_or_else(denied)));
    let r = rec.clone();
    on!("ccm:dev-rec-save", move |ctx, args| {
        if !is_rec_token(arg(args, 0)) {
            return Ok(denied());
        }
        let token = arg(args, 0).as_str().unwrap_or("");
        if !r.has_file(token) {
            return Ok(invalid());
        }
        let base = save_name(arg(args, 1));
        let default_name = format!("{}.mp4", if base.is_empty() { "screen" } else { &base });
        let dest = host_str(ctx, "devices.recSave", json!({"defaultName": default_name})).filter(|d| !d.is_empty());
        Ok(r.save(token, dest.as_deref()))
    });
    let r = rec.clone();
    on!("ccm:dev-rec-discard", move |_, args| Ok::<Value, DeskError>(json!(is_rec_token(arg(args, 0)) && r.discard(arg(args, 0).as_str().unwrap_or("")))));

    let f = files.clone();
    on!("ccm:devfs-list", move |_, args| {
        let (id, scope) = (id_of(arg(args, 0)), clean_scope(arg(args, 1)));
        Ok::<Value, DeskError>(match (id, scope) {
            (Some(id), Some(s)) => valid_path(&s, arg(args, 2)).map(|d| f.list(id, &s, &d)).unwrap_or_else(invalid),
            _ => invalid(),
        })
    });
    let f = files.clone();
    on!("ccm:devfs-mkdir", move |_, args| {
        let (id, scope, name) = (id_of(arg(args, 0)), clean_scope(arg(args, 1)), arg(args, 3).as_str());
        Ok::<Value, DeskError>(match (id, scope, name) {
            (Some(id), Some(s), Some(name)) => valid_path(&s, arg(args, 2)).map(|d| f.mkdir(id, &s, &d, name)).unwrap_or_else(invalid),
            _ => invalid(),
        })
    });
    let f = files.clone();
    on!("ccm:devfs-delete", move |_, args| {
        let (id, scope, dir) = (id_of(arg(args, 0)), clean_scope(arg(args, 1)), arg(args, 3).as_bool());
        Ok::<Value, DeskError>(match (id, scope, dir) {
            (Some(id), Some(s), Some(dir)) => valid_path(&s, arg(args, 2)).map(|p| f.remove(id, &s, &p, dir)).unwrap_or_else(invalid),
            _ => invalid(),
        })
    });
    let f = files.clone();
    on!("ccm:devfs-pull", move |ctx, args| {
        let req = arg(args, 1);
        let scope = clean_scope(&req["scope"]);
        let kind = req["kind"].as_str().filter(|k| matches!(*k, "dir" | "file" | "link" | "other"));
        let size = match &req["size"] {
            Value::Null => Some(None),
            v => v.as_u64().filter(|n| *n <= 9_007_199_254_740_991).map(Some),
        };
        let (Some(id), Some(s), Some(kind), Some(size)) = (id_of(arg(args, 0)), scope, kind, size) else { return Ok(invalid()) };
        let (Some(p), true, true) = (valid_path(&s, &req["path"]), is_transfer(&req["transfer"]), req["dest"].is_object()) else { return Ok(invalid()) };
        let dest = &req["dest"];
        let dir = match dest["kind"].as_str() {
            Some("pick") => match host_str(ctx, "devfs.pickFolder", Value::Null).filter(|d| !d.is_empty()) {
                Some(d) => d,
                None => return Ok(json!({"ok": false, "error": "cancelled"})),
            },
            Some("dir") => match host_str(ctx, "gate.dir", json!({"dir": dest["dir"]})) {
                Some(d) => d,
                None => return Ok(json!({"ok": false, "error": "outside"})),
            },
            _ => return Ok(invalid()),
        };
        let progress = |v: Value| {
            let _ = ctx.host("devfs.progress", v);
        };
        Ok(f.pull(id, &s, &p, kind, if kind == "link" { None } else { size }, Path::new(&dir), req["transfer"].as_str().unwrap_or(""), &progress))
    });
    let f = files.clone();
    on!("ccm:devfs-push", move |ctx, args| {
        let req = arg(args, 1);
        let scope = clean_scope(&req["scope"]);
        let (Some(id), Some(s)) = (id_of(arg(args, 0)), scope) else { return Ok(invalid()) };
        let (Some(dir), true, true) = (valid_path(&s, &req["dir"]), is_transfer(&req["transfer"]), req["source"].is_object()) else { return Ok(invalid()) };
        let src = &req["source"];
        let locals: Vec<String> = match src["kind"].as_str() {
            Some("pick") => match ctx.host("devfs.pickFiles", Value::Null) {
                Ok(Value::Array(a)) if !a.is_empty() => a.iter().filter_map(|v| v.as_str().map(String::from)).collect(),
                _ => return Ok(json!({"ok": false, "error": "cancelled"})),
            },
            Some("paths") => match clean_local_paths(&src["paths"]) {
                Some(l) => l,
                None => return Ok(invalid()),
            },
            _ => return Ok(invalid()),
        };
        let progress = |v: Value| {
            let _ = ctx.host("devfs.progress", v);
        };
        Ok(f.push(id, &s, &dir, &locals, req["transfer"].as_str().unwrap_or(""), &progress))
    });
    let f = files.clone();
    on!("ccm:devfs-cancel", move |_, args| Ok::<Value, DeskError>(json!(is_transfer(arg(args, 0)) && f.cancel(arg(args, 0).as_str().unwrap_or("")))));
    let f = files.clone();
    on!("ccm:devfs-open", move |ctx, args| {
        let (Some(local), Some(mode)) = (arg(args, 0).as_str(), arg(args, 1).as_str().filter(|m| matches!(*m, "open" | "reveal"))) else { return Ok(json!(false)) };
        if !f.was_pulled(local) {
            return Ok(json!(false));
        }
        let name = Path::new(local).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let how = if mode == "open" && safe_to_open(&name) { "open" } else { "reveal" };
        Ok(json!(matches!(ctx.host("devfs.open", json!({"local": local, "mode": how})), Ok(Value::Bool(true)))))
    });

    let p = perf.clone();
    on!("ccm:perf-history", move |_, args| Ok::<Value, DeskError>(pkg_of(arg(args, 0)).map(|k| p.history(k)).unwrap_or_else(invalid)));
    let p = perf.clone();
    on!("ccm:perf-clear", move |_, args| Ok::<Value, DeskError>(pkg_of(arg(args, 0)).map(|k| p.clear(k)).unwrap_or_else(invalid)));
    let p = perf.clone();
    on!("ccm:perf-reset", move |_, args| Ok::<Value, DeskError>(match (id_of(arg(args, 0)), pkg_of(arg(args, 1))) {
        (Some(id), Some(k)) => p.reset(id, k),
        _ => invalid(),
    }));
    let p = perf.clone();
    on!("ccm:perf-measure", move |_, args| Ok::<Value, DeskError>(match (id_of(arg(args, 0)), pkg_of(arg(args, 1))) {
        (Some(id), Some(k)) => p.measure(id, k, &device_label(arg(args, 2))),
        _ => invalid(),
    }));
    let p = perf.clone();
    on!("ccm:perf-cold", move |_, args| Ok::<Value, DeskError>(match (id_of(arg(args, 0)), pkg_of(arg(args, 1))) {
        (Some(id), Some(k)) => p.cold_start(id, k, &device_label(arg(args, 2))),
        _ => invalid(),
    }));

    let a = appdata.clone();
    on!("ccm:appdata-list", move |_, args| Ok::<Value, DeskError>(match (id_of(arg(args, 0)), pkg_of(arg(args, 1))) {
        (Some(id), Some(k)) => a.list(id, k),
        _ => invalid(),
    }));
    let a = appdata.clone();
    on!("ccm:appdata-db-open", move |_, args| Ok::<Value, DeskError>(match (id_of(arg(args, 0)), pkg_of(arg(args, 1)), arg(args, 2).as_str().filter(|n| is_data_file_name(n))) {
        (Some(id), Some(k), Some(n)) => a.open_db(id, k, n),
        _ => invalid(),
    }));
    let a = appdata.clone();
    on!("ccm:appdata-db-rows", move |_, args| {
        let table = arg(args, 1).as_str().filter(|t| (1..=512).contains(&js::len(t)));
        let page = arg(args, 2).as_u64().filter(|p| *p <= 1_000_000);
        Ok::<Value, DeskError>(match (is_handle(arg(args, 0)), table, page) {
            (true, Some(t), Some(p)) => a.rows(arg(args, 0).as_str().unwrap_or(""), t, p as usize),
            _ => invalid(),
        })
    });
    let a = appdata.clone();
    on!("ccm:appdata-db-query", move |_, args| Ok::<Value, DeskError>(if is_handle(arg(args, 0)) { a.query(arg(args, 0).as_str().unwrap_or(""), arg(args, 1)) } else { invalid() }));
    let a = appdata.clone();
    on!("ccm:appdata-db-close", move |_, args| {
        if is_handle(arg(args, 0)) {
            a.close(arg(args, 0).as_str().unwrap_or(""));
        }
        Ok::<Value, DeskError>(json!(true))
    });
    let a = appdata.clone();
    on!("ccm:appdata-prefs-read", move |_, args| Ok::<Value, DeskError>(match (id_of(arg(args, 0)), pkg_of(arg(args, 1)), prefs_name(arg(args, 2))) {
        (Some(id), Some(k), Some(n)) => a.read_prefs(id, k, n),
        _ => invalid(),
    }));
    let a = appdata.clone();
    on!("ccm:appdata-prefs-write", move |_, args| {
        let (Some(id), Some(k), Some(n)) = (id_of(arg(args, 0)), pkg_of(arg(args, 1)), prefs_name(arg(args, 2))) else { return Ok(invalid()) };
        Ok(clean_pref_edit(arg(args, 3)).map(|e| a.write_pref(id, k, n, &e)).unwrap_or_else(invalid))
    });
    let a = appdata.clone();
    on!("ccm:appdata-prefs-restore", move |_, args| Ok::<Value, DeskError>(match (id_of(arg(args, 0)), pkg_of(arg(args, 1)), prefs_name(arg(args, 2))) {
        (Some(id), Some(k), Some(n)) => a.restore_prefs(id, k, n),
        _ => invalid(),
    }));
}

fn prefs_name(v: &Value) -> Option<&str> {
    v.as_str().filter(|n| is_data_file_name(n) && n.ends_with(".xml"))
}
