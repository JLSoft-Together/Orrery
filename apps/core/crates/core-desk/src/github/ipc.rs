use super::ci;
use super::ci_watch::{CiWatch, Target};
use super::err::GhErr;
use super::pulls::{self, sha_ok, MergeMethod, RepoRef};
use super::{api, GitHub};
use crate::preview::{branches, gate};
use crate::Ctx;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;

const LOG_LINES: usize = 60;

pub struct GhIpc {
    pub gh: Arc<GitHub>,
    pub watch: Arc<CiWatch>,
}

pub struct Scope {
    pub repo: String,
    pub url: String,
    pub owner: String,
    pub name: String,
}

fn s<'a>(v: &'a Value, k: &str) -> &'a str {
    v.get(k).and_then(Value::as_str).unwrap_or("")
}

fn ok(mut v: Value) -> Value {
    if let Value::Object(m) = &mut v {
        m.insert("ok".into(), json!(true));
    }
    v
}

fn remote_name(req: &Value) -> String {
    let r = s(req, "remote").trim();
    if r.is_empty() { "origin".into() } else { r.to_string() }
}

pub fn head_sha(repo: &str, rev: &str) -> Option<String> {
    if rev.is_empty() || rev.starts_with('-') {
        return None;
    }
    let spec = format!("{rev}^{{commit}}");
    let res = crate::git::run_git(std::path::Path::new(repo), &["rev-parse", "--verify", "--quiet", &spec], Duration::from_secs(10));
    let out = res.stdout.trim().to_string();
    (res.ok && sha_ok(&out)).then_some(out)
}

impl GhIpc {
    fn scope(&self, ctx: &Ctx, req: &Value) -> Result<Scope, GhErr> {
        let repo = gate::repo_of(ctx, req.get("root").unwrap_or(&Value::Null)).ok_or_else(|| GhErr::new("denied"))?;
        if !gate::has_cap(&gate::caps(ctx), "github") {
            return Err(GhErr::new("denied"));
        }
        let url = branches::remote_url(&repo, &json!(remote_name(req))).ok_or_else(|| GhErr::new("no-remote"))?;
        let Some((owner, name, _)) = api::github_repo_of(&url) else { return Err(GhErr::new("gh.notGithub")) };
        Ok(Scope { repo, url, owner, name })
    }

    fn with_repo<T>(&self, ctx: &Ctx, req: &Value, f: impl FnOnce(&Scope, &RepoRef) -> Result<T, GhErr>) -> Result<T, GhErr> {
        let sc = self.scope(ctx, req)?;
        let token = self.gh.creds(ctx, &sc.repo, &sc.url)?;
        let r = RepoRef { base: &self.gh.base, token: &token, owner: &sc.owner, repo: &sc.name };
        f(&sc, &r)
    }

    fn watch_target(&self, sc: &Scope, branch: &str, sha: &str, pr: Option<u64>) -> Option<&'static str> {
        if !pulls::branch_ok(branch) || !sha_ok(sha) {
            return None;
        }
        let t = Target { root: sc.repo.clone(), remote_url: sc.url.clone(), owner: sc.owner.clone(), repo: sc.name.clone(), branch: branch.into(), sha: sha.into(), pr };
        self.watch.add(t).ok().map(|a| a.as_str())
    }

    pub fn pr(&self, ctx: &Ctx, req: &Value) -> Value {
        let out = match s(req, "op") {
            "find" => self.with_repo(ctx, req, |_, r| pulls::find_pr(r, s(req, "head")).map(|pr| json!({"pr": pr}))),
            "base" => self.with_repo(ctx, req, |_, r| {
                let default = pulls::default_branch(r)?;
                let branches = pulls::remote_branches(r)?;
                Ok(json!({"default": default, "branches": branches}))
            }),
            "create" => self.with_repo(ctx, req, |sc, r| {
                let pr = pulls::create_pr(r, s(req, "base"), s(req, "head"), s(req, "title"), s(req, "body"))?;
                let watch = self.watch_target(sc, s(req, "head"), s(&pr, "headSha"), pr.get("number").and_then(Value::as_u64));
                Ok(json!({"pr": pr, "watch": watch}))
            }),
            "merge" => self.merge(ctx, req),
            "deleteBranch" => self.delete_branch(ctx, req),
            _ => Err(GhErr::new("invalid")),
        };
        out.map(ok).unwrap_or_else(|e| e.value())
    }

    fn merge(&self, ctx: &Ctx, req: &Value) -> Result<Value, GhErr> {
        let number = req.get("number").and_then(Value::as_u64).filter(|n| *n > 0).ok_or_else(|| GhErr::new("invalid"))?;
        let method = MergeMethod::parse(s(req, "method")).ok_or_else(|| GhErr::new("invalid"))?;
        let sha = s(req, "sha");
        let remove = req.get("deleteBranch") == Some(&json!(true));
        if !sha_ok(sha) {
            return Err(GhErr::new("invalid"));
        }
        self.with_repo(ctx, req, |sc, r| {
            let facts = pulls::pr_facts(r, number)?;
            if !facts.open {
                return Err(GhErr::new("gh.notMergeable"));
            }
            let mut ask = json!({"number": number, "base": facts.base, "head": facts.head, "headRepo": facts.head_repo, "method": method.as_str(), "repo": format!("{}/{}", sc.owner, sc.name)});
            let will_delete = remove && facts.same_repo();
            if will_delete {
                ask["deleteBranch"] = json!(facts.head);
            }
            if !gate::confirm(ctx, "ghMerge", ask) {
                return Err(GhErr::new("cancelled"));
            }
            let merged = pulls::merge_pr(r, number, method, sha)?;
            let did_merge = merged["merged"].as_bool().unwrap_or(false);
            let mut out = json!({"merged": did_merge, "sha": merged["sha"], "branchDeleted": false});
            if remove {
                if facts.may_delete_head(did_merge) {
                    match pulls::delete_branch(r, &facts.head) {
                        Ok(()) => out["branchDeleted"] = json!(true),
                        Err(e) => out["deleteError"] = e.value(),
                    }
                } else if did_merge {
                    out["deleteError"] = GhErr::new("gh.forkHead").value();
                }
            }
            if let Some(m) = merged["sha"].as_str() {
                out["watch"] = json!(self.watch_target(sc, &facts.base, m, Some(number)));
            }
            Ok(out)
        })
    }

    fn delete_branch(&self, ctx: &Ctx, req: &Value) -> Result<Value, GhErr> {
        let branch = s(req, "branch");
        if !pulls::branch_ok(branch) {
            return Err(GhErr::new("invalid"));
        }
        self.with_repo(ctx, req, |sc, r| {
            if !gate::confirm(ctx, "ghMerge", json!({"deleteBranch": branch, "repo": format!("{}/{}", sc.owner, sc.name)})) {
                return Err(GhErr::new("cancelled"));
            }
            pulls::delete_branch(r, branch)?;
            self.watch.remove(&sc.repo, branch);
            Ok(json!({"branchDeleted": true}))
        })
    }

    pub fn ci(&self, ctx: &Ctx, req: &Value) -> Value {
        let out = match s(req, "op") {
            "status" => self.with_repo(ctx, req, |_, r| ci::ci_status(r, s(req, "sha"))),
            "log" => {
                let job = req.get("jobId").and_then(Value::as_u64).unwrap_or(0);
                let lines = req.get("lines").and_then(Value::as_u64).map(|n| n as usize).unwrap_or(LOG_LINES);
                self.with_repo(ctx, req, |_, r| ci::job_log_tail(r, job, lines))
            }
            "watch" => self.scope(ctx, req).and_then(|sc| {
                let (branch, sha) = (s(req, "branch"), s(req, "sha"));
                if !pulls::branch_ok(branch) || !sha_ok(sha) {
                    return Err(GhErr::new("invalid"));
                }
                let pr = req.get("pr").and_then(Value::as_u64).filter(|n| *n > 0);
                let t = Target { root: sc.repo, remote_url: sc.url, owner: sc.owner, repo: sc.name, branch: branch.into(), sha: sha.into(), pr };
                self.watch.add(t).map(|a| json!({"watch": a.as_str()}))
            }),
            "unwatch" => {
                let repo = gate::repo_of(ctx, req.get("root").unwrap_or(&Value::Null));
                match repo {
                    Some(repo) => Ok(json!({"removed": self.watch.remove(&repo, s(req, "branch"))})),
                    None => Err(GhErr::new("denied")),
                }
            }
            "list" => Ok(json!({"watches": self.watch.list()})),
            _ => Err(GhErr::new("invalid")),
        };
        out.map(ok).unwrap_or_else(|e| e.value())
    }

    pub fn after_push(&self, ctx: &Ctx, req: &Value) -> Value {
        let repo = s(req, "root");
        let branch = s(req, "branch");
        if repo.is_empty() || !pulls::branch_ok(branch) || !gate::has_cap(&gate::caps(ctx), "github") {
            return json!(false);
        }
        let Some(url) = branches::remote_url(repo, &json!(remote_name(req))) else { return json!(false) };
        let Some((owner, name, _)) = api::github_repo_of(&url) else { return json!(false) };
        if self.gh.store().accounts.is_empty() {
            return json!(false);
        }
        let Some(sha) = head_sha(repo, s(req, "ref")) else { return json!(false) };
        let sc = Scope { repo: repo.into(), url, owner, name };
        json!(self.watch_target(&sc, branch, &sha, None).is_some())
    }
}
