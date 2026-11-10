use std::time::{Duration, Instant};

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

use crate::config::{uuid, LINK_SCHEME};
use crate::presets::{is_session_id, resolve_cwd};
use crate::serve::request;
use crate::state::{Shell, MAIN};
use crate::window::show_main;

const CWD_MAX: usize = 1024;
const PANE_OPEN: Duration = Duration::from_secs(5);
const FOCUS_ASK: Duration = Duration::from_millis(1500);
const LINK_PRESETS: [&str; 4] = ["claude", "codex", "deepseek", "shell"];

#[derive(Debug, PartialEq)]
pub enum Link {
    Open { preset: String, cwd: String, resume: Option<String> },
    Focus { key: String, terminal: Option<String> },
    Snooze { query: String },
}

fn win_absolute(cwd: &str) -> bool {
    let b = cwd.as_bytes();
    b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && (b[2] == b'\\' || b[2] == b'/')
}

pub fn parse(raw: &str, windows: bool) -> Option<Link> {
    let url = tauri::Url::parse(raw).ok()?;
    if url.scheme() != LINK_SCHEME {
        return None;
    }
    let get = |k: &str| url.query_pairs().find(|(n, _)| n == k).map(|(_, v)| v.into_owned());
    match url.host_str()? {
        "open" => {
            let preset = get("agent").unwrap_or_else(|| "claude".into());
            let cwd = get("cwd")?;
            let control = cwd.chars().any(|c| c.is_control());
            let unc = cwd.starts_with("\\\\") || cwd.starts_with("//");
            if !LINK_PRESETS.contains(&preset.as_str()) || cwd.is_empty() || cwd.chars().count() > CWD_MAX || control || unc {
                return None;
            }
            let absolute = if windows { win_absolute(&cwd) } else { cwd.starts_with('/') };
            if !absolute {
                return None;
            }
            match get("resume") {
                None => Some(Link::Open { preset, cwd, resume: None }),
                Some(r) if preset != "shell" && is_session_id(&r) => Some(Link::Open { preset, cwd, resume: Some(r) }),
                Some(_) => None,
            }
        }
        "focus" => {
            let key = get("k").filter(|k| !k.is_empty())?;
            Some(Link::Focus { key, terminal: get("t").filter(|t| !t.is_empty()) })
        }
        "snooze" => Some(Link::Snooze { query: url.query().unwrap_or("").to_owned() }),
        _ => None,
    }
}

pub fn link_in(args: &[String]) -> Option<String> {
    let prefix = format!("{LINK_SCHEME}://");
    args.iter().find(|a| a.starts_with(&prefix)).cloned()
}

fn warn(app: &AppHandle, message: &str, detail: Option<&str>) {
    let text = detail.map_or_else(|| message.to_owned(), |d| format!("{message}\n\n{d}"));
    app.dialog().message(text).title("Orrery").kind(MessageDialogKind::Warning).blocking_show();
}

fn open(app: &AppHandle, preset: String, cwd: String, resume: Option<String>) {
    let shell = app.state::<Shell>();
    if !shell.take_link_slot(Instant::now()) || app.get_webview_window(MAIN).is_none() {
        return;
    }
    show_main(app);
    let t = shell.t();
    let home = std::env::temp_dir();
    let Some(real) = resolve_cwd(Some(&cwd), &home) else { return warn(app, t.link_bad_folder, Some(&cwd)) };
    let mut args = json!({ "id": uuid(), "preset": preset, "cwd": real.to_string_lossy() });
    if let Some(r) = resume {
        args["resume"] = json!(r);
    }
    let id = uuid();
    let req = json!({ "id": id, "action": "session.open", "args": args, "caller": { "source": "url" } });
    let result = shell.ask(app, &id, "ccm:orrery", req, PANE_OPEN).unwrap_or(Value::Null);
    let ok = result.get("ok") == Some(&Value::Bool(true)) && result.get("paneKey").is_some_and(Value::is_string);
    let error = result.get("error").and_then(Value::as_str).unwrap_or("failed");
    if !ok && error != "setup" {
        warn(app, if error == "live" { t.link_live } else { t.link_failed }, None);
    }
}

fn focus(app: &AppHandle, key: String, terminal: Option<String>) {
    let shell = app.state::<Shell>();
    let Some(terminal) = terminal else { return show_main(app) };
    let handled = shell.ask(app, "focus", "ccm:focus", json!(terminal), FOCUS_ASK);
    if handled == Some(Value::Bool(true)) {
        return show_main(app);
    }
    let body = json!({ "t": terminal, "k": key }).to_string();
    let reply = request(shell.port, "POST", "/focus", Some(&shell.key), Some(&body));
    let result = reply.and_then(|r| serde_json::from_str::<Value>(&r.body).ok()).and_then(|v| v.get("result").and_then(Value::as_str).map(str::to_owned));
    if result.as_deref() == Some("ok") {
        return;
    }
    show_main(app);
    let _ = app.emit_to(MAIN, "ccm:select", terminal);
}

pub fn handle(app: &AppHandle, raw: &str) {
    let Some(link) = parse(raw, cfg!(windows)) else { return };
    let app = app.clone();
    std::thread::spawn(move || match link {
        Link::Open { preset, cwd, resume } => open(&app, preset, cwd, resume),
        Link::Focus { key, terminal } => focus(&app, key, terminal),
        Link::Snooze { query } => {
            let shell = app.state::<Shell>();
            let _ = request(shell.port, "GET", &format!("/snooze?{query}"), Some(&shell.key), None);
        }
    });
}

pub fn second_instance(app: &AppHandle, args: Vec<String>) {
    match link_in(&args) {
        Some(link) => handle(app, &link),
        None => show_main(app),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "0f8fad5b-d9cb-469f-a165-70867728950e";

    #[test]
    fn open_links_need_a_preset_and_an_absolute_local_folder() {
        assert_eq!(parse("agent-monitor://open?cwd=D:%5Cwork", true), Some(Link::Open { preset: "claude".into(), cwd: "D:\\work".into(), resume: None }));
        assert_eq!(parse("agent-monitor://open?agent=shell&cwd=/Users/a", false), Some(Link::Open { preset: "shell".into(), cwd: "/Users/a".into(), resume: None }));
        assert_eq!(
            parse(&format!("agent-monitor://open?agent=codex&cwd=C:/x&resume={ID}"), true),
            Some(Link::Open { preset: "codex".into(), cwd: "C:/x".into(), resume: Some(ID.into()) })
        );
        for bad in [
            "agent-monitor://open?cwd=relative",
            "agent-monitor://open?cwd=%5C%5Cserver%5Cshare",
            "agent-monitor://open?agent=rm&cwd=C:%5Cx",
            "agent-monitor://open?cwd=C:%5Cx%0A",
            "agent-monitor://open?agent=shell&cwd=C:%5Cx&resume=0f8fad5b-d9cb-469f-a165-70867728950e",
            "agent-monitor://open?cwd=C:%5Cx&resume=nope",
            "agent-monitor://open",
            "other://open?cwd=C:%5Cx",
            "agent-monitor://nope?cwd=C:%5Cx",
        ] {
            assert_eq!(parse(bad, true), None, "{bad}");
        }
        assert_eq!(parse("agent-monitor://open?cwd=/Users/a", true), None);
    }

    #[test]
    fn toast_links_carry_their_key() {
        assert_eq!(parse("agent-monitor://focus?k=abc&t=t1", true), Some(Link::Focus { key: "abc".into(), terminal: Some("t1".into()) }));
        assert_eq!(parse("agent-monitor://focus?t=t1", true), None);
        assert_eq!(parse("agent-monitor://snooze?k=a&t=b&m=5", true), Some(Link::Snooze { query: "k=a&t=b&m=5".into() }));
    }

    #[test]
    fn the_link_is_found_among_the_second_instance_args() {
        let args = vec!["C:\\orrery.exe".to_owned(), "agent-monitor://open?cwd=C:%5Cx".to_owned()];
        assert_eq!(link_in(&args).as_deref(), Some("agent-monitor://open?cwd=C:%5Cx"));
        assert_eq!(link_in(&["x".to_owned()]), None);
    }
}
