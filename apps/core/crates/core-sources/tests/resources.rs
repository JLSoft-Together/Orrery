use std::collections::HashSet;

use core_sources::resources::ports::{ports_by_pid, ConflictCall};
use core_sources::resources::procs::{ProcReader, ProcRow};
use core_sources::resources::sampler::{ResourceSampler, SampleInput};
use core_sources::resources::tree::{build_tree, cpu_percent, sum_of, TreeInput};
use core_procs::TcpRow;
use core_store::extra::BackgroundJob;
use core_store::model::{Agent, TerminalSession};
use serde_json::{json, Value};

const MB: f64 = 1024.0 * 1024.0;
const T0: f64 = 1_793_527_200_000.0;

fn row(pid: u32, ppid: u32, name: &str, f: impl FnOnce(&mut ProcRow)) -> ProcRow {
    let mut r = ProcRow::new(pid, ppid, name);
    r.start = Some(format!("s{pid}"));
    r.start_ms = Some(f64::from(pid));
    r.ram_bytes = Some(10.0 * MB);
    r.cpu_ms = Some(0.0);
    r.cmd = Some(Some(name.to_owned()));
    f(&mut r);
    r
}

fn plain(pid: u32, ppid: u32, name: &str) -> ProcRow {
    row(pid, ppid, name, |_| {})
}

fn with_cmd(pid: u32, ppid: u32, name: &str, cmd: &str) -> ProcRow {
    row(pid, ppid, name, |r| r.cmd = Some(Some(cmd.to_owned())))
}

fn pids(rows: &[ProcRow], idx: &[usize]) -> Vec<u32> {
    idx.iter().map(|&i| rows[i].pid).collect()
}

fn tree_rows() -> Vec<ProcRow> {
    vec![
        plain(10, 1, "claude.exe"),
        plain(11, 10, "bash.exe"),
        with_cmd(12, 11, "node.exe", "node vite"),
        with_cmd(13, 10, "node.exe", "node mcp-server"),
        with_cmd(14, 11, "java.exe", "java org.gradle.launcher.daemon.bootstrap.GradleDaemon 8.14"),
        with_cmd(20, 1, "emulator.exe", "emulator -avd Pixel"),
        with_cmd(21, 20, "qemu-system-x86_64.exe", "qemu -avd Pixel"),
        row(5, 10, "explorer.exe", |r| r.start_ms = Some(5.0)),
    ]
}

fn input<'a>(rows: &'a [ProcRow], sessions: &'a [(String, u32)], lineage: &'a std::collections::HashMap<String, String>, ports: Option<&'a core_sources::resources::ports::PortMap>, keeps: bool) -> TreeInput<'a> {
    TreeInput { rows, sessions, lineage, ports, keeps_parent: keeps }
}

fn a10() -> Vec<(String, u32)> {
    vec![("a".to_owned(), 10)]
}

#[test]
fn splits_the_session_tree_the_shell_under_it_daemons_and_emulators() {
    let rows = tree_rows();
    let lineage = Default::default();
    let sessions = a10();
    let t = build_tree(&input(&rows, &sessions, &lineage, None, false));
    assert_eq!(pids(&rows, &t.sessions[0].members), vec![10, 11, 13, 12]);
    let shell = t.servers.iter().find(|s| rows[s.root].pid == 11).unwrap();
    assert_eq!((shell.kind, shell.session_id.as_deref(), shell.orphan), ("shell", Some("a"), false));
    assert_eq!(rows[shell.main].pid, 12);
    assert_eq!(pids(&rows, &shell.members), vec![11, 12]);
    assert!(t.servers.iter().all(|s| rows[s.root].pid != 13));
    let gradle = t.servers.iter().find(|s| rows[s.root].pid == 14).unwrap();
    assert_eq!((gradle.kind, gradle.session_id.as_deref()), ("daemon", Some("a")));
    assert_eq!(gradle.daemon.as_ref().unwrap().version.as_deref(), Some("8.14"));
    let emu: Vec<(u32, usize)> = t.servers.iter().filter(|s| s.kind == "emulator").map(|s| (rows[s.root].pid, s.members.len())).collect();
    assert_eq!(emu, vec![(20, 2)]);
    let mut keys: Vec<&String> = t.seen.iter().map(|(k, _)| k).collect();
    keys.sort();
    let mut want: Vec<String> = [11, 12, 13, 14].iter().map(|p| format!("{p}:s{p}")).collect();
    want.sort();
    assert_eq!(keys, want.iter().collect::<Vec<_>>());
    assert!(t.servers.iter().all(|s| rows[s.root].pid != 5));
}

#[test]
fn names_a_piped_shell_command_after_the_listening_process() {
    let rows = vec![plain(10, 1, "claude"), plain(11, 10, "zsh"), plain(30, 11, "cut"), with_cmd(31, 11, "node", "node dev.js"), plain(32, 31, "python3")];
    let ports = ports_by_pid(&[TcpRow { pid: 32, state: 2, address: "127.0.0.1".into(), port: 8765 }]);
    let lineage = Default::default();
    let sessions = a10();
    let with = build_tree(&input(&rows, &sessions, &lineage, Some(&ports), false));
    let shell = with.servers.iter().find(|s| rows[s.root].pid == 11).unwrap();
    assert_eq!(rows[shell.main].name, "python3");
    let without = build_tree(&input(&rows, &sessions, &lineage, None, false));
    assert_eq!(rows[without.servers.iter().find(|s| rows[s.root].pid == 11).unwrap().main].name, "cut");
}

#[test]
fn marks_dev_servers_and_orphans_once_the_session_is_gone() {
    let rows = tree_rows();
    let ports = ports_by_pid(&[TcpRow { pid: 12, state: 2, address: "127.0.0.1".into(), port: 5173 }]);
    let lineage = Default::default();
    let sessions = a10();
    let first = build_tree(&input(&rows, &sessions, &lineage, Some(&ports), false));
    let shell = first.servers.iter().find(|s| rows[s.root].pid == 11).unwrap();
    assert_eq!((shell.kind, shell.ports[0].port), ("dev-server", 5173));
    let after: Vec<ProcRow> = rows.iter().filter(|r| r.pid != 10).cloned().collect();
    let mut lineage2 = std::collections::HashMap::new();
    for (k, v) in first.seen.iter() {
        lineage2.insert(k.clone(), v.clone());
    }
    let t = build_tree(&input(&after, &[], &lineage2, None, false));
    let orphan = t.servers.iter().find(|s| after[s.root].pid == 11).unwrap();
    assert_eq!((orphan.orphan, orphan.session_id.as_deref(), orphan.kind), (true, Some("a"), "shell"));
    assert_eq!(pids(&after, &orphan.members), vec![11, 12]);
    let mcp = t.servers.iter().find(|s| after[s.root].pid == 13).unwrap();
    assert_eq!((mcp.orphan, mcp.kind), (true, "other"));
    let g = t.servers.iter().find(|s| after[s.root].pid == 14).unwrap();
    assert_eq!((g.kind, g.orphan, g.session_id.as_deref()), ("daemon", false, Some("a")));
    let never = build_tree(&input(&after, &[], &Default::default(), None, false));
    assert!(never.servers.iter().all(|s| !s.orphan));
}

#[test]
fn on_windows_drops_a_remembered_process_whose_live_parent_is_outside_every_session() {
    let svc = vec![plain(600, 1, "services.exe"), plain(601, 600, "AnyDesk.exe"), plain(602, 601, "AnyDesk.exe"), plain(10, 1, "claude.exe")];
    let lineage: std::collections::HashMap<String, String> = [("601:s601".to_owned(), "a".to_owned()), ("602:s602".to_owned(), "a".to_owned())].into();
    let ports = ports_by_pid(&[TcpRow { pid: 601, state: 2, address: "0.0.0.0".into(), port: 7070 }]);
    let sessions = a10();
    assert!(build_tree(&input(&svc, &sessions, &lineage, Some(&ports), true)).servers.is_empty());
    let gone: Vec<ProcRow> = svc.iter().filter(|r| r.pid != 600).cloned().collect();
    let t = build_tree(&input(&gone, &sessions, &lineage, Some(&ports), true));
    assert_eq!(t.servers.iter().map(|s| (gone[s.root].pid, s.session_id.clone(), s.kind)).collect::<Vec<_>>(), vec![(601, Some("a".to_owned()), "dev-server")]);
    let posix = build_tree(&input(&svc, &sessions, &lineage, Some(&ports), false));
    assert_eq!(posix.servers.iter().map(|s| svc[s.root].pid).collect::<Vec<_>>(), vec![601]);
}

#[test]
fn treats_cmd_exe_as_a_tool_shell_only_when_it_hosts_one() {
    let rows = vec![
        plain(10, 1, "claude.exe"),
        with_cmd(30, 10, "cmd.exe", "cmd.exe /d /s /c \"npx figma-ui-mcp\""),
        plain(31, 30, "node.exe"),
        with_cmd(40, 10, "cmd.exe", "cmd.exe /d /s /c \"\"chcp.com\" 65001 >nul & powershell.exe -Command x\""),
        plain(41, 40, "conhost.exe"),
        plain(42, 40, "powershell.exe"),
        with_cmd(43, 42, "node.exe", "node server.js"),
        plain(50, 10, "uvx.exe"),
    ];
    let sessions = a10();
    let t = build_tree(&input(&rows, &sessions, &Default::default(), None, false));
    assert_eq!(t.servers.iter().map(|s| (rows[s.root].pid, rows[s.main].pid)).collect::<Vec<_>>(), vec![(40, 43)]);
    assert_eq!(t.sessions[0].members.len(), 8);
}

#[test]
fn computes_cpu_share_and_sums() {
    assert_eq!(cpu_percent(None, Some(100.0), 1000.0, 4.0), None);
    assert_eq!(cpu_percent(Some((0.0, 0.0)), Some(30_000.0), 30_000.0, 4.0), Some(25.0));
    assert_eq!(cpu_percent(Some((50.0, 0.0)), Some(10.0), 1000.0, 2.0), Some(0.0));
    let rows = vec![plain(1, 0, "a"), row(2, 1, "b", |r| r.ram_bytes = None)];
    assert_eq!(sum_of(&rows, &[0, 1], &|r| r.ram_bytes), Some(10.0 * MB));
    let none = vec![row(1, 0, "a", |r| r.ram_bytes = None)];
    assert_eq!(sum_of(&none, &[0], &|r| r.ram_bytes), None);
}

struct FakeReader {
    passes: Vec<Vec<ProcRow>>,
    i: usize,
}

impl ProcReader for FakeReader {
    fn read(&mut self) -> Option<Vec<ProcRow>> {
        let k = self.i.min(self.passes.len() - 1);
        self.i += 1;
        Some(self.passes[k].clone())
    }

    fn enrich(&mut self, _rows: &mut [ProcRow], _usage: &HashSet<u32>, _cmd: &HashSet<u32>) {}
}

fn term(last: f64) -> TerminalSession {
    serde_json::from_value(json!({"id": "a", "title": "proj", "status": "idle", "processId": 10, "lastActivityAt": core_transcript::jsstr::iso_of_ms(last).unwrap()})).unwrap()
}

#[test]
fn reports_ram_cpu_forgotten_servers_orphans_and_emulator_idleness() {
    let min = 60_000.0;
    let base = [
        row(10, 1, "claude.exe", |_| {}),
        row(11, 10, "bash.exe", |r| r.ram_bytes = Some(MB)),
        row(12, 11, "node.exe", |r| {
            r.ram_bytes = Some(100.0 * MB);
            r.cmd = Some(Some("node vite --token=abcd1234secret".into()));
        }),
        row(20, 1, "emulator.exe", |r| {
            r.ram_bytes = Some(2048.0 * MB);
            r.cmd = Some(Some("emulator -avd Pixel".into()));
        }),
    ];
    let at = |cpu: &[(u32, f64)], drop: &[u32]| -> Vec<ProcRow> {
        base.iter().filter(|r| !drop.contains(&r.pid)).map(|r| ProcRow { cpu_ms: Some(cpu.iter().find(|c| c.0 == r.pid).map_or(0.0, |c| c.1)), ..r.clone() }).collect()
    };
    let passes = vec![at(&[], &[]), at(&[(12, 600.0), (20, 30_000.0)], &[]), at(&[(12, 600.0), (20, 60_000.0)], &[]), at(&[(12, 600.0), (20, 90_000.0)], &[10])];
    let mut sampler = ResourceSampler::new(Box::new(FakeReader { passes, i: 0 })).with_options(2.0, Box::new(|c: &str| c.replace("token=abcd1234secret", "token=[masked]")), false);
    let mut now = T0;
    let mut terminals = vec![term(T0 - 61.0 * min)];
    let agents: Vec<Agent> = Vec::new();
    let jobs: Vec<BackgroundJob> = Vec::new();
    let run = |s: &mut ResourceSampler, now: f64, terminals: &[TerminalSession], last_call: Option<f64>| -> Value {
        s.sample(&SampleInput { terminals, agents: &agents, jobs: &jobs, idle_minutes: 60.0, last_device_call_at: last_call, conflicts: &[], ports: None, now }).unwrap()
    };
    let server = |r: &Value, pid: u64| r["servers"].as_array().unwrap().iter().find(|s| s["pid"] == pid).cloned().unwrap();

    let r1 = run(&mut sampler, now, &terminals, None);
    let s0 = &r1["sessions"][0];
    assert_eq!((s0["sessionId"].as_str(), s0["pid"].as_u64(), s0["ramBytes"].as_f64(), s0["forgotten"].as_bool()), (Some("a"), Some(10), Some(111.0 * MB), Some(true)));
    assert!(s0["cpuPct"].is_null());
    let shell = server(&r1, 11);
    assert_eq!((shell["kind"].as_str(), shell["name"].as_str(), shell["cmd"].as_str(), shell["start"].as_str(), shell["forgotten"].as_bool(), shell["orphan"].as_bool()), (Some("shell"), Some("node.exe"), Some("node vite --token=[masked]"), Some("s11"), Some(false), Some(false)));

    now += 30_000.0;
    let r2 = run(&mut sampler, now, &terminals, None);
    assert!((server(&r2, 11)["cpuPct"].as_f64().unwrap() - 1.0).abs() < 1e-9);
    let emu = server(&r2, 20);
    assert_eq!((emu["kind"].as_str(), emu["daemon"]["avd"].as_str(), emu["cpuPct"].as_f64()), (Some("emulator"), Some("Pixel"), Some(50.0)));

    now += 61.0 * min;
    sampler.device_used("emulator-5554", Some("Pixel"), now - min);
    let r3 = run(&mut sampler, now, &terminals, None);
    let shell3 = server(&r3, 11);
    assert_eq!((shell3["forgotten"].as_bool(), shell3["idleSinceMs"].as_f64()), (Some(true), Some(T0 + 30_000.0)));
    let emu3 = server(&r3, 20);
    assert_eq!((emu3["forgotten"].as_bool(), emu3["idleSinceMs"].as_f64()), (Some(false), Some(now - min)));

    terminals.clear();
    now += 30_000.0;
    let r4 = run(&mut sampler, now, &terminals, Some(now));
    assert_eq!(r4["sessions"], json!([]));
    let s4 = server(&r4, 11);
    assert_eq!((s4["orphan"].as_bool(), s4["forgotten"].as_bool(), s4["sessionId"].as_str()), (Some(true), Some(true), Some("a")));
    assert_eq!(r4["totals"]["forgotten"], 1);
    assert_eq!(r4["totals"]["forgottenRamBytes"].as_f64(), Some(101.0 * MB));
    assert_eq!(r4["totals"]["ramBytes"].as_f64(), Some(2149.0 * MB));
    assert!(r4.get("outside").is_none() && r4.get("conflicts").is_none());
    assert_eq!(sampler.report(), Some(&r4));
}

#[test]
fn port_tables_add_outside_listeners_and_recent_conflicts() {
    let rows = vec![plain(10, 1, "claude.exe"), plain(11, 10, "bash.exe"), plain(12, 11, "node.exe"), plain(90, 1, "postgres.exe")];
    let ports = ports_by_pid(&[
        TcpRow { pid: 12, state: 2, address: "127.0.0.1".into(), port: 5173 },
        TcpRow { pid: 90, state: 2, address: "0.0.0.0".into(), port: 5432 },
        TcpRow { pid: 77, state: 5, address: "127.0.0.1".into(), port: 5173 },
    ]);
    let mut sampler = ResourceSampler::new(Box::new(FakeReader { passes: vec![rows], i: 0 })).with_options(4.0, Box::new(|c: &str| c.to_owned()), false);
    let terminals = vec![term(T0)];
    let conflicts = vec![ConflictCall { session_id: "a".into(), port: 5173, at: T0 - 1000.0 }, ConflictCall { session_id: "a".into(), port: 1, at: T0 - 99.0 * 60_000.0 }];
    let r = sampler.sample(&SampleInput { terminals: &terminals, agents: &[], jobs: &[], idle_minutes: 0.0, last_device_call_at: None, conflicts: &conflicts, ports: Some(ports), now: T0 }).unwrap();
    assert_eq!(r["outside"], json!([{"pid": 90, "name": "postgres.exe", "ports": [{"port": 5432, "address": "0.0.0.0", "established": 0}]}]));
    assert_eq!(r["conflicts"], json!([{"port": 5173, "sessionId": "a", "at": (T0 as i64) - 1000, "holder": {"pid": 12, "name": "node.exe", "sessionId": "a"}}]));
    let server = &r["servers"][0];
    assert_eq!((server["kind"].as_str(), server["ports"][0]["established"].as_i64()), (Some("dev-server"), Some(1)));
}

#[test]
#[ignore]
fn measures_a_real_pass_over_this_machine() {
    use core_sources::resources::procs::CoreReader;
    let me = std::process::id();
    let t: TerminalSession = serde_json::from_value(json!({"id": "me", "title": "t", "status": "idle", "processId": me, "lastActivityAt": "2026-11-01T10:00:00.000Z"})).unwrap();
    let mut sampler = ResourceSampler::new(Box::new(CoreReader::host()));
    let mut total = std::time::Duration::ZERO;
    let mut last = Value::Null;
    for i in 0..5 {
        let started = std::time::Instant::now();
        let ports = core_procs::ports().map(|rows| ports_by_pid(&rows));
        last = sampler.sample(&SampleInput { terminals: std::slice::from_ref(&t), agents: &[], jobs: &[], idle_minutes: 60.0, last_device_call_at: None, conflicts: &[], ports, now: T0 + f64::from(i) * 30_000.0 }).unwrap();
        total += started.elapsed();
    }
    println!("5 passes: {total:?}, sessions {}, servers {}, outside {}", last["sessions"].as_array().unwrap().len(), last["servers"].as_array().unwrap().len(), last["outside"].as_array().map_or(0, Vec::len));
}
