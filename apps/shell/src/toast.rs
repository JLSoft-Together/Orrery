use std::path::Path;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Mutex, OnceLock};

use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use tauri::{AppHandle, Manager};

use crate::state::{lock, Shell};

const MAX_TEXT: usize = 4096;
const MAX_ID: usize = 200;
pub const MAX_ANSWERS: usize = 3;
pub const GROUP: &str = "ccm";
pub const POWERSHELL_APP_ID: &str = "{1AC14E77-02E7-4E5D-B744-2EB1AE5198B7}\\WindowsPowerShell\\v1.0\\powershell.exe";

#[derive(Debug, Clone, PartialEq)]
pub struct Answer {
    pub label: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
    pub tag: String,
    pub title: String,
    pub body: String,
    pub url: String,
    pub open: String,
    pub dismiss: String,
    pub persistent: bool,
    pub snooze: Option<(String, String)>,
    pub answers: Vec<Answer>,
}

pub fn tag_of(key: &str) -> String {
    Sha1::digest(key.as_bytes()).iter().take(8).map(|b| format!("{b:02x}")).collect()
}

pub fn js_len(s: &str) -> usize {
    s.encode_utf16().count()
}

fn text(v: &Value, max: usize) -> Option<&str> {
    v.as_str().filter(|s| js_len(s) <= max)
}

pub fn is_host_message(v: &Value) -> bool {
    matches!(v.get("type").and_then(Value::as_str), Some("busy" | "toast.show" | "toast.hide"))
}

pub fn parse_full(v: &Value) -> Option<Toast> {
    if v.get("type").and_then(Value::as_str) != Some("toast.show") {
        return None;
    }
    let tag = text(&v["tag"], MAX_ID).filter(|t| !t.is_empty())?;
    let persistent = v.get("persistent").and_then(Value::as_bool)?;
    let (title, body, url) = (text(&v["title"], MAX_TEXT)?, text(&v["body"], MAX_TEXT)?, text(&v["url"], MAX_TEXT)?);
    let (open, dismiss) = (text(&v["open"], MAX_ID)?, text(&v["dismiss"], MAX_ID)?);
    let snooze_url = match v.get("snoozeUrl") {
        None => None,
        Some(s) => Some(text(s, MAX_TEXT)?),
    };
    let snooze_label = match v.get("snoozeLabel") {
        None => None,
        Some(s) => Some(text(s, MAX_ID)?),
    };
    Some(Toast {
        tag: tag.into(),
        title: title.into(),
        body: body.into(),
        url: url.into(),
        open: open.into(),
        dismiss: dismiss.into(),
        persistent,
        snooze: snooze_url.zip(snooze_label).map(|(u, l)| (u.to_owned(), l.to_owned())),
        answers: Vec::new(),
    })
}

pub fn esc(v: &str) -> String {
    let mut out = String::with_capacity(v.len());
    for c in v.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            c => out.push(c),
        }
    }
    out
}

pub fn xml(t: &Toast) -> String {
    let url = esc(&t.url);
    let scenario = if t.persistent { " scenario='reminder'" } else { "" };
    let answers: String = t.answers.iter().take(MAX_ANSWERS).map(|a| format!("<action content='{}' arguments='{}' activationType='protocol'/>", esc(&a.label), esc(&a.url))).collect();
    let snooze = t.snooze.as_ref().filter(|(u, _)| !u.is_empty()).map_or(String::new(), |(u, l)| format!("<action content='{}' arguments='{}' activationType='protocol'/>", esc(l), esc(u)));
    format!(
        "<toast{scenario} activationType='protocol' launch='{url}'><visual><binding template='ToastGeneric'><text>{}</text><text>{}</text></binding></visual><actions>{answers}<action content='{}' arguments='{url}' activationType='protocol'/>{snooze}<action content='{}' arguments='dismiss' activationType='system'/></actions></toast>",
        esc(&t.title),
        esc(&t.body),
        esc(&t.open),
        esc(&t.dismiss)
    )
}

pub fn dev_build(exe_dir: &Path) -> bool {
    exe_dir.ends_with(Path::new("target").join("debug")) || exe_dir.ends_with(Path::new("target").join("release"))
}

pub fn app_id(identifier: &str, exe_dir: Option<&Path>) -> String {
    if exe_dir.is_none_or(dev_build) {
        POWERSHELL_APP_ID.to_owned()
    } else {
        identifier.to_owned()
    }
}

fn app_id_for(app: &AppHandle) -> String {
    static ID: OnceLock<String> = OnceLock::new();
    ID.get_or_init(|| {
        let exe = std::env::current_exe().ok();
        app_id(&app.config().identifier, exe.as_deref().and_then(Path::parent))
    })
    .clone()
}

type Job = Box<dyn FnOnce() + Send>;

fn run(job: Job) {
    static WORKER: OnceLock<Mutex<Sender<Job>>> = OnceLock::new();
    let tx = WORKER.get_or_init(|| {
        let (tx, rx) = channel::<Job>();
        let _ = std::thread::Builder::new().name("toasts".into()).spawn(move || {
            for job in rx {
                job();
            }
        });
        Mutex::new(tx)
    });
    let _ = lock(tx).send(job);
}

pub fn show_on_worker(app: &AppHandle, t: Toast) {
    let id = app_id_for(app);
    run(Box::new(move || {
        if let Err(e) = native::show(&id, &t) {
            eprintln!("[shell] toast show: {e}");
        }
    }));
}

pub fn hide_on_worker(app: &AppHandle, tag: String) {
    let id = app_id_for(app);
    run(Box::new(move || {
        let _ = native::hide(&id, &tag);
    }));
}

pub fn enabled(app: &AppHandle) -> bool {
    app.state::<Shell>().serve_cfg.toast.is_some()
}

pub fn refresh_health(app: &AppHandle) {
    if !cfg!(windows) || !enabled(app) {
        return;
    }
    let (id, handle) = (app_id_for(app), app.clone());
    run(Box::new(move || {
        let health = native::health(&id);
        if let Some(serve) = lock(&handle.state::<Shell>().serve).as_mut() {
            serve.post(&json!({ "type": "toast.health", "health": health }));
        }
    }));
}

pub fn handle(app: &AppHandle, msg: &Value) {
    if !enabled(app) {
        return;
    }
    if !cfg!(windows) {
        crate::attention::handle(app, msg);
        return;
    }
    match msg.get("type").and_then(Value::as_str) {
        Some("toast.hide") => {
            if let Some(tag) = text(&msg["tag"], MAX_ID).filter(|t| !t.is_empty()) {
                hide_on_worker(app, tag.to_owned());
            }
        }
        Some("toast.show") => {
            let Some(t) = parse_full(msg) else { return };
            let test = t.tag == tag_of("test");
            show_on_worker(app, t);
            if test {
                refresh_health(app);
            }
        }
        _ => {}
    }
}

#[cfg(windows)]
mod native {
    use windows::core::HSTRING;
    use windows::Data::Xml::Dom::XmlDocument;
    use windows::UI::Notifications::{NotificationSetting, ToastNotification, ToastNotificationManager};

    use super::{xml, Toast, GROUP};

    pub fn show(app_id: &str, t: &Toast) -> windows::core::Result<()> {
        let doc = XmlDocument::new()?;
        doc.LoadXml(&HSTRING::from(xml(t)))?;
        let toast = ToastNotification::CreateToastNotification(&doc)?;
        toast.SetTag(&HSTRING::from(t.tag.as_str()))?;
        toast.SetGroup(&HSTRING::from(GROUP))?;
        ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_id))?.Show(&toast)
    }

    pub fn hide(app_id: &str, tag: &str) -> windows::core::Result<()> {
        ToastNotificationManager::History()?.RemoveGroupedTagWithId(&HSTRING::from(tag), &HSTRING::from(GROUP), &HSTRING::from(app_id))
    }

    pub fn health(app_id: &str) -> &'static str {
        match ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(app_id)).and_then(|n| n.Setting()) {
            Ok(NotificationSetting::Enabled) => "ok",
            Ok(NotificationSetting::DisabledForApplication) => "appOff",
            Ok(NotificationSetting::DisabledForUser) => "globalOff",
            _ => "unknown",
        }
    }
}

#[cfg(not(windows))]
mod native {
    use super::Toast;

    pub fn show(_app_id: &str, _t: &Toast) -> Result<(), String> {
        Err("toasts are Windows only".into())
    }

    pub fn hide(_app_id: &str, _tag: &str) -> Result<(), String> {
        Ok(())
    }

    pub fn health(_app_id: &str) -> &'static str {
        "unknown"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full() -> Value {
        json!({ "type": "toast.show", "tag": "abc", "title": "T <1>", "body": "B & 'q'", "url": "agent-monitor://focus?k=a&t=b", "open": "Open", "dismiss": "Dismiss", "persistent": true })
    }

    #[test]
    fn the_tag_is_the_collector_sha1_prefix() {
        assert_eq!(tag_of("test"), "a94a8fe5ccb19ba6");
        assert_eq!(tag_of("waiting:t1").len(), 16);
    }

    #[test]
    fn only_busy_and_toasts_go_to_the_host() {
        for t in ["busy", "toast.show", "toast.hide"] {
            assert!(is_host_message(&json!({ "type": t })));
        }
        assert!(!is_host_message(&json!({ "type": "phone" })));
        assert!(!is_host_message(&json!({ "type": "listening" })));
    }

    #[test]
    fn a_full_toast_parses_like_serve_toast_ts() {
        let t = parse_full(&full()).unwrap();
        assert_eq!((t.tag.as_str(), t.persistent, t.snooze.clone()), ("abc", true, None));
        let mut with = full();
        with["snoozeUrl"] = json!("agent-monitor://snooze?k=a&t=b&m=30");
        with["snoozeLabel"] = json!("Snooze 30 min");
        assert_eq!(parse_full(&with).unwrap().snooze, Some(("agent-monitor://snooze?k=a&t=b&m=30".into(), "Snooze 30 min".into())));
        let mut only_url = full();
        only_url["snoozeUrl"] = json!("x");
        assert_eq!(parse_full(&only_url).unwrap().snooze, None);
        for (k, v) in [("tag", json!("")), ("tag", json!("x".repeat(201))), ("persistent", json!("yes")), ("body", json!(5)), ("open", json!("o".repeat(201))), ("snoozeLabel", json!(1)), ("type", json!("toast.hide"))] {
            let mut bad = full();
            bad[k] = v;
            assert_eq!(parse_full(&bad), None, "{k}");
        }
        let mut short = full();
        short.as_object_mut().unwrap().remove("body");
        assert_eq!(parse_full(&short), None);
    }

    #[test]
    fn the_xml_matches_the_powershell_toast() {
        let mut t = parse_full(&full()).unwrap();
        assert_eq!(
            xml(&t),
            "<toast scenario='reminder' activationType='protocol' launch='agent-monitor://focus?k=a&amp;t=b'><visual><binding template='ToastGeneric'><text>T &lt;1&gt;</text><text>B &amp; &apos;q&apos;</text></binding></visual><actions><action content='Open' arguments='agent-monitor://focus?k=a&amp;t=b' activationType='protocol'/><action content='Dismiss' arguments='dismiss' activationType='system'/></actions></toast>"
        );
        t.persistent = false;
        t.snooze = Some(("agent-monitor://snooze?m=30".into(), "Later".into()));
        t.answers = (0..5).map(|i| Answer { label: format!("A{i}"), url: format!("agent-monitor://menu?i={i}") }).collect();
        let x = xml(&t);
        assert!(x.starts_with("<toast activationType='protocol'"));
        assert!(x.contains("<actions><action content='A0' arguments='agent-monitor://menu?i=0' activationType='protocol'/><action content='A1'"));
        assert!(x.contains("A2") && !x.contains("A3"));
        assert!(x.contains("<action content='Later' arguments='agent-monitor://snooze?m=30' activationType='protocol'/><action content='Dismiss'"));
    }

    #[test]
    #[cfg(windows)]
    fn dev_builds_borrow_the_powershell_app_id() {
        let dev = Path::new("F:\\r\\apps\\shell\\target\\debug");
        assert_eq!(app_id("dev.ccm.monitor", Some(dev)), POWERSHELL_APP_ID);
        assert_eq!(app_id("dev.ccm.monitor", Some(Path::new("/r/target/release"))), POWERSHELL_APP_ID);
        assert_eq!(app_id("dev.ccm.monitor", Some(Path::new("C:\\Program Files\\Orrery"))), "dev.ccm.monitor");
        assert_eq!(app_id("dev.ccm.monitor", None), POWERSHELL_APP_ID);
    }

    #[cfg(windows)]
    #[test]
    #[ignore]
    fn a_real_toast_shows_and_hides() {
        let mut t = parse_full(&full()).unwrap();
        t.tag = tag_of("wp104m-probe");
        t.title = "Orrery WP-104m".into();
        t.body = "Toast probe from cargo test".into();
        t.persistent = false;
        t.answers = vec![Answer { label: "1. Yes".into(), url: "agent-monitor://menu?k=0".into() }];
        native::show(POWERSHELL_APP_ID, &t).unwrap();
        assert_eq!(native::health(POWERSHELL_APP_ID), "ok");
        std::thread::sleep(std::time::Duration::from_secs(3));
        native::hide(POWERSHELL_APP_ID, &t.tag).unwrap();
    }
}
