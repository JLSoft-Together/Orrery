use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use tauri::{AppHandle, LogicalSize, Manager, Theme};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio_tungstenite::tungstenite::Message;

use crate::state::{lock, MAIN};

const EVAL_WAIT: Duration = Duration::from_secs(20);
const HELPERS: &str = include_str!("e2e_cdp.js");
const CLIPBOARD: &str = "(()=>{const c=navigator.clipboard;if(!c||c.__ccmE2E)return;const w=c.writeText.bind(c),r=c.readText.bind(c);let mem=null;const late=(p,ms)=>Promise.race([p,new Promise((_,no)=>setTimeout(()=>no(new Error('late')),ms))]);const set=DataTransfer.prototype.setData;DataTransfer.prototype.setData=function(t,v){if(/^text(\\/plain)?$/i.test(t))mem=String(v);return set.call(this,t,v)};document.addEventListener('copy',()=>{mem=String(getSelection()||'')},true);document.addEventListener('cut',()=>{mem=String(getSelection()||'')},true);c.writeText=async(t)=>{mem=String(t);try{await late(w(t),1000)}catch{}};c.readText=async()=>{if(mem!==null)return mem;try{return await late(r(),1000)}catch{return ''}};c.__ccmE2E=true})()";

struct State {
    clients: Mutex<Vec<UnboundedSender<String>>>,
    init: Mutex<Vec<String>>,
    size: Mutex<Option<LogicalSize<f64>>>,
}

static STATE: OnceLock<State> = OnceLock::new();
static SEQ: AtomicU64 = AtomicU64::new(0);

fn state() -> &'static State {
    STATE.get_or_init(|| State { clients: Mutex::new(Vec::new()), init: Mutex::new(Vec::new()), size: Mutex::new(None) })
}

pub fn port_from(mac: bool, allowed: bool, raw: Option<String>) -> Option<u16> {
    if !mac || !allowed {
        return None;
    }
    raw?.trim().parse::<u16>().ok().filter(|p| *p != 0 && *p < u16::MAX)
}

fn port() -> Option<u16> {
    port_from(cfg!(target_os = "macos"), crate::config::devtools_allowed(), std::env::var("CCM_E2E_CDP_PORT").ok())
}

pub fn start(app: &AppHandle) {
    let Some(port) = port() else { return };
    state();
    let (a, b) = (app.clone(), app.clone());
    tauri::async_runtime::spawn(async move {
        if let Err(e) = serve_json(a, port).await {
            eprintln!("[shell] e2e cdp json: {e}");
        }
    });
    tauri::async_runtime::spawn(async move {
        if let Err(e) = serve_ws(b, port + 1).await {
            eprintln!("[shell] e2e cdp ws: {e}");
        }
    });
    eprintln!("[shell] e2e cdp on 127.0.0.1:{port}");
}

pub fn page_loaded(app: &AppHandle) {
    let Some(st) = STATE.get() else { return };
    if let Some(w) = app.get_webview_window(MAIN) {
        let _ = w.eval(CLIPBOARD);
        for src in lock(&st.init).iter() {
            let _ = w.eval(src);
        }
    }
    broadcast(&json!({ "method": "Page.loadEventFired", "params": { "timestamp": 0 } }).to_string());
}

fn broadcast(msg: &str) {
    let Some(st) = STATE.get() else { return };
    lock(&st.clients).retain(|tx| tx.send(msg.to_owned()).is_ok());
}

pub fn targets(url: &str, ws_port: u16) -> Value {
    json!([{ "id": "main", "type": "page", "title": "Orrery", "url": url, "webSocketDebuggerUrl": format!("ws://127.0.0.1:{ws_port}/devtools/page/main") }])
}

async fn serve_json(app: AppHandle, port: u16) -> std::io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    loop {
        let (mut sock, _) = listener.accept().await?;
        let url = app.get_webview_window(MAIN).and_then(|w| w.url().ok()).map(|u| u.to_string()).unwrap_or_default();
        tokio::spawn(async move {
            let mut buf = [0u8; 4096];
            let _ = sock.read(&mut buf).await;
            let body = targets(&url, port + 1).to_string();
            let head = format!("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len());
            let _ = sock.write_all(head.as_bytes()).await;
            let _ = sock.write_all(body.as_bytes()).await;
            let _ = sock.shutdown().await;
        });
    }
}

async fn serve_ws(app: AppHandle, port: u16) -> std::io::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", port)).await?;
    loop {
        let (sock, _) = listener.accept().await?;
        let app = app.clone();
        tokio::spawn(async move {
            let Ok(ws) = tokio_tungstenite::accept_async(sock).await else { return };
            let (mut sink, mut stream) = ws.split();
            let (tx, mut rx) = unbounded_channel::<String>();
            lock(&state().clients).push(tx.clone());
            let writer = tokio::spawn(async move {
                while let Some(m) = rx.recv().await {
                    if sink.send(Message::text(m)).await.is_err() {
                        break;
                    }
                }
            });
            while let Some(Ok(msg)) = stream.next().await {
                let Message::Text(text) = msg else { continue };
                let Ok(req) = serde_json::from_str::<Value>(text.as_str()) else { continue };
                let (app, tx) = (app.clone(), tx.clone());
                tokio::spawn(async move {
                    let id = req.get("id").cloned().unwrap_or(Value::Null);
                    let method = req.get("method").and_then(Value::as_str).unwrap_or_default().to_owned();
                    let params = req.get("params").cloned().unwrap_or_else(|| json!({}));
                    let reply = match handle(&app, &method, &params).await {
                        Ok(result) => json!({ "id": id, "result": result }),
                        Err(e) => json!({ "id": id, "error": { "code": -32000, "message": e } }),
                    };
                    let _ = tx.send(reply.to_string());
                });
            }
            writer.abort();
        });
    }
}

const QUIET: &str = "if(!globalThis.__ccmE2Equiet){globalThis.__ccmE2Equiet=(e)=>{if(e&&(e.message==='Script error.'||/SyntaxError/.test(e.message||''))){e.stopImmediatePropagation();e.preventDefault();removeEventListener('error',globalThis.__ccmE2Equiet,true)}}}addEventListener('error',globalThis.__ccmE2Equiet,true);return 'null'";
const UNQUIET: &str = "removeEventListener('error',globalThis.__ccmE2Equiet||(()=>{}),true);";

pub fn may_be_statements(expr: &str) -> bool {
    let t = expr.trim_start();
    const KEYWORDS: [&str; 14] = ["const ", "let ", "var ", "if ", "if(", "for ", "for(", "while", "try", "function ", "class ", "throw ", "return ", "switch"];
    expr.contains(';') || expr.trim().contains('\n') || KEYWORDS.iter().any(|k| t.starts_with(k))
}

pub fn split_last(src: &str) -> Option<(String, String)> {
    let b = src.as_bytes();
    let (mut i, mut depth, mut cuts) = (0usize, 0i32, Vec::new());
    let mut tpl: Vec<i32> = Vec::new();
    while i < b.len() {
        let c = b[i];
        match c {
            b'\'' | b'"' => {
                i += 1;
                while i < b.len() && b[i] != c {
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
            }
            b'`' => tpl.push(depth),
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            b'/' if b.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < b.len() && !(b[i] == b'*' && b[i + 1] == b'/') {
                    i += 1;
                }
                i += 1;
            }
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => depth -= 1,
            b';' if depth == 0 => cuts.push(i),
            _ => {}
        }
        if c == b'`' {
            i += 1;
            while i < b.len() && b[i] != b'`' {
                if b[i] == b'\\' {
                    i += 1;
                } else if b[i] == b'$' && b.get(i + 1) == Some(&b'{') {
                    let mut d = 0;
                    i += 1;
                    while i < b.len() {
                        match b[i] {
                            b'{' => d += 1,
                            b'}' => {
                                d -= 1;
                                if d == 0 {
                                    break;
                                }
                            }
                            _ => {}
                        }
                        i += 1;
                    }
                }
                i += 1;
            }
            tpl.pop();
        }
        i += 1;
    }
    let trimmed = src.trim_end();
    let cuts: Vec<usize> = cuts.into_iter().filter(|&c| c < trimmed.len().saturating_sub(1) || !trimmed.ends_with(';')).collect();
    let body = trimmed.strip_suffix(';').unwrap_or(trimmed);
    let cut = *cuts.iter().filter(|&&c| c < body.len()).last()?;
    let (head, last) = (&body[..cut], body[cut + 1..].trim());
    const KEYWORDS: [&str; 15] = ["const ", "let ", "var ", "if ", "if(", "for ", "for(", "while", "try", "function ", "class ", "throw ", "return ", "switch", "{"];
    if last.is_empty() || KEYWORDS.iter().any(|k| last.starts_with(k)) {
        return None;
    }
    Some((head.to_owned(), last.to_owned()))
}

const RESULT: &str = "try{return JSON.stringify({ok:true,t:typeof v,v:v===undefined?null:v})}catch(_){return JSON.stringify({ok:true,t:typeof v,v:String(v)})}";

pub fn eval_bodies(expr: &str, token: &str) -> Vec<String> {
    let t = serde_json::to_string(token).unwrap_or_default();
    let guard = format!("{UNQUIET}if(globalThis.__ccmE2Eran==={t})return JSON.stringify({{ran:true}});globalThis.__ccmE2Eran={t};");
    let mut out = vec![format!("{UNQUIET}globalThis.__ccmE2Eran={t};const v=await (\n{expr}\n);{RESULT}")];
    if let Some((head, last)) = split_last(expr) {
        out.push(format!("{guard}\n{head}\n;const v=await (\n{last}\n);{RESULT}"));
    }
    out.push(format!("{guard}\n{expr}\n;return JSON.stringify({{ok:true,t:'undefined',v:null}})"));
    out
}

fn parse_reply(raw: &str) -> Value {
    serde_json::from_str(raw).unwrap_or_else(|_| json!({ "ok": true, "t": "undefined", "v": null }))
}

async fn call(app: &AppHandle, body: String) -> Result<Result<String, String>, String> {
    let wv = app.get_webview(MAIN).ok_or("no webview")?;
    tauri::async_runtime::spawn_blocking(move || crate::native::call_js(&wv, body, EVAL_WAIT)).await.map_err(|e| e.to_string())
}

async fn eval(app: &AppHandle, expr: &str) -> Result<Value, String> {
    let token = format!("e2e:{}", SEQ.fetch_add(1, Ordering::SeqCst));
    let probe = may_be_statements(expr);
    let bodies = eval_bodies(expr, &token);
    let mut first: Option<String> = None;
    for (n, body) in bodies.into_iter().enumerate() {
        if n > 0 && !probe {
            break;
        }
        if probe {
            call(app, QUIET.to_owned()).await?.ok();
        }
        match call(app, body).await? {
            Ok(raw) => {
                let r = parse_reply(&raw);
                return Ok(match (r.get("ran") == Some(&Value::Bool(true)), first) {
                    (true, Some(e)) => json!({ "ok": false, "e": e }),
                    _ => r,
                });
            }
            Err(e) if e.contains("timeout") => return Err(e),
            Err(e) if e.starts_with("SyntaxError") => {
                first.get_or_insert(e);
            }
            Err(e) => return Ok(json!({ "ok": false, "e": e })),
        }
    }
    Ok(json!({ "ok": false, "e": first.unwrap_or_else(|| "SyntaxError".into()) }))
}

pub fn evaluate_reply(r: &Value) -> Value {
    if r.get("ok") == Some(&Value::Bool(true)) {
        let t = r.get("t").and_then(Value::as_str).unwrap_or("object");
        json!({ "result": { "type": t, "value": r.get("v").cloned().unwrap_or(Value::Null) } })
    } else {
        let e = r.get("e").and_then(Value::as_str).unwrap_or("error");
        json!({ "result": { "type": "object", "subtype": "error", "description": e }, "exceptionDetails": { "text": e, "exception": { "description": e } } })
    }
}

#[cfg(not(target_os = "macos"))]
fn drag_files(_paths: &[String]) {}

#[cfg(target_os = "macos")]
fn drag_files(paths: &[String]) {
    use objc2::runtime::ProtocolObject;
    use objc2_app_kit::{NSPasteboard, NSPasteboardNameDrag, NSPasteboardWriting};
    use objc2_foundation::{NSArray, NSString, NSURL};

    let pb = NSPasteboard::pasteboardWithName(unsafe { NSPasteboardNameDrag });
    pb.clearContents();
    let urls: Vec<_> = paths.iter().map(|p| NSURL::fileURLWithPath(&NSString::from_str(p))).collect();
    let items: Vec<&ProtocolObject<dyn NSPasteboardWriting>> = urls.iter().map(|u| ProtocolObject::from_ref(&**u)).collect();
    pb.writeObjects(&NSArray::from_slice(&items));
}

async fn helper(app: &AppHandle, call: &str, params: &Value) -> Result<Value, String> {
    let expr = format!("({CLIPBOARD},{},window.__ccmE2E.{call}({params}))", HELPERS.trim().trim_end_matches(';'));
    let r = eval(app, &expr).await?;
    if r.get("ok") == Some(&Value::Bool(true)) {
        Ok(json!({}))
    } else {
        Err(r.get("e").and_then(Value::as_str).unwrap_or("input").to_owned())
    }
}

async fn settle(app: &AppHandle, width: f64, height: f64) {
    let expr = format!("Math.abs(innerWidth-{width})<2&&Math.abs(innerHeight-{height})<2");
    for _ in 0..40 {
        if eval(app, &expr).await.ok().and_then(|r| r.get("v").and_then(Value::as_bool)) == Some(true) {
            return;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

fn resize(app: &AppHandle, width: f64, height: f64) -> Result<Value, String> {
    let w = app.get_webview_window(MAIN).ok_or("no window")?;
    let st = state();
    if lock(&st.size).is_none() {
        let scale = w.scale_factor().unwrap_or(1.0);
        let cur = w.inner_size().map_err(|e| e.to_string())?.to_logical::<f64>(scale);
        *lock(&st.size) = Some(cur);
    }
    w.set_size(LogicalSize::new(width, height)).map_err(|e| e.to_string())?;
    Ok(json!({}))
}

fn restore(app: &AppHandle) -> Result<Option<LogicalSize<f64>>, String> {
    let size = lock(&state().size).take();
    if let (Some(w), Some(size)) = (app.get_webview_window(MAIN), size) {
        w.set_size(size).map_err(|e| e.to_string())?;
    }
    Ok(size)
}

async fn resized(app: &AppHandle, width: f64, height: f64) -> Result<Value, String> {
    let r = resize(app, width, height)?;
    settle(app, width, height).await;
    Ok(r)
}

fn num(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(Value::as_f64).filter(|n| *n > 0.0)
}

async fn handle(app: &AppHandle, method: &str, p: &Value) -> Result<Value, String> {
    match method {
        "Runtime.evaluate" => {
            let expr = p.get("expression").and_then(Value::as_str).unwrap_or("undefined");
            Ok(evaluate_reply(&eval(app, expr).await?))
        }
        "Page.reload" => {
            let w = app.get_webview_window(MAIN).ok_or("no window")?;
            w.eval("location.reload()").map_err(|e| e.to_string())?;
            Ok(json!({}))
        }
        "Page.captureScreenshot" => {
            let wv = app.get_webview(MAIN).ok_or("no webview")?;
            let png = tauri::async_runtime::spawn_blocking(move || crate::native::capture(&wv)).await.map_err(|e| e.to_string())??;
            Ok(json!({ "data": crate::shot::base64(&png) }))
        }
        "Input.dispatchMouseEvent" => helper(app, "mouse", p).await,
        "Input.dispatchKeyEvent" => helper(app, "key", p).await,
        "Input.insertText" => helper(app, "insert", p).await,
        "Input.dispatchDragEvent" => {
            let files: Vec<String> = p.pointer("/data/files").and_then(Value::as_array).map(|a| a.iter().filter_map(Value::as_str).map(str::to_owned).collect()).unwrap_or_default();
            if p.get("type").and_then(Value::as_str) == Some("dragEnter") && !files.is_empty() {
                let (tx, rx) = std::sync::mpsc::channel();
                let list = files.clone();
                app.run_on_main_thread(move || {
                    drag_files(&list);
                    let _ = tx.send(());
                })
                .map_err(|e| e.to_string())?;
                let _ = rx.recv_timeout(std::time::Duration::from_secs(2));
            }
            let names: Vec<&str> = files.iter().map(|f| f.rsplit('/').next().unwrap_or(f)).collect();
            let mut q = p.clone();
            q["names"] = json!(names);
            helper(app, "drag", &q).await
        }
        "Emulation.setDeviceMetricsOverride" => match (num(p, "width"), num(p, "height")) {
            (Some(w), Some(h)) => resized(app, w, h).await,
            _ => Ok(json!({})),
        },
        "Emulation.clearDeviceMetricsOverride" => {
            if let Some(size) = restore(app)? {
                settle(app, size.width, size.height).await;
            }
            Ok(json!({}))
        }
        "Browser.getWindowForTarget" => {
            let w = app.get_webview_window(MAIN).ok_or("no window")?;
            let scale = w.scale_factor().unwrap_or(1.0);
            let size = w.inner_size().map_err(|e| e.to_string())?.to_logical::<f64>(scale);
            Ok(json!({ "windowId": 1, "bounds": { "width": size.width, "height": size.height, "windowState": "normal" } }))
        }
        "Browser.setWindowBounds" => {
            let b = p.get("bounds").cloned().unwrap_or_default();
            let w = app.get_webview_window(MAIN).ok_or("no window")?;
            match b.get("windowState").and_then(Value::as_str) {
                Some("minimized") => w.minimize().map_err(|e| e.to_string())?,
                Some("maximized") => w.maximize().map_err(|e| e.to_string())?,
                Some("normal") => {
                    let _ = w.unminimize();
                    let _ = w.unmaximize();
                }
                _ => {}
            }
            match (num(&b, "width"), num(&b, "height")) {
                (Some(width), Some(height)) => resized(app, width, height).await,
                _ => Ok(json!({})),
            }
        }
        "Emulation.setEmulatedMedia" => {
            let w = app.get_webview_window(MAIN).ok_or("no window")?;
            let scheme = p.get("features").and_then(Value::as_array).and_then(|f| f.iter().find(|x| x.get("name").and_then(Value::as_str) == Some("prefers-color-scheme"))).and_then(|x| x.get("value").and_then(Value::as_str));
            let theme = match scheme {
                Some("dark") => Some(Theme::Dark),
                Some("light") => Some(Theme::Light),
                _ => None,
            };
            w.set_theme(theme).map_err(|e| e.to_string())?;
            Ok(json!({}))
        }
        "Page.addScriptToEvaluateOnNewDocument" => {
            let src = p.get("source").and_then(Value::as_str).unwrap_or_default().to_owned();
            let mut init = lock(&state().init);
            init.push(src);
            Ok(json!({ "identifier": init.len().to_string() }))
        }
        "Browser.close" => {
            app.exit(0);
            Ok(json!({}))
        }
        "Page.enable" | "Page.bringToFront" | "Runtime.enable" | "Network.enable" | "DOM.enable" => Ok(json!({})),
        other => Err(format!("{other} is not supported by the macOS e2e bridge")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bridge_only_opens_on_mac_with_devtools_and_a_port() {
        assert_eq!(port_from(true, true, Some("9333".into())), Some(9333));
        assert_eq!(port_from(false, true, Some("9333".into())), None);
        assert_eq!(port_from(true, false, Some("9333".into())), None);
        assert_eq!(port_from(true, true, None), None);
        assert_eq!(port_from(true, true, Some("0".into())), None);
        assert_eq!(port_from(true, true, Some("65535".into())), None);
        assert_eq!(port_from(true, true, Some("x".into())), None);
    }

    #[test]
    fn targets_point_the_page_at_the_socket_port() {
        let t = targets("http://127.0.0.1:4399/?k=a", 9334);
        assert_eq!(t[0]["type"], "page");
        assert_eq!(t[0]["url"], "http://127.0.0.1:4399/?k=a");
        assert_eq!(t[0]["webSocketDebuggerUrl"], "ws://127.0.0.1:9334/devtools/page/main");
    }

    #[test]
    fn eval_bodies_inline_the_code_and_mark_a_run() {
        let v = eval_bodies("a; b", "e2e:1");
        let (a, b) = (v[0].clone(), v[2].clone());
        assert_eq!(v.len(), 3);
        assert!(v[1].contains("\na\n;const v=await (\nb\n);"));
        assert!(a.contains("globalThis.__ccmE2Eran=\"e2e:1\";const v=await (\na; b\n);"));
        assert!(b.contains("if(globalThis.__ccmE2Eran===\"e2e:1\")return JSON.stringify({ran:true});"));
        assert!(a.starts_with(UNQUIET) && b.starts_with(UNQUIET));
        assert!(may_be_statements("const a = 1") && may_be_statements("a; b") && may_be_statements("a\nb"));
        assert!(!may_be_statements("document.title") && !may_be_statements("(async () => 1)()"));
        assert!(b.contains("\na; b\n;"));
        assert!(!a.contains("eval(") && !b.contains("eval("));
    }

    #[test]
    fn split_last_keeps_the_value_of_the_last_statement() {
        assert_eq!(split_last("a(); b"), Some(("a()".into(), "b".into())));
        assert_eq!(split_last("a(); b;"), Some(("a()".into(), "b".into())));
        assert_eq!(split_last("x = 'a;b'; f(`;${g(';')}`)"), Some(("x = 'a;b'".into(), "f(`;${g(';')}`)".into())));
        assert_eq!(split_last("(() => { a; b })()"), None);
        assert_eq!(split_last("a(); const c = 1"), None);
        assert_eq!(split_last("a() // ; b"), None);
        assert_eq!(split_last("single"), None);
    }

    #[test]
    fn evaluate_reply_matches_cdp_shapes() {
        assert_eq!(evaluate_reply(&json!({ "ok": true, "t": "number", "v": 3 })), json!({ "result": { "type": "number", "value": 3 } }));
        let err = evaluate_reply(&json!({ "ok": false, "e": "boom" }));
        assert_eq!(err["exceptionDetails"]["text"], "boom");
    }
}
