use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub const MAX_DEPTH: usize = 32;
pub const MAX_PIDS: usize = 64;
pub const TABLE_TTL: Duration = Duration::from_millis(1500);

pub type PidTable = HashMap<u64, u64>;

pub fn wanted_pids(v: Option<&Value>) -> Vec<u64> {
    let Some(Value::Array(items)) = v else { return Vec::new() };
    items
        .iter()
        .filter_map(|p| {
            let f = p.as_f64()?;
            (f.fract() == 0.0 && f > 0.0 && f <= u64::MAX as f64).then_some(f as u64)
        })
        .take(MAX_PIDS)
        .collect()
}

pub fn owners_of(pids: &[u64], roots: &HashMap<u64, String>, parents: &PidTable) -> BTreeMap<u64, String> {
    let mut out = BTreeMap::new();
    for &pid in pids {
        let mut cur = pid;
        for _ in 0..MAX_DEPTH {
            if cur == 0 {
                break;
            }
            if let Some(owner) = roots.get(&cur) {
                out.entry(pid).or_insert_with(|| owner.clone());
                break;
            }
            match parents.get(&cur) {
                Some(&next) if next != cur => cur = next,
                _ => break,
            }
        }
    }
    out
}

pub fn to_json(owners: &BTreeMap<u64, String>) -> Value {
    let mut o = Map::new();
    for (pid, id) in owners {
        o.insert(pid.to_string(), Value::String(id.clone()));
    }
    Value::Object(o)
}

pub fn parse_ps(stdout: &str) -> PidTable {
    let mut t = PidTable::new();
    for line in stdout.lines() {
        let mut it = line.split_whitespace();
        let (Some(a), Some(b), None) = (it.next(), it.next(), it.next()) else { continue };
        if let (Ok(pid), Ok(ppid)) = (a.parse::<u64>(), b.parse::<u64>()) {
            t.insert(pid, ppid);
        }
    }
    t
}

#[cfg(windows)]
pub fn read_pid_table() -> Option<PidTable> {
    use crate::jobs::windows::Owned;
    use std::mem::{size_of, zeroed};
    use windows_sys::Win32::Foundation::INVALID_HANDLE_VALUE;
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
    let snap = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snap == INVALID_HANDLE_VALUE {
        return None;
    }
    let snap = Owned(snap);
    let mut e: PROCESSENTRY32W = unsafe { zeroed() };
    e.dwSize = size_of::<PROCESSENTRY32W>() as u32;
    let mut t = PidTable::new();
    let mut more = unsafe { Process32FirstW(snap.0, &mut e) } != 0;
    while more {
        t.insert(e.th32ProcessID as u64, e.th32ParentProcessID as u64);
        more = unsafe { Process32NextW(snap.0, &mut e) } != 0;
    }
    (!t.is_empty()).then_some(t)
}

#[cfg(unix)]
pub fn read_pid_table() -> Option<PidTable> {
    let out = run_with_deadline("/bin/ps", &["-A", "-o", "pid=", "-o", "ppid="], PS_DEADLINE)?;
    let t = parse_ps(&out);
    (!t.is_empty()).then_some(t)
}

#[cfg_attr(windows, allow(dead_code))]
pub const PS_DEADLINE: Duration = Duration::from_secs(15);

#[cfg_attr(windows, allow(dead_code))]
pub fn run_with_deadline(file: &str, args: &[&str], deadline: Duration) -> Option<String> {
    use std::io::Read;
    use std::process::{Command, Stdio};
    let mut child = Command::new(file).args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()).spawn().ok()?;
    let mut stdout = child.stdout.take()?;
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::Builder::new()
        .name("owners-ps".into())
        .spawn(move || {
            let mut buf = Vec::new();
            let ok = stdout.read_to_end(&mut buf).is_ok();
            let _ = tx.send(ok.then_some(buf));
        })
        .ok()?;
    let end = Instant::now() + deadline;
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) if Instant::now() < end => std::thread::sleep(Duration::from_millis(10)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };
    let out = rx.recv_timeout(end.saturating_duration_since(Instant::now()).max(Duration::from_millis(200))).ok()??;
    status.success().then(|| String::from_utf8_lossy(&out).into_owned())
}

pub struct Ask {
    pub conn: u64,
    pub pids: Vec<u64>,
    pub roots: HashMap<u64, String>,
}

#[derive(Default)]
pub struct TableCache {
    table: Option<(Instant, Option<Arc<PidTable>>)>,
    scanning: bool,
    waiting: Vec<Ask>,
}

pub enum Lookup {
    Ready(Option<Arc<PidTable>>),
    Scan,
    Wait,
}

impl TableCache {
    pub fn lookup(&mut self, now: Instant, ask: Ask) -> (Lookup, Option<Ask>) {
        if let Some((at, t)) = &self.table {
            if now.saturating_duration_since(*at) <= TABLE_TTL {
                return (Lookup::Ready(t.clone()), Some(ask));
            }
        }
        self.waiting.push(ask);
        if self.scanning {
            return (Lookup::Wait, None);
        }
        self.scanning = true;
        (Lookup::Scan, None)
    }

    pub fn scanned(&mut self, started: Instant, table: Option<PidTable>) -> (Option<Arc<PidTable>>, Vec<Ask>) {
        let t = table.map(Arc::new);
        self.table = Some((started, t.clone()));
        self.scanning = false;
        (t, std::mem::take(&mut self.waiting))
    }
}

pub fn answer(ask: &Ask, table: Option<&PidTable>) -> Value {
    match table {
        None => Value::Null,
        Some(t) => to_json(&owners_of(&ask.pids, &ask.roots, t)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn roots() -> HashMap<u64, String> {
        HashMap::from([(100, "P1".to_owned()), (200, "P2".to_owned())])
    }

    #[test]
    fn walks_three_levels_and_more() {
        let parents: PidTable = HashMap::from([(100, 4), (101, 100), (102, 101), (103, 102), (300, 1), (1, 0), (200, 4)]);
        let got = owners_of(&[103, 102, 101, 100, 300, 999, 200], &roots(), &parents);
        assert_eq!(got, BTreeMap::from([(100, "P1".into()), (101, "P1".into()), (102, "P1".into()), (103, "P1".into()), (200, "P2".into())]));
        assert_eq!(to_json(&got)["103"], json!("P1"));
    }

    #[test]
    fn stops_at_depth_32_cycles_and_self_parents() {
        let mut parents = PidTable::new();
        for p in 1001..=1040u64 {
            parents.insert(p, p - 1);
        }
        parents.insert(1000, 100);
        let r = roots();
        assert_eq!(owners_of(&[1030], &r, &parents).get(&1030).map(String::as_str), Some("P1"));
        assert!(owners_of(&[1031], &r, &parents).is_empty());
        let cyc: PidTable = HashMap::from([(5, 6), (6, 5), (7, 7)]);
        assert!(owners_of(&[5, 7], &r, &cyc).is_empty());
    }

    #[test]
    fn wanted_pids_mirror_node_filter() {
        let v = json!([1, 0, -3, 2.5, "7", 8.0, null, 9]);
        assert_eq!(wanted_pids(Some(&v)), vec![1, 8, 9]);
        let many: Vec<u64> = (1..=100).collect();
        assert_eq!(wanted_pids(Some(&json!(many))).len(), MAX_PIDS);
        assert!(wanted_pids(Some(&json!("x"))).is_empty());
        assert!(wanted_pids(None).is_empty());
    }

    #[test]
    fn parse_ps_rows() {
        let t = parse_ps("    1     0\n  412     1\nbad row\n 9 8 7\n\n");
        assert_eq!(t, HashMap::from([(1, 0), (412, 1)]));
    }

    #[test]
    fn cache_shares_one_scan_per_burst_and_expires_after_1500ms() {
        let mut c = TableCache::default();
        let t0 = Instant::now();
        let ask = |conn| Ask { conn, pids: vec![101], roots: roots() };
        assert!(matches!(c.lookup(t0, ask(1)).0, Lookup::Scan));
        assert!(matches!(c.lookup(t0, ask(2)).0, Lookup::Wait));
        let (table, waiting) = c.scanned(t0, Some(HashMap::from([(101, 100)])));
        assert_eq!(waiting.iter().map(|a| a.conn).collect::<Vec<_>>(), vec![1, 2]);
        assert_eq!(answer(&waiting[0], table.as_deref()), json!({ "101": "P1" }));
        let (hit, back) = c.lookup(t0 + Duration::from_millis(1500), ask(3));
        assert!(matches!(hit, Lookup::Ready(Some(_))));
        assert_eq!(back.unwrap().conn, 3);
        assert!(matches!(c.lookup(t0 + Duration::from_millis(1501), ask(4)).0, Lookup::Scan));
        let (failed, _) = c.scanned(t0 + Duration::from_millis(1501), None);
        assert!(failed.is_none());
        let (miss, back) = c.lookup(t0 + Duration::from_millis(1600), ask(5));
        assert!(matches!(miss, Lookup::Ready(None)));
        assert_eq!(answer(&back.unwrap(), None), Value::Null);
    }

    #[test]
    fn table_command_is_killed_at_its_deadline() {
        #[cfg(windows)]
        let slow: &[&str] = &["powershell.exe", "-NoProfile", "-Command", "Start-Sleep -Seconds 20"];
        #[cfg(windows)]
        let quick: &[&str] = &["cmd.exe", "/d", "/c", "echo 12 1"];
        #[cfg(unix)]
        let slow: &[&str] = &["/bin/sleep", "20"];
        #[cfg(unix)]
        let quick: &[&str] = &["/bin/echo", "12 1"];
        let t = Instant::now();
        assert!(run_with_deadline(slow[0], &slow[1..], Duration::from_millis(500)).is_none());
        assert!(t.elapsed() < Duration::from_secs(5), "deadline not honoured: {:?}", t.elapsed());
        assert_eq!(parse_ps(&run_with_deadline(quick[0], &quick[1..], Duration::from_secs(10)).unwrap()), HashMap::from([(12, 1)]));
        assert!(run_with_deadline("/definitely/missing/ps", &[], Duration::from_secs(1)).is_none());
    }

    #[test]
    fn real_table_has_this_process_and_its_parent() {
        let t = read_pid_table().expect("process table");
        let me = std::process::id() as u64;
        assert!(t.contains_key(&me));
    }
}
