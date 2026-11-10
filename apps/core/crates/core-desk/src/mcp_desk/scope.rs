use crate::resources::Target;
use crate::util;
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[derive(Debug, Clone, PartialEq)]
pub struct ProcRow {
    pub pid: i64,
    pub ppid: i64,
    pub start_ms: Option<f64>,
}

impl ProcRow {
    pub fn parse(v: &Value) -> Option<ProcRow> {
        Some(ProcRow { pid: v.get("pid")?.as_i64()?, ppid: v.get("ppid")?.as_i64()?, start_ms: v.get("startMs").and_then(Value::as_f64) })
    }
}

#[derive(Debug, Clone)]
pub struct PaneScope {
    pub root: String,
    pub pane_pid: Option<i64>,
    pub tree: HashSet<i64>,
    pub project_rule: bool,
}

pub fn tree_pids(root: i64, rows: &[ProcRow]) -> Vec<i64> {
    let start_of: HashMap<i64, Option<f64>> = rows.iter().map(|r| (r.pid, r.start_ms)).collect();
    let mut kids: HashMap<i64, Vec<i64>> = HashMap::new();
    for r in rows {
        if r.pid == r.ppid {
            continue;
        }
        if let (Some(Some(parent)), Some(own)) = (start_of.get(&r.ppid), r.start_ms) {
            if *parent > own {
                continue;
            }
        }
        kids.entry(r.ppid).or_default().push(r.pid);
    }
    fn walk(pid: i64, depth: u32, kids: &HashMap<i64, Vec<i64>>, seen: &mut HashSet<i64>, out: &mut Vec<i64>) {
        if depth > 64 || !seen.insert(pid) {
            return;
        }
        for k in kids.get(&pid).into_iter().flatten() {
            walk(*k, depth + 1, kids, seen, out);
        }
        out.push(pid);
    }
    let mut out = Vec::new();
    walk(root, 0, &kids, &mut HashSet::new(), &mut out);
    out
}

fn norm(x: &str, win: bool) -> String {
    let r = util::path_str(&util::resolve(x));
    if win {
        r.to_lowercase()
    } else {
        r
    }
}

pub fn pane_scope(root: &str, pane_pid: Option<i64>, rows: Option<&[ProcRow]>, home: &str, win: bool) -> PaneScope {
    let r = norm(root, win);
    let h = norm(home, win);
    let at_root = Path::new(&r).parent().is_none();
    let project_rule = !at_root && r != h && !util::is_inside_or_same(&r, &h);
    let tree = match (pane_pid.filter(|p| *p != 0), rows) {
        (Some(p), Some(rows)) => tree_pids(p, rows).into_iter().collect(),
        _ => HashSet::new(),
    };
    PaneScope { root: root.to_string(), pane_pid, tree, project_rule }
}

fn list<'a>(report: &'a Value, key: &str) -> &'a [Value] {
    report.get(key).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

fn pid_of(v: &Value) -> Option<i64> {
    v.get("pid").and_then(Value::as_i64)
}

fn ports_of(s: &Value) -> Vec<i64> {
    let mut seen = Vec::new();
    for p in list(s, "ports") {
        if let Some(n) = p.get("port").and_then(Value::as_i64) {
            if !seen.contains(&n) {
                seen.push(n);
            }
        }
    }
    seen
}

pub fn is_mine(report: &Value, scope: &PaneScope, s: &Value) -> bool {
    if s.get("kind").and_then(Value::as_str) == Some("emulator") {
        return false;
    }
    if pid_of(s).is_some_and(|p| scope.tree.contains(&p)) {
        return true;
    }
    if let Some(id) = s.get("sessionId").and_then(Value::as_str) {
        if list(report, "sessions").iter().any(|x| pid_of(x).is_some_and(|p| scope.tree.contains(&p)) && x.get("sessionId").and_then(Value::as_str) == Some(id)) {
            return true;
        }
    }
    scope.project_rule && s.get("cwd").and_then(Value::as_str).is_some_and(|c| !c.is_empty() && util::is_inside_or_same(&scope.root, c))
}

fn port_key(v: &Value) -> i64 {
    v["ports"].get(0).and_then(Value::as_i64).unwrap_or(1_000_000)
}

pub fn servers_for(report: &Value, scope: &PaneScope, now: f64) -> (Vec<Value>, Vec<Value>) {
    let mut servers: Vec<Value> = list(report, "servers")
        .iter()
        .filter(|s| !ports_of(s).is_empty() || s.get("kind").and_then(Value::as_str) == Some("dev-server"))
        .map(|s| {
            let own = is_mine(report, scope, s);
            let mut ports = ports_of(s);
            ports.sort_unstable();
            let mut m = Map::new();
            for k in ["pid", "name", "kind"] {
                if let Some(v) = s.get(k) {
                    m.insert(k.into(), v.clone());
                }
            }
            m.insert("ports".into(), json!(ports));
            m.insert("mine".into(), json!(own));
            if own {
                if let Some(cmd) = s.get("cmd").filter(|c| c.as_str().is_some_and(|c| !c.is_empty())) {
                    m.insert("cmd".into(), cmd.clone());
                }
            }
            let idle = s.get("idleSinceMs").and_then(Value::as_f64).map(|t| (((now - t) / 1000.0).round()).max(0.0) as i64);
            m.insert("idleSec".into(), idle.map(|i| json!(i)).unwrap_or(Value::Null));
            if let Some(o) = s.get("orphan") {
                m.insert("orphan".into(), o.clone());
            }
            Value::Object(m)
        })
        .collect();
    servers.sort_by(|a, b| {
        let (ma, mb) = (a["mine"] == json!(true), b["mine"] == json!(true));
        mb.cmp(&ma).then(port_key(a).cmp(&port_key(b)))
    });
    let outside = list(report, "outside")
        .iter()
        .flat_map(|o| ports_of(o).into_iter().map(move |port| json!({"port": port, "name": o.get("name").cloned().unwrap_or(Value::Null)})))
        .collect();
    (servers, outside)
}

pub fn target_json(t: &Target) -> Value {
    let mut m = Map::new();
    m.insert("pid".into(), json!(t.pid));
    m.insert("start".into(), json!(t.start));
    if let Some(l) = &t.label {
        m.insert("label".into(), json!(l));
    }
    Value::Object(m)
}

fn err(code: &str) -> Value {
    json!({"ok": false, "error": code})
}

pub fn plan_free_port(report: Option<&Value>, scope: &PaneScope, port: i64, self_pids: &[i64]) -> Result<Vec<Target>, Value> {
    let Some(report) = report else { return Err(err("no-report")) };
    let servers = list(report, "servers");
    let holders: Vec<&Value> = servers.iter().filter(|s| list(s, "ports").iter().any(|p| p.get("port").and_then(Value::as_i64) == Some(port))).collect();
    if holders.is_empty() {
        let out = list(report, "outside").iter().find(|o| list(o, "ports").iter().any(|p| p.get("port").and_then(Value::as_i64) == Some(port)));
        return Err(match out {
            Some(o) => json!({"ok": false, "error": "outside", "detail": o.get("name").cloned().unwrap_or(Value::Null)}),
            None => err("not-found"),
        });
    }
    let session_pids: HashSet<i64> = list(report, "sessions").iter().filter_map(pid_of).collect();
    let kind = |s: &Value| s.get("kind").and_then(Value::as_str).unwrap_or("").to_string();
    if holders.iter().any(|s| kind(s) == "emulator") {
        return Err(err("emulator"));
    }
    if holders.iter().any(|s| pid_of(s).is_some_and(|p| Some(p) == scope.pane_pid || session_pids.contains(&p) || self_pids.contains(&p))) {
        return Err(err("self"));
    }
    if let Some(f) = holders.iter().find(|s| !is_mine(report, scope, s)) {
        return Err(json!({"ok": false, "error": "not-mine", "detail": f.get("name").cloned().unwrap_or(Value::Null)}));
    }
    if holders.iter().any(|s| s.get("start").and_then(Value::as_str).is_none_or(str::is_empty)) {
        return Err(err("gone"));
    }
    Ok(holders
        .iter()
        .map(|s| Target {
            pid: pid_of(s).unwrap_or(0),
            start: s["start"].as_str().unwrap_or("").to_string(),
            label: Some(format!("{} :{port}", s.get("name").and_then(Value::as_str).unwrap_or(""))),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<ProcRow> {
        vec![
            ProcRow { pid: 10, ppid: 1, start_ms: None },
            ProcRow { pid: 11, ppid: 10, start_ms: None },
            ProcRow { pid: 12, ppid: 11, start_ms: None },
            ProcRow { pid: 20, ppid: 1, start_ms: None },
            ProcRow { pid: 13, ppid: 10, start_ms: Some(1.0) },
        ]
    }

    #[test]
    fn tree_follows_children_and_skips_reused_ppid() {
        let mut r = rows();
        r[0].start_ms = Some(5.0);
        let t: HashSet<i64> = tree_pids(10, &r).into_iter().collect();
        assert_eq!(t, HashSet::from([10, 11, 12]));
        assert_eq!(tree_pids(99, &r), vec![99]);
    }

    #[test]
    fn project_rule_is_off_for_roots_and_home() {
        let r = rows();
        assert!(!pane_scope("/", None, None, "/Users/a", false).project_rule);
        assert!(!pane_scope("/Users/a", None, None, "/Users/a", false).project_rule);
        assert!(!pane_scope("/Users", None, None, "/Users/a", false).project_rule);
        let s = pane_scope("/Users/a/app", Some(10), Some(&r), "/Users/a", false);
        assert!(s.project_rule);
        assert!(s.tree.contains(&12));
    }

    fn report() -> Value {
        json!({
            "at": "t",
            "sessions": [{"pid": 11, "sessionId": "s1"}],
            "servers": [
                {"pid": 12, "start": "1", "name": "vite", "kind": "dev-server", "ports": [{"port": 5173}], "cwd": "/Users/a/app", "cmd": "node vite", "idleSinceMs": 1000.0, "orphan": false},
                {"pid": 30, "start": "2", "name": "other", "kind": "dev-server", "ports": [{"port": 3000}, {"port": 3000}], "cwd": "/x", "cmd": "secret", "idleSinceMs": null, "orphan": true},
                {"pid": 31, "start": "3", "name": "qemu", "kind": "emulator", "ports": [{"port": 5554}]},
                {"pid": 32, "start": "", "name": "blank", "kind": "dev-server", "ports": [{"port": 4000}], "cwd": "/Users/a/app"},
                {"pid": 33, "start": "9", "name": "idle", "kind": "dev-server", "ports": [], "sessionId": "s1"}
            ],
            "outside": [{"name": "postgres", "ports": [{"port": 5432}, {"port": 5432}]}]
        })
    }

    #[test]
    fn servers_mine_first_and_cmd_only_for_mine() {
        let scope = pane_scope("/Users/a/app", Some(10), Some(&rows()), "/Users/a", false);
        let (servers, outside) = servers_for(&report(), &scope, 6000.0);
        let names: Vec<&str> = servers.iter().map(|s| s["name"].as_str().unwrap()).collect();
        assert_eq!(names, ["blank", "vite", "idle", "other", "qemu"]);
        assert_eq!(servers[1]["cmd"], "node vite");
        assert_eq!(servers[1]["idleSec"], 5);
        assert!(servers[3].get("cmd").is_none());
        assert_eq!(servers[3]["idleSec"], Value::Null);
        assert_eq!(servers[3]["ports"], json!([3000]));
        assert_eq!(outside, vec![json!({"port": 5432, "name": "postgres"})]);
    }

    #[test]
    fn plan_refuses_foreign_outside_emulator_self_and_gone() {
        let scope = pane_scope("/Users/a/app", Some(10), Some(&rows()), "/Users/a", false);
        let rep = report();
        let code = |p: i64| plan_free_port(Some(&rep), &scope, p, &[]).unwrap_err();
        assert_eq!(code(3000)["error"], "not-mine");
        assert_eq!(code(3000)["detail"], "other");
        assert_eq!(code(5432), json!({"ok": false, "error": "outside", "detail": "postgres"}));
        assert_eq!(code(5554)["error"], "emulator");
        assert_eq!(code(4000)["error"], "gone");
        assert_eq!(code(1)["error"], "not-found");
        assert_eq!(plan_free_port(None, &scope, 1, &[]).unwrap_err()["error"], "no-report");
        let t = plan_free_port(Some(&rep), &scope, 5173, &[]).unwrap();
        assert_eq!(t, vec![Target { pid: 12, start: "1".into(), label: Some("vite :5173".into()) }]);
        assert_eq!(plan_free_port(Some(&rep), &scope, 5173, &[12]).unwrap_err()["error"], "self");
    }
}
