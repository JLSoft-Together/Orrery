use std::collections::{HashMap, HashSet};

use core_procs::Proc;
use core_transcript::jsstr::js_space;

pub const MAX_ANCESTORS: usize = 16;
const FILETIME_EPOCH_OFFSET: u64 = 11_644_473_600_000;
const SHELLISH: [&str; 23] = [
    "bash", "sh", "dash", "zsh", "fish", "timeout", "env", "xargs", "nohup", "time", "script", "node", "bun", "deno", "python", "python3", "py", "cmd", "powershell", "pwsh", "npx", "npm", "codex",
];

#[derive(Debug, Clone, PartialEq)]
pub struct CliProcess {
    pub pid: u32,
    pub start: i64,
    pub ancestors: Vec<u32>,
    pub orphan: bool,
    pub cmd: String,
}

pub trait Rooted {
    fn pid(&self) -> u32;
    fn ancestors(&self) -> &[u32];
}

impl Rooted for CliProcess {
    fn pid(&self) -> u32 {
        self.pid
    }
    fn ancestors(&self) -> &[u32] {
        &self.ancestors
    }
}

pub fn is_shellish(name: &str) -> bool {
    let lower = name.to_lowercase();
    let bare = lower.strip_suffix(".exe").unwrap_or(&lower);
    SHELLISH.contains(&bare)
}

pub fn split_args(cmd: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut has = false;
    for ch in cmd.chars() {
        if ch == '"' {
            quoted = !quoted;
            has = true;
        } else if !quoted && (ch == ' ' || ch == '\t') {
            if has {
                out.push(std::mem::take(&mut cur));
            }
            cur.clear();
            has = false;
        } else {
            cur.push(ch);
            has = true;
        }
    }
    if has {
        out.push(cur);
    }
    out
}

pub fn top_level<T: Rooted>(list: Vec<T>) -> Vec<T> {
    let pids: HashSet<u32> = list.iter().map(Rooted::pid).collect();
    list.into_iter().filter(|p| !p.ancestors().iter().any(|a| pids.contains(a))).collect()
}

fn filetime_of(p: &Proc) -> Option<u64> {
    let raw = p.start_raw.as_deref()?;
    if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse().ok()
}

pub fn windows_processes_named(table: &[Proc], names: &HashSet<String>, command_line: &dyn Fn(u32) -> Option<String>) -> Option<Vec<CliProcess>> {
    let all: HashMap<u32, &Proc> = table.iter().map(|p| (p.pid, p)).collect();
    let newer = |a: &Proc, b: &Proc| matches!((filetime_of(a), filetime_of(b)), (Some(x), Some(y)) if x > y);
    let mut out = Vec::new();
    for proc in table {
        if !names.contains(&proc.name.to_lowercase()) {
            continue;
        }
        let ft = filetime_of(proc)?;
        let cmd = command_line(proc.pid)?;
        let mut ancestors = Vec::new();
        let mut cur = proc;
        let mut cut = false;
        for _ in 0..MAX_ANCESTORS {
            let parent = all.get(&cur.ppid).copied();
            match parent {
                Some(p) if p.pid != cur.pid && !newer(p, cur) => {
                    ancestors.push(p.pid);
                    cur = p;
                }
                _ => {
                    cut = cur.ppid > 4 && parent.map(|p| p.pid) != Some(cur.pid);
                    break;
                }
            }
        }
        out.push(CliProcess {
            pid: proc.pid,
            start: (ft / 10_000) as i64 - FILETIME_EPOCH_OFFSET as i64,
            cmd,
            ancestors: ancestors.into_iter().filter(|a| *a > 0).collect(),
            orphan: cut && is_shellish(&cur.name),
        });
    }
    Some(out)
}

struct UnixRow {
    ppid: u32,
    start: i64,
    name: String,
}

fn unix_name(raw: &str) -> String {
    let base = raw.rsplit('/').next().unwrap_or(raw);
    base.strip_prefix('-').unwrap_or(base).to_owned()
}

pub fn unix_processes_named(rows: &[Proc], cmds: &HashMap<u32, String>, names: &HashSet<String>) -> Vec<CliProcess> {
    let mut order: Vec<u32> = Vec::new();
    let mut all: HashMap<u32, UnixRow> = HashMap::new();
    for r in rows {
        let Some(start) = r.start_ms else { continue };
        if all.insert(r.pid, UnixRow { ppid: r.ppid, start, name: unix_name(&r.name) }).is_none() {
            order.push(r.pid);
        }
    }
    let mut out = Vec::new();
    for pid in order {
        let row = &all[&pid];
        if !names.contains(&row.name) {
            continue;
        }
        let mut ancestors = Vec::new();
        let mut cur = row;
        let mut cut = false;
        for _ in 0..MAX_ANCESTORS {
            let parent = all.get(&cur.ppid);
            let broken = parent.is_none_or(|p| p.start > cur.start);
            if parent.is_none() || cur.ppid <= 1 || broken {
                cut = cur.ppid == 1 || (cur.ppid > 1 && broken);
                break;
            }
            ancestors.push(cur.ppid);
            if let Some(p) = parent {
                cur = p;
            }
        }
        let cmd = cmds.get(&pid).map(|c| flatten_cmd(c)).unwrap_or_default();
        out.push(CliProcess { pid, start: row.start, cmd, ancestors, orphan: cut && is_shellish(&cur.name) });
    }
    out
}

fn flatten_cmd(raw: &str) -> String {
    let mut s = String::with_capacity(raw.len());
    let mut in_break = false;
    for c in raw.chars() {
        if c == '\r' || c == '\n' {
            if !in_break {
                s.push(' ');
            }
            in_break = true;
        } else {
            in_break = false;
            s.push(c);
        }
    }
    s.trim_start_matches(js_space).to_owned()
}

pub fn list_named(win: &[&str], unix: &[&str]) -> Option<Vec<CliProcess>> {
    let table = core_procs::table()?;
    if cfg!(windows) {
        let wanted: HashSet<String> = win.iter().map(|s| (*s).to_owned()).collect();
        let pids: Vec<u32> = table.iter().filter(|p| wanted.contains(&p.name.to_lowercase()) && p.start_ms.is_some()).map(|p| p.pid).collect();
        let cmds: HashMap<u32, String> = core_procs::cmdline(&pids).into_iter().map(|c| (c.pid, c.cmd)).collect();
        return windows_processes_named(&table, &wanted, &|pid| cmds.get(&pid).cloned());
    }
    if cfg!(target_os = "macos") {
        let wanted: HashSet<String> = unix.iter().map(|s| (*s).to_owned()).collect();
        let pids: Vec<u32> = table.iter().filter(|p| p.start_ms.is_some() && wanted.contains(&unix_name(&p.name))).map(|p| p.pid).collect();
        let cmds: HashMap<u32, String> = core_procs::cmdline(&pids).into_iter().map(|c| (c.pid, c.cmd)).collect();
        return Some(unix_processes_named(&table, &cmds, &wanted));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(pid: u32, ppid: u32, name: &str, ft: Option<u64>) -> Proc {
        Proc { pid, ppid, name: name.into(), start_ms: ft.map(|f| (f / 10_000) as i64), start_raw: ft.map(|f| f.to_string()) }
    }

    fn ft(ms: u64) -> u64 {
        (ms + FILETIME_EPOCH_OFFSET) * 10_000
    }

    #[test]
    fn windows_rows_walk_ancestors_and_mark_shell_cut_chains() {
        let t0 = 1_791_000_000_000u64;
        let table = vec![
            p(7, 0, "System", Some(ft(t0))),
            p(8, 7, "explorer.exe", Some(ft(t0))),
            p(9, 8, "claude.exe", Some(ft(t0 + 10))),
            p(10, 9, "codex.exe", Some(ft(t0 + 20))),
            p(12, 20, "codex.exe", Some(ft(t0 + 30))),
            p(13, 20, "Codex.exe", Some(ft(t0 + 30))),
            p(20, 99, "timeout.exe", Some(ft(t0 + 40))),
        ];
        let names: HashSet<String> = ["codex.exe".to_owned()].into_iter().collect();
        let rows = windows_processes_named(&table, &names, &|pid| Some(format!("codex.exe {pid}"))).unwrap();
        assert_eq!(rows.iter().map(|r| r.pid).collect::<Vec<_>>(), vec![10, 12, 13]);
        assert_eq!(rows[0].ancestors, vec![9, 8, 7]);
        assert_eq!(rows[0].start, t0 as i64 + 20);
        assert!(!rows[0].orphan);
        assert_eq!(rows[1].ancestors, Vec::<u32>::new());
        assert!(rows[1].orphan);
        assert!(windows_processes_named(&table, &names, &|_| None).is_none());
    }

    #[test]
    fn unix_rows_cut_at_launchd_and_use_command_lines() {
        let row = |pid: u32, ppid: u32, name: &str, s: i64| Proc { pid, ppid, name: name.into(), start_ms: Some(s), start_raw: None };
        let rows = vec![row(1, 0, "launchd", 0), row(100, 1, "claude", 1_000), row(200, 100, "-zsh", 2_000), row(300, 200, "codex", 3_000), row(400, 1, "node", 4_000), row(401, 400, "codex", 5_000)];
        let cmds: HashMap<u32, String> = [(300, "/opt/codex/codex exec -".to_owned()), (401, "codex\r\n resume".to_owned())].into_iter().collect();
        let names: HashSet<String> = ["codex".to_owned()].into_iter().collect();
        let out = unix_processes_named(&rows, &cmds, &names);
        assert_eq!(out[0].ancestors, vec![200, 100]);
        assert!(!out[0].orphan);
        assert_eq!(out[1].ancestors, vec![400]);
        assert!(out[1].orphan);
        assert_eq!(out[1].cmd, "codex  resume");
    }

    #[test]
    fn splits_args_like_the_c_runtime() {
        assert_eq!(split_args("\"C:\\npm\\codex.exe\" exec -s \"read only\" -"), vec!["C:\\npm\\codex.exe", "exec", "-s", "read only", "-"]);
        assert_eq!(split_args("a  b\t\"\" c"), vec!["a", "b", "", "c"]);
    }
}
