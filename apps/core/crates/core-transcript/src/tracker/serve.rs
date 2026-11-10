use super::{Out, Plan, Step, SyncArgs, Tracker};
use crate::sigserve::READ_BUDGET;
use crate::time::now_ms;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::path::PathBuf;

pub const MAX_TRACKERS: usize = 4096;
pub const MAX_FILES: usize = 4096;

type Reply = Result<Map<String, Value>, &'static str>;

#[derive(Default)]
pub struct Trackers {
    open: HashMap<String, Tracker>,
}

fn opt_str(m: &Map<String, Value>, k: &str) -> Result<Option<String>, &'static str> {
    match m.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err("args"),
    }
}

fn plan_of(v: Option<&Value>) -> Result<Plan, &'static str> {
    let Some(Value::Object(p)) = v else { return Ok(Plan::default()) };
    let flag = |k: &str| p.get(k) == Some(&Value::Bool(true));
    let files = match p.get("files") {
        None | Some(Value::Null) => Default::default(),
        Some(Value::Array(a)) if a.len() <= MAX_FILES => a.iter().filter_map(Value::as_str).map(str::to_owned).collect(),
        Some(_) => return Err("plan"),
    };
    Ok(Plan { all: flag("all"), unresolved: flag("unresolved"), files })
}

fn args_of(m: &Map<String, Value>) -> Result<SyncArgs, &'static str> {
    Ok(SyncArgs {
        session_id: opt_str(m, "sessionId")?,
        cwd: opt_str(m, "cwd")?,
        epoch: m.get("epoch").and_then(Value::as_f64).unwrap_or(0.0),
        catch_up: m.get("catchUp") == Some(&Value::Bool(true)),
        plan: plan_of(m.get("plan"))?,
        now: m.get("nowMs").and_then(Value::as_f64).filter(|v| v.is_finite()).map_or_else(now_ms, |v| v as i64),
    })
}

fn key_of(m: &Map<String, Value>) -> Option<&str> {
    m.get("key").and_then(Value::as_str).filter(|k| !k.is_empty())
}

impl Trackers {
    fn open(&mut self, m: &Map<String, Value>) -> Reply {
        let key = key_of(m).ok_or("key")?;
        let terminal = m.get("terminalId").and_then(Value::as_str).filter(|s| !s.is_empty()).ok_or("terminalId")?;
        let projects = m.get("projectsDir").and_then(Value::as_str).filter(|s| !s.is_empty()).ok_or("projectsDir")?;
        if self.open.contains_key(key) {
            return Err("key");
        }
        if self.open.len() >= MAX_TRACKERS {
            return Err("trackers");
        }
        let mut t = Tracker::new(terminal, PathBuf::from(projects));
        t.hold = false;
        self.open.insert(key.to_owned(), t);
        Ok(Map::new())
    }

    fn sync(&mut self, m: &Map<String, Value>) -> Reply {
        let t = key_of(m).and_then(|k| self.open.get_mut(k)).ok_or("tracker")?;
        let args = match args_of(m) {
            Ok(a) => Some(a),
            Err(e) if !t.running() => return Err(e),
            Err(_) => None,
        };
        let budget = m.get("maxBytes").and_then(Value::as_f64).filter(|v| v.is_finite() && *v >= 1.0).map_or(READ_BUDGET, |v| v as u64);
        let mut out = Out::default();
        let step = t.sync(args, budget, &mut out);
        let mut r = Map::new();
        r.insert("activity".into(), Value::Array(out.activity));
        r.insert("spans".into(), Value::Array(out.spans));
        r.insert("inbound".into(), Value::Array(out.inbound));
        r.insert("sent".into(), Value::Array(out.sent));
        match step {
            Step::More => {
                r.insert("more".into(), Value::Bool(true));
            }
            Step::Done => {
                if let Some(v) = t.take_view() {
                    r.insert("view".into(), v);
                }
            }
        }
        Ok(r)
    }

    fn close_spans(&mut self, m: &Map<String, Value>) -> Reply {
        let t = key_of(m).and_then(|k| self.open.get_mut(k)).ok_or("tracker")?;
        let at = m.get("at").and_then(Value::as_str).filter(|s| !s.is_empty()).ok_or("at")?;
        let mut r = Map::new();
        r.insert("spans".into(), Value::Array(t.close_spans(at)));
        Ok(r)
    }

    pub fn handle(&mut self, op: &str, m: &Map<String, Value>) -> Option<Reply> {
        Some(match op {
            "tracker.open" => self.open(m),
            "tracker.sync" => self.sync(m),
            "tracker.closeSpans" => self.close_spans(m),
            "tracker.close" => match key_of(m).and_then(|k| self.open.remove(k)) {
                Some(_) => Ok(Map::new()),
                None => Err("tracker"),
            },
            _ => return None,
        })
    }
}
