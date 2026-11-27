use super::adb::Adb;
use super::ipc::{denied, invalid, is_id, MODULE};
use super::manager::DeviceManager;
use super::parse::{err, err_detail, is_ok, ok, Row};
use super::toggles::{parse_night, parse_setting};
use crate::run::android::apk::{find_apk, module_dir};
use crate::run::android::jsre::Js;
use crate::{arg, js, Desk, DeskError};
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const ACTIVE_MAX: Duration = Duration::from_secs(10 * 60);
pub const FONT_MIN: f64 = 0.5;
pub const FONT_MAX: f64 = 3.0;

pub trait Shell: Send + Sync {
    fn sh(&self, id: &str, args: &[&str]) -> Option<String>;
}

impl Shell for Adb {
    fn sh(&self, id: &str, args: &[&str]) -> Option<String> {
        static BAD: Js = Js::new(r"^(Error|Exception|Unknown|Bad|java\.)", "m");
        let list: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        self.shell(id, &list, None).filter(|r| !r.timed_out && r.code == Some(0) && !BAD.test(&r.out)).map(|r| r.out)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Saved {
    pub serial: String,
    pub night: Option<String>,
    pub font: Option<Option<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    pub night: Option<bool>,
    pub font: Option<f64>,
}

fn numeric(s: &str) -> bool {
    static RE: Js = Js::new(r"^\d+(\.\d+)?$", "");
    RE.test(s)
}

pub fn clean_request(v: &Value) -> Option<Request> {
    let o = v.as_object()?;
    let night = match o.get("night") {
        None | Some(Value::Null) => None,
        Some(Value::Bool(b)) => Some(*b),
        _ => return None,
    };
    let font = match o.get("fontScale") {
        None | Some(Value::Null) => None,
        Some(n) => Some(n.as_f64().filter(|f| f.is_finite() && (FONT_MIN..=FONT_MAX).contains(f)).map(|f| (f * 100.0).round() / 100.0)?),
    };
    (night.is_some() || font.is_some()).then_some(Request { night, font })
}

pub fn file_name(serial: &str) -> String {
    let mut s = String::new();
    for b in serial.bytes() {
        if b.is_ascii_alphanumeric() || b == b'_' || b == b'-' {
            s.push(b as char);
        } else {
            s.push_str(&format!("~{b:02x}"));
        }
    }
    format!("{s}.json")
}

impl Saved {
    fn to_json(&self) -> Value {
        let mut o = Map::new();
        o.insert("serial".into(), json!(self.serial));
        if let Some(n) = &self.night {
            o.insert("night".into(), json!(n));
        }
        if let Some(f) = &self.font {
            o.insert("fontScale".into(), json!(f));
        }
        Value::Object(o)
    }

    fn from_json(v: &Value) -> Option<Saved> {
        let o = v.as_object()?;
        let serial = o.get("serial").filter(|s| is_id(s))?.as_str()?.to_string();
        let night = match o.get("night") {
            None => None,
            Some(Value::String(n)) if matches!(n.as_str(), "yes" | "no" | "auto" | "custom") => Some(n.clone()),
            _ => return None,
        };
        let font = match o.get("fontScale") {
            None => None,
            Some(Value::Null) => Some(None),
            Some(Value::String(s)) if numeric(s) => Some(Some(s.clone())),
            _ => return None,
        };
        (night.is_some() || font.is_some()).then_some(Saved { serial, night, font })
    }

    fn restore_commands(&self) -> Vec<Vec<String>> {
        let strs = |v: &[&str]| -> Vec<String> { v.iter().map(|s| s.to_string()).collect() };
        let mut out = Vec::new();
        if let Some(n) = &self.night {
            out.push(strs(&["cmd", "uimode", "night", if n == "custom" { "auto" } else { n }]));
        }
        if let Some(f) = &self.font {
            out.push(match f {
                Some(v) => strs(&["settings", "put", "system", "font_scale", v]),
                None => strs(&["settings", "delete", "system", "font_scale"]),
            });
        }
        out
    }
}

pub struct Variants {
    sh: Arc<dyn Shell>,
    dir: PathBuf,
    active: Mutex<HashMap<String, Instant>>,
    restoring: Mutex<HashSet<String>>,
}

impl Variants {
    pub fn new(sh: Arc<dyn Shell>, dir: PathBuf) -> Arc<Self> {
        Arc::new(Variants { sh, dir, active: Mutex::new(HashMap::new()), restoring: Mutex::new(HashSet::new()) })
    }

    fn path(&self, id: &str) -> PathBuf {
        self.dir.join(file_name(id))
    }

    fn load(&self, id: &str) -> Option<Saved> {
        let text = std::fs::read_to_string(self.path(id)).ok()?;
        js::parse(&text).and_then(|v| Saved::from_json(&v)).filter(|s| s.serial == id)
    }

    fn store(&self, s: &Saved) -> bool {
        if std::fs::create_dir_all(&self.dir).is_err() {
            return false;
        }
        let path = self.path(&s.serial);
        let mut tmp = path.clone().into_os_string();
        tmp.push(".tmp");
        let tmp = PathBuf::from(tmp);
        std::fs::write(&tmp, js::stringify_indent(&s.to_json(), "  ")).is_ok() && std::fs::rename(&tmp, &path).is_ok()
    }

    pub fn pending(&self) -> Vec<Saved> {
        let Ok(rd) = std::fs::read_dir(&self.dir) else { return Vec::new() };
        let mut out: Vec<Saved> = rd
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
            .filter_map(|e| std::fs::read_to_string(e.path()).ok())
            .filter_map(|t| js::parse(&t))
            .filter_map(|v| Saved::from_json(&v))
            .collect();
        out.sort_by(|a, b| a.serial.cmp(&b.serial));
        out
    }

    fn mark(&self, id: &str, on: bool) {
        let mut a = self.active.lock().unwrap();
        if on {
            a.insert(id.to_string(), Instant::now());
        } else {
            a.remove(id);
        }
    }

    fn is_active(&self, id: &str, now: Instant) -> bool {
        self.active.lock().unwrap().get(id).is_some_and(|t| now.saturating_duration_since(*t) < ACTIVE_MAX)
    }

    pub fn apply(&self, id: &str, req: &Request) -> Value {
        let mut saved = self.load(id).unwrap_or(Saved { serial: id.to_string(), night: None, font: None });
        if req.night.is_some() && saved.night.is_none() {
            match self.sh.sh(id, &["cmd", "uimode", "night"]).and_then(|o| parse_night(&o)) {
                Some(n) => saved.night = Some(n),
                None => return err_detail("read-failed", "night"),
            }
        }
        if req.font.is_some() && saved.font.is_none() {
            match self.sh.sh(id, &["settings", "get", "system", "font_scale"]) {
                Some(o) => saved.font = Some(parse_setting(&o)),
                None => return err_detail("read-failed", "fontScale"),
            }
        }
        if !self.store(&saved) {
            return err("save-failed");
        }
        self.mark(id, true);
        let mut cmds: Vec<Vec<String>> = Vec::new();
        if let Some(n) = req.night {
            cmds.push(["cmd", "uimode", "night", if n { "yes" } else { "no" }].iter().map(|s| s.to_string()).collect());
        }
        if let Some(f) = req.font {
            cmds.push(["settings", "put", "system", "font_scale", &js::number(f)].iter().map(|s| s.to_string()).collect());
        }
        for c in cmds {
            let args: Vec<&str> = c.iter().map(String::as_str).collect();
            if self.sh.sh(id, &args).is_none() {
                let back = self.restore(id);
                return err_detail("apply-failed", if is_ok(&back) { "restored" } else { "restore-failed" });
            }
        }
        ok()
    }

    pub fn restore(&self, id: &str) -> Value {
        if !self.path(id).exists() {
            self.mark(id, false);
            return json!({"ok": true, "restored": false});
        }
        let Some(saved) = self.load(id) else {
            let _ = std::fs::remove_file(self.path(id));
            self.mark(id, false);
            return err("corrupt");
        };
        let mut failed = false;
        for c in saved.restore_commands() {
            let args: Vec<&str> = c.iter().map(String::as_str).collect();
            if self.sh.sh(id, &args).is_none() {
                failed = true;
            }
        }
        if failed {
            return err("restore-failed");
        }
        let _ = std::fs::remove_file(self.path(id));
        self.mark(id, false);
        json!({"ok": true, "restored": true})
    }

    pub fn restore_ready(&self, ready: &[String], now: Instant) -> Vec<String> {
        let mut done = Vec::new();
        for s in self.pending() {
            if !ready.contains(&s.serial) || self.is_active(&s.serial, now) {
                continue;
            }
            if !self.restoring.lock().unwrap().insert(s.serial.clone()) {
                continue;
            }
            let r = self.restore(&s.serial);
            self.restoring.lock().unwrap().remove(&s.serial);
            if is_ok(&r) {
                done.push(s.serial);
            }
        }
        done
    }

    pub fn restore_pending(self: &Arc<Self>, rows: &[Row]) {
        let ready: Vec<String> = rows.iter().filter(|r| r.state == "ready").map(|r| r.id.clone()).collect();
        if ready.is_empty() || !self.dir.is_dir() {
            return;
        }
        let me = self.clone();
        std::thread::spawn(move || {
            let _ = me.restore_ready(&ready, Instant::now());
        });
    }
}

fn handle(m: &DeviceManager, req: &Value) -> Value {
    let op = req.get("op").and_then(Value::as_str).unwrap_or("");
    if op == "sweep" {
        m.snapshot();
        return ok();
    }
    let Some(id) = req.get("id").filter(|v| is_id(v)).and_then(Value::as_str) else { return denied() };
    let parsed = match op {
        "apply" => clean_request(req).map(Some),
        "restore" => Some(None),
        _ => None,
    };
    let Some(parsed) = parsed else { return invalid() };
    let ready = m.ready_check(id);
    if !is_ok(&ready) {
        return ready;
    }
    match parsed {
        Some(r) => m.variants.apply(id, &r),
        None => m.variants.restore(id),
    }
}

fn is_module(v: &str) -> bool {
    static RE: Js = Js::new(r"^:?[A-Za-z0-9_.:-]{0,128}$", "");
    RE.test(v)
}

fn is_variant(v: &str) -> bool {
    static RE: Js = Js::new(r"^[A-Za-z0-9_]{1,64}$", "");
    RE.test(v)
}

pub fn built_apk(root: &str, module: &str, variant: &str) -> Value {
    match find_apk(&module_dir(root, module), variant) {
        Some(a) => json!({"ok": true, "apk": a.apk, "appId": a.application_id}),
        None => err("apk-missing"),
    }
}

pub fn register(desk: &mut Desk, m: &Arc<DeviceManager>) {
    let mm = m.clone();
    desk.on(MODULE, "ccm:dev-variant", move |_, args| Ok::<Value, DeskError>(handle(&mm, arg(args, 0))));
    desk.on(MODULE, "ccm:dev-built-apk", move |ctx, args| {
        let root = ctx.host("gate.dir", json!({"dir": arg(args, 0)})).ok().and_then(|v| v.as_str().map(String::from));
        let (module, variant) = (arg(args, 1).as_str().filter(|x| is_module(x)), arg(args, 2).as_str().filter(|x| is_variant(x)));
        Ok::<Value, DeskError>(match (root, module, variant) {
            (Some(r), Some(m), Some(v)) => built_apk(&r, m, v),
            (None, _, _) => denied(),
            _ => invalid(),
        })
    });
}
