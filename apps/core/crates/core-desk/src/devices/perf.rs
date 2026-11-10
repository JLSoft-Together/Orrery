use super::history::num;
use super::manager::DeviceManager;
use super::parse::{err, err_detail, is_ok};
use super::toggles::{is_package, sh_quote};
use crate::js;
use crate::run::android::jsre::{g, groups, Js};
use crate::run::parse::parse_resolved_activity;
use crate::util::now_ms;
use regress::Regex;
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub const PERF_DIR: &str = "perf";
pub const PERF_HISTORY_MAX: usize = 10;
const LAUNCHER: &str = "android.intent.category.LAUNCHER";
const DUMP_MS: u64 = 20_000;
const COLD_MS: u64 = 60_000;

fn not_running(out: &str) -> bool {
    static NR: Js = Js::new(r"No process found for", "i");
    NR.test(out)
}

fn fail(code: &str, detail: &str) -> Value {
    err_detail(code, detail)
}

fn collapsed(text: &str, max: usize) -> String {
    static WS: Js = Js::new(r"\s+", "g");
    js::slice(WS.replace_all(text, " ").trim(), max)
}

fn number(re: &Regex, text: &str) -> Value {
    re.find(text).and_then(|m| groups(&m, text)[1].as_deref().and_then(|s| s.parse::<f64>().ok())).map(num).unwrap_or(Value::Null)
}

fn section_of(out: &str, header: &Regex, pkg: &str) -> Option<String> {
    let matches: Vec<_> = header.find_iter(out).collect();
    if matches.is_empty() {
        return None;
    }
    let mut parts: Vec<String> = vec![out[..matches[0].start()].to_string()];
    for (i, m) in matches.iter().enumerate() {
        let gs = groups(m, out);
        parts.push(g(&gs, 1).unwrap_or("").to_string());
        parts.push(g(&gs, 2).unwrap_or("").to_string());
        let end = matches.get(i + 1).map(|n| n.start()).unwrap_or(out.len());
        parts.push(out[m.end()..end].to_string());
    }
    if parts.len() < 4 {
        return None;
    }
    let mut first: Option<String> = None;
    let mut i = 1;
    while i + 2 < parts.len() {
        let body = &parts[i + 2];
        if parts[i + 1] == pkg {
            return Some(body.clone());
        }
        first.get_or_insert_with(|| body.clone());
        i += 3;
    }
    first
}

#[derive(Debug)]
pub enum Parsed {
    NotRunning,
    Unparsed,
    Got(Value),
}

impl Parsed {
    fn value(self) -> Option<Value> {
        match self {
            Parsed::Got(v) => Some(v),
            _ => None,
        }
    }
}

pub fn parse_gfxinfo(out: &str, pkg: &str) -> Parsed {
    static HEAD: Js = Js::new(r"\*\* Graphics info for pid (\d+) \[([^\]]+)\] \*\*", "");
    static TOTAL: Js = Js::new(r"Total frames rendered:\s*(\d+)", "");
    static JANK: Js = Js::new(r"^\s*Janky frames:\s*(\d+)(?:\s*\(([\d.]+)%\))?", "m");
    if not_running(out) {
        return Parsed::NotRunning;
    }
    let body = section_of(out, HEAD.re(), pkg).unwrap_or_else(|| out.to_string());
    let Some(total) = TOTAL.exec(&body).and_then(|m| g(&m, 1).and_then(|s| s.parse::<f64>().ok())) else { return Parsed::Unparsed };
    let jank = JANK.exec(&body);
    let janky = jank.as_ref().and_then(|j| g(j, 1)).and_then(|s| s.parse::<f64>().ok()).unwrap_or(0.0);
    let pct = match jank.as_ref().and_then(|j| g(j, 2)) {
        Some(p) => p.parse::<f64>().unwrap_or(f64::NAN),
        None if total != 0.0 => (janky / total) * 100.0,
        None => 0.0,
    };
    let pc = |n: u32| number(&Regex::with_flags(&format!(r"^\s*{n}th percentile:\s*(\d+(?:\.\d+)?)\s*ms"), "m").expect("regex"), &body);
    Parsed::Got(json!({"total": num(total), "janky": num(janky), "jankyPct": num((pct * 100.0).round() / 100.0), "p50": pc(50), "p90": pc(90), "p95": pc(95), "p99": pc(99)}))
}

pub fn parse_meminfo(out: &str, pkg: &str) -> Parsed {
    static HEAD: Js = Js::new(r"\*\* MEMINFO in pid (\d+) \[([^\]]+)\] \*\*", "");
    static A: Js = Js::new(r"TOTAL PSS:\s*(\d+)", "");
    static B: Js = Js::new(r"^\s*TOTAL:\s*(\d+)", "m");
    static C: Js = Js::new(r"^\s*TOTAL\s+(\d+)", "m");
    static JAVA: Js = Js::new(r"Java Heap:\s*(\d+)", "");
    static NATIVE: Js = Js::new(r"Native Heap:\s*(\d+)", "");
    static GFX: Js = Js::new(r"Graphics:\s*(\d+)", "");
    static RSS: Js = Js::new(r"TOTAL RSS:\s*(\d+)", "");
    if not_running(out) {
        return Parsed::NotRunning;
    }
    let body = section_of(out, HEAD.re(), pkg).unwrap_or_else(|| out.to_string());
    let n = |r: &Js| number(r.re(), &body);
    let total = [&A, &B, &C].iter().map(|r| n(r)).find(|v| !v.is_null());
    let Some(total) = total else { return Parsed::Unparsed };
    Parsed::Got(json!({"totalPss": total, "javaHeap": n(&JAVA), "nativeHeap": n(&NATIVE), "graphics": n(&GFX), "totalRss": n(&RSS)}))
}

pub fn parse_am_start(out: &str) -> Value {
    static ERR: Js = Js::new(r"^\s*(Error[^\n]*)$", "m");
    static STATUS: Js = Js::new(r"^\s*Status:\s*(\w+)", "m");
    static TOTAL: Js = Js::new(r"^\s*TotalTime:\s*(\d+)", "m");
    static THIS: Js = Js::new(r"^\s*ThisTime:\s*(\d+)", "m");
    static WAIT: Js = Js::new(r"^\s*WaitTime:\s*(\d+)", "m");
    static STATE: Js = Js::new(r"^\s*LaunchState:\s*(\w+)", "m");
    if let Some(e) = ERR.exec(out) {
        return fail("start-failed", &js::slice(g(&e, 1).unwrap_or("").trim(), 200));
    }
    if let Some(s) = STATUS.exec(out).and_then(|m| g(&m, 1).map(String::from)) {
        if s.to_lowercase() != "ok" {
            return fail("start-failed", &format!("Status: {s}"));
        }
    }
    let num_of = |r: &Js| r.re().find(out).and_then(|m| groups(&m, out)[1].as_deref().and_then(|s| s.parse::<f64>().ok()));
    let total = num_of(&TOTAL).or_else(|| num_of(&THIS));
    let wait = num_of(&WAIT);
    if total.is_none() && wait.is_none() {
        return fail("parse-failed", &collapsed(out, 200));
    }
    json!({"totalMs": total.map(num).unwrap_or(Value::Null), "waitMs": wait.map(num).unwrap_or(Value::Null), "launchState": STATE.exec(out).and_then(|m| g(&m, 1).map(String::from)).map(Value::from).unwrap_or(Value::Null)})
}

pub fn clean_samples(raw: &Value, pkg: &str) -> Vec<Value> {
    let list = raw.get("samples").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut out: Vec<Value> = list
        .into_iter()
        .filter(|s| s.is_object() && s["id"].is_string() && s["at"].is_number() && s["pkg"].as_str() == Some(pkg) && matches!(s["kind"].as_str(), Some("measure" | "cold")))
        .map(|mut s| {
            let dev = s["device"].as_str().map(|d| js::slice(d, 80)).unwrap_or_default();
            s["device"] = json!(dev);
            s
        })
        .collect();
    out.sort_by(|a, b| b["at"].as_f64().partial_cmp(&a["at"].as_f64()).unwrap_or(std::cmp::Ordering::Equal));
    out.truncate(PERF_HISTORY_MAX);
    out
}

pub fn device_label(v: &Value) -> String {
    match v.as_str() {
        Some(s) => js::slice(s.chars().filter(|c| (*c as u32) >= 0x20 && *c != '\u{7f}').collect::<String>().trim(), 80),
        None => String::new(),
    }
}

pub struct Perf {
    m: Arc<DeviceManager>,
    dir: PathBuf,
    resets: Mutex<HashMap<String, f64>>,
    busy: Mutex<HashSet<String>>,
}

fn rand_hex(n: usize) -> String {
    let mut raw = vec![0u8; n];
    let _ = getrandom::fill(&mut raw);
    raw.iter().map(|b| format!("{b:02x}")).collect()
}

impl Perf {
    pub fn new(m: Arc<DeviceManager>, data_dir: &std::path::Path) -> Self {
        Perf { m, dir: data_dir.join(PERF_DIR), resets: Mutex::new(HashMap::new()), busy: Mutex::new(HashSet::new()) }
    }

    fn file_of(&self, pkg: &str) -> PathBuf {
        self.dir.join(format!("{pkg}.json"))
    }

    fn load(&self, pkg: &str) -> Vec<Value> {
        std::fs::read_to_string(self.file_of(pkg)).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()).map(|v| clean_samples(&v, pkg)).unwrap_or_default()
    }

    fn add(&self, sample: Value) -> Vec<Value> {
        let pkg = sample["pkg"].as_str().unwrap_or("").to_string();
        let mut samples = vec![sample];
        samples.extend(self.load(&pkg));
        samples.truncate(PERF_HISTORY_MAX);
        let file = self.file_of(&pkg);
        let _ = std::fs::create_dir_all(&self.dir);
        let tmp = file.with_extension(format!("json.{}.tmp", std::process::id()));
        let mut body = Map::new();
        body.insert("version".into(), json!(1));
        body.insert("samples".into(), Value::Array(samples.clone()));
        if std::fs::write(&tmp, js::stringify_indent(&Value::Object(body), " ")).is_ok() {
            let _ = std::fs::rename(&tmp, &file);
        }
        samples
    }

    fn once(&self, id: &str, f: impl FnOnce() -> Value) -> Value {
        if !self.busy.lock().unwrap().insert(id.to_string()) {
            return err("busy");
        }
        let r = f();
        self.busy.lock().unwrap().remove(id);
        r
    }

    pub fn history(&self, pkg: &str) -> Value {
        if !is_package(pkg) {
            return err("invalid");
        }
        json!({"ok": true, "pkg": pkg, "samples": self.load(pkg)})
    }

    pub fn clear(&self, pkg: &str) -> Value {
        if !is_package(pkg) {
            return err("invalid");
        }
        let _ = std::fs::remove_file(self.file_of(pkg));
        json!({"ok": true, "pkg": pkg, "samples": []})
    }

    fn sh(&self, id: &str, args: Vec<String>, ms: u64) -> Result<crate::run::collect::Collected, Value> {
        match self.m.adb.shell(id, &args, Some(ms)) {
            None => Err(err("adb-missing")),
            Some(r) if r.timed_out => Err(err("timeout")),
            Some(r) => Ok(r),
        }
    }

    fn with_device(&self, id: &str, f: impl FnOnce() -> Value) -> Value {
        let ready = self.m.ready_check(id);
        if is_ok(&ready) {
            f()
        } else {
            ready
        }
    }

    pub fn reset(&self, id: &str, pkg: &str) -> Value {
        self.once(id, || {
            self.with_device(id, || {
                let r = match self.sh(id, vec!["dumpsys".into(), "gfxinfo".into(), sh_quote(pkg), "reset".into()], DUMP_MS) {
                    Ok(r) => r,
                    Err(e) => return e,
                };
                if not_running(&r.out) {
                    return err("not-running");
                }
                let at = now_ms();
                self.resets.lock().unwrap().insert(format!("{id}|{pkg}"), at);
                json!({"ok": true, "at": num(at)})
            })
        })
    }

    pub fn measure(&self, id: &str, pkg: &str, device: &str) -> Value {
        self.once(id, || {
            self.with_device(id, || {
                let gfx = match self.sh(id, vec!["dumpsys".into(), "gfxinfo".into(), sh_quote(pkg)], DUMP_MS) {
                    Ok(r) => r,
                    Err(e) => return e,
                };
                let mem = match self.sh(id, vec!["dumpsys".into(), "meminfo".into(), sh_quote(pkg)], DUMP_MS) {
                    Ok(r) => r,
                    Err(e) => return e,
                };
                let (frames, memory) = (parse_gfxinfo(&gfx.out, pkg), parse_meminfo(&mem.out, pkg));
                if matches!(frames, Parsed::NotRunning) && matches!(memory, Parsed::NotRunning) {
                    return err("not-running");
                }
                let (f, m) = (frames.value(), memory.value());
                if f.is_none() && m.is_none() {
                    return fail("parse-failed", &collapsed(&gfx.out, 200));
                }
                let at = now_ms();
                let reset = self.resets.lock().unwrap().get(&format!("{id}|{pkg}")).copied();
                let mut sample = json!({"id": rand_hex(6), "at": num(at), "pkg": pkg, "device": device, "kind": "measure", "frames": f.unwrap_or(Value::Null), "memory": m.unwrap_or(Value::Null), "coldStart": null});
                if let Some(r) = reset {
                    sample["sinceResetMs"] = num(at - r);
                }
                json!({"ok": true, "sample": sample.clone(), "samples": self.add(sample)})
            })
        })
    }

    pub fn cold_start(&self, id: &str, pkg: &str, device: &str) -> Value {
        static COMPONENT: Js = Js::new(r"^[A-Za-z][\w.]*/[\w.$]+$", "");
        self.once(id, || {
            self.with_device(id, || {
                let resolved = match self.sh(id, ["cmd", "package", "resolve-activity", "--brief", "-c", LAUNCHER].iter().map(|s| s.to_string()).chain([sh_quote(pkg)]).collect(), DUMP_MS) {
                    Ok(r) => r,
                    Err(e) => return e,
                };
                let Some(component) = parse_resolved_activity(&resolved.out, pkg).filter(|c| COMPONENT.test(c)) else { return err("no-launcher") };
                if !matches!(self.m.adb.host.call("devices.confirm", json!({"kind": "perf-cold", "pkg": pkg})), Ok(Value::Bool(true))) {
                    return err("cancelled");
                }
                let r = match self.sh(id, vec!["am".into(), "start".into(), "-W".into(), "-S".into(), "-n".into(), sh_quote(&component)], COLD_MS) {
                    Ok(r) => r,
                    Err(e) => return e,
                };
                let cold = parse_am_start(&r.out);
                if cold.get("ok").is_some() {
                    return cold;
                }
                self.resets.lock().unwrap().remove(&format!("{id}|{pkg}"));
                let sample = json!({"id": rand_hex(6), "at": num(now_ms()), "pkg": pkg, "device": device, "kind": "cold", "frames": null, "memory": null, "coldStart": cold});
                json!({"ok": true, "sample": sample.clone(), "samples": self.add(sample)})
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_gfxinfo_and_meminfo() {
        let gfx = "** Graphics info for pid 12 [com.a:svc] **\nTotal frames rendered: 5\n\n** Graphics info for pid 13 [com.a] **\nTotal frames rendered: 100\nJanky frames: 7 (7.00%)\n50th percentile: 8ms\n90th percentile: 16ms\n";
        let f = parse_gfxinfo(gfx, "com.a").value().unwrap();
        assert_eq!(f["total"], json!(100));
        assert_eq!(f["jankyPct"], json!(7));
        assert_eq!(f["p90"], json!(16));
        assert!(matches!(parse_gfxinfo("No process found for: com.a", "com.a"), Parsed::NotRunning));
        let mem = "** MEMINFO in pid 13 [com.a] **\n Java Heap: 100\n TOTAL PSS: 300\n";
        assert_eq!(parse_meminfo(mem, "com.a").value().unwrap()["totalPss"], json!(300));
    }

    #[test]
    fn parses_am_start() {
        assert_eq!(parse_am_start("Status: ok\nTotalTime: 420\nWaitTime: 430\nLaunchState: COLD\n")["totalMs"], json!(420));
        assert_eq!(parse_am_start("Error: Activity class")["error"], json!("start-failed"));
    }
}
