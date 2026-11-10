use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};
use tauri::webview::{NewWindowResponse, PageLoadEvent};
use tauri::{AppHandle, Emitter, LogicalPosition, LogicalSize, Manager, Url, Webview, WebviewBuilder, WebviewUrl};

use crate::appearance::zoom_factor;
use crate::config::random_hex;
use crate::hosts::eval_value;
use crate::native;
use crate::state::{lock, Shell};
use crate::window::open_external;

pub const EVENT: &str = "shell:guest";
const PREFIX: &str = "guest-";
const SCRIPT: &str = include_str!("guest.js");
const EXEC_START: Duration = Duration::from_secs(5);
const EXEC_MAX: Duration = Duration::from_secs(600);

pub struct Guest {
    pub owner: String,
    pub zoom: f64,
    pub scale: f64,
    pub shown: bool,
    pub place: Place,
}

type Reply = (u64, mpsc::Sender<Result<Value, String>>);

#[derive(Default)]
pub struct Guests {
    next: AtomicU64,
    map: Mutex<HashMap<u64, Guest>>,
    replies: Mutex<HashMap<String, Reply>>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Place {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub show: bool,
    pub scale: f64,
}

pub fn label(id: u64) -> String {
    format!("{PREFIX}{id}")
}

pub fn id_of(label: &str) -> Option<u64> {
    label.strip_prefix(PREFIX)?.parse().ok()
}

pub fn web_url(raw: &str, app_origin: &str) -> Option<Url> {
    let url = Url::parse(raw).ok()?;
    let web = matches!(url.scheme(), "http" | "https") && url.host_str().is_some_and(|h| !h.is_empty());
    (web && url.origin().ascii_serialization() != app_origin).then_some(url)
}

pub fn is_loopback(url: &Url) -> bool {
    matches!(url.host_str(), Some("127.0.0.1" | "localhost" | "[::1]"))
}

pub fn parse_place(raw: &Value) -> Option<Place> {
    let n = |k: &str| raw.get(k).and_then(Value::as_f64).filter(|v| v.is_finite());
    let scale = n("scale").filter(|s| *s > 0.01 && *s <= 4.0).unwrap_or(1.0);
    Some(Place { x: n("x")?, y: n("y")?, w: n("w")?.max(1.0), h: n("h")?.max(1.0), show: raw.get("show") == Some(&Value::Bool(true)), scale })
}

pub fn clean_zoom(v: &Value) -> f64 {
    v.as_f64().filter(|z| z.is_finite() && *z >= 0.25 && *z <= 5.0).unwrap_or(1.0)
}

fn main_zoom(app: &AppHandle) -> f64 {
    zoom_factor(lock(&app.state::<Shell>().look).state.zoom)
}

impl Guests {
    pub fn owner(&self, id: u64) -> Option<String> {
        lock(&self.map).get(&id).map(|g| g.owner.clone())
    }

    fn fail_replies(&self, id: u64, why: &str) {
        let mut replies = lock(&self.replies);
        let gone: Vec<String> = replies.iter().filter(|(_, (g, _))| *g == id).map(|(k, _)| k.clone()).collect();
        for k in gone {
            if let Some((_, tx)) = replies.remove(&k) {
                let _ = tx.send(Err(why.to_owned()));
            }
        }
    }

    pub fn reply(&self, id: u64, nonce: &str, result: Result<Value, String>) {
        let mut replies = lock(&self.replies);
        if replies.get(nonce).is_some_and(|(g, _)| *g == id) {
            if let Some((_, tx)) = replies.remove(nonce) {
                let _ = tx.send(result);
            }
        }
    }
}

pub fn emit(app: &AppHandle, id: u64, payload: Value) {
    let Some(owner) = app.state::<Guests>().owner(id) else { return };
    let mut payload = payload;
    payload["id"] = json!(id);
    let _ = app.emit_to(owner.as_str(), EVENT, payload);
}

pub fn emit_history(app: &AppHandle, id: u64, kind: &str) {
    let app = app.clone();
    let kind = kind.to_owned();
    std::thread::spawn(move || {
        let Some(wv) = app.get_webview(&label(id)) else { return };
        let (back, forward) = native::history(&wv);
        let url = wv.url().map(|u| u.to_string()).unwrap_or_default();
        emit(&app, id, json!({ "type": kind, "url": url, "canBack": back, "canForward": forward }));
    });
}

fn builder(app: &AppHandle, id: u64, url: Url) -> WebviewBuilder<tauri::Wry> {
    let shell = app.state::<Shell>();
    let origin = shell.origin.clone();
    let (nav_origin, pop_app, load_app, title_app) = (origin.clone(), app.clone(), app.clone(), app.clone());
    WebviewBuilder::new(label(id), WebviewUrl::External(url))
        .data_directory(shell.webview_dir.clone())
        .initialization_script(SCRIPT)
        .zoom_hotkeys_enabled(false)
        .devtools(crate::config::devtools_allowed())
        .on_navigation(move |u| web_url(u.as_str(), &nav_origin).is_some())
        .on_new_window(move |u, _| {
            match web_url(u.as_str(), &origin) {
                Some(u) if is_loopback(&u) => {
                    if let Some(wv) = pop_app.get_webview(&label(id)) {
                        let _ = wv.navigate(u);
                    }
                }
                Some(u) => open_external(&pop_app, u.as_str()),
                None => {}
            }
            NewWindowResponse::Deny
        })
        .on_page_load(move |_, payload| match payload.event() {
            PageLoadEvent::Started => {
                load_app.state::<Guests>().fail_replies(id, "navigated");
                emit(&load_app, id, json!({ "type": "start", "url": payload.url().as_str() }));
            }
            PageLoadEvent::Finished => emit_history(&load_app, id, "finish"),
        })
        .on_document_title_changed(move |_, title| emit(&title_app, id, json!({ "type": "title", "title": title })))
}

fn apply(app: &AppHandle, wv: &Webview, guest: &mut Guest, place: Place, first: bool) {
    let z = main_zoom(app);
    guest.place = place;
    let _ = wv.set_position(LogicalPosition::new(place.x * z, place.y * z));
    let _ = wv.set_size(LogicalSize::new(place.w * z, place.h * z));
    if first || place.scale != guest.scale {
        guest.scale = place.scale;
        let _ = wv.set_zoom(guest.zoom * guest.scale * z);
    }
    if first || place.show != guest.shown {
        guest.shown = place.show;
        let _ = if place.show { wv.show() } else { wv.hide() };
    }
}

pub fn open(app: &AppHandle, owner: &str, raw_url: &str, place: Place, zoom: f64) -> Result<u64, String> {
    let shell = app.state::<Shell>();
    let url = web_url(raw_url, &shell.origin).ok_or("url")?;
    let window = app.get_window(owner).ok_or("no window")?;
    let guests = app.state::<Guests>();
    let id = guests.next.fetch_add(1, Ordering::SeqCst) + 1;
    lock(&guests.map).insert(id, Guest { owner: owner.to_owned(), zoom, scale: place.scale, shown: false, place });
    let z = main_zoom(app);
    let created = window.add_child(builder(app, id, url), LogicalPosition::new(place.x * z, place.y * z), LogicalSize::new(place.w * z, place.h * z));
    let wv = match created {
        Ok(wv) => wv,
        Err(e) => {
            lock(&guests.map).remove(&id);
            return Err(e.to_string());
        }
    };
    if let Some(g) = lock(&guests.map).get_mut(&id) {
        apply(app, &wv, g, place, true);
    }
    Ok(id)
}

pub fn refresh_all(app: &AppHandle) {
    let z = main_zoom(app);
    let guests = app.state::<Guests>();
    for (id, g) in lock(&guests.map).iter() {
        if let Some(wv) = app.get_webview(&label(*id)) {
            let p = g.place;
            let _ = wv.set_position(LogicalPosition::new(p.x * z, p.y * z));
            let _ = wv.set_size(LogicalSize::new(p.w * z, p.h * z));
            let _ = wv.set_zoom(g.zoom * g.scale * z);
        }
    }
}

pub fn close(app: &AppHandle, id: u64) {
    let guests = app.state::<Guests>();
    lock(&guests.map).remove(&id);
    guests.fail_replies(id, "closed");
    if let Some(wv) = app.get_webview(&label(id)) {
        let _ = wv.close();
    }
}

pub fn close_owned(app: &AppHandle, owner: &str) {
    let ids: Vec<u64> = lock(&app.state::<Guests>().map).iter().filter(|(_, g)| g.owner == owner).map(|(id, _)| *id).collect();
    for id in ids {
        close(app, id);
    }
}

pub fn exec_script(code: &str, nonce: &str) -> String {
    let body = code.trim_end().trim_end_matches(';');
    let n = Value::String(nonce.to_owned());
    format!("(()=>{{const r=window.__ccmGuestReply,n={n};if(typeof r!=='function')return false;try{{Promise.resolve(({body}\n)).then((v)=>r(n,true,v),(e)=>r(n,false,e))}}catch(e){{r(n,false,e)}}return true}})()")
}

fn exec(app: &AppHandle, id: u64, wv: &Webview, code: &str) -> Result<Value, String> {
    let guests = app.state::<Guests>();
    let nonce = random_hex(16);
    let (tx, rx) = mpsc::channel();
    lock(&guests.replies).insert(nonce.clone(), (id, tx));
    if eval_value(wv, &exec_script(code, &nonce), EXEC_START) != Some(Value::Bool(true)) {
        lock(&guests.replies).remove(&nonce);
        return Err("script failed".into());
    }
    let got = rx.recv_timeout(EXEC_MAX);
    lock(&guests.replies).remove(&nonce);
    got.map_err(|_| "no reply".to_owned())?
}

pub fn capture_data_url(wv: &Webview) -> Result<Value, String> {
    let png = native::capture(wv)?;
    Ok(json!(format!("data:image/png;base64,{}", crate::shot::base64(&png))))
}

fn devtools(wv: &Webview, action: &str) -> bool {
    let open = match action {
        "open" => true,
        "close" => false,
        _ => !wv.is_devtools_open(),
    } && crate::config::devtools_allowed();
    if open {
        wv.open_devtools();
    } else {
        wv.close_devtools();
    }
    open
}

pub fn toggle_devtools(app: &AppHandle, id: u64) {
    if let Some(wv) = app.get_webview(&label(id)) {
        let open = devtools(&wv, "toggle");
        emit(app, id, json!({ "type": "devtools", "open": open }));
    }
}

pub fn op(app: &AppHandle, owner: &str, args: &[Value]) -> Result<Value, String> {
    let arg = |i: usize| args.get(i).unwrap_or(&Value::Null);
    let name = arg(0).as_str().unwrap_or("");
    if name == "open" {
        let place = parse_place(arg(3)).ok_or("place")?;
        return open(app, owner, arg(2).as_str().unwrap_or(""), place, clean_zoom(arg(4))).map(|id| json!(id));
    }
    let id = arg(1).as_u64().ok_or("id")?;
    let guests = app.state::<Guests>();
    if guests.owner(id).as_deref() != Some(owner) {
        return Err("no guest".into());
    }
    if name == "close" {
        close(app, id);
        return Ok(Value::Null);
    }
    let wv = app.get_webview(&label(id)).ok_or("no guest")?;
    let script = |js: &str| wv.eval(js).map(|()| Value::Null).map_err(|e| e.to_string());
    match name {
        "bounds" => {
            let place = parse_place(arg(2)).ok_or("place")?;
            if let Some(g) = lock(&guests.map).get_mut(&id) {
                apply(app, &wv, g, place, false);
            }
            Ok(Value::Null)
        }
        "load" => {
            let url = web_url(arg(2).as_str().unwrap_or(""), &app.state::<Shell>().origin).ok_or("url")?;
            wv.navigate(url).map(|()| Value::Null).map_err(|e| e.to_string())
        }
        "back" => script("history.back()"),
        "forward" => script("history.forward()"),
        "stop" => script("window.stop()"),
        "reload" => script("location.reload()"),
        "zoom" => {
            let z = clean_zoom(arg(2));
            let factor = lock(&guests.map).get_mut(&id).map(|g| {
                g.zoom = z;
                g.zoom * g.scale
            });
            if let Some(f) = factor {
                let _ = wv.set_zoom(f * main_zoom(app));
            }
            Ok(Value::Null)
        }
        "focus" => wv.set_focus().map(|()| Value::Null).map_err(|e| e.to_string()),
        "devtools" => Ok(json!(devtools(&wv, arg(2).as_str().unwrap_or("toggle")))),
        "exec" => exec(app, id, &wv, arg(2).as_str().unwrap_or("undefined")),
        "capture" => capture_data_url(&wv),
        _ => Err(format!("unknown op {name}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_round_trip() {
        assert_eq!(label(7), "guest-7");
        assert_eq!(id_of("guest-7"), Some(7));
        assert_eq!(id_of("main"), None);
        assert_eq!(id_of("guest-x"), None);
    }

    #[test]
    fn guests_load_web_pages_but_never_the_app() {
        let app = "http://127.0.0.1:4419";
        assert!(web_url("https://example.com/a", app).is_some());
        assert!(web_url("http://127.0.0.1:5173/x", app).is_some());
        assert!(web_url("http://127.0.0.1:4419/", app).is_none());
        assert!(web_url("file:///C:/x.html", app).is_none());
        assert!(web_url("javascript:alert(1)", app).is_none());
        assert!(web_url("about:blank", app).is_none());
        assert!(is_loopback(&Url::parse("http://localhost:3000").unwrap()));
        assert!(!is_loopback(&Url::parse("https://example.com").unwrap()));
    }

    #[test]
    fn places_need_numbers_and_keep_sane_scales() {
        let p = parse_place(&json!({"x": 1, "y": 2, "w": 0, "h": 40, "show": true, "scale": 0.5})).unwrap();
        assert_eq!(p, Place { x: 1.0, y: 2.0, w: 1.0, h: 40.0, show: true, scale: 0.5 });
        assert_eq!(parse_place(&json!({"x": 1, "y": 2, "w": 3, "h": 4, "scale": 99})).unwrap().scale, 1.0);
        assert!(!parse_place(&json!({"x": 1, "y": 2, "w": 3, "h": 4})).unwrap().show);
        assert!(parse_place(&json!({"x": "1", "y": 2, "w": 3, "h": 4})).is_none());
        assert_eq!(clean_zoom(&json!(1.5)), 1.5);
        assert_eq!(clean_zoom(&json!(50)), 1.0);
    }

    #[test]
    fn exec_wraps_the_expression_and_awaits_it() {
        let s = exec_script("document.title;  ", "ab");
        assert!(s.contains("Promise.resolve((document.title\n))"));
        assert!(s.contains("n=\"ab\""));
        assert!(s.ends_with("return true})()"));
    }

    #[test]
    fn replies_only_reach_the_guest_that_asked() {
        let g = Guests::default();
        let (tx, rx) = mpsc::channel();
        lock(&g.replies).insert("n1".into(), (3, tx));
        g.reply(4, "n1", Ok(json!(1)));
        assert!(rx.try_recv().is_err());
        g.reply(3, "n1", Ok(json!(2)));
        assert_eq!(rx.try_recv().unwrap(), Ok(json!(2)));
        let (tx, rx) = mpsc::channel();
        lock(&g.replies).insert("n2".into(), (5, tx));
        g.fail_replies(5, "closed");
        assert_eq!(rx.try_recv().unwrap(), Err("closed".into()));
    }
}
