use super::host::{tool_result, Args, ToolHost};
use super::paths::{resolve_inside, Inside};
use super::t_common::{as_int, num_text, str_of, truthy};
use serde_json::{json, Value};

pub use super::t_telegram::{telegram_notify, telegram_receive, telegram_send};

pub const PREVIEW_RATE_MAX: usize = 10;
pub const PREVIEW_RATE_MS: u64 = 60_000;
pub const SERVERS_RATE_MAX: usize = 20;
pub const SERVERS_RATE_MS: u64 = 60_000;
pub const FREE_PORT_RATE_MAX: usize = 3;
pub const FREE_PORT_RATE_MS: u64 = 60_000;

pub fn desk_call(host: &dyn ToolHost, call: Value) -> Option<Value> {
    host.desk(call).map(|r| if r.is_object() { r } else { json!({ "ok": false, "error": "failed" }) })
}

pub fn layout_error_text(code: &str) -> &'static str {
    match code {
        "path" => "path must be a file path (no UNC or device paths).",
        "outside" => "That file is outside this session's project folder; only layouts of the project can be rendered.",
        "missing" => "That file does not exist.",
        "folder" => "That is a folder; give a layout XML file.",
        "device" => "device must be phone or tablet.",
        "theme" => "theme must be light or dark.",
        "xml" => "Only .xml layout files can be rendered.",
        "rate" => "Too many layouts were rendered from this session in the last minute; wait a little.",
        "not-layout" => "That XML is not a layout (res/layout*/ or a view root element).",
        "too-large" => "That layout is larger than 1 MB.",
        "jar-missing" => "The layout renderer is not built into this Orrery copy.",
        "bundle-missing" => "The layout preview kit (layoutlib, about 170 MB) is not downloaded yet; ask the user to open a layout in Orrery once and download it.",
        "jdk-missing" => "No Java 17+ was found for the layout renderer; ask the user to set one in Orrery Run settings.",
        "jdk-old" => "The Java found for the layout renderer is older than 17; ask the user to set a newer one in Orrery Run settings.",
        "crash-loop" => "The layout renderer crashed several times in a row; it is paused for a few minutes.",
        "render-failed" => "layoutlib could not render this layout.",
        "timeout" => "The render took too long; try again.",
        "starting" => "The layout renderer is starting.",
        "unavailable" => "Orrery cannot render layouts right now.",
        "busy" => "Another render is still running for this session; try again.",
        _ => "Orrery could not render the layout.",
    }
}

fn layout_fail(error: &str, retry: Option<i64>, detail: Option<&str>) -> Value {
    let wait = if error == "starting" { format!(" Call again in about {} seconds.", retry.unwrap_or(10)) } else { String::new() };
    let det = detail.filter(|d| !d.is_empty()).map(|d| format!(" ({d})")).unwrap_or_default();
    let mut out = json!({ "ok": false, "error": error });
    if let Some(r) = retry.filter(|r| *r != 0) {
        out["retryInSec"] = json!(r);
    }
    if let Some(d) = detail.filter(|d| !d.is_empty()) {
        out["detail"] = json!(d);
    }
    tool_result(&format!("{}{wait}{det}", layout_error_text(error)), out, true)
}

fn layout_text(res: &Value) -> String {
    let framework = if res.get("frameworkOnly") == Some(&json!(true)) { ", framework views only: no project model loaded" } else { "" };
    let head = format!("Rendered {}×{} PNG: {} ({} ms{framework}).", num_text(res.get("width")), num_text(res.get("height")), str_of(res, "png"), num_text(res.get("renderMs")));
    let diags = res.get("diagnostics").and_then(Value::as_array).cloned().unwrap_or_default();
    if diags.is_empty() {
        return head;
    }
    let mut lines = vec![head, "Diagnostics:".to_string()];
    for d in &diags {
        let at = if truthy(d.get("line")) { format!(" line {}", num_text(d.get("line"))) } else { String::new() };
        lines.push(format!("- {}{at}: {}", str_of(d, "severity"), str_of(d, "message")));
    }
    lines.join("\n")
}

pub fn preview_layout(args: &Args, host: &dyn ToolHost) -> Value {
    if !host.take("layout", PREVIEW_RATE_MAX, PREVIEW_RATE_MS) {
        return layout_fail("rate", None, None);
    }
    let device = match args.get("device") {
        None | Some(Value::Null) => "phone",
        Some(Value::String(d)) if d == "phone" || d == "tablet" => d.as_str(),
        Some(_) => return layout_fail("device", None, None),
    };
    let theme = match args.get("theme") {
        None | Some(Value::Null) => "light",
        Some(Value::String(t)) if t == "light" || t == "dark" => t.as_str(),
        Some(_) => return layout_fail("theme", None, None),
    };
    let path = match resolve_inside(&host.root(), args.get("path"), false) {
        Inside::Ok { path, .. } => path,
        Inside::Err(e) => return layout_fail(e, None, None),
    };
    if !path.to_ascii_lowercase().ends_with(".xml") {
        return layout_fail("xml", None, None);
    }
    let call = json!({ "tool": "preview_layout", "path": path, "device": device, "night": theme == "dark" });
    let Some(res) = desk_call(host, call) else { return layout_fail("unavailable", None, None) };
    if res.get("ok") != Some(&json!(true)) {
        let retry = as_int(res.get("retryInSec"));
        return layout_fail(str_of(&res, "error"), retry, res.get("detail").and_then(Value::as_str));
    }
    if str_of(&res, "tool") != "preview_layout" {
        return layout_fail("failed", None, None);
    }
    let mut out = res.clone();
    out["path"] = json!(path);
    tool_result(&layout_text(&res), out, false)
}

fn servers_fail(error: &str) -> Value {
    let text = match error {
        "rate" => "Too many lists were asked from this session in the last minute; wait a little.",
        "no-report" => "Orrery has no process sample yet (Resources samples every 30 s while the window is open); try again shortly.",
        "unavailable" => "Orrery cannot list servers right now.",
        _ => "Orrery could not list servers.",
    };
    tool_result(text, json!({ "ok": false, "error": error }), true)
}

fn idle(sec: Option<i64>) -> String {
    match sec {
        None => "busy".to_string(),
        Some(s) if s < 60 => format!("idle {s} s"),
        Some(s) if s < 3600 => format!("idle {} min", (s * 2 + 60) / 120),
        Some(s) => {
            let tenths = (20 * s + 3600) / 7200;
            format!("idle {}.{} h", tenths / 10, tenths % 10)
        }
    }
}

fn server_line(s: &Value) -> String {
    let ports: Vec<String> = s.get("ports").and_then(Value::as_array).map(|p| p.iter().map(|p| num_text(Some(p))).collect()).unwrap_or_default();
    let ports = if ports.is_empty() { "no port".to_string() } else { format!(":{}", ports.join(", :")) };
    let mine = if truthy(s.get("mine")) { "[mine] " } else { "" };
    let orphan = if truthy(s.get("orphan")) { ", session ended" } else { "" };
    let cmd = match s.get("cmd").and_then(Value::as_str).filter(|c| !c.is_empty()) {
        Some(c) => format!("\n  {c}"),
        None => String::new(),
    };
    format!("- {mine}{} ({}, pid {}) {ports} — {}{orphan}{cmd}", str_of(s, "name"), str_of(s, "kind"), num_text(s.get("pid")), idle(as_int(s.get("idleSec"))))
}

pub fn servers_text(res: &Value) -> String {
    let at = Some(str_of(res, "at")).filter(|a| !a.is_empty()).unwrap_or("unknown time");
    let servers = res.get("servers").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut lines = vec![format!("Sample of {at}.")];
    lines.push(if servers.is_empty() { "No agent servers.".to_string() } else { format!("Agent servers ({}):", servers.len()) });
    lines.extend(servers.iter().map(server_line));
    let outside = res.get("outside").and_then(Value::as_array).cloned().unwrap_or_default();
    if !outside.is_empty() {
        lines.push("Ports held outside every agent (cannot be freed from Orrery):".to_string());
        lines.extend(outside.iter().map(|o| format!("- :{} {}", num_text(o.get("port")), str_of(o, "name"))));
    }
    lines.join("\n")
}

pub fn list_servers(_args: &Args, host: &dyn ToolHost) -> Value {
    if !host.take("servers", SERVERS_RATE_MAX, SERVERS_RATE_MS) {
        return servers_fail("rate");
    }
    let Some(res) = desk_call(host, json!({ "tool": "list_servers" })) else { return servers_fail("unavailable") };
    if res.get("ok") != Some(&json!(true)) {
        return servers_fail(str_of(&res, "error"));
    }
    if str_of(&res, "tool") != "list_servers" {
        return servers_fail("failed");
    }
    tool_result(&servers_text(&res), res.clone(), false)
}

fn port_fail(error: &str, detail: Option<&str>) -> Value {
    let text = match error {
        "port" => "port must be a whole number from 1 to 65535.",
        "rate" => "Too many ports were freed from this session in the last minute; wait a little.",
        "no-report" => "Orrery has no process sample yet (Resources samples every 30 s while the window is open); try again shortly.",
        "not-found" => "No agent server listens on that port in the latest Orrery sample (taken every 30 s); a server started just now may need a moment.",
        "not-mine" => "That port is held by a process another session or project started; it can only be stopped from Orrery's Resources by the user.",
        "outside" => "That port is held by a process outside every agent session; Orrery never stops those.",
        "emulator" => "That port belongs to an Android emulator; ask the user to stop it from Orrery Devices.",
        "self" => "That port is held by an agent session itself, not a server; it can't be freed this way.",
        "busy" => "That daemon is building right now (CPU in the last 2 minutes); try again later.",
        "gone" => "That process already exited or was replaced.",
        "cancelled" => "The user declined: nothing was stopped.",
        "confirm-open" => "Another stop request is already waiting for the user; try again after they answer it.",
        "unavailable" => "Orrery cannot stop processes right now.",
        _ => "Orrery could not stop the process.",
    };
    let det = detail.filter(|d| !d.is_empty()).map(|d| format!(" ({d})")).unwrap_or_default();
    tool_result(&format!("{text}{det}"), json!({ "ok": false, "error": error }), true)
}

pub fn free_port(args: &Args, host: &dyn ToolHost) -> Value {
    let port = match args.get("port") {
        Some(v) if v.is_number() => as_int(Some(v)).filter(|p| (1..=65535).contains(p)),
        _ => None,
    };
    let Some(port) = port else { return port_fail("port", None) };
    if !host.take("free-port", FREE_PORT_RATE_MAX, FREE_PORT_RATE_MS) {
        return port_fail("rate", None);
    }
    let Some(res) = desk_call(host, json!({ "tool": "free_port", "port": port })) else { return port_fail("unavailable", None) };
    if res.get("ok") != Some(&json!(true)) {
        let error = str_of(&res, "error");
        let detail = if error == "not-mine" || error == "outside" { res.get("detail").and_then(Value::as_str) } else { None };
        return port_fail(error, detail);
    }
    if str_of(&res, "tool") != "free_port" {
        return port_fail("failed", None);
    }
    let stopped = res.get("stopped").and_then(Value::as_array).cloned().unwrap_or_default();
    let names: Vec<String> = stopped.iter().map(|s| format!("{} (pid {})", str_of(s, "name"), num_text(s.get("pid")))).collect();
    tool_result(&format!("The user confirmed: stopped {}; port {} is free once it has exited.", names.join(", "), num_text(res.get("port"))), res.clone(), false)
}
