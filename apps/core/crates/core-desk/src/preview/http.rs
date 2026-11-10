use crate::js;
use regex::Regex;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{mpsc, Arc, Condvar, Mutex, OnceLock};
use std::time::{Duration, Instant};
use ureq::tls::{RootCerts, TlsConfig};

pub const HTTP_TIMEOUT_MS: u64 = 30_000;
pub const MAX_RESPONSE_BYTES: usize = 5 * 1024 * 1024;
pub const MAX_REQUEST_BODY: usize = 10 * 1024 * 1024;
const MAX_HEADERS: usize = 100;
const MAX_URL: usize = 16 * 1024;

fn token_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^[!#$%&'*+.^_`|~0-9A-Za-z-]+$").expect("regex"))
}

fn binary_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)^(?:image|audio|video|font)/|^application/(?:octet-stream|pdf|zip|gzip|x-protobuf|protobuf|vnd\.)").expect("regex"))
}

pub struct HttpRequest {
    pub id: Option<String>,
    pub method: String,
    pub url: String,
    pub host: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

fn ipv4_local(host: &str) -> Option<bool> {
    let parts: Vec<&str> = host.split('.').collect();
    if parts.len() != 4 || parts.iter().any(|p| p.is_empty() || p.len() > 3 || !p.bytes().all(|b| b.is_ascii_digit())) {
        return None;
    }
    let n: Vec<u32> = parts.iter().map(|p| p.parse().unwrap_or(999)).collect();
    if n.iter().any(|v| *v > 255) {
        return None;
    }
    let (a, b) = (n[0], n[1]);
    Some(a == 127 || a == 10 || a == 0 || (a == 172 && (16..=31).contains(&b)) || (a == 192 && b == 168) || (a == 169 && b == 254))
}

pub fn is_local_host(hostname: &str) -> bool {
    let lower = hostname.to_lowercase();
    let h = lower.trim_start_matches('[');
    let h = h.strip_suffix(']').unwrap_or(h);
    let h = h.strip_suffix('.').unwrap_or(h);
    if h == "localhost" || h.ends_with(".localhost") || h.ends_with(".local") {
        return true;
    }
    if let Some(v4) = ipv4_local(h) {
        return v4;
    }
    if h.contains(':') {
        if h == "::1" || h == "::" {
            return true;
        }
        if let Some(rest) = h.strip_prefix("::ffff:") {
            return ipv4_local(rest).unwrap_or(false);
        }
        let b = h.as_bytes();
        let run = |from: usize, max: usize| {
            let mut i = from;
            while i < from + max && b.get(i).is_some_and(u8::is_ascii_hexdigit) {
                i += 1;
            }
            b.get(i) == Some(&b':')
        };
        if b.first() == Some(&b'f') && matches!(b.get(1), Some(b'c' | b'd')) && run(2, 2) {
            return true;
        }
        if b.first() == Some(&b'f') && b.get(1) == Some(&b'e') && matches!(b.get(2), Some(b'8' | b'9' | b'a' | b'b')) && run(3, 1) {
            return true;
        }
    }
    false
}

pub fn clean_request(input: &Value) -> Option<HttpRequest> {
    let o = input.as_object()?;
    let method = o.get("method").and_then(Value::as_str).map(str::to_uppercase).unwrap_or_default();
    if !token_re().is_match(&method) || method.len() > 16 {
        return None;
    }
    let raw_url = o.get("url").and_then(Value::as_str).filter(|u| js::len(u) <= MAX_URL)?;
    let url = url::Url::parse(raw_url).ok()?;
    if url.scheme() != "http" && url.scheme() != "https" {
        return None;
    }
    let headers = o.get("headers").and_then(Value::as_array).filter(|h| h.len() <= MAX_HEADERS)?;
    let mut out: Vec<(String, String)> = Vec::new();
    for pair in headers {
        let p = pair.as_array()?;
        let (name, value) = (p.first()?.as_str()?, p.get(1)?.as_str()?);
        if !token_re().is_match(name) || value.contains(['\r', '\n', '\0']) || js::len(value) > 16 * 1024 {
            return None;
        }
        out.push((name.to_string(), value.to_string()));
    }
    let body = match o.get("body") {
        None => None,
        Some(Value::String(s)) if s.len() <= MAX_REQUEST_BODY || js::len(s) <= MAX_REQUEST_BODY => Some(s.clone()),
        Some(_) => return None,
    };
    let id = o.get("id").and_then(Value::as_str).filter(|i| js::len(i) <= 64).map(String::from);
    let host = url.host_str().unwrap_or("").to_lowercase();
    Some(HttpRequest { id, method, url: url.as_str().to_string(), host, headers: out, body })
}

fn fail(error: &str) -> Value {
    json!({"ok": false, "error": error})
}

fn network_detail(e: &ureq::Error) -> String {
    let code = match e {
        ureq::Error::HostNotFound => "ENOTFOUND".to_string(),
        ureq::Error::Io(io) => match io.kind() {
            std::io::ErrorKind::ConnectionRefused => "ECONNREFUSED".into(),
            std::io::ErrorKind::ConnectionReset => "ECONNRESET".into(),
            std::io::ErrorKind::ConnectionAborted => "ECONNABORTED".into(),
            std::io::ErrorKind::UnexpectedEof => "UND_ERR_SOCKET".into(),
            _ => io.to_string(),
        },
        other => other.to_string(),
    };
    js::slice(&code, 200)
}

struct Fetched {
    status: u16,
    reason: String,
    headers: Vec<(String, String)>,
    bytes: Vec<u8>,
    size: usize,
    truncated: bool,
}

fn sorted_headers(map: &ureq::http::HeaderMap) -> Vec<(String, String)> {
    let mut grouped: Vec<(String, Vec<String>)> = Vec::new();
    for (k, v) in map.iter() {
        let name = k.as_str().to_string();
        let value = String::from_utf8_lossy(v.as_bytes()).to_string();
        match grouped.iter_mut().find(|(n, _)| *n == name) {
            Some((_, list)) => list.push(value),
            None => grouped.push((name, vec![value])),
        }
    }
    grouped.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = Vec::new();
    for (name, list) in grouped {
        if name == "set-cookie" {
            out.extend(list.into_iter().map(|v| (name.clone(), v)));
        } else {
            out.push((name, list.join(", ")));
        }
    }
    out
}

fn fetch(req: &HttpRequest) -> Result<Fetched, ureq::Error> {
    let tls = TlsConfig::builder().root_certs(RootCerts::PlatformVerifier).build();
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_millis(HTTP_TIMEOUT_MS)))
        .http_status_as_error(false)
        .max_redirects(0)
        .max_redirects_will_error(false)
        .proxy(None)
        .tls_config(tls)
        .build();
    let agent: ureq::Agent = config.into();
    let mut builder = ureq::http::Request::builder().method(req.method.as_str()).uri(req.url.as_str());
    let has = |name: &str| req.headers.iter().any(|(k, _)| k.eq_ignore_ascii_case(name));
    for (k, v) in &req.headers {
        builder = builder.header(k.as_str(), v.as_str());
    }
    for (k, v) in [("accept", "*/*"), ("accept-language", "*"), ("user-agent", "node")] {
        if !has(k) {
            builder = builder.header(k, v);
        }
    }
    if req.body.as_deref().is_some_and(|b| !b.is_empty()) && !has("content-type") {
        builder = builder.header("content-type", "text/plain;charset=UTF-8");
    }
    let mut res = match req.body.as_deref().filter(|b| !b.is_empty()) {
        Some(b) => agent.run(builder.body(b.to_string()).map_err(ureq::Error::Http)?)?,
        None => agent.run(builder.body(()).map_err(ureq::Error::Http)?)?,
    };
    let status = res.status();
    let headers = sorted_headers(res.headers());
    let mut reader = res.body_mut().with_config().limit(u64::MAX).reader();
    let mut bytes = Vec::new();
    let mut limited = (&mut reader).take(MAX_RESPONSE_BYTES as u64 + 1);
    let read = limited.read_to_end(&mut bytes);
    if let Err(e) = read {
        if bytes.is_empty() {
            return Err(ureq::Error::Io(e));
        }
    }
    let truncated = bytes.len() > MAX_RESPONSE_BYTES;
    bytes.truncate(MAX_RESPONSE_BYTES);
    let size = bytes.len();
    Ok(Fetched { status: status.as_u16(), reason: status.canonical_reason().unwrap_or("").to_string(), headers, bytes, size, truncated })
}

pub fn send_http(req: &HttpRequest, cancel: &Arc<AtomicBool>) -> Value {
    if req.body.as_deref().is_some_and(|b| !b.is_empty()) && (req.method == "GET" || req.method == "HEAD") {
        return fail("get-body");
    }
    let started = Instant::now();
    let (tx, rx) = mpsc::channel();
    let copy = HttpRequest { id: None, method: req.method.clone(), url: req.url.clone(), host: req.host.clone(), headers: req.headers.clone(), body: req.body.clone() };
    std::thread::spawn(move || {
        let _ = tx.send(fetch(&copy));
    });
    let deadline = started + Duration::from_millis(HTTP_TIMEOUT_MS);
    let result = loop {
        if cancel.load(Ordering::SeqCst) {
            return fail("cancelled");
        }
        match rx.recv_timeout(Duration::from_millis(15)) {
            Ok(r) => break r,
            Err(RecvTimeoutError::Timeout) => {
                if Instant::now() >= deadline {
                    return fail("timeout");
                }
            }
            Err(RecvTimeoutError::Disconnected) => return fail("network"),
        }
    };
    if cancel.load(Ordering::SeqCst) {
        return fail("cancelled");
    }
    match result {
        Ok(f) => {
            let ctype = f.headers.iter().find(|(k, _)| k == "content-type").map(|(_, v)| v.as_str()).unwrap_or("");
            let binary = binary_re().is_match(ctype);
            let raw = f.bytes.strip_prefix(&[0xef, 0xbb, 0xbf][..]).unwrap_or(&f.bytes);
            let body = if binary { String::new() } else { String::from_utf8_lossy(raw).to_string() };
            let headers: Vec<Value> = f.headers.iter().map(|(k, v)| json!([k, v])).collect();
            json!({
                "ok": true, "status": f.status, "statusText": f.reason, "headers": headers, "body": body, "binary": binary,
                "truncated": f.truncated, "size": f.size, "timeMs": started.elapsed().as_millis() as u64, "url": req.url,
            })
        }
        Err(ureq::Error::Timeout(_)) => fail("timeout"),
        Err(ureq::Error::Io(e)) if e.kind() == std::io::ErrorKind::TimedOut => fail("timeout"),
        Err(ureq::Error::Http(e)) => json!({"ok": false, "error": "bad-request", "detail": js::slice(&e.to_string(), 200)}),
        Err(e) => json!({"ok": false, "error": "network", "detail": network_detail(&e)}),
    }
}

struct Pending {
    result: Mutex<Option<bool>>,
    cv: Condvar,
}

#[derive(Default)]
pub struct HttpState {
    allowed: Mutex<HashSet<String>>,
    asking: Mutex<HashMap<String, Arc<Pending>>>,
    running: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl HttpState {
    pub fn host_ok(&self, req: &HttpRequest, ask: &dyn Fn(&str, &str) -> bool) -> bool {
        if is_local_host(&req.host) || self.allowed.lock().unwrap().contains(&req.host) {
            return true;
        }
        let (pending, mine) = {
            let mut asking = self.asking.lock().unwrap();
            match asking.get(&req.host) {
                Some(p) => (p.clone(), false),
                None => {
                    let p = Arc::new(Pending { result: Mutex::new(None), cv: Condvar::new() });
                    asking.insert(req.host.clone(), p.clone());
                    (p, true)
                }
            }
        };
        let ok = if mine {
            let ok = ask(&req.host, &req.method);
            self.asking.lock().unwrap().remove(&req.host);
            *pending.result.lock().unwrap() = Some(ok);
            pending.cv.notify_all();
            ok
        } else {
            let mut guard = pending.result.lock().unwrap();
            while guard.is_none() {
                guard = pending.cv.wait(guard).unwrap();
            }
            guard.unwrap_or(false)
        };
        if ok {
            self.allowed.lock().unwrap().insert(req.host.clone());
        }
        ok
    }

    pub fn start(&self, id: &str) -> Arc<AtomicBool> {
        let flag = Arc::new(AtomicBool::new(false));
        if let Some(old) = self.running.lock().unwrap().insert(id.to_string(), flag.clone()) {
            old.store(true, Ordering::SeqCst);
        }
        flag
    }

    pub fn finish(&self, id: &str, flag: &Arc<AtomicBool>) {
        let mut m = self.running.lock().unwrap();
        if m.get(id).is_some_and(|f| Arc::ptr_eq(f, flag)) {
            m.remove(id);
        }
    }

    pub fn cancel(&self, id: &str) -> bool {
        match self.running.lock().unwrap().remove(id) {
            Some(f) => {
                f.store(true, Ordering::SeqCst);
                true
            }
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;

    fn serve(reply: &'static [u8]) -> (String, std::thread::JoinHandle<String>) {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://127.0.0.1:{}/p?q=1", l.local_addr().unwrap().port());
        let h = std::thread::spawn(move || {
            let (mut s, _) = l.accept().unwrap();
            let mut seen: Vec<u8> = Vec::new();
            let mut buf = [0u8; 4096];
            loop {
                let n = s.read(&mut buf).unwrap();
                seen.extend_from_slice(&buf[..n]);
                let text = String::from_utf8_lossy(&seen).to_string();
                if let Some(at) = text.find("\r\n\r\n") {
                    let want = text.to_lowercase().split("content-length: ").nth(1).and_then(|r| r.split("\r\n").next()).and_then(|v| v.parse::<usize>().ok()).unwrap_or(0);
                    if seen.len() >= at + 4 + want {
                        break;
                    }
                }
                if n == 0 {
                    break;
                }
            }
            let n = seen.len();
            let buf = seen;
            s.write_all(reply).unwrap();
            String::from_utf8_lossy(&buf[..n]).to_string()
        });
        (url, h)
    }

    #[test]
    fn hosts_and_clean() {
        for h in ["localhost", "a.local", "127.0.0.1", "10.1.2.3", "192.168.0.1", "172.20.0.1", "169.254.1.1", "[::1]", "::ffff:10.0.0.1", "fd00::1", "fe80::1"] {
            assert!(is_local_host(h), "{h}");
        }
        for h in ["example.com", "8.8.8.8", "172.32.0.1", "2001:db8::1", "999.1.1.1"] {
            assert!(!is_local_host(h), "{h}");
        }
        assert!(clean_request(&json!({"method": "get", "url": "ftp://x", "headers": []})).is_none());
        assert!(clean_request(&json!({"method": "g e", "url": "http://x", "headers": []})).is_none());
        assert!(clean_request(&json!({"method": "get", "url": "http://x", "headers": [["a", "b\nc"]]})).is_none());
        let r = clean_request(&json!({"method": "post", "url": "http://X.com", "headers": [["A", "b"]], "body": "x", "id": "i"})).unwrap();
        assert_eq!((r.method.as_str(), r.url.as_str(), r.host.as_str()), ("POST", "http://x.com/", "x.com"));
    }

    #[test]
    fn sends_to_loopback() {
        let (url, h) = serve(b"HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nX-A: 1\r\nContent-Length: 2\r\nConnection: close\r\n\r\nhi");
        let req = clean_request(&json!({"method": "post", "url": url, "headers": [["X-Q", "z"]], "body": "data"})).unwrap();
        let out = send_http(&req, &Arc::new(AtomicBool::new(false)));
        assert_eq!(out["ok"], true, "{out}");
        assert_eq!((out["status"].as_u64(), out["body"].as_str(), out["binary"].as_bool()), (Some(200), Some("hi"), Some(false)));
        let seen = h.join().unwrap();
        assert!(seen.starts_with("POST /p?q=1 HTTP/1.1") && seen.to_lowercase().contains("x-q: z") && seen.ends_with("data"), "{seen}");
        let (url, _h) = serve(b"HTTP/1.1 302 Found\r\nLocation: http://other/\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        let req = clean_request(&json!({"method": "get", "url": url, "headers": []})).unwrap();
        let out = send_http(&req, &Arc::new(AtomicBool::new(false)));
        assert_eq!(out["status"], 302, "{out}");
        let get_body = clean_request(&json!({"method": "get", "url": "http://127.0.0.1:1/", "headers": [], "body": "x"})).unwrap();
        assert_eq!(send_http(&get_body, &Arc::new(AtomicBool::new(false))), fail("get-body"));
    }

    #[test]
    fn refused_and_cancel() {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        drop(l);
        let req = clean_request(&json!({"method": "get", "url": format!("http://127.0.0.1:{port}/"), "headers": []})).unwrap();
        let out = send_http(&req, &Arc::new(AtomicBool::new(false)));
        assert_eq!(out["error"], "network", "{out}");
        let flag = Arc::new(AtomicBool::new(true));
        assert_eq!(send_http(&req, &flag), fail("cancelled"));
    }

    #[test]
    fn host_confirm_shared() {
        let st = HttpState::default();
        let req = clean_request(&json!({"method": "get", "url": "https://example.com/", "headers": []})).unwrap();
        assert!(!st.host_ok(&req, &|_, _| false));
        assert!(st.host_ok(&req, &|_, _| true));
        assert!(st.host_ok(&req, &|_, _| panic!("asked twice")));
        let id = st.start("a");
        let id2 = st.start("a");
        assert!(id.load(Ordering::SeqCst) && !id2.load(Ordering::SeqCst));
        assert!(st.cancel("a") && !st.cancel("a"));
    }
}
