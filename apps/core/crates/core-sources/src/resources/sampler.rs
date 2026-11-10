use std::collections::{HashMap, HashSet};

use core_store::extra::BackgroundJob;
use core_store::model::{Agent, TerminalSession};
use core_store::JsNum;
use core_transcript::jsstr::{iso_of_ms, js_slice};
use serde_json::{json, Map, Value};

use super::daemon::{may_be_daemon, DaemonInfo};
use super::idle::{emulator_idle_since, idle_since_of, is_forgotten, server_busy, session_idle_since, track_busy, BusyState, ResPort};
use super::ports::{outside_ports, port_conflicts, ConflictCall, Holder, Owners, PortMap};
use super::procs::{proc_key, ProcReader, ProcRow};
use super::tree::{build_tree, cpu_percent, sum_of, TreeInput};
use crate::mask::mask_secrets;

pub const RES_CMD_MAX: usize = 200;
const LINEAGE_MAX: usize = 20_000;

pub struct SampleInput<'a> {
    pub terminals: &'a [TerminalSession],
    pub agents: &'a [Agent],
    pub jobs: &'a [BackgroundJob],
    pub idle_minutes: f64,
    pub last_device_call_at: Option<f64>,
    pub conflicts: &'a [ConflictCall],
    pub ports: Option<PortMap>,
    pub now: f64,
}

pub struct ResourceSampler {
    reader: Box<dyn ProcReader>,
    prev_cpu: HashMap<String, (f64, f64)>,
    lineage: HashMap<String, String>,
    busy: HashMap<String, BusyState>,
    device_use: HashMap<String, f64>,
    cores: f64,
    mask: Box<dyn Fn(&str) -> String + Send>,
    last: Option<Value>,
    keeps_parent: bool,
}

pub fn host_platform() -> &'static str {
    if cfg!(windows) {
        "win32"
    } else if cfg!(target_os = "macos") {
        "darwin"
    } else {
        "linux"
    }
}

pub fn home_dir() -> String {
    let key = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var(key).unwrap_or_default()
}

fn n(v: f64) -> Value {
    serde_json::to_value(JsNum(v)).unwrap_or(Value::Null)
}

fn opt(v: Option<f64>) -> Value {
    v.map_or(Value::Null, n)
}

pub fn port_json(p: &ResPort) -> Value {
    json!({"port": p.port, "address": p.address, "established": p.established})
}

pub fn daemon_json(d: &DaemonInfo) -> Value {
    let mut m = Map::new();
    m.insert("type".into(), json!(d.kind));
    for (k, v) in [("version", &d.version), ("jdk", &d.jdk), ("avd", &d.avd), ("serial", &d.serial)] {
        if let Some(v) = v {
            m.insert(k.into(), json!(v));
        }
    }
    Value::Object(m)
}

impl ResourceSampler {
    pub fn new(reader: Box<dyn ProcReader>) -> Self {
        let home = home_dir();
        let mask = Box::new(move |cmd: &str| mask_secrets(cmd, &home, host_platform()));
        Self {
            reader,
            prev_cpu: HashMap::new(),
            lineage: HashMap::new(),
            busy: HashMap::new(),
            device_use: HashMap::new(),
            cores: std::thread::available_parallelism().map_or(1, |c| c.get()) as f64,
            mask,
            last: None,
            keeps_parent: cfg!(windows),
        }
    }

    pub fn with_options(mut self, cores: f64, mask: Box<dyn Fn(&str) -> String + Send>, keeps_parent: bool) -> Self {
        self.cores = cores;
        self.mask = mask;
        self.keeps_parent = keeps_parent;
        self
    }

    pub fn report(&self) -> Option<&Value> {
        self.last.as_ref()
    }

    pub fn lineage(&self) -> &HashMap<String, String> {
        &self.lineage
    }

    pub fn device_used(&mut self, serial: &str, avd: Option<&str>, at: f64) {
        self.device_use.insert(format!("serial:{serial}"), at);
        if let Some(avd) = avd.filter(|a| !a.is_empty()) {
            self.device_use.insert(format!("avd:{avd}"), at);
        }
    }

    fn last_use_of(&self, d: Option<&DaemonInfo>) -> Option<f64> {
        let a = d.and_then(|d| d.avd.as_ref()).and_then(|v| self.device_use.get(&format!("avd:{v}"))).copied();
        let s = d.and_then(|d| d.serial.as_ref()).and_then(|v| self.device_use.get(&format!("serial:{v}"))).copied();
        let m = a.unwrap_or(0.0).max(s.unwrap_or(0.0));
        (m != 0.0).then_some(m)
    }

    pub fn sample(&mut self, input: &SampleInput) -> Option<Value> {
        let mut rows = self.reader.read()?;
        let now = input.now;
        let terms: Vec<&TerminalSession> = input.terminals.iter().filter(|t| t.ended_at.is_none() && t.process_id.is_some_and(|p| p != 0)).collect();
        let daemon_pids: HashSet<u32> = rows.iter().filter(|r| may_be_daemon(&r.name)).map(|r| r.pid).collect();
        self.reader.enrich(&mut rows, &HashSet::new(), &daemon_pids);
        let sessions_in: Vec<(String, u32)> = terms.iter().map(|t| (t.id.clone(), t.process_id.unwrap_or(0) as u32)).collect();
        let ports = input.ports.as_ref();
        let tree = build_tree(&TreeInput { rows: &rows, sessions: &sessions_in, lineage: &self.lineage, ports, keeps_parent: self.keeps_parent });

        let mut usage_pids: HashSet<u32> = HashSet::new();
        for s in &tree.sessions {
            usage_pids.extend(s.members.iter().map(|&m| rows[m].pid));
        }
        for e in &tree.servers {
            usage_pids.extend(e.members.iter().map(|&m| rows[m].pid));
        }
        let main_pids: HashSet<u32> = tree.servers.iter().map(|e| rows[e.main].pid).collect();
        self.reader.enrich(&mut rows, &usage_pids, &main_pids);

        let live_keys: HashSet<String> = rows.iter().filter_map(proc_key).collect();
        for (key, session) in tree.seen.iter() {
            if self.lineage.len() < LINEAGE_MAX || self.lineage.contains_key(key) {
                self.lineage.insert(key.clone(), session.clone());
            }
        }
        self.lineage.retain(|k, _| live_keys.contains(k));

        let mut cpu: HashMap<u32, Option<f64>> = HashMap::new();
        let mut sampled: HashSet<String> = HashSet::new();
        for r in &rows {
            if !usage_pids.contains(&r.pid) {
                continue;
            }
            let key = proc_key(r);
            let (Some(key), Some(cpu_ms)) = (key, r.cpu_ms) else {
                cpu.insert(r.pid, None);
                continue;
            };
            cpu.insert(r.pid, cpu_percent(self.prev_cpu.get(&key).copied(), Some(cpu_ms), now, self.cores));
            self.prev_cpu.insert(key.clone(), (cpu_ms, now));
            sampled.insert(key);
        }
        self.prev_cpu.retain(|k, _| sampled.contains(k));
        let cpu_of = |r: &ProcRow| cpu.get(&r.pid).copied().flatten();
        let ram_of = |r: &ProcRow| r.ram_bytes;

        let idle_minutes = input.idle_minutes;
        let term_by_id: HashMap<&str, &TerminalSession> = terms.iter().map(|t| (t.id.as_str(), *t)).collect();
        let agents: Vec<&Agent> = input.agents.iter().collect();
        let jobs: Vec<&BackgroundJob> = input.jobs.iter().collect();
        let mut session_forgotten: Vec<bool> = Vec::new();
        let mut sessions_json: Vec<Value> = Vec::new();
        for s in &tree.sessions {
            let idle_since = term_by_id.get(s.session_id.as_str()).and_then(|t| session_idle_since(t, &agents, &jobs));
            let forgotten = is_forgotten(idle_since, now, idle_minutes);
            session_forgotten.push(forgotten);
            let root = &rows[s.root];
            sessions_json.push(json!({
                "sessionId": s.session_id,
                "pid": root.pid,
                "start": root.start.clone().unwrap_or_default(),
                "name": root.name,
                "ramBytes": opt(sum_of(&rows, &s.members, &ram_of)),
                "cpuPct": opt(sum_of(&rows, &s.members, &cpu_of)),
                "idleSinceMs": opt(idle_since),
                "forgotten": forgotten,
            }));
        }

        let last_call = input.last_device_call_at;
        let mut busy_keys: HashSet<String> = HashSet::new();
        let mut servers: Vec<(Value, bool, f64)> = Vec::new();
        for e in &tree.servers {
            let root = &rows[e.root];
            let key = proc_key(root).unwrap_or_else(|| format!("pid:{}", root.pid));
            busy_keys.insert(key.clone());
            let cpu_pct = sum_of(&rows, &e.members, &cpu_of);
            let (idle_since, last_busy_at) = if e.kind == "emulator" {
                let state = *self.busy.entry(key.clone()).or_insert(BusyState { first_seen: now, last_busy: None });
                let last_use = self.last_use_of(e.daemon.as_ref());
                let idle = emulator_idle_since(state.first_seen, last_use, last_call);
                let lb = last_use.unwrap_or(0.0).max(last_call.unwrap_or(0.0));
                (Some(idle), (lb != 0.0).then_some(lb))
            } else {
                let state = track_busy(self.busy.get(&key).copied(), server_busy(cpu_pct, &e.ports), now);
                self.busy.insert(key.clone(), state);
                (idle_since_of(&state, now), state.last_busy)
            };
            let main = &rows[e.main];
            let cmd = main.cmd_text().filter(|c| !c.is_empty()).map(|c| js_slice(&(self.mask)(c), RES_CMD_MAX).to_owned()).unwrap_or_default();
            let forgotten = e.orphan || is_forgotten(idle_since, now, idle_minutes);
            let mut o = Map::new();
            o.insert("pid".into(), json!(root.pid));
            o.insert("start".into(), json!(root.start.clone().unwrap_or_default()));
            o.insert("kind".into(), json!(e.kind));
            o.insert("name".into(), json!(main.name));
            o.insert("cmd".into(), json!(cmd));
            o.insert("ports".into(), Value::Array(e.ports.iter().map(port_json).collect()));
            o.insert("ramBytes".into(), opt(sum_of(&rows, &e.members, &ram_of)));
            o.insert("cpuPct".into(), opt(cpu_pct));
            o.insert("idleSinceMs".into(), opt(idle_since));
            o.insert("lastBusyAt".into(), opt(last_busy_at));
            o.insert("forgotten".into(), json!(forgotten));
            o.insert("sessionId".into(), e.session_id.as_ref().map_or(Value::Null, |s| json!(s)));
            o.insert("orphan".into(), json!(e.orphan));
            if let Some(d) = &e.daemon {
                o.insert("daemon".into(), daemon_json(d));
            }
            let ram = sum_of(&rows, &e.members, &ram_of).unwrap_or(0.0);
            servers.push((Value::Object(o), forgotten, ram));
        }
        self.busy.retain(|k, _| busy_keys.contains(k));
        servers.sort_by(|a, b| b.2.total_cmp(&a.2));

        let mut all: Vec<usize> = Vec::new();
        let mut all_seen: HashSet<u32> = HashSet::new();
        let mut forgotten_pids: HashSet<u32> = HashSet::new();
        for (i, s) in tree.sessions.iter().enumerate() {
            for &m in &s.members {
                if all_seen.insert(rows[m].pid) {
                    all.push(m);
                }
                if session_forgotten[i] {
                    forgotten_pids.insert(rows[m].pid);
                }
            }
        }
        for e in &tree.servers {
            let root_pid = rows[e.root].pid;
            let forgotten = servers.iter().find(|s| s.0["pid"].as_u64() == Some(u64::from(root_pid))).is_some_and(|s| s.1);
            for &m in &e.members {
                if all_seen.insert(rows[m].pid) {
                    all.push(m);
                }
                if forgotten {
                    forgotten_pids.insert(rows[m].pid);
                }
            }
        }
        let (mut ram_total, mut cpu_total, mut forgotten_ram) = (0.0, 0.0, 0.0);
        for &i in &all {
            let r = &rows[i];
            ram_total += r.ram_bytes.unwrap_or(0.0);
            cpu_total += cpu.get(&r.pid).copied().flatten().unwrap_or(0.0);
            if forgotten_pids.contains(&r.pid) {
                forgotten_ram += r.ram_bytes.unwrap_or(0.0);
            }
        }
        let forgotten_count = session_forgotten.iter().filter(|f| **f).count() + servers.iter().filter(|s| s.1).count();
        let mut report = Map::new();
        report.insert("at".into(), json!(iso_of_ms(now).unwrap_or_default()));
        report.insert("idleMinutes".into(), n(idle_minutes));
        report.insert("cores".into(), n(self.cores));
        report.insert("sessions".into(), Value::Array(sessions_json));
        report.insert("servers".into(), Value::Array(servers.into_iter().map(|s| s.0).collect()));
        report.insert("totals".into(), json!({"ramBytes": n(ram_total), "cpuPct": n((cpu_total * 10.0).round() / 10.0), "forgotten": forgotten_count, "forgottenRamBytes": n(forgotten_ram)}));
        if let Some(ports) = ports {
            let mut owner: Owners = HashMap::new();
            for s in &tree.sessions {
                for &m in &s.members {
                    owner.insert(rows[m].pid, Some(s.session_id.clone()));
                }
            }
            for e in &tree.servers {
                for &m in &e.members {
                    owner.insert(rows[m].pid, e.session_id.clone());
                }
            }
            let name_of: HashMap<u32, String> = rows.iter().map(|r| (r.pid, r.name.clone())).collect();
            let outside = outside_ports(ports, &owner, &name_of);
            report.insert("outside".into(), Value::Array(outside.iter().map(|o| json!({"pid": o.pid, "name": o.name, "ports": o.ports.iter().map(port_json).collect::<Vec<_>>()})).collect()));
            let conflicts = port_conflicts(input.conflicts, now, ports, &owner, &name_of);
            report.insert("conflicts".into(), Value::Array(conflicts.iter().map(|c| json!({"port": c.port, "sessionId": c.session_id, "at": n(c.at), "holder": c.holder.as_ref().map_or(Value::Null, holder_json)})).collect()));
        }
        let value = Value::Object(report);
        self.last = Some(value.clone());
        Some(value)
    }
}

fn holder_json(h: &Holder) -> Value {
    json!({"pid": h.pid, "name": h.name, "sessionId": h.session_id})
}
