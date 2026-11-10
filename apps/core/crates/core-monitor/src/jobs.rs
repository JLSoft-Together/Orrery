use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use core_store::extra::{BackgroundJob, JobState};
use core_store::time::iso_of_ms;
use core_transcript::time::parse_iso;
use serde_json::{Map, Value};

const MAX_STATE_BYTES: u64 = 256 * 1024;
const TAIL_BYTES: u64 = 4096;
const MAX_JOBS: usize = 50;
const DONE_KEEP_MS: f64 = 24.0 * 3_600_000.0;

pub fn is_job_id(id: &str) -> bool {
    (4..=64).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

pub fn is_uuid(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 36
        && b.iter().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => *c == b'-',
            _ => c.is_ascii_hexdigit(),
        })
}

fn short(v: Option<&Value>, max: usize) -> Option<String> {
    let s = v?.as_str().filter(|s| !s.is_empty())?;
    Some(core_transcript::jsstr::js_slice(s, max).to_owned())
}

fn count(v: Option<&Value>) -> Option<u64> {
    v?.as_f64().filter(|n| n.is_finite() && *n >= 0.0).map(|n| n as u64)
}

fn job_state(v: Option<&Value>) -> JobState {
    match v.and_then(Value::as_str) {
        Some("working") => JobState::Working,
        Some("blocked") => JobState::Blocked,
        Some("done") => JobState::Done,
        _ => JobState::Unknown,
    }
}

pub fn parse_job_state(id: &str, text: &str) -> Option<BackgroundJob> {
    let Ok(Value::Object(raw)) = serde_json::from_str::<Value>(text) else { return None };
    let empty = Map::new();
    let in_flight = raw.get("inFlight").and_then(Value::as_object).unwrap_or(&empty);
    Some(BackgroundJob {
        id: id.to_owned(),
        name: short(raw.get("name"), 80),
        state: job_state(raw.get("state")),
        tempo: short(raw.get("tempo"), 20),
        tasks: count(in_flight.get("tasks")),
        queued: count(in_flight.get("queued")),
        tokens: count(raw.get("tokens")),
        state_since: None,
        updated_at: None,
        session_id: raw.get("sessionId").and_then(Value::as_str).filter(|s| is_uuid(s)).map(str::to_owned),
    })
}

pub fn last_state_change(tail: &str, state: JobState) -> Option<String> {
    let mut since = None;
    for line in tail.split('\n').filter(|l| !l.is_empty()).rev() {
        let Ok(Value::Object(row)) = serde_json::from_str::<Value>(line) else { continue };
        let Some(at) = row.get("at").and_then(Value::as_str) else { continue };
        if row.get("state").and_then(Value::as_str) != Some(state.as_str()) {
            break;
        }
        since = Some(at.to_owned());
    }
    since
}

fn read_tail(file: &Path) -> std::io::Result<String> {
    let mut f = std::fs::File::open(file)?;
    let size = f.metadata()?.len();
    let start = size.saturating_sub(TAIL_BYTES);
    f.seek(SeekFrom::Start(start))?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    let text = String::from_utf8_lossy(&buf).into_owned();
    if start == 0 {
        return Ok(text);
    }
    Ok(match text.find('\n') {
        Some(i) => text[i + 1..].to_owned(),
        None => text,
    })
}

fn mtime_ms(meta: &std::fs::Metadata) -> f64 {
    meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map_or(0.0, |d| d.as_secs_f64() * 1000.0)
}

struct Cached {
    mtime_ms: f64,
    job: BackgroundJob,
}

pub struct JobReader {
    jobs_dir: PathBuf,
    cache: HashMap<String, Cached>,
}

impl JobReader {
    pub fn new(jobs_dir: PathBuf) -> Self {
        JobReader { jobs_dir, cache: HashMap::new() }
    }

    pub fn read(&mut self, now: i64) -> Vec<BackgroundJob> {
        let Ok(entries) = std::fs::read_dir(&self.jobs_dir) else {
            self.cache.clear();
            return Vec::new();
        };
        let mut seen = std::collections::HashSet::new();
        let mut jobs = Vec::new();
        for e in entries.flatten() {
            let Ok(id) = e.file_name().into_string() else { continue };
            if !is_job_id(&id) {
                continue;
            }
            let Some(job) = self.read_one(&id) else { continue };
            seen.insert(id);
            let updated = job.updated_at.as_deref().and_then(parse_iso);
            if job.state == JobState::Done && updated.is_some_and(|u| now as f64 - u as f64 > DONE_KEEP_MS) {
                continue;
            }
            jobs.push(job);
        }
        self.cache.retain(|id, _| seen.contains(id));
        jobs.sort_by(|a, b| b.updated_at.as_deref().unwrap_or("").cmp(a.updated_at.as_deref().unwrap_or("")));
        jobs.truncate(MAX_JOBS);
        jobs
    }

    fn read_one(&mut self, id: &str) -> Option<BackgroundJob> {
        let dir = self.jobs_dir.join(id);
        let state_file = dir.join("state.json");
        let meta = std::fs::metadata(&state_file).ok()?;
        let mtime = mtime_ms(&meta);
        let cached = self.cache.get(id);
        if let Some(c) = cached.filter(|c| c.mtime_ms == mtime) {
            return Some(c.job.clone());
        }
        if meta.len() > MAX_STATE_BYTES {
            return None;
        }
        let text = std::fs::read(&state_file).ok()?;
        let mut job = parse_job_state(id, &String::from_utf8_lossy(&text))?;
        job.updated_at = Some(iso_of_ms(mtime.trunc() as i64));
        job.state_since = read_tail(&dir.join("timeline.jsonl")).ok().and_then(|t| last_state_change(&t, job.state));
        if job.state_since.is_none() {
            if let Some(c) = cached.filter(|c| c.job.state == job.state) {
                job.state_since = c.job.state_since.clone();
            }
        }
        self.cache.insert(id.to_owned(), Cached { mtime_ms: mtime, job: job.clone() });
        Some(job)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_whitelisted_state_fields() {
        let job = parse_job_state("job-1", r#"{"name":"Nightly","state":"working","tempo":"steady","inFlight":{"tasks":2,"queued":1},"tokens":1234,"intent":"secret","detail":"x","sessionId":"11111111-2222-3333-4444-555555555555"}"#).unwrap();
        let v = serde_json::to_value(&job).unwrap();
        assert_eq!(v, serde_json::json!({"id":"job-1","name":"Nightly","state":"working","tempo":"steady","tasks":2,"queued":1,"tokens":1234,"sessionId":"11111111-2222-3333-4444-555555555555"}));
        assert_eq!(parse_job_state("j", r#"{"state":"odd","sessionId":"x"}"#).unwrap().state, JobState::Unknown);
        assert!(parse_job_state("j", "[]").is_none());
    }

    #[test]
    fn finds_when_the_state_started_from_the_tail() {
        let tail = "{\"at\":\"1\",\"state\":\"working\"}\n{\"at\":\"2\",\"state\":\"blocked\"}\nbad\n{\"at\":\"3\",\"state\":\"blocked\"}\n";
        assert_eq!(last_state_change(tail, JobState::Blocked).as_deref(), Some("2"));
        assert_eq!(last_state_change(tail, JobState::Done), None);
    }

    #[test]
    fn reads_job_dirs_and_drops_old_finished_jobs() {
        let dir = std::env::temp_dir().join(format!("ccm-jobs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for (id, state) in [("job-a", "working"), ("job-b", "done"), ("x", "working")] {
            std::fs::create_dir_all(dir.join(id)).unwrap();
            std::fs::write(dir.join(id).join("state.json"), format!(r#"{{"state":"{state}"}}"#)).unwrap();
        }
        std::fs::write(dir.join("job-a").join("timeline.jsonl"), "{\"at\":\"2026-01-01T00:00:00.000Z\",\"state\":\"working\"}\n").unwrap();
        let mut r = JobReader::new(dir.clone());
        let now = core_store::time::now_ms();
        let ids: Vec<String> = r.read(now).into_iter().map(|j| j.id).collect();
        assert_eq!(ids.len(), 2);
        let later: Vec<String> = r.read(now + 25 * 3_600_000).into_iter().map(|j| j.id).collect();
        assert_eq!(later, ["job-a"]);
        let a = r.read(now).into_iter().find(|j| j.id == "job-a").unwrap();
        assert_eq!(a.state_since.as_deref(), Some("2026-01-01T00:00:00.000Z"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
