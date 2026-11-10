use super::checkpoints::CheckpointStore;
use super::gate::{self, denied, Busy, Latest};
use super::ipc_git::MODULE;
use super::worktrees;
use crate::{arg, Ctx, Desk, DeskEnv};
use serde_json::{json, Value};
use std::sync::Arc;

fn field<'a>(req: &'a Value, key: &str) -> &'a Value {
    req.get(key).unwrap_or(&Value::Null)
}

pub fn js_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number(n.as_f64().unwrap_or(f64::NAN)),
        Value::Array(a) => a.iter().map(|x| if x.is_null() { String::new() } else { js_string(x) }).collect::<Vec<_>>().join(","),
        Value::Object(_) => "[object Object]".into(),
    }
}

fn confirm_run(ctx: &Ctx, kind: &str, a: &str, b: &str, lines: Vec<String>) -> bool {
    gate::confirm(ctx, "run", json!({"kind": kind, "a": a, "b": b, "lines": lines}))
}

fn cp_repo(ctx: &Ctx, root: &Value) -> Option<String> {
    if !gate::has_cap(&gate::caps(ctx), "history") {
        return None;
    }
    gate::register_repo(ctx, root).ok()
}

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    desk.on(MODULE, "ccm:wt-compare", |ctx, args| {
        let req = arg(args, 0);
        Ok(gate::check_dir(ctx, field(req, "dir")).map(|d| worktrees::compare_branch(&d, field(req, "branch"))).unwrap_or_else(denied))
    });
    desk.on(MODULE, "ccm:wt-diff", |ctx, args| {
        let req = arg(args, 0);
        Ok(gate::check_dir(ctx, field(req, "dir")).map(|d| worktrees::diff_between(&d, field(req, "a"), field(req, "b"))).unwrap_or_else(denied))
    });
    desk.on(MODULE, "ccm:wt-file-diff", |ctx, args| {
        let req = arg(args, 0);
        Ok(match gate::register_repo(ctx, field(req, "dir")) {
            Ok(repo) => worktrees::file_between(&repo, &repo, field(req, "a"), field(req, "b"), field(req, "path")),
            Err(_) => denied(),
        })
    });
    desk.on(MODULE, "ccm:wt-take", |ctx, args| {
        let req = arg(args, 0);
        let Ok(repo) = gate::register_repo(ctx, field(req, "dir")) else { return Ok(denied()) };
        if !gate::has_cap(&gate::caps(ctx), "confirmRun") {
            return Ok(denied());
        }
        Ok(worktrees::take_file(&repo, field(req, "from"), field(req, "path"), &|lines, branch, file| confirm_run(ctx, "take", file, branch, lines.to_vec())))
    });
    desk.on(MODULE, "ccm:overlap-files", |ctx, args| {
        Ok(gate::check_dir(ctx, arg(args, 0)).map(|d| worktrees::changed_vs_base(&d)).unwrap_or_else(|| json!({"ok": false, "error": "failed"})))
    });
    let dry = Arc::new(Latest::default());
    desk.on(MODULE, "ccm:overlap-dryrun", move |ctx, args| {
        let req = arg(args, 0);
        let Some(dir) = gate::check_dir(ctx, field(req, "dir")) else { return Ok(json!({"ok": false, "error": "unknown"})) };
        let key = gate::sender(ctx).to_string();
        let flag = dry.start(&key);
        let res = worktrees::dry_merge(&dir, field(req, "a"), field(req, "b"), flag.clone());
        dry.finish(&key, &flag);
        Ok(res)
    });
    let merging = Arc::new(Busy::default());
    desk.on(MODULE, "ccm:wt-merge", move |ctx, args| {
        let req = arg(args, 0);
        let dir = gate::check_dir(ctx, field(req, "dir"));
        let mode = field(req, "mode").as_str().filter(|m| *m == "ff" || *m == "squash");
        let (Some(dir), Some(mode)) = (dir, mode) else { return Ok(denied()) };
        if !gate::has_cap(&gate::caps(ctx), "confirmRun") {
            return Ok(denied());
        }
        let Some(_g) = merging.enter() else { return Ok(json!({"ok": false, "error": "failed", "detail": "busy"})) };
        let branch = js_string(field(req, "branch"));
        let res = worktrees::merge_branch(
            &dir,
            field(req, "branch"),
            mode,
            &|commands, target, base| {
                let mut lines = vec![format!("cd {target}")];
                lines.extend(commands.iter().cloned());
                confirm_run(ctx, mode, &branch, base, lines)
            },
            field(req, "keep") == &json!(true),
        );
        if res["ok"] != json!(true) {
            if let Some(kept) = res.get("kept").filter(|k| k.as_str().map(|s| !s.is_empty()).unwrap_or(false)) {
                let _ = gate::register_repo(ctx, kept);
            }
        }
        Ok(res)
    });

    let cps = Arc::new(CheckpointStore::new(env.data_dir.clone()));
    let c = cps.clone();
    desk.on(MODULE, "ccm:cp-state", move |ctx, args| {
        let Some(repo) = cp_repo(ctx, arg(args, 0)) else { return Ok(denied()) };
        Ok(json!({"ok": true, "root": repo, "on": c.is_on(&repo), "list": c.list(&repo)}))
    });
    let c = cps.clone();
    desk.on(MODULE, "ccm:cp-set", move |ctx, args| {
        let req = arg(args, 0);
        let Some(repo) = cp_repo(ctx, field(req, "root")) else { return Ok(json!(false)) };
        let Some(on) = field(req, "on").as_bool() else { return Ok(json!(false)) };
        c.set_on(&repo, on);
        Ok(json!(true))
    });
    let c = cps.clone();
    desk.on(MODULE, "ccm:cp-take", move |ctx, args| {
        let req = arg(args, 0);
        let Some(repo) = cp_repo(ctx, field(req, "root")) else { return Ok(denied()) };
        Ok(c.take(&repo, field(req, "label").as_str().unwrap_or("turn")))
    });
    let c = cps.clone();
    desk.on(MODULE, "ccm:cp-diff", move |ctx, args| {
        let req = arg(args, 0);
        let Some(repo) = cp_repo(ctx, field(req, "root")) else { return Ok(denied()) };
        Ok(c.diff(&repo, field(req, "id"), field(req, "to")))
    });
    let c = cps.clone();
    desk.on(MODULE, "ccm:cp-file-diff", move |ctx, args| {
        let req = arg(args, 0);
        let Some(repo) = cp_repo(ctx, field(req, "root")) else { return Ok(denied()) };
        Ok(c.file_diff(&repo, field(req, "from"), field(req, "to"), field(req, "path")))
    });
    let c = cps.clone();
    desk.on(MODULE, "ccm:cp-revert-hunk", move |ctx, args| {
        let req = arg(args, 0);
        let Some(repo) = cp_repo(ctx, field(req, "root")) else { return Ok(denied()) };
        Ok(c.revert_hunk(&repo, field(req, "path"), field(req, "patch")))
    });
    desk.on(MODULE, "ccm:cp-restore", move |ctx, args| {
        let req = arg(args, 0);
        let repo = cp_repo(ctx, field(req, "root"));
        let caps = gate::caps(ctx);
        let paths = field(req, "paths").as_array().filter(|a| !a.is_empty());
        let (Some(repo), Some(paths), true, true) = (repo, paths, gate::has_cap(&caps, "trash"), gate::has_cap(&caps, "confirmRun")) else { return Ok(denied()) };
        let shown: Vec<String> = paths.iter().take(12).map(js_string).collect();
        let a = if paths.len() == 1 { shown[0].clone() } else { format!("{} files", paths.len()) };
        let mut lines = shown.clone();
        if paths.len() > 12 {
            lines.push("…".into());
        }
        if !confirm_run(ctx, "restore", &a, "", lines) {
            return Ok(json!({"ok": false, "error": "cancelled"}));
        }
        let res = cps.restore(&repo, field(req, "id"), field(req, "paths"), &|p| gate::trash(ctx, p));
        gate::invalidate(ctx, &repo);
        Ok(res)
    });
}
