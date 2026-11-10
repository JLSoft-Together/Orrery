use std::collections::{BTreeSet, HashMap};

use core_procs::tcp::{TCP_ESTABLISHED, TCP_LISTEN};
use core_procs::TcpRow;

use super::idle::ResPort;

type Listening = (Vec<u16>, HashMap<u16, BTreeSet<String>>);

pub const RES_CONFLICT_MS: f64 = 15.0 * 60_000.0;
pub const RES_OUTSIDE_MAX: usize = 100;

#[derive(Debug, Default, Clone, PartialEq)]
pub struct PortMap {
    order: Vec<u32>,
    map: HashMap<u32, Vec<ResPort>>,
}

impl PortMap {
    pub fn insert(&mut self, pid: u32, list: Vec<ResPort>) {
        if self.map.insert(pid, list).is_none() {
            self.order.push(pid);
        }
    }

    pub fn get(&self, pid: &u32) -> Option<&Vec<ResPort>> {
        self.map.get(pid)
    }

    pub fn contains(&self, pid: u32) -> bool {
        self.map.contains_key(&pid)
    }

    pub fn iter(&self) -> impl Iterator<Item = (u32, &Vec<ResPort>)> {
        self.order.iter().map(|p| (*p, &self.map[p]))
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
}

pub fn ports_by_pid(rows: &[TcpRow]) -> PortMap {
    let mut listen_order: Vec<u32> = Vec::new();
    let mut listen: HashMap<u32, Listening> = HashMap::new();
    let mut established: HashMap<u16, i64> = HashMap::new();
    for r in rows {
        if r.pid <= 4 {
            continue;
        }
        if r.state == TCP_LISTEN {
            let entry = listen.entry(r.pid).or_insert_with(|| {
                listen_order.push(r.pid);
                (Vec::new(), HashMap::new())
            });
            if !entry.1.contains_key(&r.port) {
                entry.0.push(r.port);
            }
            entry.1.entry(r.port).or_default().insert(r.address.clone());
        } else if r.state == TCP_ESTABLISHED {
            *established.entry(r.port).or_insert(0) += 1;
        }
    }
    let mut out = PortMap::default();
    for pid in listen_order {
        let (order, mut ports) = listen.remove(&pid).unwrap_or_default();
        let mut list: Vec<ResPort> = order
            .into_iter()
            .map(|port| {
                let addrs = ports.remove(&port).unwrap_or_default();
                ResPort { port: i64::from(port), address: addrs.into_iter().collect::<Vec<_>>().join(", "), established: established.get(&port).copied().unwrap_or(0) }
            })
            .collect();
        list.sort_by_key(|p| p.port);
        out.insert(pid, list);
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutsidePort {
    pub pid: u32,
    pub name: String,
    pub ports: Vec<ResPort>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Holder {
    pub pid: u32,
    pub name: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PortConflict {
    pub port: i64,
    pub session_id: String,
    pub at: f64,
    pub holder: Option<Holder>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConflictCall {
    pub session_id: String,
    pub port: i64,
    pub at: f64,
}

pub type Owners = HashMap<u32, Option<String>>;

pub fn outside_ports(ports: &PortMap, owner: &Owners, name_of: &HashMap<u32, String>) -> Vec<OutsidePort> {
    let mut out: Vec<OutsidePort> = ports
        .iter()
        .filter(|(pid, list)| !owner.contains_key(pid) && !list.is_empty())
        .map(|(pid, list)| OutsidePort { pid, name: name_of.get(&pid).cloned().unwrap_or_default(), ports: list.clone() })
        .collect();
    out.sort_by_key(|o| o.ports[0].port);
    out.truncate(RES_OUTSIDE_MAX);
    out
}

pub fn port_conflicts(calls: &[ConflictCall], now: f64, ports: &PortMap, owner: &Owners, name_of: &HashMap<u32, String>) -> Vec<PortConflict> {
    let mut holders: HashMap<i64, u32> = HashMap::new();
    for (pid, list) in ports.iter() {
        for p in list {
            holders.entry(p.port).or_insert(pid);
        }
    }
    let mut out: Vec<PortConflict> = calls
        .iter()
        .filter(|c| now - c.at <= RES_CONFLICT_MS && c.at <= now + 60_000.0)
        .map(|c| {
            let holder = holders.get(&c.port).map(|&pid| Holder { pid, name: name_of.get(&pid).cloned().unwrap_or_default(), session_id: owner.get(&pid).cloned().flatten() });
            PortConflict { port: c.port, session_id: c.session_id.clone(), at: c.at, holder }
        })
        .collect();
    out.sort_by(|a, b| b.at.total_cmp(&a.at));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(pid: u32, state: u32, address: &str, port: u16) -> TcpRow {
        TcpRow { pid, state, address: address.into(), port }
    }

    #[test]
    fn groups_listen_sockets_and_counts_connections_per_port() {
        let rows = vec![
            row(10, TCP_LISTEN, "127.0.0.1", 5173),
            row(10, TCP_LISTEN, "::1", 5173),
            row(10, TCP_LISTEN, "0.0.0.0", 80),
            row(4, TCP_LISTEN, "0.0.0.0", 445),
            row(99, TCP_ESTABLISHED, "127.0.0.1", 5173),
            row(98, TCP_ESTABLISHED, "127.0.0.1", 5173),
            row(11, TCP_LISTEN, "0.0.0.0", 3000),
        ];
        let m = ports_by_pid(&rows);
        assert_eq!(m.len(), 2);
        let p = m.get(&10).unwrap();
        assert_eq!(p.iter().map(|x| (x.port, x.address.as_str(), x.established)).collect::<Vec<_>>(), vec![(80, "0.0.0.0", 0), (5173, "127.0.0.1, ::1", 2)]);
        assert_eq!(m.iter().map(|(pid, _)| pid).collect::<Vec<_>>(), vec![10, 11]);
    }

    #[test]
    fn outside_lists_unowned_listeners_by_first_port_and_conflicts_name_the_holder() {
        let m = ports_by_pid(&[row(10, TCP_LISTEN, "a", 9000), row(11, TCP_LISTEN, "a", 3000), row(12, TCP_LISTEN, "a", 4000)]);
        let owner: Owners = HashMap::from([(12, Some("s1".to_owned())), (13, None)]);
        let names: HashMap<u32, String> = HashMap::from([(10, "node".to_owned()), (12, "vite".to_owned())]);
        let out = outside_ports(&m, &owner, &names);
        assert_eq!(out.iter().map(|o| (o.pid, o.name.as_str())).collect::<Vec<_>>(), vec![(11, ""), (10, "node")]);
        let calls = vec![
            ConflictCall { session_id: "s1".into(), port: 4000, at: 900_000.0 },
            ConflictCall { session_id: "s2".into(), port: 9000, at: 950_000.0 },
            ConflictCall { session_id: "s3".into(), port: 1, at: 1.0 },
            ConflictCall { session_id: "s4".into(), port: 7, at: 2_000_000.0 },
        ];
        let c = port_conflicts(&calls, 1_000_000.0, &m, &owner, &names);
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].session_id, "s2");
        assert_eq!(c[0].holder, Some(Holder { pid: 10, name: "node".into(), session_id: None }));
        assert_eq!(c[1].holder, Some(Holder { pid: 12, name: "vite".into(), session_id: Some("s1".into()) }));
    }
}
