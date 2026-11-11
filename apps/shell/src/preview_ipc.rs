use std::sync::Arc;

use core_desk::preview::paths::real_or_null;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::DialogExt;

use crate::hosts;
use crate::preview_host::{is_dir, pane_roots, Gate};
use crate::preview_text::{access_spec, full_access_spec, grant_title};
use crate::preview_watch::{GitDirWatcher, RootWatcher, Subs, GIT_DEBOUNCE, ROOT_DEBOUNCE};
use crate::state::{Shell, MAIN};

pub const CHANNELS: [&str; 10] = [
    "ccm:grant-folder",
    "ccm:request-access",
    "ccm:forget-grants",
    "ccm:full-access",
    "ccm:list-files",
    "ccm:watch",
    "ccm:unwatch",
    "ccm:changes-watch",
    "ccm:changes-unwatch",
    "ccm:project-root",
];

pub struct PreviewWatch {
    roots: RootWatcher,
    git: GitDirWatcher,
    files: Subs,
    changes: Subs,
}

pub fn handles(channel: &str) -> bool {
    CHANNELS.contains(&channel)
}

fn notify_changes(app: &AppHandle, root: &str) {
    let Some(w) = app.try_state::<PreviewWatch>() else { return };
    for owner in w.changes.owners_of(root) {
        if crate::remote::host::is_viewer_owner(&owner) {
            crate::remote::host::viewer_event(app, &owner, crate::remote::VEvent::Git { root: root.to_owned() });
            continue;
        }
        let _ = app.emit_to(owner.as_str(), "ccm:changes-updated", json!({"root": root}));
    }
}

fn on_root_change(app: &AppHandle, root: &str, changed: Vec<String>) {
    if let Some(g) = app.try_state::<Gate>() {
        g.files.invalidate(root);
    }
    notify_changes(app, root);
    let Some(w) = app.try_state::<PreviewWatch>() else { return };
    for owner in w.files.owners_of(root) {
        if crate::remote::host::is_viewer_owner(&owner) {
            crate::remote::host::viewer_event(app, &owner, crate::remote::VEvent::Fs { root: root.to_owned(), paths: changed.clone() });
            continue;
        }
        let _ = app.emit_to(owner.as_str(), "ccm:preview-changed", json!({"root": root, "tokens": crate::preview_serve::tokens_under(root), "paths": changed}));
    }
}

pub fn manage(app: &AppHandle) {
    app.manage(Gate::load(&app.state::<Shell>().data_dir));
    let (a, b) = (app.clone(), app.clone());
    app.manage(PreviewWatch {
        roots: RootWatcher::new(Arc::new(move |root: &str, changed: Vec<String>| on_root_change(&a, root, changed)), ROOT_DEBOUNCE),
        git: GitDirWatcher::new(Arc::new(move |root: &str, _: Vec<String>| notify_changes(&b, root)), GIT_DEBOUNCE),
        files: Subs::default(),
        changes: Subs::default(),
    });
}

pub fn release(app: &AppHandle, owner: &str) {
    let Some(w) = app.try_state::<PreviewWatch>() else { return };
    for root in w.files.release_all(owner) {
        w.roots.remove(&root);
    }
    for root in w.changes.release_all(owner) {
        w.roots.remove(&root);
        w.git.remove(&root);
    }
}

fn arg(args: &[Value], i: usize) -> &Value {
    args.get(i).unwrap_or(&Value::Null)
}

fn unwatch_key(root: &Value) -> Option<String> {
    let r = root.as_str()?;
    Some(real_or_null(r).unwrap_or_else(|| r.to_owned()))
}

fn pick_grant(app: &AppHandle, gate: &Gate) -> Value {
    let mut d = app.dialog().file().set_title(grant_title(&hosts::locale(app)));
    if let Some(w) = app.get_webview_window(MAIN) {
        d = d.set_parent(&w);
    }
    let Some(folder) = d.blocking_pick_folder().and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().into_owned()) else { return Value::Null };
    if gate.grant_folder(&folder) {
        json!(folder)
    } else {
        Value::Null
    }
}

pub fn dispatch(app: &AppHandle, owner: &str, channel: &str, args: &[Value]) -> Result<Value, String> {
    let gate = app.try_state::<Gate>().ok_or("preview gate not ready")?;
    let watch = app.try_state::<PreviewWatch>().ok_or("preview watch not ready")?;
    let roots = |_fresh: bool| pane_roots(app);
    let a0 = arg(args, 0);
    Ok(match channel {
        "ccm:grant-folder" => pick_grant(app, &gate),
        "ccm:request-access" => json!(gate.request_access(&roots, a0, &|folder| {
            let c = access_spec(&hosts::locale(app), folder);
            hosts::confirm(app, &c.message, &c.detail, c.ok, c.cancel)
        })),
        "ccm:forget-grants" => {
            gate.grants.clear();
            json!(true)
        }
        "ccm:full-access" => json!(gate.full_access(a0, &|| {
            let c = full_access_spec(&hosts::locale(app));
            hosts::confirm(app, &c.message, &c.detail, c.ok, c.cancel)
        })),
        "ccm:list-files" => gate.list_dir_files(&roots, a0),
        "ccm:project-root" => gate.project_root(&roots, a0, arg(args, 1) == &Value::Bool(true)),
        "ccm:watch" => {
            let Some((real, _)) = gate.allowed(&roots, a0, false).filter(|(r, _)| is_dir(r)) else { return Ok(json!(false)) };
            if !watch.roots.add(&real) {
                return Ok(json!(false));
            }
            watch.files.hold(owner, &real);
            json!(true)
        }
        "ccm:unwatch" => match unwatch_key(a0).and_then(|k| watch.files.release_one(owner, &k)) {
            Some(held) => {
                watch.roots.remove(&held);
                json!(true)
            }
            None => json!(false),
        },
        "ccm:changes-watch" => {
            let Some(repo) = gate.repo_of(&roots, a0) else { return Ok(json!(false)) };
            if !watch.roots.add(&repo) {
                return Ok(json!(false));
            }
            watch.git.add(&repo);
            watch.changes.hold(owner, &repo);
            json!(true)
        }
        "ccm:changes-unwatch" => match unwatch_key(a0).and_then(|k| watch.changes.release_one(owner, &k)) {
            Some(held) => {
                watch.roots.remove(&held);
                watch.git.remove(&held);
                json!(true)
            }
            None => json!(false),
        },
        _ => return Err(format!("not ported: {channel}")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_preview_channels_exist_in_the_generated_shim_and_not_in_the_desk() {
        let shim = include_str!("shim.js");
        let dir = std::env::temp_dir().join(format!("orrery-preview-ipc-{}", std::process::id()));
        let desk = crate::desk::boot(&crate::desk::env_for(&dir, "1.0.0")).desk.channels();
        for c in CHANNELS {
            assert!(shim.contains(&format!("\"{c}\"")), "{c}");
            assert!(!desk.iter().any(|d| d == c), "{c}");
            assert!(handles(c));
        }
        assert!(!handles("ccm:list-dir"));
        let _ = std::fs::remove_dir_all(dir);
    }
}
