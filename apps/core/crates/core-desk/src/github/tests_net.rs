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

#[test]
fn branch_names_follow_check_ref_format_and_reject_invisible_chars() {
    for good in ["main", "feat/x", "feat/a-b_c.d", "release/1.2.3", "é/ü", "a@b"] {
        assert!(pulls::branch_ok(good), "{good}");
    }
    let long = "a".repeat(256);
    for bad in [
        "", "@", "/a", "a/", "a//b", ".a", "a/.b", "a/b.", "a..b", "a.lock", "x/a.lock/y", "a@{b", "a b", "a~b", "a^b", "a:b", "a?b", "a*b", "a[b", "a\\b", "a\tb", "a\u{7f}b", "-a", "a/-b",
        "a\u{200b}b", "a\u{202e}b", "a\u{feff}b", "a\u{2066}b", "a\u{00ad}b", "a\u{2028}b", "a\u{a0}b", "a\u{e0041}b", long.as_str(),
    ] {
        assert!(!pulls::branch_ok(bad), "{bad:?}");
    }
}

#[test]
fn owner_or_repo_dot_segments_never_reach_the_network() {
    let f = fake(Box::new(|_, _| (200, vec![], j(json!({"default_branch": "main"})))));
    for (o, n) in [("..", "r"), ("o", ".."), (".", "r"), ("o", "a/b"), ("o", "r%2F..")] {
        let r = RepoRef { base: &f.base, token: TOKEN, owner: o, repo: n };
        assert_eq!(pulls::default_branch(&r).unwrap_err().code, "invalid", "{o}/{n}");
        assert_eq!(ci::ci_status(&r, "abc1234").unwrap_err().code, "invalid");
        assert_eq!(ci::job_log_tail(&r, 1, 10).unwrap_err().code, "invalid");
    }
    assert!(f.seen.lock().unwrap().is_empty());
}

#[test]
fn pr_facts_come_from_the_real_pull_and_gate_branch_deletion() {
    let pr = |head_repo: &str, state: &str| {
        json!({"number": 5, "state": state, "head": {"ref": "feat/x", "sha": "abc1234", "repo": {"full_name": head_repo}}, "base": {"ref": "main", "repo": {"full_name": "O/r"}}})
    };
    let f = fake(Box::new(move |_, p| match p {
        "/repos/o/r/pulls/5" => (200, vec![], j(pr("o/R", "open"))),
        "/repos/o/r/pulls/6" => (200, vec![], j(json!({"number": 6, "state": "open", "head": {"ref": "x", "repo": {"full_name": "evil/r"}}, "base": {"ref": "main", "repo": {"full_name": "o/r"}}}))),
        "/repos/o/r/pulls/7" => (200, vec![], j(json!({"number": 7, "state": "open", "head": {"ref": "bad..ref", "repo": null}, "base": {"ref": "main", "repo": {"full_name": "o/r"}}}))),
        _ => (404, vec![], j(json!({"message": "Not Found"}))),
    }));
    let r = repo(&f);
    let facts = pulls::pr_facts(&r, 5).unwrap();
    assert_eq!((facts.head.as_str(), facts.base.as_str(), facts.open), ("feat/x", "main", true));
    assert!(facts.same_repo());
    assert!(facts.may_delete_head(true));
    assert!(!facts.may_delete_head(false));
    let fork = pulls::pr_facts(&r, 6).unwrap();
    assert!(!fork.same_repo());
    assert!(!fork.may_delete_head(true));
    assert_eq!(pulls::pr_facts(&r, 7).unwrap_err().code, "gh.http");
    assert_eq!(pulls::pr_facts(&r, 8).unwrap_err().code, "gh.notFound");
    assert_eq!(pulls::pr_facts(&r, 0).unwrap_err().code, "invalid");
}

fn run_item(i: usize, state: (&str, Option<&str>)) -> Value {
    json!({"id": i, "name": format!("job{i}"), "status": state.0, "conclusion": state.1, "app": {"slug": "github-actions"}})
}

#[test]
fn overall_ci_state_covers_every_page_not_just_the_displayed_runs() {
    let f = fake(Box::new(|_, p| {
        if p.contains("/check-runs") {
            let items: Vec<Value> = if p.contains("page=2") {
                (100..150).map(|i| run_item(i, if i == 149 { ("in_progress", None) } else { ("completed", Some("success")) })).collect()
            } else {
                (0..100).map(|i| run_item(i, ("completed", Some("success")))).collect()
            };
            (200, vec![], j(json!({"total_count": 150, "check_runs": items})))
        } else {
            (200, vec![], j(json!({"state": "success", "statuses": []})))
        }
    }));
    let out = ci::ci_status(&repo(&f), "abc1234").unwrap();
    assert_eq!(out["state"], "pending");
    assert_eq!(out["runs"].as_array().unwrap().len(), 100);
    let pages = f.seen.lock().unwrap().iter().filter(|s| s.path.contains("/check-runs")).count();
    assert_eq!(pages, 2);

    let many = fake(Box::new(|_, p| {
        if p.contains("/check-runs") {
            let items: Vec<Value> = (0..100).map(|i| run_item(i, ("completed", Some("success")))).collect();
            (200, vec![], j(json!({"total_count": 9999, "check_runs": items})))
        } else {
            (200, vec![], j(json!({"statuses": []})))
        }
    }));
    ci::ci_status(&repo(&many), "abc1234").unwrap();
    let pages = many.seen.lock().unwrap().iter().filter(|s| s.path.contains("/check-runs")).count();
    assert_eq!(pages, 5);
}

#[test]
fn check_runs_403_still_uses_the_combined_status() {
    let f = fake(Box::new(|_, p| {
        if p.contains("/check-runs") {
            (403, vec![], j(json!({"message": "Resource not accessible by personal access token"})))
        } else {
            (200, vec![], j(json!({"state": "pending", "statuses": [{"context": "ci/x", "state": "pending"}]})))
        }
    }));
    let out = ci::ci_status(&repo(&f), "abc1234").unwrap();
    assert_eq!(out["state"], "pending");
    assert_eq!(out["runs"][0]["name"], "ci/x");

    let both = fake(Box::new(|_, _| (403, vec![], j(json!({"message": "nope"})))));
    assert_eq!(ci::ci_status(&repo(&both), "abc1234").unwrap_err().code, "gh.scope");

    let status_denied = fake(Box::new(|_, p| {
        if p.contains("/check-runs") {
            (200, vec![], j(json!({"total_count": 1, "check_runs": [run_item(1, ("completed", Some("success")))]})))
        } else {
            (403, vec![], j(json!({"message": "nope"})))
        }
    }));
    assert_eq!(ci::ci_status(&repo(&status_denied), "abc1234").unwrap()["state"], "success");
}

#[test]
fn expired_log_blob_is_log_gone_but_api_403_stays_scope() {
    let f = fake(Box::new(|_, p| match p {
        "/repos/o/r/actions/jobs/1/logs" => (302, vec![("Location".into(), "/blob/old.txt".into())], vec![]),
        "/blob/old.txt" => (403, vec![], b"expired".to_vec()),
        "/repos/o/r/actions/jobs/2/logs" => (410, vec![], j(json!({"message": "Gone"}))),
        "/repos/o/r/actions/jobs/3/logs" => (403, vec![], j(json!({"message": "Resource not accessible"}))),
        "/repos/o/r/actions/jobs/4/logs" => (302, vec![("Location".into(), "/blob/gone.txt".into())], vec![]),
        "/blob/gone.txt" => (410, vec![], vec![]),
        _ => (404, vec![], vec![]),
    }));
    let r = repo(&f);
    assert_eq!(ci::job_log_tail(&r, 1, 10).unwrap_err().code, "gh.logGone");
    assert_eq!(ci::job_log_tail(&r, 2, 10).unwrap_err().code, "gh.logGone");
    assert_eq!(ci::job_log_tail(&r, 3, 10).unwrap_err().code, "gh.scope");
    assert_eq!(ci::job_log_tail(&r, 4, 10).unwrap_err().code, "gh.logGone");
}

fn stalling_server(sent: usize, stall_ms: u64) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    std::thread::spawn(move || {
        if let Some(Ok(mut s)) = listener.incoming().next() {
            let _ = read_request(&mut s);
            let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 10000000\r\nConnection: close\r\n\r\n");
            let body: Vec<u8> = b"some log line\n".iter().copied().cycle().take(sent).collect();
            let _ = s.write_all(&body);
            let _ = s.flush();
            std::thread::sleep(std::time::Duration::from_millis(stall_ms));
        }
    });
    base
}

#[test]
fn stalled_log_returns_the_tail_read_so_far_as_cut() {
    let base = stalling_server(5000, 4000);
    let res = super::api::fetch_tail(&base, TOKEN, "/x", std::time::Duration::from_millis(800), 1024, 1 << 20).unwrap();
    assert_eq!(res.tail.len(), 1024);
    assert!(res.cut);

    let silent = stalling_server(0, 4000);
    let err = super::api::fetch_tail(&silent, TOKEN, "/x", std::time::Duration::from_millis(800), 1024, 1 << 20).err();
    assert_eq!(err, Some(super::api::NetErr::Timeout));
}
