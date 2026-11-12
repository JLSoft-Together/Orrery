use std::time::Duration;

use ureq::tls::{RootCerts, TlsConfig};
use zeroize::Zeroizing;

pub const API_BASE: &str = "https://api.cloudflare.com/client/v4";
const HTTP_TIMEOUT: Duration = Duration::from_secs(60);
const PROBE_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_BODY: u64 = 4 << 20;

pub struct Req {
    pub method: &'static str,
    pub path: String,
    pub content_type: String,
    pub body: Vec<u8>,
}

impl Req {
    pub fn get(path: impl Into<String>) -> Req {
        Req { method: "GET", path: path.into(), content_type: String::new(), body: Vec::new() }
    }

    pub fn json(method: &'static str, path: impl Into<String>, body: &serde_json::Value) -> Req {
        Req { method, path: path.into(), content_type: "application/json".into(), body: body.to_string().into_bytes() }
    }
}

pub struct Resp {
    pub status: u16,
    pub body: String,
}

pub trait Http {
    fn call(&self, req: Req) -> Option<Resp>;
}

pub struct UreqHttp {
    base: String,
    token: Zeroizing<String>,
    timeout: Duration,
}

impl UreqHttp {
    pub fn new(base: &str, token: &str) -> UreqHttp {
        UreqHttp { base: base.trim_end_matches('/').to_owned(), token: Zeroizing::new(token.to_owned()), timeout: if token.is_empty() { PROBE_TIMEOUT } else { HTTP_TIMEOUT } }
    }
}

impl Http for UreqHttp {
    fn call(&self, req: Req) -> Option<Resp> {
        let tls = TlsConfig::builder().root_certs(RootCerts::PlatformVerifier).build();
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(self.timeout)).max_redirects(0).http_status_as_error(false).tls_config(tls).build().into();
        let url = format!("{}{}", self.base, req.path);
        let authed = !self.token.is_empty();
        let auth = Zeroizing::new(if authed { format!("Bearer {}", self.token.as_str()) } else { String::new() });
        let mut res = match req.method {
            "GET" | "DELETE" => {
                let mut b = if req.method == "GET" { agent.get(&url) } else { agent.delete(&url) };
                if authed {
                    b = b.header("authorization", auth.as_str());
                }
                b.call().ok()?
            }
            method => {
                let mut b = if method == "PUT" { agent.put(&url) } else { agent.post(&url) };
                if authed {
                    b = b.header("authorization", auth.as_str());
                }
                b.header("content-type", &req.content_type).send(&req.body[..]).ok()?
            }
        };
        let status = res.status().as_u16();
        let body = res.body_mut().with_config().limit(MAX_BODY).read_to_string().unwrap_or_default();
        Some(Resp { status, body })
    }
}

pub struct Multipart {
    boundary: String,
    body: Vec<u8>,
}

impl Multipart {
    pub fn new() -> Option<Multipart> {
        let mut raw = [0u8; 12];
        getrandom::fill(&mut raw).ok()?;
        let boundary = format!("----orrery{}", raw.iter().map(|b| format!("{b:02x}")).collect::<String>());
        Some(Multipart { boundary, body: Vec::new() })
    }

    pub fn part(&mut self, name: &str, filename: Option<&str>, content_type: &str, data: &[u8]) {
        let file = filename.map(|f| format!("; filename=\"{f}\"")).unwrap_or_default();
        let head = format!("--{}\r\nContent-Disposition: form-data; name=\"{name}\"{file}\r\nContent-Type: {content_type}\r\n\r\n", self.boundary);
        self.body.extend_from_slice(head.as_bytes());
        self.body.extend_from_slice(data);
        self.body.extend_from_slice(b"\r\n");
    }

    pub fn finish(mut self) -> (String, Vec<u8>) {
        self.body.extend_from_slice(format!("--{}--\r\n", self.boundary).as_bytes());
        (format!("multipart/form-data; boundary={}", self.boundary), self.body)
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::sync::{Arc, Mutex};

    use super::*;

    fn serve(reply: &'static str) -> (u16, Arc<Mutex<String>>, std::thread::JoinHandle<()>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen = Arc::new(Mutex::new(String::new()));
        let s = seen.clone();
        let t = std::thread::spawn(move || {
            let (mut sock, _) = listener.accept().unwrap();
            sock.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
            let mut buf = Vec::new();
            let mut chunk = [0u8; 4096];
            loop {
                let n = sock.read(&mut chunk).unwrap_or(0);
                buf.extend_from_slice(&chunk[..n]);
                let text = String::from_utf8_lossy(&buf).to_string();
                let multipart = text.contains("multipart/form-data");
                if n == 0 || (text.contains("\r\n\r\n") && (!multipart || text.contains("--\r\n"))) {
                    break;
                }
            }
            *s.lock().unwrap() = String::from_utf8_lossy(&buf).to_string();
            let _ = sock.write_all(reply.as_bytes());
        });
        (port, seen, t)
    }

    const OK: &str = "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 27\r\nconnection: close\r\n\r\n{\"success\":true,\"result\":{}}";

    #[test]
    fn the_client_sends_the_bearer_header_and_a_well_formed_multipart() {
        let (port, seen, t) = serve(OK);
        let http = UreqHttp::new(&format!("http://127.0.0.1:{port}"), "tok_abcdefghijklmnopqrstuvwx");
        let mut form = Multipart::new().unwrap();
        form.part("metadata", None, "application/json", b"{}");
        form.part("worker.js", Some("worker.js"), "application/javascript+module", b"export default {}");
        let (content_type, body) = form.finish();
        let res = http.call(Req { method: "PUT", path: "/x".into(), content_type: content_type.clone(), body }).unwrap();
        t.join().unwrap();
        assert_eq!(res.status, 200);
        let raw = seen.lock().unwrap().clone();
        assert!(raw.to_lowercase().contains("authorization: bearer tok_abcdefghijklmnopqrstuvwx"));
        assert!(raw.starts_with("PUT /x"));
        assert!(raw.contains("name=\"worker.js\"; filename=\"worker.js\""));
        assert!(raw.to_lowercase().contains(&content_type.to_lowercase()));
    }

    #[test]
    fn a_probe_without_a_token_sends_no_authorization_header() {
        let (port, seen, t) = serve(OK);
        let http = UreqHttp::new(&format!("http://127.0.0.1:{port}"), "");
        assert_eq!(http.call(Req::get("/health")).unwrap().status, 200);
        t.join().unwrap();
        assert!(!seen.lock().unwrap().to_lowercase().contains("authorization"));
    }

    #[test]
    fn redirects_are_not_followed() {
        let (port, _, t) = serve("HTTP/1.1 302 Found\r\nlocation: http://127.0.0.1:1/elsewhere\r\ncontent-length: 0\r\nconnection: close\r\n\r\n");
        let http = UreqHttp::new(&format!("http://127.0.0.1:{port}"), "");
        let res = http.call(Req::get("/health"));
        t.join().unwrap();
        assert_eq!(res.map(|r| r.status), Some(302));
    }
}
