use std::collections::{HashMap, HashSet};

use core_transcript::time::parse_iso;

use super::process::{CodexProcess, ProcessMode};
use super::state::CodexMode;
use crate::monitor::ClaudeTarget;

#[derive(Debug, Clone)]
pub struct ThreadKey {
    pub id: String,
    pub created_at: i64,
    pub mode: CodexMode,
    pub parent_thread_id: Option<String>,
    pub pid: Option<u32>,
    pub start: Option<i64>,
}

const BEFORE_START_MS: i64 = 2_000;
const AFTER_START_MS: i64 = 120_000;
const CALLER_LOOKBACK_MS: i64 = 10 * 60_000;
const CALLER_SLACK_MS: i64 = 5_000;

fn process_mode(mode: CodexMode) -> Option<ProcessMode> {
    match mode {
        CodexMode::Exec => Some(ProcessMode::Exec),
        CodexMode::Tui => Some(ProcessMode::Tui),
        _ => None,
    }
}

pub fn match_processes(threads: &[ThreadKey], procs: &[CodexProcess]) -> HashMap<String, CodexProcess> {
    let by_pid: HashMap<u32, &CodexProcess> = procs.iter().map(|p| (p.pid, p)).collect();
    let mut result: HashMap<String, CodexProcess> = HashMap::new();
    let mut taken: HashSet<u32> = HashSet::new();
    let mut open: Vec<&ThreadKey> = Vec::new();
    for t in threads {
        let kept = t.pid.and_then(|p| by_pid.get(&p).copied());
        if let Some(k) = kept.filter(|k| Some(k.start) == t.start) {
            result.insert(t.id.clone(), k.clone());
            if t.mode != CodexMode::Subagent {
                taken.insert(k.pid);
            }
            continue;
        }
        if let Some(r) = procs.iter().find(|p| p.resume_id.as_deref() == Some(t.id.as_str())) {
            result.insert(t.id.clone(), r.clone());
            taken.insert(r.pid);
            continue;
        }
        open.push(t);
    }

    let mut pairs: Vec<(&ThreadKey, &CodexProcess, i64)> = Vec::new();
    for t in &open {
        let Some(want) = process_mode(t.mode) else { continue };
        for p in procs {
            let gap = t.created_at - p.start;
            if p.mode == want && (-BEFORE_START_MS..=AFTER_START_MS).contains(&gap) {
                pairs.push((t, p, gap.abs()));
            }
        }
    }
    pairs.sort_by_key(|x| x.2);
    for (t, p, _) in pairs {
        if result.contains_key(&t.id) || taken.contains(&p.pid) {
            continue;
        }
        result.insert(t.id.clone(), p.clone());
        taken.insert(p.pid);
    }

    for t in &open {
        if result.contains_key(&t.id) || t.mode != CodexMode::Tui {
            continue;
        }
        let candidates: Vec<&CodexProcess> = procs.iter().filter(|p| p.mode == ProcessMode::Tui && p.start <= t.created_at).collect();
        if let [only] = candidates.as_slice() {
            result.insert(t.id.clone(), (*only).clone());
        }
    }

    for t in threads {
        let Some(first) = t.parent_thread_id.as_deref().filter(|p| !p.is_empty()) else { continue };
        if t.mode != CodexMode::Subagent || result.contains_key(&t.id) {
            continue;
        }
        let mut parent = first.to_owned();
        let mut depth = 0;
        while depth < 8 && !parent.is_empty() {
            if let Some(owner) = result.get(&parent).cloned() {
                result.insert(t.id.clone(), owner);
                break;
            }
            parent = threads.iter().find(|x| x.id == parent).and_then(|x| x.parent_thread_id.clone()).unwrap_or_default();
            depth += 1;
        }
    }
    result
}

pub fn normalize_dir(dir: Option<&str>, win: bool) -> Option<String> {
    let dir = dir.filter(|d| !d.is_empty())?;
    let mut d = dir.strip_prefix("\\\\?\\").unwrap_or(dir).to_owned();
    if win {
        d = d.replace('/', "\\").to_lowercase();
    }
    let trimmed = d.trim_end_matches(['\\', '/']);
    Some(if trimmed.is_empty() { d.clone() } else { trimmed.to_owned() })
}

pub struct ThreadFacts<'a> {
    pub mode: CodexMode,
    pub cwd: Option<&'a str>,
    pub created_at: i64,
}

fn latest_call(target: &ClaudeTarget, created_at: i64) -> Option<i64> {
    let mut best: Option<i64> = None;
    for (_, at) in &target.codex_calls {
        let Some(at) = parse_iso(at) else { continue };
        if at > created_at + CALLER_SLACK_MS || at < created_at - CALLER_LOOKBACK_MS {
            continue;
        }
        if best.is_none_or(|b| at > b) {
            best = Some(at);
        }
    }
    best
}

pub fn claude_target_for<'a>(proc: Option<&CodexProcess>, thread: &ThreadFacts, targets: &'a [ClaudeTarget], win: bool) -> Option<&'a ClaudeTarget> {
    let live: Vec<&ClaudeTarget> = targets.iter().filter(|t| !t.ended).collect();
    if let Some(p) = proc {
        for pid in &p.ancestors {
            if let Some(hit) = live.iter().find(|t| t.pid == *pid) {
                return Some(hit);
            }
        }
    }
    if thread.mode == CodexMode::Plugin {
        let dir = normalize_dir(thread.cwd, win)?;
        let mut same: Vec<&ClaudeTarget> = live.iter().copied().filter(|t| normalize_dir(t.cwd.as_deref(), win).as_deref() == Some(dir.as_str())).collect();
        same.sort_by(|a, b| core_transcript::jsstr::utf16_cmp(b.last_activity_at.as_deref().unwrap_or(""), a.last_activity_at.as_deref().unwrap_or("")));
        return same.first().copied();
    }
    if proc.is_some_and(|p| !p.orphan) {
        return None;
    }
    let mut best: Option<(&ClaudeTarget, i64)> = None;
    for t in live {
        if let Some(at) = latest_call(t, thread.created_at) {
            if best.is_none_or(|(_, b)| at > b) {
                best = Some((t, at));
            }
        }
    }
    best.map(|(t, _)| t)
}

pub fn caller_of(target: &ClaudeTarget, created_at: i64) -> String {
    let mut best: Option<(&str, i64)> = None;
    for (agent, at) in &target.codex_calls {
        let Some(at) = parse_iso(at) else { continue };
        if at > created_at + CALLER_SLACK_MS || at < created_at - CALLER_LOOKBACK_MS {
            continue;
        }
        if best.is_none_or(|(_, b)| at > b) {
            best = Some((agent, at));
        }
    }
    best.map_or_else(|| target.main_agent_id.clone(), |(a, _)| a.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_791_702_000_000;

    fn proc(pid: u32, start: i64, mode: ProcessMode) -> CodexProcess {
        CodexProcess { pid, start, mode, resume_id: None, ancestors: Vec::new(), orphan: false }
    }

    fn thread(id: &str, created_at: i64, mode: CodexMode) -> ThreadKey {
        ThreadKey { id: id.into(), created_at, mode, parent_thread_id: None, pid: None, start: None }
    }

    fn iso(ms: i64) -> String {
        core_store::time::iso_of_ms(ms)
    }

    fn target(id: &str, pid: u32, cwd: &str, calls: Vec<(&str, i64)>) -> ClaudeTarget {
        ClaudeTarget {
            terminal_id: id.into(),
            pid,
            cwd: Some(cwd.into()),
            main_agent_id: format!("{id}:main"),
            ended: false,
            last_activity_at: None,
            codex_calls: calls.into_iter().map(|(a, at)| (a.to_owned(), iso(at))).collect(),
        }
    }

    #[test]
    fn pairs_threads_with_the_process_that_started_just_before_them() {
        let mut sub = thread("s", T0 + 5_000, CodexMode::Subagent);
        sub.parent_thread_id = Some("a".into());
        let mut resumed = proc(3, T0 + 2_000, ProcessMode::Tui);
        resumed.resume_id = Some("r".into());
        let m = match_processes(
            &[thread("a", T0 + 300, CodexMode::Exec), thread("b", T0 + 1_200, CodexMode::Exec), thread("r", T0 - 99_000_000, CodexMode::Tui), thread("p", T0, CodexMode::Plugin), sub],
            &[proc(1, T0 + 1_000, ProcessMode::Exec), proc(2, T0, ProcessMode::Exec), resumed, proc(4, T0 - 10_000, ProcessMode::AppServer)],
        );
        assert_eq!(m["a"].pid, 2);
        assert_eq!(m["b"].pid, 1);
        assert_eq!(m["r"].pid, 3);
        assert!(!m.contains_key("p"));
        assert_eq!(m["s"].pid, 2);
    }

    #[test]
    fn gives_a_late_tui_thread_to_the_only_tui_process() {
        let mut old = thread("old", T0 + 100, CodexMode::Tui);
        old.pid = Some(5);
        old.start = Some(T0);
        let m = match_processes(&[old, thread("new", T0 + 3_600_000, CodexMode::Tui)], &[proc(5, T0, ProcessMode::Tui)]);
        assert_eq!(m["old"].pid, 5);
        assert_eq!(m["new"].pid, 5);
    }

    #[test]
    fn links_by_ancestry_by_the_latest_codex_call_and_by_cwd_for_plugins() {
        let targets = vec![target("c1", 100, "D:\\work\\demo", vec![("sub-a", T0 - 2_000)]), target("c2", 200, "D:\\work\\other", vec![("c2:main", T0 - 30_000)])];
        let exec = ThreadFacts { mode: CodexMode::Exec, cwd: Some("C:\\Temp\\x"), created_at: T0 };
        let with = |ancestors: Vec<u32>, orphan: bool| CodexProcess { ancestors, orphan, ..proc(9, T0, ProcessMode::Exec) };
        assert_eq!(claude_target_for(Some(&with(vec![50, 200], false)), &exec, &targets, true).map(|t| t.terminal_id.as_str()), Some("c2"));
        assert_eq!(claude_target_for(Some(&with(vec![50], true)), &exec, &targets, true).map(|t| t.terminal_id.as_str()), Some("c1"));
        assert_eq!(claude_target_for(None, &exec, &targets, true).map(|t| t.terminal_id.as_str()), Some("c1"));
        assert!(claude_target_for(Some(&with(vec![50], false)), &exec, &targets, true).is_none());
        let plugin = ThreadFacts { mode: CodexMode::Plugin, cwd: Some("d:/work/other/"), created_at: T0 };
        assert_eq!(claude_target_for(None, &plugin, &targets, true).map(|t| t.terminal_id.as_str()), Some("c2"));
        assert_eq!(caller_of(&targets[0], T0), "sub-a");
        assert_eq!(caller_of(&targets[0], T0 + 3_600_000), "c1:main");
        assert_eq!(normalize_dir(Some("\\\\?\\C:\\Temp\\X\\"), true).as_deref(), Some("c:\\temp\\x"));
    }
}
