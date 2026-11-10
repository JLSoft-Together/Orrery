use super::scope::{target_json, ProcRow};
use crate::resources::{self, Target};
use crate::{Caller, Ctx};
use base64::Engine;
use serde_json::{json, Value};

pub trait Deps {
    fn render(&self, file: &str, device: &Value, night: bool, slot: &str) -> Value;
    fn write_shot(&self, png: &[u8], name: &str) -> Option<String>;
    fn report(&self) -> Option<Value>;
    fn proc_rows(&self) -> Option<Vec<ProcRow>>;
    fn kill(&self, targets: &[Target], pane: &str, port: i64) -> Value;
    fn confirm_receive(&self, pane: &str, owner: &str) -> i64;
    fn tg_available(&self) -> bool;
    fn tg_state(&self) -> Value;
    fn tg_set(&self, patch: Value) -> Value;
    fn tg_route_for(&self, cwd: &str) -> Value;
    fn tg_report(&self, body: Value) -> Value;
    fn snapshot(&self) -> Option<Value>;
    fn screencap(&self, serial: &str) -> Value;
    fn runs(&self) -> Vec<Value>;
    fn capture_active(&self) -> bool;
}

fn unavailable_render() -> Value {
    json!({"ok": false, "code": "unavailable"})
}

fn object_or(v: Option<Value>, fallback: Value) -> Value {
    match v {
        Some(v) if v.is_object() => v,
        _ => fallback,
    }
}

fn rows_of(v: &Value) -> Option<Vec<ProcRow>> {
    v.as_array().map(|a| a.iter().filter_map(ProcRow::parse).collect())
}

fn b64(png: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(png)
}

pub struct HostDeps<'a> {
    pub ctx: &'a Ctx<'a>,
}

impl HostDeps<'_> {
    fn h(&self, method: &str, params: Value) -> Option<Value> {
        self.ctx.host(&format!("mcpDesk.{method}"), params).ok()
    }
}

impl Deps for HostDeps<'_> {
    fn render(&self, file: &str, device: &Value, night: bool, slot: &str) -> Value {
        object_or(self.h("render", json!({"file": file, "device": device, "night": night, "slot": slot})), unavailable_render())
    }

    fn write_shot(&self, png: &[u8], name: &str) -> Option<String> {
        self.h("writeShot", json!({"png": b64(png), "name": name})).and_then(|v| v.as_str().map(str::to_string))
    }

    fn report(&self) -> Option<Value> {
        self.h("report", Value::Null).filter(Value::is_object)
    }

    fn proc_rows(&self) -> Option<Vec<ProcRow>> {
        self.h("procRows", Value::Null).as_ref().and_then(rows_of)
    }

    fn kill(&self, targets: &[Target], pane: &str, port: i64) -> Value {
        let list: Vec<Value> = targets.iter().map(target_json).collect();
        object_or(self.h("kill", json!({"targets": list, "pane": pane, "port": port})), json!({"ok": false, "error": "failed"}))
    }

    fn confirm_receive(&self, pane: &str, owner: &str) -> i64 {
        self.h("confirm", json!({"kind": "telegram_receive", "pane": pane, "owner": owner})).and_then(|v| v.as_i64()).unwrap_or(-1)
    }

    fn tg_available(&self) -> bool {
        self.h("tgAvailable", Value::Null) == Some(json!(true))
    }

    fn tg_state(&self) -> Value {
        self.h("tgState", Value::Null).unwrap_or(Value::Null)
    }

    fn tg_set(&self, patch: Value) -> Value {
        object_or(self.h("tgSet", json!({"patch": patch})), json!({"ok": false}))
    }

    fn tg_route_for(&self, cwd: &str) -> Value {
        self.h("tgRouteFor", json!({"cwd": cwd})).unwrap_or(Value::Null)
    }

    fn tg_report(&self, body: Value) -> Value {
        object_or(self.h("tgReport", body), json!({"ok": false, "error": "network"}))
    }

    fn snapshot(&self) -> Option<Value> {
        self.h("snapshot", Value::Null).filter(Value::is_object)
    }

    fn screencap(&self, serial: &str) -> Value {
        object_or(self.h("screencap", json!({"serial": serial})), json!({"ok": false, "error": "capture-failed"}))
    }

    fn runs(&self) -> Vec<Value> {
        self.h("runs", Value::Null).and_then(|v| v.as_array().cloned()).unwrap_or_default()
    }

    fn capture_active(&self) -> bool {
        self.h("captureActive", Value::Null) == Some(json!(true))
    }
}

pub struct NativeDeps<'a> {
    pub ctx: &'a Ctx<'a>,
    pub caller: &'a Caller,
}

impl NativeDeps<'_> {
    fn ch(&self, channel: &str, args: Value) -> Value {
        self.caller.call(self.ctx, channel, args).unwrap_or(Value::Null)
    }
}

impl Deps for NativeDeps<'_> {
    fn render(&self, file: &str, device: &Value, night: bool, slot: &str) -> Value {
        object_or(Some(self.ch("desk:layout-agent-render", json!([file, {"device": device, "night": night, "slot": slot}]))), unavailable_render())
    }

    fn write_shot(&self, png: &[u8], name: &str) -> Option<String> {
        let r = self.ch("ccm:capture-shot", json!([{"png": format!("data:image/png;base64,{}", b64(png)), "device": name}]));
        if r["ok"] == json!(true) {
            r["path"].as_str().map(str::to_string)
        } else {
            None
        }
    }

    fn report(&self) -> Option<Value> {
        self.ctx.host("resources.report", Value::Null).ok().filter(Value::is_object)
    }

    fn proc_rows(&self) -> Option<Vec<ProcRow>> {
        core_procs::table().map(|t| t.into_iter().map(|p| ProcRow { pid: p.pid as i64, ppid: p.ppid as i64, start_ms: if cfg!(windows) { p.start_ms.map(|m| m as f64) } else { None } }).collect())
    }

    fn kill(&self, targets: &[Target], pane: &str, port: i64) -> Value {
        let list: Vec<Value> = targets.iter().map(target_json).collect();
        resources::res_kill_with(self.ctx, &Value::Array(list), &|items| self.ctx.host("mcpDesk.confirmKill", json!({"items": items, "pane": pane, "port": port})).ok() == Some(json!(true)))
    }

    fn confirm_receive(&self, pane: &str, owner: &str) -> i64 {
        self.ctx.host("mcpDesk.confirm", json!({"kind": "telegram_receive", "pane": pane, "owner": owner})).ok().and_then(|v| v.as_i64()).unwrap_or(-1)
    }

    fn tg_available(&self) -> bool {
        true
    }

    fn tg_state(&self) -> Value {
        self.ch("desk:tg-state", Value::Null)
    }

    fn tg_set(&self, patch: Value) -> Value {
        self.ch("desk:tg-set", json!([{"patch": patch}]))
    }

    fn tg_route_for(&self, cwd: &str) -> Value {
        self.ch("desk:tg-route-for", json!([{"cwd": cwd}]))
    }

    fn tg_report(&self, body: Value) -> Value {
        self.ch("desk:tg-report", json!([body]))
    }

    fn snapshot(&self) -> Option<Value> {
        Some(self.ch("ccm:dev-snapshot", Value::Null)).filter(Value::is_object)
    }

    fn screencap(&self, serial: &str) -> Value {
        self.ch("ccm:dev-screencap", json!([serial]))
    }

    fn runs(&self) -> Vec<Value> {
        self.ch("ccm:run-sessions", Value::Null).as_array().cloned().unwrap_or_default()
    }

    fn capture_active(&self) -> bool {
        !self.ch("ccm:capture-get", Value::Null)["active"].is_null()
    }
}
