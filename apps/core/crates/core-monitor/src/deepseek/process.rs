use std::collections::{HashMap, HashSet};

use crate::cliprocs::{list_named, list_named_in, split_args, top_level, CliProcess, Rooted};
use crate::codex::link::normalize_dir;

#[derive(Debug, Clone, PartialEq)]
pub struct DeepseekProcess {
    pub pid: u32,
    pub start: i64,
    pub ancestors: Vec<u32>,
    pub workspace: Option<String>,
    pub resume_id: Option<String>,
}

impl Rooted for DeepseekProcess {
    fn pid(&self) -> u32 {
        self.pid
    }
    fn ancestors(&self) -> &[u32] {
        &self.ancestors
    }
}

const WIN_EXE: [&str; 3] = ["codewhale.exe", "codewhale-windows-x64.exe", "codew.exe"];
const UNIX_NAMES: [&str; 4] = ["codewhale", "codew", "codewhale-macos", "codewhale-darwi"];
const VALUE_FLAGS: [&str; 10] = ["-C", "--workspace", "-r", "--resume", "--session-id", "-m", "--model", "-p", "--provider", "--profile"];
const VALUE_FLAGS_TAIL: [&str; 2] = ["-c", "--config"];
const NON_SESSION: [&str; 14] = ["auth", "config", "mcp", "doctor", "login", "logout", "help", "completion", "completions", "update", "upgrade", "version", "models", "telemetry"];
const BEFORE_START_MS: i64 = 2_000;
const AFTER_START_MS: i64 = 120_000;

fn is_value_flag(a: &str) -> bool {
    VALUE_FLAGS.contains(&a) || VALUE_FLAGS_TAIL.contains(&a) || a == "--output-format"
}

fn is_absolute(p: &str) -> bool {
    let b = p.as_bytes();
    p.starts_with('/') || p.starts_with('\\') || (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && matches!(b[2], b'/' | b'\\'))
}

fn is_resume_id(s: &str) -> bool {
    (1..=128).contains(&s.len()) && s.bytes().all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b':' | b'-'))
}

pub fn parse_codewhale_args(cmd: &str) -> Option<(Option<String>, Option<String>)> {
    let all = split_args(cmd);
    let args = all.get(1..).unwrap_or(&[]);
    let mut workspace = None;
    let mut resume = None;
    let mut first: Option<&str> = None;
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        i += 1;
        if a == "--" {
            break;
        }
        if a.starts_with('-') {
            let eq = a.find('=').filter(|e| *e > 0);
            let flag = eq.map_or(a, |e| &a[..e]);
            let value: Option<&str> = match eq {
                Some(e) => Some(&a[e + 1..]),
                None if is_value_flag(a) => {
                    let v = args.get(i).map(String::as_str);
                    i += 1;
                    v
                }
                None => None,
            };
            let value = value.filter(|v| !v.is_empty());
            if matches!(flag, "-C" | "--workspace") {
                if let Some(v) = value.filter(|v| is_absolute(v)) {
                    workspace = Some(v.to_owned());
                }
            }
            if matches!(flag, "-r" | "--resume" | "--session-id") {
                if let Some(v) = value.filter(|v| is_resume_id(v)) {
                    resume = Some(v.to_owned());
                }
            }
            continue;
        }
        first.get_or_insert(a);
    }
    if first.is_some_and(|f| NON_SESSION.contains(&f)) {
        return None;
    }
    Some((workspace, resume))
}

pub fn deepseek_processes(rows: Vec<CliProcess>) -> Vec<DeepseekProcess> {
    top_level(rows)
        .into_iter()
        .filter_map(|p| parse_codewhale_args(&p.cmd).map(|(workspace, resume_id)| DeepseekProcess { pid: p.pid, start: p.start, ancestors: p.ancestors, workspace, resume_id }))
        .collect()
}

pub fn list_deepseek_processes() -> Option<Vec<DeepseekProcess>> {
    list_named(&WIN_EXE, &UNIX_NAMES).map(deepseek_processes)
}

pub fn list_deepseek_processes_in(table: &[core_procs::Proc]) -> Option<Vec<DeepseekProcess>> {
    list_named_in(table, &WIN_EXE, &UNIX_NAMES).map(deepseek_processes)
}

#[derive(Debug, Clone)]
pub struct DeepseekThreadKey {
    pub id: String,
    pub workspace: Option<String>,
    pub started_at: i64,
    pub pid: Option<u32>,
    pub start: Option<i64>,
}

struct Matcher<'a> {
    procs: &'a [DeepseekProcess],
    result: HashMap<String, usize>,
    taken: HashSet<u32>,
    win: bool,
}

impl Matcher<'_> {
    fn claim(&mut self, id: &str, idx: usize) {
        self.result.insert(id.to_owned(), idx);
        self.taken.insert(self.procs[idx].pid);
    }

    fn same_dir(&self, t: &DeepseekThreadKey, idx: usize) -> bool {
        let p = &self.procs[idx];
        match (p.workspace.as_deref().filter(|w| !w.is_empty()), t.workspace.as_deref().filter(|w| !w.is_empty())) {
            (Some(a), Some(b)) => normalize_dir(Some(a), self.win) == normalize_dir(Some(b), self.win),
            _ => false,
        }
    }

    fn greedy(&mut self, open: &[&DeepseekThreadKey], fits: impl Fn(&Self, &DeepseekThreadKey, usize, i64) -> bool) {
        let mut pairs: Vec<(&DeepseekThreadKey, usize, i64)> = Vec::new();
        for t in open {
            if self.result.contains_key(&t.id) {
                continue;
            }
            for (idx, p) in self.procs.iter().enumerate() {
                let gap = t.started_at - p.start;
                if !self.taken.contains(&p.pid) && fits(self, t, idx, gap) {
                    pairs.push((t, idx, gap.abs()));
                }
            }
        }
        pairs.sort_by_key(|x| x.2);
        for (t, idx, _) in pairs {
            if !self.result.contains_key(&t.id) && !self.taken.contains(&self.procs[idx].pid) {
                self.claim(&t.id, idx);
            }
        }
    }
}

pub fn match_deepseek_processes(threads: &[DeepseekThreadKey], procs: &[DeepseekProcess], win: bool) -> HashMap<String, DeepseekProcess> {
    let by_pid: HashMap<u32, usize> = procs.iter().enumerate().map(|(i, p)| (p.pid, i)).collect();
    let mut m = Matcher { procs, result: HashMap::new(), taken: HashSet::new(), win };
    let mut open: Vec<&DeepseekThreadKey> = Vec::new();
    for t in threads {
        let kept = t.pid.and_then(|p| by_pid.get(&p).copied());
        match kept {
            Some(idx) if Some(procs[idx].start) == t.start && !m.taken.contains(&procs[idx].pid) => m.claim(&t.id, idx),
            _ => open.push(t),
        }
    }
    let snapshot = open.clone();
    for t in snapshot {
        let found = procs.iter().position(|x| !m.taken.contains(&x.pid) && x.resume_id.as_deref().is_some_and(|r| !r.is_empty() && t.id.starts_with(r)));
        if let Some(idx) = found {
            m.claim(&t.id, idx);
            open.retain(|o| o.id != t.id);
        }
    }
    m.greedy(&open, |s, t, idx, gap| s.same_dir(t, idx) && gap >= -BEFORE_START_MS);
    m.greedy(&open, |s, t, idx, gap| (procs[idx].workspace.as_deref().is_none_or(str::is_empty) || s.same_dir(t, idx)) && (-BEFORE_START_MS..=AFTER_START_MS).contains(&gap));
    for t in &open {
        if m.result.contains_key(&t.id) {
            continue;
        }
        let candidates: Vec<usize> = (0..procs.len())
            .filter(|i| !m.taken.contains(&procs[*i].pid) && (procs[*i].workspace.as_deref().is_none_or(str::is_empty) || m.same_dir(t, *i)) && procs[*i].start <= t.started_at + BEFORE_START_MS)
            .collect();
        if let [only] = candidates.as_slice() {
            m.claim(&t.id, *only);
        }
    }
    m.result.into_iter().map(|(id, idx)| (id, procs[idx].clone())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_791_702_000_000;

    fn proc(pid: u32, start: i64, ws: Option<&str>, resume: Option<&str>) -> DeepseekProcess {
        DeepseekProcess { pid, start, ancestors: Vec::new(), workspace: ws.map(str::to_owned), resume_id: resume.map(str::to_owned) }
    }

    fn key(id: &str, ws: Option<&str>, started_at: i64) -> DeepseekThreadKey {
        DeepseekThreadKey { id: id.into(), workspace: ws.map(str::to_owned), started_at, pid: None, start: None }
    }

    #[test]
    fn reads_workspace_and_resume_id_only() {
        assert_eq!(parse_codewhale_args("codewhale.exe -C D:\\work\\x --model m \"a prompt\""), Some((Some("D:\\work\\x".into()), None)));
        assert_eq!(parse_codewhale_args("codewhale -r sess-1 --workspace=/tmp/p"), Some((Some("/tmp/p".into()), Some("sess-1".into()))));
        assert_eq!(parse_codewhale_args("codewhale auth login"), None);
        assert_eq!(parse_codewhale_args("codewhale -C rel"), Some((None, None)));
    }

    #[test]
    fn matches_by_resume_id_then_workspace_then_start_time() {
        let procs = vec![proc(1, T0, Some("D:\\a"), None), proc(2, T0 + 50, Some("D:\\b"), None), proc(3, T0 + 100, None, Some("abc"))];
        let m = match_deepseek_processes(&[key("abc-123", None, T0 + 9_000_000), key("t-b", Some("d:/b"), T0 + 500), key("t-a", Some("D:\\a"), T0 + 600)], &procs, true);
        assert_eq!(m["abc-123"].pid, 3);
        assert_eq!(m["t-b"].pid, 2);
        assert_eq!(m["t-a"].pid, 1);
    }

    #[test]
    fn a_lone_older_process_takes_the_last_unmatched_session() {
        let m = match_deepseek_processes(&[key("x", Some("D:\\z"), T0 + 500_000)], &[proc(7, T0, None, None)], true);
        assert_eq!(m["x"].pid, 7);
    }
}
