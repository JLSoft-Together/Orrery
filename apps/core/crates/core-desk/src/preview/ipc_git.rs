use super::branches::{self, Auth};
use super::gate::{self, denied, Latest};
use super::paths::{dirname, root_key};
use super::{changes, commit, gitmisc, log};
use crate::{arg, Ctx, Desk};
use serde_json::{json, Value};
use std::sync::Arc;

pub const MODULE: &str = "previewGit";

fn field<'a>(req: &'a Value, key: &str) -> &'a Value {
    req.get(key).unwrap_or(&Value::Null)
}

fn opt_field<'a>(req: &'a Value, key: &str) -> Option<&'a Value> {
    req.get(key)
}

fn failed_err() -> Value {
    json!({"ok": false, "error": "failed"})
}

fn with_repo(ctx: &Ctx, req: &Value, f: impl FnOnce(&str) -> Value) -> Value {
    match gate::repo_of(ctx, field(req, "root")) {
        Some(repo) => f(&repo),
        None => denied(),
    }
}

type EnvFn<'a> = Box<dyn Fn(&str) -> Option<Vec<(String, String)>> + 'a>;

fn auth_for<'a>(ctx: &'a Ctx<'a>, repo: &'a str, github: bool) -> Option<EnvFn<'a>> {
    if !github {
        return None;
    }
    Some(Box::new(move |remote: &str| branches::remote_url(repo, &json!(remote)).and_then(|url| gate::gh_env(ctx, repo, &url))))
}

fn run_auth(ctx: &Ctx, repo: &str, f: impl FnOnce(Auth) -> Value) -> Value {
    let caps = gate::caps(ctx);
    let a = auth_for(ctx, repo, gate::has_cap(&caps, "github"));
    f(a.as_deref())
}

pub fn register(desk: &mut Desk) {
    desk.on(MODULE, "ccm:changes-status", |ctx, args| {
        Ok(match gate::register_repo(ctx, arg(args, 0)) {
            Ok(repo) => changes::status(&repo),
            Err(e) => json!({"ok": false, "error": e}),
        })
    });
    desk.on(MODULE, "ccm:changes-diff", |ctx, args| {
        let req = arg(args, 0);
        Ok(with_repo(ctx, req, |repo| changes::diff(repo, field(req, "path"), field(req, "opts"))))
    });
    desk.on(MODULE, "ccm:git-show-image", |ctx, args| {
        let req = arg(args, 0);
        Ok(with_repo(ctx, req, |repo| gitmisc::head_image(repo, field(req, "path"))))
    });
    desk.on(MODULE, "ccm:changes-file", |ctx, args| {
        let Some(ok) = gate::allowed(ctx, arg(args, 0), false) else { return Ok(denied()) };
        if !std::fs::metadata(&ok.real).map(|m| m.is_file()).unwrap_or(false) {
            return Ok(denied());
        }
        let real = ok.real.clone();
        Ok(changes::file_diff(&ok.real, |dir| {
            if let Some(known) = gate::repo_of(ctx, &json!(dir)) {
                return Ok(known);
            }
            let top = changes::toplevel(dir).map_err(String::from)?;
            gate::add_repo(ctx, &top, &real);
            Ok(top)
        }))
    });
    desk.on(MODULE, "ccm:changes-discard", |ctx, args| {
        let req = arg(args, 0);
        let Some(repo) = gate::repo_of(ctx, field(req, "root")) else { return Ok(denied()) };
        if let Some(h) = opt_field(req, "hunk") {
            return Ok(changes::discard_hunk(&repo, field(req, "path"), h));
        }
        if !gate::has_cap(&gate::caps(ctx), "trash") {
            return Ok(json!({"ok": false, "error": "failed", "detail": "no trash"}));
        }
        let res = changes::discard_file(&repo, field(req, "path"), &|p| gate::trash(ctx, p));
        gate::invalidate(ctx, &repo);
        Ok(res)
    });
    desk.on(MODULE, "ccm:log-refs", |ctx, args| {
        Ok(match gate::register_repo(ctx, arg(args, 0)) {
            Ok(repo) => log::list_refs(&repo),
            Err(e) => json!({"ok": false, "error": e}),
        })
    });
    desk.on(MODULE, "ccm:log-page", |ctx, args| {
        let req = arg(args, 0);
        Ok(with_repo(ctx, req, |repo| log::log_page(repo, field(req, "query"))))
    });
    desk.on(MODULE, "ccm:log-commit", |ctx, args| {
        let req = arg(args, 0);
        Ok(with_repo(ctx, req, |repo| log::commit_detail(repo, field(req, "hash"))))
    });
    desk.on(MODULE, "ccm:log-contains", |ctx, args| {
        let req = arg(args, 0);
        Ok(with_repo(ctx, req, |repo| log::branches_containing(repo, field(req, "hash"))))
    });
    desk.on(MODULE, "ccm:log-diff", |ctx, args| {
        let req = arg(args, 0);
        Ok(with_repo(ctx, req, |repo| log::commit_diff(repo, field(req, "hash"), field(req, "path"), field(req, "opts"), opt_field(req, "origPath"))))
    });
    desk.on(MODULE, "ccm:git-branch-create", |ctx, args| {
        let req = arg(args, 0);
        Ok(with_repo(ctx, req, |repo| branches::create_branch(repo, field(req, "name"), field(req, "base"), field(req, "checkout"))))
    });
    desk.on(MODULE, "ccm:git-branch-delete", |ctx, args| {
        let req = arg(args, 0);
        let Some(repo) = gate::repo_of(ctx, field(req, "root")) else { return Ok(denied()) };
        if !gate::has_cap(&gate::caps(ctx), "confirmGit") {
            return Ok(denied());
        }
        let plan = match branches::delete_plan(&repo, field(req, "ref"), false) {
            Ok(p) => p,
            Err(e) => return Ok(json!({"ok": false, "error": e})),
        };
        if !gate::confirm(ctx, "git", json!({"title": plan.label, "commands": [format!("git {}", plan.args.join(" "))]})) {
            return Ok(json!({"ok": false, "error": "cancelled"}));
        }
        let res = run_auth(ctx, &repo, |a| branches::run_delete(&repo, &plan.args, a));
        if res["ok"] == json!(true) || res["error"] != json!("not-merged") || plan.remote {
            return Ok(res);
        }
        let force = match branches::delete_plan(&repo, field(req, "ref"), true) {
            Ok(p) => p,
            Err(e) => return Ok(json!({"ok": false, "error": e})),
        };
        if !gate::confirm(ctx, "git", json!({"title": force.label, "commands": [format!("git {}", force.args.join(" "))], "force": true})) {
            return Ok(json!({"ok": false, "error": "cancelled"}));
        }
        Ok(run_auth(ctx, &repo, |a| branches::run_delete(&repo, &force.args, a)))
    });
    desk.on(MODULE, "ccm:git-fetch", |ctx, args| {
        let Some(repo) = gate::repo_of(ctx, arg(args, 0)) else { return Ok(denied()) };
        Ok(run_auth(ctx, &repo, |a| branches::fetch_all(&repo, a)))
    });
    desk.on(MODULE, "ccm:git-branch-update", |ctx, args| {
        let req = arg(args, 0);
        let Some(repo) = gate::repo_of(ctx, field(req, "root")) else { return Ok(denied()) };
        Ok(run_auth(ctx, &repo, |a| branches::update_branch(&repo, field(req, "ref"), a)))
    });
    desk.on(MODULE, "ccm:git-push-preview", |ctx, args| {
        let req = arg(args, 0);
        Ok(with_repo(ctx, req, |repo| branches::push_preview(repo, field(req, "ref"), field(req, "remote"), field(req, "branch"))))
    });
    desk.on(MODULE, "ccm:git-push", |ctx, args| {
        let req = arg(args, 0);
        let Some(repo) = gate::repo_of(ctx, field(req, "root")) else { return Ok(denied()) };
        Ok(run_auth(ctx, &repo, |a| branches::push_branch(&repo, field(req, "ref"), field(req, "remote"), field(req, "branch"), field(req, "setUpstream"), a)))
    });
    desk.on(MODULE, "ccm:gh-repo-account", |ctx, args| {
        let req = arg(args, 0);
        let Some(repo) = gate::repo_of(ctx, field(req, "root")) else { return Ok(json!({"github": false})) };
        if !gate::has_cap(&gate::caps(ctx), "github") {
            return Ok(json!({"github": false}));
        }
        Ok(match branches::remote_url(&repo, field(req, "remote")) {
            Some(url) => gate::gh_resolve(ctx, &repo, &url),
            None => json!({"github": false}),
        })
    });
    desk.on(MODULE, "ccm:gh-repo-account-set", |ctx, args| {
        let req = arg(args, 0);
        let Some(repo) = gate::repo_of(ctx, field(req, "root")) else { return Ok(json!(false)) };
        if !gate::has_cap(&gate::caps(ctx), "github") {
            return Ok(json!(false));
        }
        Ok(json!(gate::gh_set_choice(ctx, &repo, field(req, "choice"))))
    });
    desk.on(MODULE, "ccm:git-stage", |ctx, args| {
        let req = arg(args, 0);
        Ok(with_repo(ctx, req, |repo| match opt_field(req, "hunk") {
            Some(h) => {
                let paths = field(req, "paths");
                let first = match paths {
                    Value::Array(a) => a.first().cloned().unwrap_or(Value::Null),
                    other => other.clone(),
                };
                commit::stage_hunk(repo, &first, h)
            }
            None => commit::stage_paths(repo, field(req, "paths")),
        }))
    });
    desk.on(MODULE, "ccm:git-unstage", |ctx, args| {
        let req = arg(args, 0);
        Ok(with_repo(ctx, req, |repo| commit::unstage_paths(repo, field(req, "paths"))))
    });
    desk.on(MODULE, "ccm:git-can-amend", |ctx, args| {
        Ok(match gate::repo_of(ctx, arg(args, 0)) {
            Some(repo) => json!(commit::head_unpushed(&repo)),
            None => json!(false),
        })
    });
    let commits = Arc::new(Latest::default());
    let c2 = commits.clone();
    desk.on(MODULE, "ccm:git-commit", move |ctx, args| {
        let req = arg(args, 0);
        let repo = gate::repo_of(ctx, field(req, "root"));
        let input = commit::clean_commit(req);
        let (Some(repo), Some(input)) = (repo, input) else { return Ok(denied()) };
        let key = root_key(&repo);
        if commits.has(&key) {
            return Ok(json!({"ok": false, "error": "failed", "detail": "busy"}));
        }
        if input.amend {
            if !commit::head_unpushed(&repo) {
                return Ok(json!({"ok": false, "error": "pushed"}));
            }
            if !gate::confirm(ctx, "run", json!({"kind": "amend", "a": "", "b": "", "lines": ["git commit --amend"]})) {
                return Ok(json!({"ok": false, "error": "cancelled"}));
            }
        }
        let Some(flag) = commits.try_start(&key) else { return Ok(json!({"ok": false, "error": "failed", "detail": "busy"})) };
        let res = commit::commit(&repo, &input, flag.clone());
        commits.finish(&key, &flag);
        Ok(res)
    });
    desk.on(MODULE, "ccm:git-commit-cancel", move |ctx, args| {
        Ok(json!(gate::repo_of(ctx, arg(args, 0)).map(|repo| c2.cancel(&root_key(&repo))).unwrap_or(false)))
    });
    let blames = Arc::new(Latest::default());
    desk.on(MODULE, "ccm:git-blame", move |ctx, args| {
        let Some(ok) = gate::allowed(ctx, arg(args, 0), false) else { return Ok(denied()) };
        if !std::fs::metadata(&ok.real).map(|m| m.is_file()).unwrap_or(false) {
            return Ok(denied());
        }
        let Ok(repo) = gate::register_repo(ctx, &json!(dirname(&ok.real))) else { return Ok(json!({"ok": false, "error": "not-repo"})) };
        let key = gate::sender(ctx).to_string();
        let flag = blames.start(&key);
        let res = gitmisc::blame(&repo, &ok.real, flag.clone());
        blames.finish(&key, &flag);
        Ok(res)
    });
    desk.on(MODULE, "ccm:git-merge-state", |ctx, args| {
        Ok(match gate::repo_of(ctx, arg(args, 0)) {
            Some(repo) => gitmisc::merge_state_json(&repo),
            None => failed_err(),
        })
    });
    desk.on(MODULE, "ccm:git-merge-abort", |ctx, args| {
        let Some(repo) = gate::repo_of(ctx, arg(args, 0)) else { return Ok(failed_err()) };
        if !gate::has_cap(&gate::caps(ctx), "confirmRun") {
            return Ok(failed_err());
        }
        let cd = format!("cd {repo}");
        Ok(gitmisc::abort_merge(&repo, &|kind, command| gate::confirm(ctx, "run", json!({"kind": "abort", "a": kind, "b": repo, "lines": [cd, command]}))))
    });
}
