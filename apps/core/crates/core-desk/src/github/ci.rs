use super::api::{self, encode_component};
use super::err::{self, GhErr};
use super::pulls::{sha_ok, RepoRef, TIMEOUT};
use serde_json::{json, Value};
use std::time::Duration;

pub const LOG_KEEP: usize = 2 * 1024 * 1024;
const LOG_READ_MAX: u64 = 64 * 1024 * 1024;
const LOG_TIMEOUT: Duration = Duration::from_secs(30);
const LOG_LINES_MAX: usize = 500;
const LINE_MAX: usize = 2000;
const RUNS_MAX: usize = 100;
const CHECK_PAGES_MAX: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum State {
    None,
    Neutral,
    Success,
    Failure,
    Pending,
}

impl State {
    pub fn as_str(self) -> &'static str {
        match self {
            State::None => "none",
            State::Neutral => "neutral",
            State::Success => "success",
            State::Failure => "failure",
            State::Pending => "pending",
        }
    }
}

pub fn check_run_state(status: &str, conclusion: Option<&str>) -> State {
    if status != "completed" {
        return State::Pending;
    }
    match conclusion.unwrap_or("") {
        "success" => State::Success,
        "failure" | "timed_out" | "cancelled" | "action_required" | "startup_failure" => State::Failure,
        _ => State::Neutral,
    }
}

pub fn status_state(state: &str) -> State {
    match state {
        "success" => State::Success,
        "failure" | "error" => State::Failure,
        "pending" => State::Pending,
        _ => State::Neutral,
    }
}

pub fn overall(states: &[State]) -> State {
    if states.is_empty() {
        return State::None;
    }
    if states.contains(&State::Pending) {
        return State::Pending;
    }
    if states.contains(&State::Failure) {
        return State::Failure;
    }
    if states.contains(&State::Success) {
        return State::Success;
    }
    State::Neutral
}

fn text(v: &Value, k: &str) -> Value {
    v.get(k).and_then(Value::as_str).map(|s| json!(s)).unwrap_or(Value::Null)
}

pub fn merge_runs(check_runs: &Value, combined: &Value) -> Value {
    let mut runs: Vec<(State, Value)> = Vec::new();
    for c in check_runs.get("check_runs").and_then(Value::as_array).into_iter().flatten() {
        let st = check_run_state(c.get("status").and_then(Value::as_str).unwrap_or(""), c.get("conclusion").and_then(Value::as_str));
        let actions = c.pointer("/app/slug").and_then(Value::as_str) == Some("github-actions");
        let job = if actions { c.get("id").and_then(Value::as_u64).map(|n| json!(n)).unwrap_or(Value::Null) } else { Value::Null };
        let url = c.get("html_url").filter(|u| u.is_string()).or_else(|| c.get("details_url")).cloned().unwrap_or(Value::Null);
        runs.push((st, json!({"name": text(c, "name"), "state": st.as_str(), "url": url, "jobId": job, "startedAt": text(c, "started_at"), "completedAt": text(c, "completed_at")})));
    }
    for s in combined.get("statuses").and_then(Value::as_array).into_iter().flatten() {
        let st = status_state(s.get("state").and_then(Value::as_str).unwrap_or(""));
        let done = if st == State::Pending { Value::Null } else { text(s, "updated_at") };
        runs.push((st, json!({"name": text(s, "context"), "state": st.as_str(), "url": text(s, "target_url"), "jobId": Value::Null, "startedAt": text(s, "created_at"), "completedAt": done})));
    }
    let states: Vec<State> = runs.iter().map(|r| r.0).collect();
    runs.truncate(RUNS_MAX);
    json!({"state": overall(&states).as_str(), "runs": runs.into_iter().map(|r| r.1).collect::<Vec<_>>()})
}

fn fetch_check_runs(r: &RepoRef, s: &str) -> Result<Value, GhErr> {
    let mut all: Vec<Value> = Vec::new();
    let mut total = 0usize;
    for page in 1..=CHECK_PAGES_MAX {
        let res = api::fetch(r.base, r.token, &r.route(&format!("/commits/{s}/check-runs?per_page=100&page={page}")), TIMEOUT)?;
        let body = err::ok_or(res, r.token, "Checks (read)")?.body;
        total = body.get("total_count").and_then(Value::as_u64).map(|n| n as usize).unwrap_or(0);
        let items = body.get("check_runs").and_then(Value::as_array).cloned().unwrap_or_default();
        let n = items.len();
        all.extend(items);
        if n == 0 || all.len() >= total {
            break;
        }
    }
    Ok(json!({"total_count": total, "check_runs": all}))
}

pub fn ci_status(r: &RepoRef, sha: &str) -> Result<Value, GhErr> {
    if !sha_ok(sha) || !r.valid() {
        return Err(GhErr::new("invalid"));
    }
    let s = encode_component(sha);
    let checks = match fetch_check_runs(r, &s) {
        Ok(v) => Ok(v),
        Err(e) if e.code == "gh.scope" => Err(e),
        Err(e) => return Err(e),
    };
    let status = api::fetch(r.base, r.token, &r.route(&format!("/commits/{s}/status?per_page=100")), TIMEOUT)
        .map_err(GhErr::from)
        .and_then(|res| err::ok_or(res, r.token, "Commit statuses (read)"));
    let (checks, status) = match (checks, status) {
        (Ok(c), Ok(st)) => (c, st.body),
        (Ok(c), Err(e)) if matches!(e.code, "gh.notFound" | "gh.scope") => (c, Value::Null),
        (Ok(_), Err(e)) => return Err(e),
        (Err(_), Ok(st)) => (Value::Null, st.body),
        (Err(e), Err(_)) => return Err(e),
    };
    let mut out = merge_runs(&checks, &status);
    out["sha"] = json!(sha);
    Ok(out)
}

pub fn tail_lines(bytes: &[u8], lines: usize, token: &str) -> Vec<String> {
    let text = String::from_utf8_lossy(bytes);
    let all: Vec<&str> = text.split('\n').map(|l| l.trim_end_matches('\r')).collect();
    let mut end = all.len();
    while end > 0 && all[end - 1].trim().is_empty() {
        end -= 1;
    }
    let start = end.saturating_sub(lines);
    all[start..end].iter().map(|l| crate::js::slice(&err::clean_line(l, token), LINE_MAX)).collect()
}

pub fn job_log_tail(r: &RepoRef, job_id: u64, lines: usize) -> Result<Value, GhErr> {
    if job_id == 0 || !r.valid() {
        return Err(GhErr::new("invalid"));
    }
    let lines = lines.clamp(1, LOG_LINES_MAX);
    let res = api::fetch_tail(r.base, r.token, &r.route(&format!("/actions/jobs/{job_id}/logs")), LOG_TIMEOUT, LOG_KEEP, LOG_READ_MAX)?;
    if res.status == 410 || (res.redirected && res.status == 403) {
        return Err(GhErr::new("gh.logGone"));
    }
    if !(200..300).contains(&res.status) {
        let fake = api::ApiRes { status: res.status, body: Value::Null, scopes: None, accepted: None };
        return Err(err::from_status(&fake, r.token, "Actions (read)"));
    }
    Ok(json!({"lines": tail_lines(&res.tail, lines, r.token), "cut": res.cut}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn five_states_from_check_runs_and_statuses() {
        assert_eq!(merge_runs(&json!({"check_runs": []}), &json!({"statuses": []}))["state"], "none");
        let ok = merge_runs(&json!({"check_runs": [{"name": "build", "status": "completed", "conclusion": "success", "id": 9, "app": {"slug": "github-actions"}}]}), &Value::Null);
        assert_eq!(ok["state"], "success");
        assert_eq!(ok["runs"][0]["jobId"], 9);
        let neutral = merge_runs(&json!({"check_runs": [{"name": "lint", "status": "completed", "conclusion": "skipped", "id": 3, "app": {"slug": "other"}}]}), &Value::Null);
        assert_eq!(neutral["state"], "neutral");
        assert_eq!(neutral["runs"][0]["jobId"], Value::Null);
        let fail = merge_runs(&json!({"check_runs": [{"name": "a", "status": "completed", "conclusion": "success"}]}), &json!({"statuses": [{"context": "ci/x", "state": "error", "target_url": "https://ci/x"}]}));
        assert_eq!(fail["state"], "failure");
        assert_eq!(fail["runs"][1]["name"], "ci/x");
        assert_eq!(fail["runs"][1]["state"], "failure");
        let pending = merge_runs(&json!({"check_runs": [{"name": "a", "status": "completed", "conclusion": "failure"}, {"name": "b", "status": "in_progress"}]}), &json!({"statuses": []}));
        assert_eq!(pending["state"], "pending");
        let st_pending = merge_runs(&Value::Null, &json!({"statuses": [{"context": "c", "state": "pending"}]}));
        assert_eq!(st_pending["state"], "pending");
        assert_eq!(check_run_state("completed", Some("cancelled")), State::Failure);
        assert_eq!(check_run_state("queued", None), State::Pending);
    }

    #[test]
    fn log_tail_masks_secrets_and_keeps_last_lines() {
        let tok = "ghs_ABCDEFGHIJKLMNOPQRSTUVWXYZ012345";
        let log = format!("one\ntwo\nkey sk-ant-api03-abcdefghijklmnopqrstuvwx\nuses {tok}\nlast\n\n");
        let out = tail_lines(log.as_bytes(), 3, tok);
        assert_eq!(out.len(), 3);
        assert_eq!(out[2], "last");
        assert!(!out.join("\n").contains("sk-ant-api03-abcdefghijklmnopqrstuvwx"));
        assert!(!out.join("\n").contains(tok));
    }
}
