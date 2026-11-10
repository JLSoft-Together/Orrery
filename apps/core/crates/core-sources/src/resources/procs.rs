use std::collections::{HashMap, HashSet};

use core_procs::{Cmd, Proc, Usage};

#[derive(Debug, Clone, PartialEq)]
pub struct ProcRow {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub start: Option<String>,
    pub start_ms: Option<f64>,
    pub ram_bytes: Option<f64>,
    pub cpu_ms: Option<f64>,
    pub cmd: Option<Option<String>>,
}

impl ProcRow {
    pub fn new(pid: u32, ppid: u32, name: &str) -> Self {
        Self { pid, ppid, name: name.to_owned(), start: None, start_ms: None, ram_bytes: None, cpu_ms: None, cmd: None }
    }

    pub fn cmd_text(&self) -> Option<&str> {
        self.cmd.as_ref().and_then(|c| c.as_deref())
    }
}

pub fn proc_key(row: &ProcRow) -> Option<String> {
    row.start.as_ref().map(|s| format!("{}:{}", row.pid, s))
}

pub trait ProcReader: Send {
    fn read(&mut self) -> Option<Vec<ProcRow>>;
    fn enrich(&mut self, rows: &mut [ProcRow], usage: &HashSet<u32>, cmd: &HashSet<u32>);
}

pub trait ProcSource: Send {
    fn table(&mut self) -> Option<Vec<Proc>>;
    fn usage(&mut self, pids: &[u32]) -> Vec<Usage>;
    fn cmdline(&mut self, pids: &[u32]) -> Vec<Cmd>;
}

pub struct HostProcs;

impl ProcSource for HostProcs {
    fn table(&mut self) -> Option<Vec<Proc>> {
        core_procs::table()
    }

    fn usage(&mut self, pids: &[u32]) -> Vec<Usage> {
        core_procs::usage(pids)
    }

    fn cmdline(&mut self, pids: &[u32]) -> Vec<Cmd> {
        core_procs::cmdline(pids)
    }
}

pub struct CoreReader<S: ProcSource> {
    source: S,
    cmds: HashMap<String, Option<String>>,
    eager: bool,
}

impl CoreReader<HostProcs> {
    pub fn host() -> Self {
        Self::new(HostProcs, !cfg!(windows))
    }
}

impl<S: ProcSource> CoreReader<S> {
    pub fn new(source: S, eager: bool) -> Self {
        Self { source, cmds: HashMap::new(), eager }
    }

    fn fill(&mut self, rows: &mut [ProcRow], usage_pids: &HashSet<u32>, cmd_pids: &HashSet<u32>) {
        let usage_list: Vec<u32> = rows.iter().filter(|r| usage_pids.contains(&r.pid) && r.pid > 4).map(|r| r.pid).collect();
        let cmd_list: Vec<u32> = rows.iter().filter(|r| cmd_pids.contains(&r.pid) && r.cmd.is_none() && r.pid > 4).map(|r| r.pid).collect();
        let usage: HashMap<u32, Usage> = if usage_list.is_empty() { HashMap::new() } else { self.source.usage(&usage_list).into_iter().map(|u| (u.pid, u)).collect() };
        let fresh: HashMap<u32, String> = if cmd_list.is_empty() { HashMap::new() } else { self.source.cmdline(&cmd_list).into_iter().map(|c| (c.pid, c.cmd)).collect() };
        for row in rows.iter_mut() {
            if usage_pids.contains(&row.pid) {
                let u = usage.get(&row.pid);
                row.ram_bytes = u.and_then(|u| u.working_set).map(|v| v as f64);
                row.cpu_ms = u.and_then(|u| u.cpu_ms);
            }
            if cmd_pids.contains(&row.pid) && row.cmd.is_none() {
                let c = fresh.get(&row.pid).cloned();
                row.cmd = Some(c.clone());
                if let Some(key) = proc_key(row) {
                    self.cmds.insert(key, c);
                }
            }
        }
    }
}

impl<S: ProcSource> ProcReader for CoreReader<S> {
    fn read(&mut self) -> Option<Vec<ProcRow>> {
        let table = self.source.table()?;
        let mut live = HashSet::new();
        let mut rows: Vec<ProcRow> = table
            .into_iter()
            .map(|p| {
                let mut row = ProcRow {
                    pid: p.pid,
                    ppid: p.ppid,
                    name: p.name,
                    start: p.start_raw,
                    start_ms: p.start_ms.map(|v| v as f64),
                    ram_bytes: None,
                    cpu_ms: None,
                    cmd: None,
                };
                if let Some(key) = proc_key(&row) {
                    if let Some(c) = self.cmds.get(&key) {
                        row.cmd = Some(c.clone());
                    }
                    live.insert(key);
                }
                row
            })
            .collect();
        self.cmds.retain(|k, _| live.contains(k));
        if self.eager {
            let all: HashSet<u32> = rows.iter().map(|r| r.pid).collect();
            self.fill(&mut rows, &all, &all);
        }
        Some(rows)
    }

    fn enrich(&mut self, rows: &mut [ProcRow], usage: &HashSet<u32>, cmd: &HashSet<u32>) {
        if !self.eager {
            self.fill(rows, usage, cmd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        tables: Vec<Vec<Proc>>,
    }

    impl ProcSource for Fake {
        fn table(&mut self) -> Option<Vec<Proc>> {
            if self.tables.len() > 1 {
                Some(self.tables.remove(0))
            } else {
                self.tables.first().cloned()
            }
        }

        fn usage(&mut self, pids: &[u32]) -> Vec<Usage> {
            pids.iter().map(|&pid| Usage { pid, working_set: Some(1024), private_bytes: None, cpu_ms: Some(pid as f64) }).collect()
        }

        fn cmdline(&mut self, pids: &[u32]) -> Vec<Cmd> {
            pids.iter().filter(|&&p| p != 7).map(|&pid| Cmd { pid, cmd: format!("cmd{pid}") }).collect()
        }
    }

    fn proc(pid: u32, start: &str) -> Proc {
        Proc { pid, ppid: 1, name: "n".into(), start_ms: Some(pid as i64), start_raw: Some(start.into()) }
    }

    #[test]
    fn lazy_reader_fills_usage_and_caches_command_lines_by_pid_and_start() {
        let src = Fake { tables: vec![vec![proc(5, "a"), proc(7, "b"), proc(9, "c")], vec![proc(5, "a"), proc(9, "other")]] };
        let mut reader = CoreReader::new(src, false);
        let mut rows = reader.read().unwrap();
        assert!(rows.iter().all(|r| r.ram_bytes.is_none() && r.cmd.is_none()));
        let want: HashSet<u32> = [5, 7].into();
        reader.enrich(&mut rows, &want, &want);
        assert_eq!(rows[0].ram_bytes, Some(1024.0));
        assert_eq!(rows[0].cmd, Some(Some("cmd5".into())));
        assert_eq!(rows[1].cmd, Some(None));
        assert_eq!(rows[2].cmd, None);
        let again = reader.read().unwrap();
        assert_eq!(again[0].cmd, Some(Some("cmd5".into())));
        assert_eq!(again[1].cmd, None);
    }

    #[test]
    fn eager_reader_fills_everything_at_read() {
        let src = Fake { tables: vec![vec![proc(5, "a"), proc(2, "x")]] };
        let mut reader = CoreReader::new(src, true);
        let rows = reader.read().unwrap();
        assert_eq!(rows[0].cpu_ms, Some(5.0));
        assert_eq!(rows[1].ram_bytes, None);
        assert_eq!(rows[1].cmd, Some(None));
        assert_eq!(rows[0].cmd_text(), Some("cmd5"));
    }
}
