use super::api::{self, encode_component, encode_path, Method};
use super::err::{self, GhErr};
use serde_json::{json, Value};
use std::time::Duration;

pub const TIMEOUT: Duration = Duration::from_secs(15);
const BRANCHES_MAX: usize = 300;
const TITLE_MAX: usize = 256;
const BODY_MAX: usize = 60_000;

pub struct RepoRef<'a> {
    pub base: &'a str,
    pub token: &'a str,
    pub owner: &'a str,
    pub repo: &'a str,
}

impl RepoRef<'_> {
    pub fn route(&self, rest: &str) -> String {
        format!("/repos/{}/{}{rest}", encode_component(self.owner), encode_component(self.repo))
    }

    fn get(&self, rest: &str, need: &str) -> Result<Value, GhErr> {
        let res = api::fetch(self.base, self.token, &self.route(rest), TIMEOUT)?;
        Ok(err::ok_or(res, self.token, need)?.body)
    }

    fn send(&self, method: Method, rest: &str, body: Option<&Value>, need: &str) -> Result<Value, GhErr> {
        let res = api::send(method, self.base, self.token, &self.route(rest), body, TIMEOUT)?;
        Ok(err::ok_or(res, self.token, need)?.body)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MergeMethod {
    Merge,
    Squash,
    Rebase,
}

impl MergeMethod {
    pub fn parse(v: &str) -> Option<Self> {
        match v {
            "merge" => Some(MergeMethod::Merge),
            "squash" => Some(MergeMethod::Squash),
            "rebase" => Some(MergeMethod::Rebase),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            MergeMethod::Merge => "merge",
            MergeMethod::Squash => "squash",
            MergeMethod::Rebase => "rebase",
        }
    }
}

pub fn branch_ok(b: &str) -> bool {
    !b.is_empty() && b.len() <= 255 && !b.starts_with('-') && !b.contains("..") && b.chars().all(|c| !c.is_control() && !matches!(c, ' ' | '~' | '^' | ':' | '?' | '*' | '[' | '\\'))
}

pub fn sha_ok(s: &str) -> bool {
    (7..=64).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_hexdigit())
}

pub fn pr_view(p: &Value) -> Value {
    let s = |k: &str| p.get(k).cloned().unwrap_or(Value::Null);
    let merged = p.get("merged").and_then(Value::as_bool).unwrap_or(false) || p.get("merged_at").is_some_and(|m| !m.is_null());
    let state = if merged { "merged" } else { p.get("state").and_then(Value::as_str).unwrap_or("open") };
    json!({
        "number": s("number"),
        "title": s("title"),
        "url": s("html_url"),
        "state": state,
        "draft": p.get("draft").and_then(Value::as_bool).unwrap_or(false),
        "head": p.pointer("/head/ref").cloned().unwrap_or(Value::Null),
        "headSha": p.pointer("/head/sha").cloned().unwrap_or(Value::Null),
        "base": p.pointer("/base/ref").cloned().unwrap_or(Value::Null),
        "mergeable": s("mergeable"),
        "mergeableState": s("mergeable_state"),
    })
}

pub fn find_pr(r: &RepoRef, head: &str) -> Result<Option<Value>, GhErr> {
    if !branch_ok(head) {
        return Err(GhErr::new("invalid"));
    }
    let q = format!("/pulls?state=open&per_page=5&head={}", encode_component(&format!("{}:{head}", r.owner)));
    let list = r.get(&q, "Pull requests (read)")?;
    let Some(first) = list.as_array().and_then(|a| a.first()) else { return Ok(None) };
    let number = first.get("number").and_then(Value::as_u64).unwrap_or(0);
    if number == 0 {
        return Ok(Some(pr_view(first)));
    }
    match r.get(&format!("/pulls/{number}"), "Pull requests (read)") {
        Ok(full) => Ok(Some(pr_view(&full))),
        Err(_) => Ok(Some(pr_view(first))),
    }
}

pub fn create_pr(r: &RepoRef, base: &str, head: &str, title: &str, body: &str) -> Result<Value, GhErr> {
    let title = title.trim();
    if !branch_ok(base) || !branch_ok(head) || base == head || title.is_empty() {
        return Err(GhErr::new("invalid"));
    }
    let payload = json!({
        "title": crate::js::slice(title, TITLE_MAX),
        "head": head,
        "base": base,
        "body": crate::js::slice(body, BODY_MAX),
        "draft": false,
    });
    let out = r.send(Method::Post, "/pulls", Some(&payload), "Pull requests (write)")?;
    Ok(pr_view(&out))
}

pub fn merge_pr(r: &RepoRef, number: u64, method: MergeMethod, sha: &str) -> Result<Value, GhErr> {
    if number == 0 || !sha_ok(sha) {
        return Err(GhErr::new("invalid"));
    }
    let payload = json!({"merge_method": method.as_str(), "sha": sha});
    let out = r.send(Method::Put, &format!("/pulls/{number}/merge"), Some(&payload), "Contents (write)")?;
    Ok(json!({"merged": out.get("merged").and_then(Value::as_bool).unwrap_or(false), "sha": out.get("sha").cloned().unwrap_or(Value::Null)}))
}

pub fn delete_branch(r: &RepoRef, branch: &str) -> Result<(), GhErr> {
    if !branch_ok(branch) {
        return Err(GhErr::new("invalid"));
    }
    match r.send(Method::Delete, &format!("/git/refs/heads/{}", encode_path(branch)), None, "Contents (write)") {
        Ok(_) => Ok(()),
        Err(e) if e.code == "gh.invalid" && e.detail.as_deref().is_some_and(|d| d.contains("Reference does not exist")) => Ok(()),
        Err(e) => Err(e),
    }
}

pub fn default_branch(r: &RepoRef) -> Result<String, GhErr> {
    let repo = r.get("", "Metadata (read)")?;
    repo.get("default_branch").and_then(Value::as_str).filter(|b| !b.is_empty()).map(str::to_string).ok_or_else(|| GhErr::new("gh.http"))
}

pub fn remote_branches(r: &RepoRef) -> Result<Vec<String>, GhErr> {
    let mut out = Vec::new();
    for page in 1..=3 {
        let list = r.get(&format!("/branches?per_page=100&page={page}"), "Contents (read)")?;
        let Some(items) = list.as_array() else { break };
        out.extend(items.iter().filter_map(|b| b.get("name").and_then(Value::as_str)).map(str::to_string));
        if items.len() < 100 || out.len() >= BRANCHES_MAX {
            break;
        }
    }
    out.truncate(BRANCHES_MAX);
    Ok(out)
}
