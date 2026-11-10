use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::event::ServerMessage;
use crate::extra::{GitDiffStat, PlanLimits};
use crate::model::{LimitHit, TerminalSession};
use crate::store::{NewActivity, Store, StoreOptions};
use crate::time::iso_of_ms;

pub const OPS_FORMAT: &str = "ccm-store-ops/1";

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReplayStats {
    pub ops: usize,
    pub calls: usize,
    pub frames: usize,
    pub dropped: usize,
    pub reused_refs: usize,
    pub mutated_refs: usize,
    pub clock_skews: usize,
}

#[derive(Default)]
struct Refs {
    diff: HashMap<u64, Arc<GitDiffStat>>,
    limit_hit: HashMap<u64, Arc<LimitHit>>,
    denied_tools: HashMap<u64, Arc<Vec<String>>>,
}

fn intern<T: PartialEq>(map: &mut HashMap<u64, Arc<T>>, id: u64, slot: &mut Option<Arc<T>>, stats: &mut ReplayStats) {
    let Some(value) = slot.as_mut() else { return };
    match map.get(&id) {
        Some(known) if **known == **value => {
            *value = known.clone();
            stats.reused_refs += 1;
        }
        Some(_) => {
            stats.mutated_refs += 1;
            map.insert(id, value.clone());
        }
        None => {
            map.insert(id, value.clone());
        }
    }
}

struct Replayer {
    store: Option<Store>,
    clock: Arc<AtomicI64>,
    frames: Arc<Mutex<Vec<ServerMessage>>>,
    refs: Refs,
    stats: ReplayStats,
    steps: Vec<Value>,
}

fn arg<T: DeserializeOwned>(args: &[Value], i: usize, name: &str) -> Result<T, String> {
    serde_json::from_value(args.get(i).cloned().unwrap_or(Value::Null)).map_err(|e| format!("{name} arg {i}: {e}"))
}

fn str_of(v: &Value, key: &str) -> Result<String, String> {
    v.get(key).and_then(Value::as_str).map(str::to_string).ok_or_else(|| format!("op without {key}"))
}

impl Replayer {
    fn store(&mut self) -> Result<&mut Store, String> {
        self.store.as_mut().ok_or_else(|| "op before init".to_string())
    }

    fn take_frames(&mut self, keep: bool) {
        let frames: Vec<ServerMessage> = std::mem::take(&mut *self.frames.lock().unwrap_or_else(|e| e.into_inner()));
        if !keep {
            self.stats.dropped += frames.len();
            return;
        }
        for frame in frames {
            self.push_frame(&frame);
        }
    }

    fn push_frame(&mut self, frame: &ServerMessage) {
        let Some(step) = self.steps.last_mut() else {
            self.stats.dropped += 1;
            return;
        };
        if let Some(Value::Array(frames)) = step.get_mut("frames") {
            frames.push(serde_json::to_value(frame).unwrap_or(Value::Null));
            self.stats.frames += 1;
        }
    }

    fn init(&mut self, op: &Value) -> Result<(), String> {
        let started = op.get("startedMs").and_then(Value::as_i64).ok_or("init without startedMs")?;
        let activity_limit = op.get("activityLimit").and_then(Value::as_u64).ok_or("init without activityLimit")? as usize;
        let batch_ms = op.get("batchMs").and_then(Value::as_i64).ok_or("init without batchMs")?;
        self.clock.store(started, Ordering::SeqCst);
        let clock = self.clock.clone();
        let mut store = Store::with_clock(StoreOptions { activity_limit, batch_ms }, Box::new(move || clock.load(Ordering::SeqCst)));
        let sink = self.frames.clone();
        store.subscribe(Box::new(move |m| {
            sink.lock().map_err(|e| e.to_string())?.push(m.clone());
            Ok(())
        }));
        self.store = Some(store);
        Ok(())
    }

    fn terminal(&mut self, raw: &Value, refs: Option<&Map<String, Value>>) -> Result<TerminalSession, String> {
        let mut t: TerminalSession = serde_json::from_value(raw.clone()).map_err(|e| format!("upsertTerminal: {e}"))?;
        for (field, id) in refs.into_iter().flatten() {
            let id = id.as_u64().ok_or("ref id")?;
            match field.as_str() {
                "diff" => intern(&mut self.refs.diff, id, &mut t.diff, &mut self.stats),
                "limitHit" => intern(&mut self.refs.limit_hit, id, &mut t.limit_hit, &mut self.stats),
                "deniedTools" => intern(&mut self.refs.denied_tools, id, &mut t.denied_tools, &mut self.stats),
                other => return Err(format!("upsertTerminal: object field {other} has no identity rule")),
            }
        }
        Ok(t)
    }

    fn call(&mut self, op: &Value) -> Result<(), String> {
        let name = str_of(op, "name")?;
        let empty = Vec::new();
        let args = op.get("args").and_then(Value::as_array).unwrap_or(&empty);
        let refs = op.get("refs").and_then(Value::as_object);
        if let Some(now) = op.get("now").and_then(Value::as_i64) {
            self.clock.store(now, Ordering::SeqCst);
        }
        self.stats.calls += 1;
        let n = name.as_str();
        if refs.is_some_and(|r| !r.is_empty()) && n != "upsertTerminal" {
            return Err(format!("{n}: object fields {refs:?} have no identity rule"));
        }
        match n {
            "upsertTerminal" => {
                let t = self.terminal(args.first().unwrap_or(&Value::Null), refs)?;
                self.store()?.upsert_terminal(t);
            }
            "removeTerminal" => {
                let id: String = arg(args, 0, n)?;
                self.store()?.remove_terminal(&id);
            }
            "upsertAgent" => {
                let a = arg(args, 0, n)?;
                self.store()?.upsert_agent(a);
            }
            "removeAgent" => {
                let id: String = arg(args, 0, n)?;
                self.store()?.remove_agent(&id);
            }
            "upsertLink" => {
                let l = arg(args, 0, n)?;
                self.store()?.upsert_link(l);
            }
            "removeLink" => {
                let id: String = arg(args, 0, n)?;
                self.store()?.remove_link(&id);
            }
            "pushActivity" => {
                let mut event: NewActivity = arg(args, 0, n)?;
                if event.at.is_none() {
                    if let (Some(at), Some(now)) = (op.get("at").and_then(Value::as_str), op.get("now").and_then(Value::as_i64)) {
                        if at != iso_of_ms(now) {
                            self.stats.clock_skews += 1;
                            event.at = Some(at.to_string());
                        }
                    }
                }
                self.store()?.push_activity(event);
            }
            _ => return self.setter(n, args),
        }
        Ok(())
    }

    fn setter(&mut self, n: &str, args: &[Value]) -> Result<(), String> {
        let s = self.store.as_mut().ok_or("op before init")?;
        match n {
            "resetUsage" => s.reset_usage(arg(args, 0, n)?),
            "setProjectRoots" => s.set_project_roots(arg(args, 0, n)?),
            "updateUsage" => s.update_usage(arg(args, 0, n)?),
            "setUsageScan" => s.set_usage_scan(arg(args, 0, n)?),
            "setFavorites" => s.set_favorites(arg(args, 0, n)?),
            "rejectFavorite" => s.reject_favorite(arg(args, 0, n)?, arg(args, 1, n)?),
            "historyData" => s.history_data(arg(args, 0, n)?),
            "historyAdded" => s.history_added(arg(args, 0, n)?),
            "toolSpan" => s.tool_span(arg(args, 0, n)?),
            "timelineData" => s.timeline_data(arg(args, 0, n)?),
            "folderPicked" => s.folder_picked(arg(args, 0, n)?, arg(args, 1, n)?, arg(args, 2, n)?),
            "focusResult" => s.focus_result(arg(args, 0, n)?, arg(args, 1, n)?),
            "setJobs" => s.set_jobs(arg(args, 0, n)?),
            "setLimits" => s.set_limits(arg::<Option<PlanLimits>>(args, 0, n)?),
            "setCodexLimits" => s.set_codex_limits(arg(args, 0, n)?),
            "setResponse" => s.set_response(arg(args, 0, n)?),
            "setModelNames" => s.set_model_names(arg(args, 0, n)?),
            "setSensitive" => s.set_sensitive(arg(args, 0, n)?),
            "setResources" => s.set_resources(arg(args, 0, n)?),
            "setLoading" => s.set_loading(arg(args, 0, n)?),
            "setNotify" => s.set_notify(arg(args, 0, n)?),
            "setSnoozes" => s.set_snoozes(arg(args, 0, n)?),
            other => return Err(format!("unknown store call {other}")),
        }
        Ok(())
    }

    fn op(&mut self, op: &Value) -> Result<(), String> {
        self.stats.ops += 1;
        match str_of(op, "op")?.as_str() {
            "init" => self.init(op)?,
            "step" => {
                let sent = op.get("sent").cloned().unwrap_or(Value::Array(Vec::new()));
                self.steps.push(serde_json::json!({ "name": str_of(op, "name")?, "sent": sent, "frames": [] }));
            }
            "set" => {
                let value = op.get("value").and_then(Value::as_str).map(str::to_string);
                let s = self.store()?;
                match str_of(op, "key")?.as_str() {
                    "statusLineCommand" => s.status_line_command = value,
                    "dayStartedAt" => s.day_started_at = value,
                    other => return Err(format!("unknown store field {other}")),
                }
            }
            "call" => self.call(op)?,
            "flush" => {
                self.store()?.flush();
                let dropped = op.get("dropped").and_then(Value::as_bool).unwrap_or(false);
                self.take_frames(!dropped);
            }
            "snapshot" => {
                let message = self.store()?.snapshot_now();
                self.take_frames(true);
                self.push_frame(&message);
            }
            "direct" => {
                let message: ServerMessage = serde_json::from_value(op.get("message").cloned().unwrap_or(Value::Null)).map_err(|e| format!("direct: {e}"))?;
                self.take_frames(true);
                self.push_frame(&message);
            }
            other => return Err(format!("unknown op {other}")),
        }
        Ok(())
    }
}

fn execute(doc: &Value) -> Result<Replayer, String> {
    if doc.get("format").and_then(Value::as_str) != Some(OPS_FORMAT) {
        return Err(format!("not a {OPS_FORMAT} document"));
    }
    let ops = doc.get("ops").and_then(Value::as_array).ok_or("no ops")?;
    let mut r = Replayer {
        store: None,
        clock: Arc::new(AtomicI64::new(0)),
        frames: Arc::new(Mutex::new(Vec::new())),
        refs: Refs::default(),
        stats: ReplayStats::default(),
        steps: Vec::new(),
    };
    for (i, op) in ops.iter().enumerate() {
        r.op(op).map_err(|e| format!("op {i}: {e}"))?;
    }
    r.take_frames(true);
    Ok(r)
}

pub fn replay_store(doc: &Value) -> Result<(Store, ReplayStats), String> {
    let r = execute(doc)?;
    let store = r.store.ok_or("no init op")?;
    Ok((store, r.stats))
}

pub fn replay(doc: &Value) -> Result<(Value, ReplayStats), String> {
    let r = execute(doc)?;
    let mut trace = match doc.get("header") {
        Some(Value::Object(h)) => h.clone(),
        _ => Map::new(),
    };
    trace.insert("format".into(), Value::from("ccm-trace/1"));
    trace.insert("source".into(), Value::from("rust"));
    trace.insert("steps".into(), Value::Array(r.steps));
    Ok((Value::Object(trace), r.stats))
}
