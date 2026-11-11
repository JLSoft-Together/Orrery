use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf, MAIN_SEPARATOR};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use core_desk::preview::paths::{dirname, is_absolute, is_within, real_or_null, relative, root_key, root_of};
use tauri::Url;

use crate::state::lock;

const MAX_HEAD: usize = 16 * 1024;
const MAX_LIVE: usize = 64;
const READ_TIMEOUT: Duration = Duration::from_secs(15);
const WRITE_TIMEOUT: Duration = Duration::from_secs(60);

pub fn mime_of(file: &str) -> &'static str {
    let ext = Path::new(file).extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" | "cjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "avif" => "image/avif",
        "bmp" => "image/bmp",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "wasm" => "application/wasm",
        "txt" | "md" => "text/plain; charset=utf-8",
        "xml" => "application/xml; charset=utf-8",
        "mp4" => "video/mp4",
        "webm" => "video/webm",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

pub fn encode_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub fn decode_component(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = std::str::from_utf8(b.get(i + 1..i + 3)?).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

pub fn parse_range(header: Option<&str>, size: u64) -> Result<Option<(u64, u64)>, ()> {
    let Some(spec) = header.and_then(|h| h.strip_prefix("bytes=")) else { return Ok(None) };
    let Some((a, b)) = spec.split_once('-') else { return Ok(None) };
    if !a.bytes().chain(b.bytes()).all(|c| c.is_ascii_digit()) || (a.is_empty() && b.is_empty()) {
        return Ok(None);
    }
    let num = |s: &str| s.parse::<u64>().map_err(|_| ());
    let size = size as i128;
    let start = if a.is_empty() { (size - num(b)? as i128).max(0) } else { num(a)? as i128 };
    let end = if !a.is_empty() && !b.is_empty() { (num(b)? as i128).min(size - 1) } else { size - 1 };
    if start > end || start >= size {
        return Err(());
    }
    Ok(Some((start as u64, end as u64)))
}

fn bad_segment(s: &str) -> bool {
    s == "." || s == ".." || s.contains('\0') || s.contains('/') || s.contains('\\') || (cfg!(windows) && s.contains(':'))
}

fn rel_url(root: &str, real: &str) -> String {
    relative(root, real).split(MAIN_SEPARATOR).filter(|s| !s.is_empty()).map(encode_component).collect::<Vec<_>>().join("/")
}

fn new_token() -> Option<String> {
    crate::config::try_random_hex(16).ok()
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        206 => "Partial Content",
        301 => "Moved Permanently",
        307 => "Temporary Redirect",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        416 => "Range Not Satisfiable",
        _ => "Internal Server Error",
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub url: String,
    pub root: String,
    pub token: String,
}

#[derive(Debug, Default)]
pub struct Request {
    pub method: String,
    pub target: String,
    pub headers: HashMap<String, String>,
}

impl Request {
    pub fn parse(head: &str) -> Option<Request> {
        let mut lines = head.split("\r\n");
        let mut first = lines.next()?.split(' ');
        let (method, target, version) = (first.next()?, first.next()?, first.next()?);
        if !version.starts_with("HTTP/1.") || first.next().is_some() {
            return None;
        }
        let mut headers = HashMap::new();
        for line in lines.take_while(|l| !l.is_empty()) {
            let (k, v) = line.split_once(':')?;
            headers.insert(k.trim().to_ascii_lowercase(), v.trim().to_owned());
        }
        Some(Request { method: method.to_owned(), target: target.to_owned(), headers })
    }

    fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(String::as_str)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum Body {
    Empty,
    Text(String),
    File { path: String, start: u64, len: u64 },
}

#[derive(Debug)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Body,
}

impl Response {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }
}

fn base_headers(content_type: &str, corp: &str) -> Vec<(String, String)> {
    vec![
        ("Cache-Control".into(), "no-store".into()),
        ("X-Content-Type-Options".into(), "nosniff".into()),
        ("Cross-Origin-Resource-Policy".into(), corp.into()),
        ("Content-Type".into(), content_type.into()),
    ]
}

fn plain(status: u16, extra: &[(&str, String)]) -> Response {
    let mut headers = base_headers("text/plain; charset=utf-8", "same-origin");
    headers.extend(extra.iter().map(|(k, v)| ((*k).to_owned(), v.clone())));
    let body = if status >= 400 { Body::Text(format!("{status}\n")) } else { Body::Empty };
    Response { status, headers, body }
}

#[derive(Default)]
struct Registry {
    by_token: HashMap<String, String>,
    by_root: HashMap<String, String>,
}

pub struct PreviewServer {
    port: u16,
    reg: Mutex<Registry>,
    live: AtomicUsize,
}

impl PreviewServer {
    pub fn new(port: u16) -> PreviewServer {
        PreviewServer { port, reg: Mutex::new(Registry::default()), live: AtomicUsize::new(0) }
    }

    pub fn origin(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn register(&self, root: &str) -> Option<String> {
        let mut reg = lock(&self.reg);
        let key = root_key(root);
        if let Some(t) = reg.by_root.get(&key) {
            return Some(t.clone());
        }
        let token = new_token()?;
        reg.by_root.insert(key, token.clone());
        reg.by_token.insert(token.clone(), root.to_owned());
        Some(token)
    }

    fn root_of_token(&self, token: &str) -> Option<String> {
        lock(&self.reg).by_token.get(token).cloned()
    }

    pub fn tokens_under(&self, root: &str) -> Vec<String> {
        lock(&self.reg).by_token.iter().filter(|(_, r)| is_within(root, r) || is_within(r, root)).map(|(t, _)| t.clone()).collect()
    }

    pub fn target_for(&self, target: &str, roots: &[String]) -> Option<Target> {
        if !is_absolute(target) {
            return None;
        }
        let real = real_or_null(target)?;
        let dir = std::fs::metadata(&real).ok()?.is_dir();
        let root = root_of(&real, roots).unwrap_or_else(|| if dir { real.clone() } else { dirname(&real) });
        let token = self.register(&root)?;
        let rel = rel_url(&root, &real);
        let tail = if dir && !rel.is_empty() { format!("{rel}/") } else { rel };
        Some(Target { url: format!("{}/{token}/{tail}", self.origin()), root, token })
    }

    fn host_ok(&self, host: Option<&str>) -> bool {
        host.is_some_and(|h| h == format!("127.0.0.1:{}", self.port) || h == format!("localhost:{}", self.port))
    }

    pub fn handle(&self, req: &Request) -> Response {
        if req.method != "GET" && req.method != "HEAD" {
            return plain(405, &[("Allow", "GET, HEAD".into())]);
        }
        if !self.host_ok(req.header("host")) {
            return plain(403, &[]);
        }
        let no_hash = req.target.split('#').next().unwrap_or("");
        let (path, search) = no_hash.split_once('?').map_or((no_hash, String::new()), |(p, q)| (p, format!("?{q}")));
        if !path.starts_with('/') {
            return plain(400, &[]);
        }
        let mut parts = path.split('/').skip(1);
        let token = parts.next().unwrap_or("");
        let Some(root) = self.root_of_token(token) else { return self.redirect_from_referer(req, path, &search) };
        let Some(segments) = parts.map(decode_component).collect::<Option<Vec<_>>>() else { return plain(400, &[]) };
        if segments.iter().any(|s| bad_segment(s)) {
            return plain(404, &[]);
        }
        let wanted = segments.iter().filter(|s| !s.is_empty()).fold(PathBuf::from(&root), |p, s| p.join(s));
        let wanted = wanted.to_string_lossy();
        if !is_within(&root, &wanted) {
            return plain(404, &[]);
        }
        let Some(mut real) = real_or_null(&wanted).filter(|r| is_within(&root, r)) else { return plain(404, &[]) };
        let mut meta = std::fs::metadata(&real).ok();
        if meta.as_ref().is_some_and(|m| m.is_dir()) {
            if !path.ends_with('/') {
                return plain(301, &[("Location", format!("{path}/{search}"))]);
            }
            let index = Path::new(&real).join("index.html");
            let Some(r) = real_or_null(&index.to_string_lossy()).filter(|r| is_within(&root, r)) else { return plain(404, &[]) };
            real = r;
            meta = std::fs::metadata(&real).ok();
        }
        match meta {
            Some(m) if m.is_file() => serve_file(req, real, m.len()),
            _ => plain(404, &[]),
        }
    }

    fn redirect_from_referer(&self, req: &Request, path: &str, search: &str) -> Response {
        let Some(from) = req.header("referer").and_then(|r| Url::parse(r).ok()) else { return plain(404, &[]) };
        let token = from.path().split('/').nth(1).unwrap_or("");
        if from.origin().ascii_serialization() != self.origin() || self.root_of_token(token).is_none() {
            return plain(404, &[]);
        }
        plain(307, &[("Location", format!("/{token}{path}{search}"))])
    }

    fn serve(&self, mut stream: TcpStream) {
        let _ = stream.set_read_timeout(Some(READ_TIMEOUT));
        let _ = stream.set_write_timeout(Some(WRITE_TIMEOUT));
        let Some(head) = read_head(&mut stream) else { return };
        let res = Request::parse(&head).map_or_else(|| plain(400, &[]), |req| self.handle(&req));
        let _ = write_response(&mut stream, res);
    }
}

fn serve_file(req: &Request, file: String, size: u64) -> Response {
    let kind = mime_of(&file);
    let mut headers = base_headers(kind, if kind.starts_with("image/") { "cross-origin" } else { "same-origin" });
    headers.push(("Accept-Ranges".into(), "bytes".into()));
    let (status, start, end) = match parse_range(req.header("range"), size) {
        Err(()) => return plain(416, &[("Content-Range", format!("bytes */{size}"))]),
        Ok(Some((s, e))) => {
            headers.push(("Content-Range".into(), format!("bytes {s}-{e}/{size}")));
            (206, s, e)
        }
        Ok(None) => (200, 0, size.saturating_sub(1)),
    };
    let len = if size == 0 { 0 } else { end - start + 1 };
    headers.push(("Content-Length".into(), len.to_string()));
    let body = if req.method == "HEAD" || size == 0 { Body::Empty } else { Body::File { path: file, start, len } };
    Response { status, headers, body }
}

fn read_head(stream: &mut TcpStream) -> Option<String> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 2048];
    while buf.len() <= MAX_HEAD {
        let n = stream.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if let Some(end) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            return String::from_utf8(buf[..end + 4].to_vec()).ok();
        }
    }
    None
}

fn write_response(stream: &mut TcpStream, mut res: Response) -> std::io::Result<()> {
    let mut file = None;
    if let Body::File { path, start, .. } = &res.body {
        match File::open(path).and_then(|mut f| f.seek(SeekFrom::Start(*start)).map(|_| f)) {
            Ok(f) => file = Some(f),
            Err(_) => res = plain(404, &[]),
        }
    }
    if let Body::Text(t) = &res.body {
        res.headers.push(("Content-Length".into(), t.len().to_string()));
    } else if res.body == Body::Empty && res.header("Content-Length").is_none() {
        res.headers.push(("Content-Length".into(), "0".into()));
    }
    let mut head = format!("HTTP/1.1 {} {}\r\n", res.status, reason(res.status));
    for (k, v) in &res.headers {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str("Connection: close\r\n\r\n");
    stream.write_all(head.as_bytes())?;
    match (&res.body, file) {
        (Body::Text(t), _) => stream.write_all(t.as_bytes())?,
        (Body::File { len, .. }, Some(f)) => {
            std::io::copy(&mut f.take(*len), stream)?;
        }
        _ => {}
    }
    stream.flush()
}

pub fn start() -> std::io::Result<Arc<PreviewServer>> {
    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let server = Arc::new(PreviewServer::new(listener.local_addr()?.port()));
    let s = server.clone();
    std::thread::Builder::new().name("preview-http".into()).spawn(move || {
        for conn in listener.incoming() {
            let Ok(stream) = conn else { continue };
            if s.live.fetch_add(1, Ordering::SeqCst) >= MAX_LIVE {
                s.live.fetch_sub(1, Ordering::SeqCst);
                continue;
            }
            let one = s.clone();
            let spawned = std::thread::Builder::new().name("preview-http-conn".into()).spawn(move || {
                one.serve(stream);
                one.live.fetch_sub(1, Ordering::SeqCst);
            });
            if spawned.is_err() {
                s.live.fetch_sub(1, Ordering::SeqCst);
            }
        }
    })?;
    Ok(server)
}

static SERVER: OnceLock<Option<Arc<PreviewServer>>> = OnceLock::new();

pub fn server() -> Option<Arc<PreviewServer>> {
    SERVER
        .get_or_init(|| match start() {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("[shell] preview server failed: {e}");
                None
            }
        })
        .clone()
}

pub fn tokens_under(root: &str) -> Vec<String> {
    SERVER.get().and_then(Option::as_ref).map_or_else(Vec::new, |s| s.tokens_under(root))
}

pub fn url_for(path: &str, roots: &[String]) -> Option<String> {
    server()?.target_for(path, roots).map(|t| t.url)
}

#[cfg(test)]
#[path = "preview_serve_tests.rs"]
mod tests;
