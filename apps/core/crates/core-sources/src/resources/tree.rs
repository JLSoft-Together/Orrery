use std::collections::{HashMap, HashSet};

use super::daemon::{base_name, daemon_of, DaemonInfo};
use super::idle::ResPort;
pub use super::ports::PortMap;
use super::procs::{proc_key, ProcRow};

#[derive(Debug, Clone, PartialEq)]
pub struct SessionTree {
    pub session_id: String,
    pub root: usize,
    pub members: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ServerEntry {
    pub root: usize,
    pub main: usize,
    pub members: Vec<usize>,
    pub kind: &'static str,
    pub session_id: Option<String>,
    pub orphan: bool,
    pub ports: Vec<ResPort>,
    pub daemon: Option<DaemonInfo>,
}

#[derive(Debug, Default)]
pub struct SeenMap {
    order: Vec<String>,
    map: HashMap<String, String>,
}

impl SeenMap {
    pub fn set(&mut self, key: String, session: &str) {
        if self.map.insert(key.clone(), session.to_owned()).is_none() {
            self.order.push(key);
        }
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.map.get(key)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.order.iter().map(|k| (k, &self.map[k]))
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
}

#[derive(Debug, Default)]
pub struct TreeResult {
    pub sessions: Vec<SessionTree>,
    pub servers: Vec<ServerEntry>,
    pub seen: SeenMap,
}

pub struct TreeInput<'a> {
    pub rows: &'a [ProcRow],
    pub sessions: &'a [(String, u32)],
    pub lineage: &'a HashMap<String, String>,
    pub ports: Option<&'a PortMap>,
    pub keeps_parent: bool,
}

const SHELLS: [&str; 10] = ["bash", "sh", "zsh", "fish", "dash", "ksh", "pwsh", "powershell", "cmd", "nu"];
const CONSOLE_HOSTS: [&str; 2] = ["conhost", "openconsole"];

pub fn is_shell(name: &str) -> bool {
    SHELLS.contains(&base_name(name).as_str())
}

struct Forest<'a> {
    rows: &'a [ProcRow],
    by_pid: HashMap<u32, usize>,
    children: HashMap<u32, Vec<usize>>,
}

impl<'a> Forest<'a> {
    fn new(rows: &'a [ProcRow]) -> Self {
        let by_pid: HashMap<u32, usize> = rows.iter().enumerate().map(|(i, r)| (r.pid, i)).collect();
        let mut f = Forest { rows, by_pid, children: HashMap::new() };
        for i in 0..rows.len() {
            if let Some(p) = f.parent_of(i) {
                f.children.entry(rows[p].pid).or_default().push(i);
            }
        }
        f
    }

    fn parent_of(&self, i: usize) -> Option<usize> {
        let row = &self.rows[i];
        if row.ppid == row.pid {
            return None;
        }
        let p = *self.by_pid.get(&row.ppid)?;
        if let (Some(ps), Some(rs)) = (self.rows[p].start_ms, row.start_ms) {
            if ps > rs {
                return None;
            }
        }
        Some(p)
    }

    fn kids(&self, i: usize) -> &[usize] {
        self.children.get(&self.rows[i].pid).map_or(&[], |v| v.as_slice())
    }

    fn subtree(&self, root: usize, stop: &dyn Fn(usize) -> bool) -> Vec<usize> {
        let mut out = vec![root];
        let mut seen: HashSet<u32> = HashSet::from([self.rows[root].pid]);
        let mut k = 0;
        while k < out.len() {
            for &c in self.kids(out[k]) {
                if seen.contains(&self.rows[c].pid) || stop(c) {
                    continue;
                }
                seen.insert(self.rows[c].pid);
                out.push(c);
            }
            k += 1;
        }
        out
    }

    fn is_tool_shell(&self, child: usize) -> bool {
        let name = &self.rows[child].name;
        if base_name(name) != "cmd" {
            return is_shell(name);
        }
        self.kids(child).iter().any(|&g| is_shell(&self.rows[g].name) && base_name(&self.rows[g].name) != "cmd")
    }
}

fn ports_of(rows: &[ProcRow], members: &[usize], ports: Option<&PortMap>) -> Vec<ResPort> {
    let Some(ports) = ports else { return Vec::new() };
    members.iter().flat_map(|&m| ports.get(&rows[m].pid).cloned().unwrap_or_default()).collect()
}

fn main_of(rows: &[ProcRow], members: &[usize], ports: Option<&PortMap>) -> usize {
    let main = |m: usize| !is_shell(&rows[m].name) && !CONSOLE_HOSTS.contains(&base_name(&rows[m].name).as_str());
    members
        .iter()
        .copied()
        .find(|&m| main(m) && ports.and_then(|p| p.get(&rows[m].pid)).is_some_and(|l| !l.is_empty()))
        .or_else(|| members.iter().copied().find(|&m| main(m)))
        .unwrap_or(members[0])
}

fn entry(rows: &[ProcRow], root: usize, members: Vec<usize>, session_id: Option<String>, orphan: bool, ports: Option<&PortMap>) -> ServerEntry {
    let p = ports_of(rows, &members, ports);
    let kind = if !p.is_empty() {
        "dev-server"
    } else if is_shell(&rows[root].name) {
        "shell"
    } else {
        "other"
    };
    let main = main_of(rows, &members, ports);
    ServerEntry { root, main, members, kind, session_id, orphan, ports: p, daemon: None }
}

pub fn build_tree(input: &TreeInput) -> TreeResult {
    let rows = input.rows;
    let f = Forest::new(rows);
    let mut daemon_order: Vec<u32> = Vec::new();
    let mut daemons: HashMap<u32, DaemonInfo> = HashMap::new();
    for r in rows {
        if let Some(d) = daemon_of(&r.name, r.cmd_text()) {
            if daemons.insert(r.pid, d).is_none() {
                daemon_order.push(r.pid);
            }
        }
    }
    let is_daemon = |i: usize| daemons.contains_key(&rows[i].pid);
    let mut out = TreeResult::default();
    let mut placed: HashSet<u32> = HashSet::new();
    let mut live: HashSet<String> = HashSet::new();

    for (session_id, pid) in input.sessions {
        let Some(&root) = f.by_pid.get(pid) else { continue };
        if placed.contains(&rows[root].pid) {
            continue;
        }
        live.insert(session_id.clone());
        let members = f.subtree(root, &is_daemon);
        for &m in &members {
            placed.insert(rows[m].pid);
        }
        for &m in members.iter().skip(1) {
            if let Some(key) = proc_key(&rows[m]) {
                out.seen.set(key, session_id);
            }
        }
        for &c in f.kids(root) {
            if !is_daemon(c) && f.is_tool_shell(c) {
                let sub = f.subtree(c, &is_daemon);
                out.servers.push(entry(rows, c, sub, Some(session_id.clone()), false, input.ports));
            }
        }
        for &m in &members {
            for &c in f.kids(m) {
                if is_daemon(c) {
                    if let Some(key) = proc_key(&rows[c]) {
                        out.seen.set(key, session_id);
                    }
                }
            }
        }
        out.sessions.push(SessionTree { session_id: session_id.clone(), root, members });
    }

    let session_of = |i: usize, seen: &SeenMap| -> Option<String> {
        let key = proc_key(&rows[i])?;
        input.lineage.get(&key).or_else(|| seen.get(&key)).cloned()
    };
    let left: Vec<usize> = (0..rows.len()).filter(|&i| !placed.contains(&rows[i].pid) && !is_daemon(i) && session_of(i, &out.seen).is_some()).collect();
    let left_pids: HashSet<u32> = left.iter().map(|&i| rows[i].pid).collect();
    for &r in &left {
        let p = f.parent_of(r);
        if let Some(p) = p {
            if left_pids.contains(&rows[p].pid) {
                continue;
            }
            if input.keeps_parent && !placed.contains(&rows[p].pid) {
                continue;
            }
        }
        let session_id = session_of(r, &out.seen).unwrap_or_default();
        let members = f.subtree(r, &|c| is_daemon(c) || placed.contains(&rows[c].pid));
        for &m in &members {
            placed.insert(rows[m].pid);
        }
        let orphan = !live.contains(&session_id);
        out.servers.push(entry(rows, r, members, Some(session_id), orphan, input.ports));
    }

    for pid in &daemon_order {
        let daemon = &daemons[pid];
        let root = f.by_pid[pid];
        if let Some(p) = f.parent_of(root) {
            if daemons.get(&rows[p].pid).is_some_and(|d| d.kind == daemon.kind) {
                continue;
            }
        }
        let members = f.subtree(root, &|_| false);
        let ports = if daemon.kind == "emulator" { Vec::new() } else { ports_of(rows, &members, input.ports) };
        let session_id = session_of(root, &out.seen);
        out.servers.push(ServerEntry {
            root,
            main: root,
            members,
            kind: if daemon.kind == "emulator" { "emulator" } else { "daemon" },
            session_id,
            orphan: false,
            ports,
            daemon: Some(daemon.clone()),
        });
    }
    out
}

pub fn cpu_percent(prev: Option<(f64, f64)>, cpu_ms: Option<f64>, at: f64, cores: f64) -> Option<f64> {
    let (prev_cpu, prev_at) = prev?;
    let cpu = cpu_ms?;
    if at <= prev_at || cores <= 0.0 {
        return None;
    }
    Some((cpu - prev_cpu).max(0.0) / (at - prev_at) / cores * 100.0)
}

pub fn sum_of(rows: &[ProcRow], members: &[usize], value: &dyn Fn(&ProcRow) -> Option<f64>) -> Option<f64> {
    let first = members.first()?;
    value(&rows[*first])?;
    Some(members.iter().map(|&m| value(&rows[m]).unwrap_or(0.0)).sum())
}
