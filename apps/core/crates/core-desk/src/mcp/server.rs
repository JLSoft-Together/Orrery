use super::host::ToolHost;
use super::rpc::{handle_mcp, mcp_gate, Gate, BODY_MAX};
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

pub type Callers = Arc<dyn Fn(&str) -> Option<Arc<dyn ToolHost>> + Send + Sync>;

const HEAD_MAX: usize = 16 * 1024;
const CONN_MAX: usize = 64;
const IDLE: Duration = Duration::from_secs(120);

pub struct McpServer {
    port: u16,
    stop: Arc<AtomicBool>,
}

struct Req {
    method: String,
    url: String,
    host: Option<String>,
    origin: Option<String>,
    auth: Option<String>,
    length: Option<usize>,
    chunked: bool,
    close: bool,
}

fn read_head(r: &mut BufReader<TcpStream>) -> Option<Req> {
    let mut line = String::new();
    let mut total = 0;
    loop {
        line.clear();
        let n = r.read_line(&mut line).ok()?;
        if n == 0 {
            return None;
        }
        if !line.trim().is_empty() {
            break;
        }
    }
    total += line.len();
    let mut parts = line.split_whitespace();
    let method = parts.next()?.to_string();
    let url = parts.next()?.to_string();
    let http10 = parts.next() == Some("HTTP/1.0");
    let mut req = Req { method, url, host: None, origin: None, auth: None, length: None, chunked: false, close: http10 };
    loop {
        let mut h = String::new();
        let n = r.read_line(&mut h).ok()?;
        total += n;
        if n == 0 || total > HEAD_MAX {
            return None;
        }
        let h = h.trim_end_matches(['\r', '\n']);
        if h.is_empty() {
            return Some(req);
        }
        let Some((k, v)) = h.split_once(':') else { continue };
        let v = v.trim().to_string();
        match k.trim().to_ascii_lowercase().as_str() {
            "host" => req.host = Some(v),
            "origin" => req.origin = Some(v),
            "authorization" => req.auth = Some(v),
            "content-length" => req.length = Some(v.parse().unwrap_or(usize::MAX)),
            "transfer-encoding" => req.chunked = v.to_ascii_lowercase().contains("chunked"),
            "connection" => req.close = v.eq_ignore_ascii_case("close") || (http10 && !v.eq_ignore_ascii_case("keep-alive")),
            _ => {}
        }
    }
}

fn read_chunked(r: &mut BufReader<TcpStream>) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    loop {
        let mut size = String::new();
        r.read_line(&mut size).ok()?;
        let n = usize::from_str_radix(size.trim().split(';').next()?.trim(), 16).ok()?;
        if n == 0 {
            let mut end = String::new();
            loop {
                end.clear();
                r.read_line(&mut end).ok()?;
                if end.trim().is_empty() {
                    return Some(out);
                }
            }
        }
        if out.len() + n > BODY_MAX {
            return None;
        }
        let mut buf = vec![0; n + 2];
        r.read_exact(&mut buf).ok()?;
        out.extend_from_slice(&buf[..n]);
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        _ => "Internal Server Error",
    }
}

fn reply(w: &mut TcpStream, status: u16, body: Option<&Value>, extra: &str, close: bool) -> bool {
    let text = body.map(Value::to_string).unwrap_or_default();
    let kind = if body.is_some() { "Content-Type: application/json\r\n" } else { "" };
    let conn = if close { "Connection: close\r\n" } else { "Connection: keep-alive\r\n" };
    let head = format!("HTTP/1.1 {status} {}\r\n{kind}{extra}{conn}Content-Length: {}\r\n\r\n", reason(status), text.len());
    w.write_all(head.as_bytes()).and_then(|_| w.write_all(text.as_bytes())).and_then(|_| w.flush()).is_ok()
}

fn serve(stream: TcpStream, port: u16, callers: Callers, version: Arc<String>, deepseek: bool) {
    let _ = stream.set_read_timeout(Some(IDLE));
    let Ok(mut w) = stream.try_clone() else { return };
    let mut r = BufReader::new(stream);
    while let Some(req) = read_head(&mut r) {
        let gate = mcp_gate(&req.method, &req.url, req.host.as_deref(), req.origin.as_deref(), req.auth.as_deref(), port);
        let token = match gate {
            Gate::Deny(status, allow) => {
                let extra = allow.map(|a| format!("Allow: {a}\r\n")).unwrap_or_default();
                reply(&mut w, status, None, &extra, true);
                break;
            }
            Gate::Token(t) => t,
        };
        let Some(host) = callers(&token) else {
            reply(&mut w, 401, None, "", true);
            break;
        };
        if req.length.is_some_and(|n| n > BODY_MAX) {
            reply(&mut w, 413, None, "", true);
            break;
        }
        let body = if req.chunked {
            read_chunked(&mut r)
        } else {
            let mut buf = vec![0; req.length.unwrap_or(0)];
            r.read_exact(&mut buf).ok().map(|_| buf)
        };
        let Some(body) = body else {
            reply(&mut w, 413, None, "", true);
            break;
        };
        let parsed = std::str::from_utf8(&body).ok().and_then(|s| serde_json::from_str::<Value>(s).ok());
        let Some(msg) = parsed else {
            let err = json!({ "jsonrpc": "2.0", "id": null, "error": { "code": -32700, "message": "Parse error" } });
            if !reply(&mut w, 400, Some(&err), "", req.close) || req.close {
                break;
            }
            continue;
        };
        let (status, out) = handle_mcp(&msg, host.as_ref(), &version, deepseek);
        if !reply(&mut w, status, out.as_ref(), "", req.close) || req.close {
            break;
        }
    }
    let _ = w.shutdown(Shutdown::Both);
}

impl McpServer {
    pub fn start(callers: Callers, version: &str, deepseek: bool) -> std::io::Result<McpServer> {
        let listener = TcpListener::bind(("127.0.0.1", 0))?;
        let port = listener.local_addr()?.port();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let version = Arc::new(version.to_string());
        let open = Arc::new(AtomicUsize::new(0));
        std::thread::Builder::new().name("mcp-accept".into()).spawn(move || {
            for conn in listener.incoming() {
                if flag.load(Ordering::SeqCst) {
                    break;
                }
                let Ok(stream) = conn else { continue };
                if open.load(Ordering::SeqCst) >= CONN_MAX {
                    let _ = stream.shutdown(Shutdown::Both);
                    continue;
                }
                open.fetch_add(1, Ordering::SeqCst);
                let (callers, version, open_err) = (callers.clone(), version.clone(), open.clone());
                let open = open.clone();
                let spawned = std::thread::Builder::new().name("mcp-conn".into()).spawn(move || {
                    serve(stream, port, callers, version, deepseek);
                    open.fetch_sub(1, Ordering::SeqCst);
                });
                if spawned.is_err() {
                    open_err.fetch_sub(1, Ordering::SeqCst);
                }
            }
        })?;
        Ok(McpServer { port, stop })
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn stop(&self) {
        if !self.stop.swap(true, Ordering::SeqCst) {
            let _ = TcpStream::connect(("127.0.0.1", self.port));
        }
    }
}

impl Drop for McpServer {
    fn drop(&mut self) {
        self.stop();
    }
}
