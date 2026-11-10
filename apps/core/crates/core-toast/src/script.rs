use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

use core_store::extra::{BackgroundJob, PlanLimits, QuietHours};
use core_store::model::{ActivityEvent, TerminalSession};
use core_store::persist::notify_file::ToastLocale;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use crate::message::{AgentNote, DiskAlert};
use crate::toaster::{NotifyPatch, ToastOptions, ToastSnapshot, Toaster};

pub struct Script {
    toaster: Option<Toaster>,
    now: Arc<AtomicI64>,
    snoozed: HashSet<String>,
}

impl Default for Script {
    fn default() -> Self {
        Self::new()
    }
}

fn parse<T: DeserializeOwned>(v: Option<&Value>) -> Result<T, String> {
    serde_json::from_value(v.cloned().unwrap_or(Value::Null)).map_err(|e| e.to_string())
}

fn str_map<T: DeserializeOwned>(v: Option<&Value>) -> Result<Option<HashMap<String, T>>, String> {
    match v {
        None | Some(Value::Null) => Ok(None),
        some => parse(some).map(Some),
    }
}

fn locale(v: Option<&Value>) -> Option<ToastLocale> {
    match v.and_then(Value::as_str) {
        Some("en") => Some(ToastLocale::En),
        Some("vi") => Some(ToastLocale::Vi),
        _ => None,
    }
}

fn patch(v: &Value) -> Result<NotifyPatch, String> {
    let kinds = v.get("kinds").and_then(Value::as_object).map(|k| k.iter().filter_map(|(n, b)| b.as_bool().map(|b| (n.clone(), b))).collect()).unwrap_or_default();
    let quiet: Option<QuietHours> = match v.get("quiet") {
        None | Some(Value::Null) => None,
        q => Some(parse(q)?),
    };
    Ok(NotifyPatch { toast: v.get("toast").and_then(Value::as_bool), kinds, quiet, stuck_minutes: v.get("stuckMinutes").and_then(Value::as_f64), idle_minutes: v.get("idleMinutes").and_then(Value::as_f64) })
}

impl Script {
    pub fn new() -> Self {
        Self { toaster: None, now: Arc::new(AtomicI64::new(0)), snoozed: HashSet::new() }
    }

    fn init(&mut self, v: &Value) -> Value {
        self.now.store(v.get("now").and_then(Value::as_i64).unwrap_or(0), Ordering::SeqCst);
        let options = ToastOptions {
            supported: v.get("supported").and_then(Value::as_bool).unwrap_or(true),
            port: v.get("port").and_then(Value::as_u64).unwrap_or(4317) as u16,
            file: v.get("file").and_then(Value::as_str).map(PathBuf::from),
            link: v.get("link").and_then(Value::as_str).map(String::from),
        };
        let now = self.now.clone();
        let mut t = Toaster::new(options, Arc::new(move || now.load(Ordering::SeqCst)));
        t.load();
        t.phone_on = v.get("phone").and_then(Value::as_bool).unwrap_or(false);
        t.telegram_on = v.get("telegram").and_then(Value::as_bool).unwrap_or(false);
        let key = t.key().to_string();
        self.toaster = Some(t);
        json!(key)
    }

    pub fn run(&mut self, v: &Value) -> Result<Value, String> {
        let cmd = v.get("cmd").and_then(Value::as_str).unwrap_or("");
        if cmd == "init" {
            let key = self.init(v);
            return Ok(json!({ "ok": true, "result": key, "out": [] }));
        }
        if cmd == "advance" {
            self.now.fetch_add(v.get("ms").and_then(Value::as_i64).unwrap_or(0), Ordering::SeqCst);
            return Ok(json!({ "ok": true, "out": [] }));
        }
        if cmd == "snooze" {
            self.snoozed = parse::<Vec<String>>(v.get("ids"))?.into_iter().collect();
            return Ok(json!({ "ok": true, "out": [] }));
        }
        let snoozed = self.snoozed.clone();
        let t = self.toaster.as_mut().ok_or("init first")?;
        let result = match cmd {
            "observe" => {
                let terminals: Vec<TerminalSession> = parse(v.get("terminals"))?;
                let jobs: Option<Vec<BackgroundJob>> = match v.get("jobs") {
                    None => None,
                    j => Some(parse(j)?),
                };
                let limits: Option<Option<PlanLimits>> = match v.get("limits") {
                    None => None,
                    Some(Value::Null) => Some(None),
                    l => Some(Some(parse(l)?)),
                };
                let ctx = str_map::<f64>(v.get("contextPct"))?;
                let tools = str_map::<String>(v.get("toolOf"))?;
                let replies = str_map::<String>(v.get("replyOf"))?;
                let asks = str_map::<Value>(v.get("askOf"))?;
                let ctx_f = |id: &str| ctx.as_ref().and_then(|m| m.get(id).copied());
                let tool_f = |id: &str| tools.as_ref().and_then(|m| m.get(id).cloned());
                let reply_f = |id: &str| replies.as_ref().and_then(|m| m.get(id).cloned());
                let ask_f = |id: &str| asks.as_ref().and_then(|m| m.get(id).cloned());
                let snooze_f = |s: &TerminalSession| snoozed.contains(&s.id);
                let snap = ToastSnapshot {
                    terminals: terminals.iter().collect(),
                    jobs: jobs.as_deref(),
                    limits: limits.as_ref().map(Option::as_ref),
                    context_pct: ctx.as_ref().map(|_| &ctx_f as &dyn Fn(&str) -> Option<f64>),
                    tool_of: tools.as_ref().map(|_| &tool_f as &dyn Fn(&str) -> Option<String>),
                    reply_of: replies.as_ref().map(|_| &reply_f as &dyn Fn(&str) -> Option<String>),
                    ask_of: asks.as_ref().map(|_| &ask_f as &dyn Fn(&str) -> Option<Value>),
                    snoozed: Some(&snooze_f),
                };
                t.observe(&snap);
                Value::Null
            }
            "update" => {
                t.update(&patch(v.get("patch").unwrap_or(&Value::Null))?);
                Value::Null
            }
            "presence" => {
                t.set_presence(v.get("clientId").and_then(Value::as_u64).unwrap_or(0), v.get("attentive").and_then(Value::as_bool).unwrap_or(false), locale(v.get("locale")));
                Value::Null
            }
            "drop" => {
                t.drop_client(v.get("clientId").and_then(Value::as_u64).unwrap_or(0));
                Value::Null
            }
            "loops" => {
                let events: Vec<ActivityEvent> = parse(v.get("events"))?;
                t.check_loops(&events);
                Value::Null
            }
            "sensitive" => {
                t.observe_sensitive(&parse::<ActivityEvent>(v.get("event"))?);
                Value::Null
            }
            "disk" => {
                t.observe_disk(&parse::<DiskAlert>(v.get("msg"))?);
                Value::Null
            }
            "note" => {
                let f = |s: &TerminalSession| snoozed.contains(&s.id);
                t.observe_note(&parse::<AgentNote>(v.get("msg"))?, Some(&f));
                Value::Null
            }
            "idle" => {
                t.observe_idle(v.get("report").unwrap_or(&Value::Null));
                Value::Null
            }
            "hideFor" => {
                t.hide_for(v.get("id").and_then(Value::as_str).unwrap_or(""));
                Value::Null
            }
            "clear" => {
                t.clear();
                Value::Null
            }
            "test" => {
                let r = t.test();
                let mut o = json!({ "result": r.result });
                if let Some(h) = r.health {
                    o["health"] = json!(h);
                }
                o
            }
            "settings" => serde_json::to_value(t.settings()).map_err(|e| e.to_string())?,
            "state" => json!({ "state": t.state(), "idleWatch": t.idle_watch(), "attentive": t.attentive(), "locale": t.locale().as_str() }),
            "accepts" => json!(t.accepts(v.get("key").and_then(Value::as_str))),
            "link" => json!(t.link_for(v.get("id").and_then(Value::as_str))),
            "snoozeLink" => json!(t.snooze_link(v.get("id").and_then(Value::as_str).unwrap_or(""), crate::TOAST_SNOOZE_MIN)),
            "flush" => json!(t.flush()),
            other => return Err(format!("unknown cmd {other}")),
        };
        let out: Vec<Value> = t.drain().iter().map(|m| m.to_message(true)).collect();
        Ok(json!({ "ok": true, "result": result, "out": out }))
    }
}
