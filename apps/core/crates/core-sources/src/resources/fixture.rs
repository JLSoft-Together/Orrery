use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::idle::ResPort;
use super::ports::PortMap;
use super::procs::{ProcReader, ProcRow};

pub struct Fixture {
    pub procs: Vec<ProcRow>,
    pub ports: PortMap,
}

fn num(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64).filter(|n| n.is_finite() && *n >= 0.0)
}

fn pid_of(v: Option<&Value>) -> Option<u32> {
    num(v).filter(|n| n.fract() == 0.0 && *n <= f64::from(u32::MAX)).map(|n| n as u32)
}

fn integer(v: Option<&Value>) -> Option<i64> {
    let n = v?.as_f64()?;
    (n.fract() == 0.0 && n.abs() < 9_007_199_254_740_992.0).then_some(n as i64)
}

pub fn parse_fixture(text: &str) -> Option<Fixture> {
    let raw: Value = serde_json::from_str(text).ok()?;
    let mut procs = Vec::new();
    for p in raw.get("procs").and_then(Value::as_array).into_iter().flatten() {
        let Some(obj) = p.as_object() else { continue };
        let (Some(pid), Some(ppid)) = (pid_of(obj.get("pid")), pid_of(obj.get("ppid"))) else { continue };
        let start = obj.get("start").and_then(Value::as_str).map(str::to_owned);
        let start_ms = start.as_deref().filter(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())).and_then(|s| s.parse::<f64>().ok());
        procs.push(ProcRow {
            pid,
            ppid,
            name: obj.get("name").and_then(Value::as_str).unwrap_or("").to_owned(),
            start,
            start_ms,
            ram_bytes: num(obj.get("ram")),
            cpu_ms: num(obj.get("cpuMs")),
            cmd: Some(obj.get("cmd").and_then(Value::as_str).map(str::to_owned)),
        });
    }
    let mut ports = PortMap::default();
    if let Some(map) = raw.get("ports").and_then(Value::as_object) {
        for (pid, list) in map {
            if pid.is_empty() || !pid.bytes().all(|b| b.is_ascii_digit()) {
                continue;
            }
            let Some(list) = list.as_array() else { continue };
            let Ok(pid) = pid.parse::<u32>() else { continue };
            let items: Vec<ResPort> = list
                .iter()
                .filter_map(|x| {
                    let port = integer(x.get("port"))?;
                    let address = x.get("address")?.as_str()?.to_owned();
                    let established = integer(x.get("established"))?;
                    Some(ResPort { port, address, established })
                })
                .collect();
            ports.insert(pid, items);
        }
    }
    Some(Fixture { procs, ports })
}

pub fn load_fixture(file: &Path) -> Option<Fixture> {
    parse_fixture(&String::from_utf8_lossy(&std::fs::read(file).ok()?))
}

pub struct FixtureReader {
    file: PathBuf,
}

impl FixtureReader {
    pub fn new(file: impl Into<PathBuf>) -> Self {
        Self { file: file.into() }
    }

    pub fn ports(&self) -> Option<PortMap> {
        load_fixture(&self.file).map(|f| f.ports)
    }
}

impl ProcReader for FixtureReader {
    fn read(&mut self) -> Option<Vec<ProcRow>> {
        load_fixture(&self.file).map(|f| f.procs)
    }

    fn enrich(&mut self, _rows: &mut [ProcRow], _usage: &HashSet<u32>, _cmd: &HashSet<u32>) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_procs_and_ports_and_drops_bad_rows() {
        let text = r#"{"procs":[{"pid":10,"ppid":1,"name":"claude","start":"1700","ram":5,"cpuMs":2,"cmd":"claude x"},{"pid":-1,"ppid":1},{"pid":11,"ppid":10,"start":"a1"},"x"],
          "ports":{"11":[{"port":3000,"address":"::","established":2},{"port":"x","address":"a","established":0}],"zz":[],"12":5}}"#;
        let f = parse_fixture(text).unwrap();
        assert_eq!(f.procs.len(), 2);
        assert_eq!(f.procs[0].start_ms, Some(1700.0));
        assert_eq!(f.procs[0].cmd_text(), Some("claude x"));
        assert_eq!((f.procs[1].start_ms, f.procs[1].ram_bytes, f.procs[1].cmd.clone()), (None, None, Some(None)));
        assert_eq!(f.ports.len(), 1);
        assert_eq!(f.ports.get(&11).unwrap().len(), 1);
        assert!(parse_fixture("{nope").is_none());
    }
}
