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

pub fn read_pid_table() -> Option<PidTable> {
    let t: PidTable = core_procs::pid_parents()?.into_iter().map(|(p, pp)| (u64::from(p), u64::from(pp))).collect();
    (!t.is_empty()).then_some(t)
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
    fn real_table_has_this_process_and_its_parent() {
        let t = read_pid_table().expect("process table");
        let me = std::process::id() as u64;
        let parent = *t.get(&me).expect("own pid");
        assert!(t.contains_key(&parent), "parent {parent} listed");
    }
}
