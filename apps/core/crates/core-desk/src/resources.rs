use crate::{arg, js, util, Ctx, Desk, DeskEnv};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::io::Write;

pub const MODULE: &str = "resources";
const KILL_MAX: usize = 50;
const LABEL_MAX: usize = 120;
const BUSY_GUARD_MS: f64 = 2.0 * 60_000.0;

#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub pid: i64,
    pub start: String,
    pub label: Option<String>,
}

fn start_ok(s: &str) -> bool {
    (1..=40).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b' ' | b':' | b'.' | b'+' | b'-'))
}

pub fn parse_targets(data: &Value) -> Option<Vec<Target>> {
    let list = data.as_array().filter(|l| !l.is_empty() && l.len() <= KILL_MAX)?;
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for raw in list {
        let t = raw.as_object()?;
        let pid = t.get("pid")?.as_f64().filter(|p| p.fract() == 0.0 && *p > 4.0 && *p <= 2_147_483_647.0)? as i64;
        let start = t.get("start")?.as_str().filter(|s| start_ok(s))?.to_string();
        let label = match t.get("label") {
            None => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(_) => return None,
        };
        if !seen.insert(pid) {
            continue;
        }
        let label = label.map(|l| l.chars().map(|c| if (c as u32) < 0x20 || c as u32 == 0x7f { ' ' } else { c }).collect::<String>()).map(|l| js::slice(crate::claude::js_trim(&l), LABEL_MAX)).filter(|l| !l.is_empty());
        out.push(Target { pid, start, label });
    }
    Some(out)
}

pub fn plan(report: &Value, targets: &[Target], now: f64) -> Result<Vec<Value>, &'static str> {
    if !report.is_object() {
        return Err("unavailable");
    }
    let list = |k: &str| report.get(k).and_then(Value::as_array).cloned().unwrap_or_default();
    let (sessions, servers) = (list("sessions"), list("servers"));
    let mut items = Vec::new();
    for t in targets {
        let same = |x: &Value| x.get("pid").and_then(Value::as_f64) == Some(t.pid as f64) && x.get("start").and_then(Value::as_str) == Some(t.start.as_str());
        if let Some(s) = sessions.iter().find(|x| same(x)) {
            let name = t.label.clone().map(Value::String).or_else(|| s.get("name").filter(|n| !n.is_null()).cloned()).unwrap_or_else(|| json!(t.pid.to_string()));
            items.push(json!({"pid": t.pid, "start": t.start, "name": name, "ramBytes": s.get("ramBytes").cloned().unwrap_or(Value::Null), "cmd": "", "session": true}));
            continue;
        }
        let Some(v) = servers.iter().find(|x| same(x) && !t.start.is_empty()) else { return Err("gone") };
        let busy = v.get("lastBusyAt").and_then(Value::as_f64);
        if v.get("kind").and_then(Value::as_str) == Some("daemon") && busy.is_some_and(|b| now - b < BUSY_GUARD_MS) {
            return Err("busy");
        }
        let mut m = Map::new();
        m.insert("pid".into(), json!(t.pid));
        m.insert("start".into(), json!(t.start));
        m.insert("name".into(), t.label.clone().map(Value::String).unwrap_or_else(|| v.get("name").cloned().unwrap_or(Value::Null)));
        m.insert("ramBytes".into(), v.get("ramBytes").cloned().unwrap_or(Value::Null));
        m.insert("cmd".into(), v.get("cmd").cloned().unwrap_or(json!("")));
        m.insert("session".into(), json!(false));
        if v.get("kind").and_then(Value::as_str) == Some("emulator") {
            if let Some(avd) = v.get("daemon").and_then(|d| d.get("avd")).filter(|a| a.as_str().is_some_and(|s| !s.is_empty())) {
                m.insert("avd".into(), avd.clone());
            }
        }
        items.push(Value::Object(m));
    }
    Ok(items)
}

fn collapse(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn live_starts(pids: &[i64]) -> HashMap<i64, String> {
    let want: HashSet<i64> = pids.iter().copied().collect();
    core_procs::table()
        .unwrap_or_default()
        .into_iter()
        .filter(|p| want.contains(&(p.pid as i64)))
        .filter_map(|p| p.start_raw.map(|s| (p.pid as i64, if cfg!(windows) { s } else { collapse(&s) })))
        .collect()
}

#[cfg(windows)]
fn kill_tree(pid: i64) -> bool {
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    let exe = format!("{root}\\System32\\taskkill.exe");
    use std::os::windows::process::CommandExt;
    let ok = std::process::Command::new(exe)
        .args(["/T", "/F", "/PID", &pid.to_string()])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .creation_flags(0x0800_0000)
        .status()
        .is_ok_and(|s| s.success());
    ok || !live_starts(&[pid]).contains_key(&pid)
}

#[cfg(unix)]
fn kill_tree(pid: i64) -> bool {
    let rows = core_procs::pid_parents().unwrap_or_default();
    let mut kids: HashMap<u32, Vec<u32>> = HashMap::new();
    for (p, pp) in rows {
        if p != pp {
            kids.entry(pp).or_default().push(p);
        }
    }
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    fn walk(pid: u32, depth: u32, kids: &HashMap<u32, Vec<u32>>, seen: &mut HashSet<u32>, out: &mut Vec<u32>) {
        if depth > 64 || !seen.insert(pid) {
            return;
        }
        for k in kids.get(&pid).into_iter().flatten() {
            walk(*k, depth + 1, kids, seen, out);
        }
        out.push(pid);
    }
    walk(pid as u32, 0, &kids, &mut seen, &mut out);
    for p in &out {
        unsafe { libc::kill(*p as i32, libc::SIGTERM) };
    }
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(5));
        for p in out {
            unsafe { libc::kill(p as i32, libc::SIGKILL) };
        }
    });
    true
}

fn fail(e: &str) -> Value {
    json!({"ok": false, "error": e})
}

pub fn res_kill(ctx: &Ctx, raw: &Value) -> Value {
    res_kill_with(ctx, raw, &|items| matches!(ctx.host("resources.confirm", json!({"items": items})), Ok(Value::Bool(true))))
}

pub fn res_kill_with(ctx: &Ctx, raw: &Value, confirm: &dyn Fn(&[Value]) -> bool) -> Value {
    let Some(targets) = parse_targets(raw) else { return fail("invalid") };
    let report = ctx.host("resources.report", Value::Null).unwrap_or(Value::Null);
    let items = match plan(&report, &targets, util::now_ms()) {
        Ok(i) => i,
        Err(e) => return fail(e),
    };
    let fixture = std::env::var("CCM_RES_FIXTURE").ok().filter(|v| !v.is_empty());
    let pid_of = |i: &Value| i["pid"].as_i64().unwrap_or(0);
    let start_of = |i: &Value| i["start"].as_str().unwrap_or("").to_string();
    let pids: Vec<i64> = items.iter().map(pid_of).collect();
    if fixture.is_none() {
        let live = live_starts(&pids);
        if !items.iter().all(|i| live.get(&pid_of(i)) == Some(&start_of(i))) {
            return fail("gone");
        }
    }
    let auto_yes = fixture.is_some() && std::env::var("CCM_E2E_RES_CONFIRM").ok().as_deref() == Some("1");
    if !auto_yes && !confirm(&items) {
        return fail("cancelled");
    }
    if let Some(f) = fixture {
        if let Ok(mut out) = std::fs::OpenOptions::new().create(true).append(true).open(format!("{f}.killed")) {
            let _ = out.write_all(pids.iter().map(|p| format!("{p}\n")).collect::<String>().as_bytes());
        }
        return json!({"ok": true, "killed": items.len()});
    }
    let live = live_starts(&pids);
    let mut killed = 0;
    for i in &items {
        if live.get(&pid_of(i)) != Some(&start_of(i)) {
            continue;
        }
        if let Some(avd) = i.get("avd") {
            if ctx.host("resources.avdStop", json!({"avd": avd})).ok() == Some(Value::Bool(true)) {
                killed += 1;
                continue;
            }
        }
        if kill_tree(pid_of(i)) {
            killed += 1;
        }
    }
    if killed > 0 {
        json!({"ok": true, "killed": killed})
    } else {
        fail("failed")
    }
}

pub fn register(desk: &mut Desk, _env: &DeskEnv) {
    desk.on(MODULE, "ccm:res-kill", |ctx, args| Ok(res_kill(ctx, arg(args, 0))));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn targets_and_plan() {
        assert!(parse_targets(&json!([])).is_none());
        assert!(parse_targets(&json!([{"pid": 3, "start": "x"}])).is_none());
        assert!(parse_targets(&json!([{"pid": 10, "start": "bad/start"}])).is_none());
        let t = parse_targets(&json!([{"pid": 10, "start": "123", "label": " a\u{1}b "}, {"pid": 10, "start": "9"}])).unwrap();
        assert_eq!(t, vec![Target { pid: 10, start: "123".into(), label: Some("a b".into()) }]);
        let report = json!({"sessions": [{"pid": 10, "start": "123", "name": null, "ramBytes": 5}], "servers": [{"pid": 11, "start": "7", "name": "gradle", "ramBytes": null, "cmd": "java", "kind": "daemon", "lastBusyAt": 1000.0}]});
        assert_eq!(plan(&Value::Null, &t, 0.0), Err("unavailable"));
        assert_eq!(plan(&report, &t, 0.0).unwrap()[0]["name"], "a b");
        let busy = vec![Target { pid: 11, start: "7".into(), label: None }];
        assert_eq!(plan(&report, &busy, 2000.0), Err("busy"));
        assert_eq!(plan(&report, &busy, 1e9).unwrap()[0]["cmd"], "java");
        assert_eq!(plan(&report, &[Target { pid: 12, start: "1".into(), label: None }], 0.0), Err("gone"));
        let me = std::process::id() as i64;
        assert!(live_starts(&[me]).contains_key(&me));
    }
}
