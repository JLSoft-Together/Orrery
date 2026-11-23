use super::ci;
use super::pulls::{self, MergeMethod, RepoRef};
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};

const TOKEN: &str = "ghp_SECRETSECRETSECRETSECRET0123";

#[derive(Debug, Clone)]
struct Seen {
    method: String,
    path: String,
    auth: String,
    body: Value,
}

type Route = Box<dyn Fn(&str, &str) -> (u16, Vec<(String, String)>, Vec<u8>) + Send + Sync>;

struct Fake {
    base: String,
    seen: Arc<Mutex<Vec<Seen>>>,
}

fn read_request(s: &mut TcpStream) -> Option<(String, String, String, Vec<u8>)> {
    let mut buf = Vec::new();
    let mut one = [0u8; 1];
    while !buf.ends_with(b"\r\n\r\n") {
        if s.read(&mut one).ok()? == 0 {
            return None;
        }
        buf.push(one[0]);
    }
    let head = String::from_utf8_lossy(&buf).to_string();
    let mut lines = head.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let (method, path) = (first.next()?.to_string(), first.next()?.to_string());
    let mut len = 0usize;
    let mut auth = String::new();
    for l in lines {
        let Some((k, v)) = l.split_once(':') else { continue };
        if k.eq_ignore_ascii_case("content-length") {
            len = v.trim().parse().unwrap_or(0);
        }
        if k.eq_ignore_ascii_case("authorization") {
            auth = v.trim().to_string();
        }
    }
    let mut body = vec![0u8; len];
    s.read_exact(&mut body).ok()?;
    Some((method, path, auth, body))
}

fn fake(route: Route) -> Fake {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let seen: Arc<Mutex<Vec<Seen>>> = Arc::default();
    let log = seen.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut s) = stream else { continue };
            let Some((method, path, auth, body)) = read_request(&mut s) else { continue };
            log.lock().unwrap().push(Seen { method: method.clone(), path: path.clone(), auth, body: serde_json::from_slice(&body).unwrap_or(Value::Null) });
            let (status, headers, out) = route(&method, &path);
            let mut head = format!("HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n", out.len());
            for (k, v) in headers {
                head.push_str(&format!("{k}: {v}\r\n"));
            }
            head.push_str("\r\n");
            let _ = s.write_all(head.as_bytes());
            let _ = s.write_all(&out);
            let _ = s.flush();
        }
    });
    Fake { base, seen }
}

fn j(v: Value) -> Vec<u8> {
    v.to_string().into_bytes()
}

fn repo(f: &Fake) -> RepoRef<'_> {
    RepoRef { base: &f.base, token: TOKEN, owner: "o", repo: "r" }
}

fn pr_json(n: u64) -> Value {
    json!({"number": n, "title": "T", "html_url": format!("https://github.com/o/r/pull/{n}"), "state": "open", "draft": false, "head": {"ref": "feat/x", "sha": "abc1234def"}, "base": {"ref": "main"}, "mergeable": true})
}

#[test]
fn pull_requests_against_a_fake_github() {
    let f = fake(Box::new(|m, p| match (m, p) {
        ("GET", p) if p.starts_with("/repos/o/r/pulls?") => (200, vec![], j(json!([pr_json(42)]))),
        ("GET", "/repos/o/r/pulls/42") => (200, vec![], j(pr_json(42))),
        ("POST", "/repos/o/r/pulls") => (201, vec![], j(pr_json(43))),
        ("PUT", "/repos/o/r/pulls/42/merge") => (200, vec![], j(json!({"merged": true, "sha": "fff0000aaa"}))),
        ("DELETE", "/repos/o/r/git/refs/heads/feat/x") => (204, vec![], vec![]),
        ("GET", "/repos/o/r") => (200, vec![], j(json!({"default_branch": "main"}))),
        ("GET", p) if p.starts_with("/repos/o/r/branches?") => (200, vec![], j(json!([{"name": "main"}, {"name": "dev"}]))),
        _ => (404, vec![], j(json!({"message": "Not Found"}))),
    }));
    let r = repo(&f);
    let found = pulls::find_pr(&r, "feat/x").unwrap().unwrap();
    assert_eq!(found["number"], 42);
    assert_eq!(found["headSha"], "abc1234def");
    let created = pulls::create_pr(&r, "main", "feat/x", " Add x ", "body").unwrap();
    assert_eq!(created["number"], 43);
    let merged = pulls::merge_pr(&r, 42, MergeMethod::Squash, "abc1234def").unwrap();
    assert_eq!(merged["merged"], true);
    pulls::delete_branch(&r, "feat/x").unwrap();
    assert_eq!(pulls::default_branch(&r).unwrap(), "main");
    assert_eq!(pulls::remote_branches(&r).unwrap(), vec!["main".to_string(), "dev".to_string()]);
    let seen = f.seen.lock().unwrap().clone();
    assert!(seen[0].path.contains("head=o%3Afeat%2Fx"));
    let post = seen.iter().find(|s| s.method == "POST").unwrap();
    assert_eq!(post.body, json!({"title": "Add x", "head": "feat/x", "base": "main", "body": "body", "draft": false}));
    let put = seen.iter().find(|s| s.method == "PUT").unwrap();
    assert_eq!(put.body, json!({"merge_method": "squash", "sha": "abc1234def"}));
    assert!(seen.iter().any(|s| s.method == "DELETE" && s.path == "/repos/o/r/git/refs/heads/feat/x"));
    assert!(seen.iter().all(|s| s.auth == format!("Bearer {TOKEN}")));
    assert_eq!(pulls::create_pr(&r, "main", "main", "t", "").unwrap_err().code, "invalid");
    assert_eq!(pulls::merge_pr(&r, 42, MergeMethod::Merge, "nothex!").unwrap_err().code, "invalid");
}

#[test]
fn errors_carry_github_message_and_permission_but_never_the_token() {
    let f = fake(Box::new(|_, p| match p {
        "/repos/o/r/pulls" => (403, vec![("X-Accepted-GitHub-Permissions".into(), "pull_requests=write".into())], j(json!({"message": format!("Resource not accessible by token {TOKEN}")}))),
        "/repos/o/r/pulls/7/merge" => (409, vec![], j(json!({"message": "Head branch was modified. Review and try the merge again."}))),
        _ => (422, vec![], j(json!({"message": "Validation Failed", "errors": [{"message": format!("bad {TOKEN}")}]}))),
    }));
    let r = repo(&f);
    let e = pulls::create_pr(&r, "main", "feat/x", "t", "").unwrap_err();
    assert_eq!(e.code, "gh.scope");
    assert_eq!(e.permission.as_deref(), Some("pull_requests=write"));
    let text = e.value().to_string();
    assert!(!text.contains(TOKEN), "{text}");
    assert!(text.contains("Resource not accessible"));
    assert_eq!(pulls::merge_pr(&r, 7, MergeMethod::Merge, "abc1234").unwrap_err().code, "gh.conflict");
    let inv = pulls::delete_branch(&r, "x").unwrap_err();
    assert_eq!(inv.code, "gh.invalid");
    assert!(!inv.value().to_string().contains(TOKEN));
}

#[test]
fn ci_status_merges_check_runs_and_statuses() {
    let f = fake(Box::new(|_, p| {
        if p.contains("/check-runs") {
            (200, vec![], j(json!({"total_count": 2, "check_runs": [
                {"id": 11, "name": "build", "status": "completed", "conclusion": "failure", "html_url": "https://github.com/o/r/runs/11", "app": {"slug": "github-actions"}, "started_at": "2026-11-23T01:00:00Z", "completed_at": "2026-11-23T01:02:00Z"},
                {"id": 12, "name": "test", "status": "completed", "conclusion": "success", "app": {"slug": "github-actions"}}
            ]})))
        } else {
            (200, vec![], j(json!({"state": "success", "statuses": [{"context": "vercel", "state": "success", "target_url": "https://v/x"}]})))
        }
    }));
    let out = ci::ci_status(&repo(&f), "abc1234").unwrap();
    assert_eq!(out["state"], "failure");
    assert_eq!(out["runs"].as_array().unwrap().len(), 3);
    assert_eq!(out["runs"][0]["jobId"], 11);
    assert_eq!(out["runs"][0]["startedAt"], "2026-11-23T01:00:00Z");
    assert_eq!(out["runs"][2]["name"], "vercel");
}

#[test]
fn job_log_follows_the_redirect_keeps_two_mb_and_masks() {
    let big = {
        let mut s = String::new();
        let line = format!("{}\n", "x".repeat(99));
        while s.len() < 3 * 1024 * 1024 {
            s.push_str(&line);
        }
        s.push_str(&format!("token {TOKEN}\nkey sk-ant-api03-abcdefghijklmnopqrstuvwx\nError: boom\n"));
        s
    };
    let body = Arc::new(big.into_bytes());
    let b = body.clone();
    let f = fake(Box::new(move |_, p| match p {
        "/repos/o/r/actions/jobs/11/logs" => (302, vec![("Location".into(), "/blob/log.txt".into())], vec![]),
        "/blob/log.txt" => (200, vec![("Content-Type".into(), "text/plain".into())], b.as_ref().clone()),
        _ => (404, vec![], vec![]),
    }));
    let out = ci::job_log_tail(&repo(&f), 11, 60).unwrap();
    let lines: Vec<String> = out["lines"].as_array().unwrap().iter().map(|l| l.as_str().unwrap().to_string()).collect();
    assert_eq!(lines.len(), 60);
    assert_eq!(lines.last().unwrap(), "Error: boom");
    assert_eq!(out["cut"], true);
    let all = lines.join("\n");
    assert!(!all.contains(TOKEN));
    assert!(!all.contains("sk-ant-api03-abcdefghijklmnopqrstuvwx"));
    let seen = f.seen.lock().unwrap().clone();
    let blob = seen.iter().find(|s| s.path == "/blob/log.txt").unwrap();
    assert!(blob.auth.is_empty(), "token must not follow the redirect");
    assert_eq!(ci::LOG_KEEP, 2 * 1024 * 1024);
}

#[test]
fn phone_and_summary_text_are_short_and_secret_free() {
    let ev = json!({"type": "ci.done", "root": "/w/app", "branch": "feat/x", "sha": "abc1234", "pr": 4, "state": "failure", "failed": ["build", "test"], "url": "https://github.com/o/r/pull/4"});
    let m = super::phone_message(&ev, true);
    assert_eq!(m["reason"], "ci");
    assert_eq!(m["name"], "app");
    assert_eq!(m["text"], "CI failed on feat/x: build, test");
    let long = json!({"branch": "b", "state": "failure", "failed": vec!["n".repeat(80); 5]});
    assert!(crate::js::len(&super::ci_summary(&long, false)) <= 120);
    assert!(!m.to_string().contains(TOKEN));
}

#[test]
fn channels_refuse_without_a_granted_repo() {
    let dir = std::env::temp_dir().join(format!("ccm-gh-ipc-{}", crate::util::uuid()));
    std::fs::create_dir_all(&dir).unwrap();
    let desk = crate::Desk::standard(&crate::DeskEnv { data_dir: dir.clone(), ..crate::DeskEnv::default() });
    let out = desk.dispatch("ccm:gh-pr", json!([{"op": "find", "root": "/x", "head": "a"}])).unwrap();
    assert_eq!(out["ok"], false);
    assert_eq!(out["error"], "denied");
    let ci = desk.dispatch("ccm:gh-ci", json!([{"op": "status", "root": "/x", "sha": "abc1234"}])).unwrap();
    assert_eq!(ci["error"], "denied");
    let list = desk.dispatch("ccm:gh-ci", json!([{"op": "list"}])).unwrap();
    assert_eq!(list["ok"], true);
    assert_eq!(list["watches"], json!([]));
    assert_eq!(desk.dispatch("ccm:gh-pr", json!([{"op": "nope"}])).unwrap()["error"], "invalid");
    assert_eq!(desk.dispatch("desk:gh-ci-push", json!([{"root": "/x", "branch": "a", "ref": "refs/heads/a"}])).unwrap(), json!(false));
    let _ = std::fs::remove_dir_all(dir);
}
