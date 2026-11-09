#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::Duration,
};

use tauri::{
    tray::TrayIconBuilder,
    webview::{NewWindowResponse, WebviewBuilder},
    AppHandle, LogicalPosition, LogicalSize, Manager, WebviewUrl, WebviewWindowBuilder,
};

struct Shared {
    port: u16,
    out: PathBuf,
    events: Arc<Mutex<Vec<String>>>,
}

fn log(out: &Path, line: &str) {
    if let Ok(mut f) = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(out.join("auto.log"))
    {
        let _ = writeln!(f, "{line}");
    }
}

fn mime(p: &Path) -> &'static str {
    match p.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript",
        "css" => "text/css",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "json" => "application/json",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "ttf" => "font/ttf",
        "txt" => "text/plain",
        _ => "application/octet-stream",
    }
}

fn handle(mut s: TcpStream, web: &Path, ui: &Path, nm: &Path, out: &Path) {
    let mut buf = Vec::new();
    let mut tmp = [0u8; 8192];
    let head_end;
    loop {
        match s.read(&mut tmp) {
            Ok(0) | Err(_) => return,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
        }
        if let Some(p) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            head_end = p + 4;
            break;
        }
        if buf.len() > 1 << 20 {
            return;
        }
    }
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.lines();
    let first = lines.next().unwrap_or("");
    let mut parts = first.split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("/");
    let path = target.split('?').next().unwrap_or("/");
    let clen: usize = lines
        .filter_map(|l| {
            let (k, v) = l.split_once(':')?;
            if k.eq_ignore_ascii_case("content-length") {
                v.trim().parse().ok()
            } else {
                None
            }
        })
        .next()
        .unwrap_or(0);
    let mut body = buf[head_end..].to_vec();
    while body.len() < clen {
        match s.read(&mut tmp) {
            Ok(0) | Err(_) => break,
            Ok(n) => body.extend_from_slice(&tmp[..n]),
        }
    }
    let cors = "Access-Control-Allow-Origin: *\r\nAccess-Control-Allow-Headers: *\r\n";
    if method == "OPTIONS" {
        let _ = s.write_all(format!("HTTP/1.1 204 No Content\r\n{cors}Content-Length: 0\r\n\r\n").as_bytes());
        return;
    }
    if method == "POST" && path == "/report" {
        if let Ok(mut f) = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(out.join("reports.jsonl"))
        {
            let _ = f.write_all(&body);
            let _ = f.write_all(b"\n");
        }
        let _ = s.write_all(format!("HTTP/1.1 204 No Content\r\n{cors}Content-Length: 0\r\n\r\n").as_bytes());
        return;
    }
    if path.contains("..") {
        let _ = s.write_all(b"HTTP/1.1 400 Bad Request\r\nContent-Length: 0\r\n\r\n");
        return;
    }
    let file = if let Some(r) = path.strip_prefix("/__ui/") {
        ui.join(r)
    } else if let Some(r) = path.strip_prefix("/__nm/") {
        nm.join(r)
    } else {
        let r = path.trim_start_matches('/');
        let r = if r.is_empty() { "index.html" } else { r };
        let p = web.join(r);
        if p.is_file() {
            p
        } else {
            web.join("index.html")
        }
    };
    match fs::read(&file) {
        Ok(data) => {
            let h = format!(
                "HTTP/1.1 200 OK\r\n{cors}Content-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n",
                mime(&file),
                data.len()
            );
            let _ = s.write_all(h.as_bytes());
            let _ = s.write_all(&data);
        }
        Err(_) => {
            let _ = s.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        }
    }
}

fn serve(web: PathBuf, ui: PathBuf, nm: PathBuf, out: PathBuf) -> u16 {
    let l = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = l.local_addr().unwrap().port();
    thread::spawn(move || {
        for s in l.incoming().flatten() {
            let (w, u, n, o) = (web.clone(), ui.clone(), nm.clone(), out.clone());
            thread::spawn(move || handle(s, &w, &u, &n, &o));
        }
    });
    port
}

#[tauri::command]
fn ping() -> &'static str {
    "pong"
}

#[tauri::command]
fn child_bounds(app: AppHandle, x: f64, y: f64, w: f64, h: f64) {
    if let Some(wv) = app.get_webview("child") {
        let _ = wv.set_position(LogicalPosition::new(x, y));
        let _ = wv.set_size(LogicalSize::new(w.max(1.0), h.max(1.0)));
    }
}

fn make_child(
    app: &AppHandle,
    label: &str,
    url: &str,
    data_dir: Option<PathBuf>,
    rect: (f64, f64, f64, f64),
) -> Result<(), String> {
    let events = app.state::<Shared>().events.clone();
    let events2 = events.clone();
    let lbl = label.to_string();
    let lbl2 = label.to_string();
    let parsed = url.parse::<tauri::Url>().map_err(|e| e.to_string())?;
    let mut b = WebviewBuilder::new(label, WebviewUrl::External(parsed))
        .on_navigation(move |u| {
            let host = u.host_str().unwrap_or("");
            let allow = host == "127.0.0.1" || host == "example.com" || host == "www.example.com";
            events.lock().unwrap().push(format!("[{lbl}] navigation allow={allow} {u}"));
            allow
        })
        .on_new_window(move |u, _f| {
            events2.lock().unwrap().push(format!("[{lbl2}] new-window denied {u}"));
            NewWindowResponse::Deny
        });
    if let Some(d) = data_dir {
        b = b.data_directory(d);
    }
    let win = app.get_window("main").ok_or("no main window")?;
    win.add_child(b, LogicalPosition::new(rect.0, rect.1), LogicalSize::new(rect.2, rect.3))
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(windows)]
unsafe fn capture_on_ui(p: tauri::webview::PlatformWebview) -> Result<Vec<u8>, String> {
    use webview2_com::{
        CapturePreviewCompletedHandler,
        Microsoft::Web::WebView2::Win32::COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG,
    };
    use windows::Win32::System::Com::{IStream, STREAM_SEEK_SET};
    use windows::Win32::UI::Shell::SHCreateMemStream;

    let core = p.controller().CoreWebView2().map_err(|e| format!("core {e:?}"))?;
    let stream: IStream = SHCreateMemStream(None).ok_or("SHCreateMemStream")?;
    let s2 = stream.clone();
    CapturePreviewCompletedHandler::wait_for_async_operation(
        Box::new(move |h| {
            core.CapturePreview(COREWEBVIEW2_CAPTURE_PREVIEW_IMAGE_FORMAT_PNG, &s2, &h)
                .map_err(webview2_com::Error::WindowsError)
        }),
        Box::new(|r| r),
    )
    .map_err(|e| format!("capture {e:?}"))?;
    stream
        .Seek(0, STREAM_SEEK_SET, None)
        .map_err(|e| format!("seek {e:?}"))?;
    let mut out = Vec::new();
    let mut chunk = vec![0u8; 65536];
    loop {
        let mut read = 0u32;
        let hr = stream.Read(chunk.as_mut_ptr() as *mut _, chunk.len() as u32, Some(&mut read));
        if hr.is_err() || read == 0 {
            break;
        }
        out.extend_from_slice(&chunk[..read as usize]);
    }
    Ok(out)
}

fn capture(app: &AppHandle, label: &str) -> Result<Vec<u8>, String> {
    let wv = app.get_webview(label).ok_or("no webview")?;
    let (tx, rx) = mpsc::channel();
    wv.with_webview(move |p| {
        #[cfg(windows)]
        let r = unsafe { capture_on_ui(p) };
        #[cfg(not(windows))]
        let r: Result<Vec<u8>, String> = {
            let _ = p;
            Err("windows only".into())
        };
        let _ = tx.send(r);
    })
    .map_err(|e| e.to_string())?;
    rx.recv_timeout(Duration::from_secs(20)).map_err(|e| e.to_string())?
}

fn png_dims(b: &[u8]) -> (u32, u32) {
    if b.len() < 24 {
        return (0, 0);
    }
    (
        u32::from_be_bytes([b[16], b[17], b[18], b[19]]),
        u32::from_be_bytes([b[20], b[21], b[22], b[23]]),
    )
}

fn bounds_of(app: &AppHandle, label: &str) -> String {
    match app.get_webview(label) {
        Some(w) => {
            let sf = w.window().scale_factor().unwrap_or(1.0);
            let p = w.position().map(|p| (p.x, p.y)).unwrap_or((0, 0));
            let s = w.size().map(|s| (s.width, s.height)).unwrap_or((0, 0));
            format!("scale={sf} pos_px={p:?} size_px={s:?}")
        }
        None => "missing".into(),
    }
}

fn save_capture(app: &AppHandle, out: &Path, label: &str, name: &str) {
    match capture(app, label) {
        Ok(b) => {
            let d = png_dims(&b);
            let _ = fs::write(out.join(name), &b);
            log(out, &format!("capture {label} -> {name} bytes={} dims={:?}", b.len(), d));
        }
        Err(e) => log(out, &format!("capture {label} FAILED {e}")),
    }
}

#[cfg(windows)]
fn style_report(app: &AppHandle, out: &Path) {
    use windows::Win32::Foundation::HWND;
    use windows::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, GWL_STYLE};
    if let Some(w) = app.get_window("main") {
        if let Ok(h) = w.hwnd() {
            let st = unsafe { GetWindowLongPtrW(HWND(h.0), GWL_STYLE) } as u32;
            log(
                out,
                &format!(
                    "style=0x{st:08X} THICKFRAME={} MAXIMIZEBOX={} MINIMIZEBOX={} CAPTION={} SYSMENU={}",
                    st & 0x40000 != 0,
                    st & 0x10000 != 0,
                    st & 0x20000 != 0,
                    st & 0xC00000 == 0xC00000,
                    st & 0x80000 != 0
                ),
            );
        }
    }
}

fn auto(app: AppHandle, out: PathBuf, tmp: PathBuf) {
    thread::sleep(Duration::from_secs(4));
    let port = app.state::<Shared>().port;
    let base = format!("http://127.0.0.1:{port}");
    log(&out, "auto start");
    let cd = if std::env::var("SPIKE_CHILD_DIR").map(|v| v == "main").unwrap_or(false) { Some(tmp.join("main")) } else { None };
    match make_child(&app, "child", "https://example.com/", cd, (320.0, 56.0, 600.0, 380.0)) {
        Ok(()) => log(&out, "child created (https://example.com)"),
        Err(e) => log(&out, &format!("child create FAILED {e}")),
    }
    thread::sleep(Duration::from_secs(6));
    log(&out, &format!("child bounds after create: {}", bounds_of(&app, "child")));
    save_capture(&app, &out, "child", "child-1.png");

    if let Some(w) = app.get_webview("child") {
        let _ = w.set_position(LogicalPosition::new(100.0, 100.0));
        let _ = w.set_size(LogicalSize::new(400.0, 300.0));
    }
    thread::sleep(Duration::from_secs(1));
    log(&out, &format!("child bounds after set_position/size: {}", bounds_of(&app, "child")));
    save_capture(&app, &out, "child", "child-2.png");

    if let Some(w) = app.get_window("main") {
        let _ = w.set_size(LogicalSize::new(1000.0, 700.0));
    }
    thread::sleep(Duration::from_millis(2000));
    log(&out, &format!("child bounds after window resize (page observer): {}", bounds_of(&app, "child")));
    save_capture(&app, &out, "child", "child-3.png");

    app.state::<Shared>().events.lock().unwrap().clear();
    match make_child(&app, "nav", &format!("{base}/__ui/nav.html"), None, (0.0, 0.0, 200.0, 200.0)) {
        Ok(()) => log(&out, "nav child created"),
        Err(e) => log(&out, &format!("nav child FAILED {e}")),
    }
    thread::sleep(Duration::from_secs(7));
    for e in app.state::<Shared>().events.lock().unwrap().iter() {
        log(&out, &format!("event {e}"));
    }
    if let Some(w) = app.get_webview("nav") {
        log(&out, &format!("nav child url now: {:?}", w.url().map(|u| u.to_string())));
        let _ = w.close();
    }

    let std_dir = tmp.join("child-b");
    match make_child(&app, "stA", &format!("{base}/__ui/storage.html?tag=A-default"), None, (0.0, 0.0, 50.0, 50.0)) {
        Ok(()) => log(&out, "stA created"),
        Err(e) => log(&out, &format!("stA FAILED {e}")),
    }
    thread::sleep(Duration::from_secs(3));
    match make_child(&app, "stB", &format!("{base}/__ui/storage.html?tag=B-own-dir"), Some(std_dir.clone()), (60.0, 0.0, 50.0, 50.0)) {
        Ok(()) => log(&out, "stB created"),
        Err(e) => log(&out, &format!("stB FAILED {e}")),
    }
    thread::sleep(Duration::from_secs(4));
    log(&out, &format!("stB data dir exists={} entries={:?}", std_dir.exists(), fs::read_dir(&std_dir).map(|d| d.count()).ok()));
    for l in ["stA", "stB"] {
        if let Some(w) = app.get_webview(l) {
            let _ = w.close();
        }
    }
    let _ = fs::write(out.join("storage-ok"), "1");

    #[cfg(windows)]
    style_report(&app, &out);
    if let Some(w) = app.get_window("main") {
        let _ = w.maximize();
        thread::sleep(Duration::from_secs(1));
        log(&out, &format!("maximized={:?}", w.is_maximized()));
        thread::sleep(Duration::from_millis(1500));
        log(&out, &format!("child bounds when maximized: {}", bounds_of(&app, "child")));
        let _ = w.unmaximize();
        thread::sleep(Duration::from_secs(1));
        log(&out, &format!("maximized after unmaximize={:?}", w.is_maximized()));
        let _ = w.set_focus();
    }
    let _ = fs::write(out.join("ready-for-shot"), "1");
    thread::sleep(Duration::from_secs(10));
    let _ = fs::write(out.join("done"), "1");
    log(&out, "auto done");
}

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let repo = root.join("..").join("..");
    let out = std::env::var("SPIKE_OUT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("orrery-spike-out"));
    let tmp = std::env::var("SPIKE_TMP")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir().join("orrery-spike-data"));
    let _ = fs::create_dir_all(&out);
    let _ = fs::create_dir_all(&tmp);
    let web = std::env::var("SPIKE_WEB_DIST")
        .map(PathBuf::from)
        .unwrap_or_else(|_| repo.join("apps").join("web").join("dist"));
    let ui = root.join("ui");
    let nm = repo.join("node_modules");
    let port = serve(web, ui, nm, out.clone());
    let events = Arc::new(Mutex::new(Vec::new()));
    let mode = std::env::var("SPIKE_MODE").unwrap_or_else(|_| "shell".into());
    let auto_on = std::env::var("SPIKE_AUTO").map(|v| v == "1").unwrap_or(false);

    tauri::Builder::default()
        .manage(Shared { port, out: out.clone(), events })
        .invoke_handler(tauri::generate_handler![ping, child_bounds])
        .setup(move |app| {
            let url = match mode.as_str() {
                "real" => WebviewUrl::External(format!("http://127.0.0.1:{port}/").parse().unwrap()),
                "xterm" => WebviewUrl::External(
                    format!("http://127.0.0.1:{port}/__ui/xterm.html?warm=8&dur=20").parse().unwrap(),
                ),
                _ => WebviewUrl::App("index.html".into()),
            };
            let shell = mode == "shell";
            let mut b = WebviewWindowBuilder::new(app, "main", url)
                .title("Orrery Spike")
                .inner_size(1280.0, 800.0)
                .decorations(!shell)
                .shadow(true)
                .data_directory(tmp.join("main"));
            if shell {
                b = b.resizable(true);
            }
            b.build()?;
            let icon = app.default_window_icon().cloned();
            let mut t = TrayIconBuilder::new().tooltip("Orrery Spike");
            if let Some(i) = icon {
                t = t.icon(i);
            }
            t.build(app)?;
            if auto_on && shell {
                let h = app.handle().clone();
                let o = app.state::<Shared>().out.clone();
                let tm = tmp.clone();
                thread::spawn(move || auto(h, o, tm));
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("run");
}
