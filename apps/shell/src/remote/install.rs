use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, timeout_at, Instant};

use super::config::now_secs;
use super::guard_ip::norm;
use super::lan::is_private_v4;
use crate::state::lock;

pub const PREFIX: &str = "/install/";
pub const FILE: &str = "orrery.apk";
pub const TTL_SECS: u64 = 600;
pub const MAX_HEAD: usize = 8192;
pub const MAX_APK: u64 = 1 << 30;
pub const MAX_URL: usize = 2048;
pub const MIME: &str = "application/vnd.android.package-archive";
const TOKEN_BYTES: usize = 16;
const MAX_TOKEN: usize = 64;
const SEND_MAX: Duration = Duration::from_secs(900);
const SNIFF_WAIT: Duration = Duration::from_millis(20);
const NEEDLES: [&[u8]; 2] = [b"GET /install/", b"HEAD /install/"];

#[derive(Clone)]
struct Ticket {
    token: String,
    path: PathBuf,
    exp: u64,
}

#[derive(Default)]
pub struct Tickets {
    slot: Mutex<Option<Ticket>>,
}

fn same(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

impl Tickets {
    pub fn issue(&self, path: PathBuf, now: u64) -> Option<(String, u64)> {
        let mut raw = [0u8; TOKEN_BYTES];
        getrandom::fill(&mut raw).ok()?;
        let token = URL_SAFE_NO_PAD.encode(raw);
        let exp = now + TTL_SECS;
        *lock(&self.slot) = Some(Ticket { token: token.clone(), path, exp });
        Some((token, exp))
    }

    pub fn revoke(&self) {
        *lock(&self.slot) = None;
    }

    pub fn lookup(&self, token: &str, now: u64) -> Option<PathBuf> {
        let mut slot = lock(&self.slot);
        let live = slot.as_ref()?;
        if now >= live.exp {
            *slot = None;
            return None;
        }
        same(live.token.as_bytes(), token.as_bytes()).then(|| live.path.clone())
    }
}

pub fn url_path(token: &str) -> String {
    format!("{PREFIX}{token}/{FILE}")
}

pub fn is_apk(path: &Path) -> bool {
    path.is_absolute()
        && path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("apk"))
        && std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.len() > 0 && m.len() <= MAX_APK)
}

pub fn newest_apk(dir: &Path) -> Option<PathBuf> {
    std::fs::read_dir(dir)
        .ok()?
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| is_apk(p))
        .filter_map(|p| std::fs::metadata(&p).and_then(|m| m.modified()).ok().map(|t| (t, p)))
        .max_by(|a, b| a.0.cmp(&b.0))
        .map(|(_, p)| p)
}

pub fn suggestion() -> Option<PathBuf> {
    let dir = crate::config::repo_root().join("apps").join("mobile").join("build").join("app").join("outputs").join("flutter-apk");
    newest_apk(&lexical(&dir))
}

pub fn lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            std::path::Component::ParentDir => {
                out.pop();
            }
            std::path::Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

pub fn valid_url(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else { return false };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    url.len() <= MAX_URL
        && !host.is_empty()
        && !host.contains('@')
        && !host.starts_with(':')
        && url.chars().all(|c| c.is_ascii_graphic())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    File { path: PathBuf, head: bool },
    NotFound,
    Forbidden,
    Method,
}

pub fn allowed_peer(ip: IpAddr) -> bool {
    matches!(norm(ip), IpAddr::V4(v4) if is_private_v4(v4))
}

pub fn resolve(tickets: &Tickets, ip: IpAddr, head: &str, now: u64) -> Reply {
    if !allowed_peer(ip) {
        return Reply::Forbidden;
    }
    let line = head.split("\r\n").next().unwrap_or("");
    let mut parts = line.split(' ');
    let (Some(method), Some(target), Some(version), None) = (parts.next(), parts.next(), parts.next(), parts.next()) else { return Reply::NotFound };
    if !version.starts_with("HTTP/1.") {
        return Reply::NotFound;
    }
    let head_only = match method {
        "GET" => false,
        "HEAD" => true,
        _ => return Reply::Method,
    };
    let Some(token) = target.strip_prefix(PREFIX).and_then(|r| r.strip_suffix(FILE)).and_then(|r| r.strip_suffix('/')) else { return Reply::NotFound };
    if token.is_empty() || token.len() > MAX_TOKEN || !token.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_') {
        return Reply::NotFound;
    }
    match tickets.lookup(token, now) {
        Some(path) if is_apk(&path) => Reply::File { path, head: head_only },
        _ => Reply::NotFound,
    }
}

pub fn classify(seen: &[u8]) -> Option<bool> {
    if NEEDLES.iter().any(|n| seen.starts_with(n)) {
        return Some(true);
    }
    if NEEDLES.iter().any(|n| n.starts_with(seen)) {
        return None;
    }
    Some(false)
}

pub async fn sniff(stream: &TcpStream, deadline: Instant) -> bool {
    let mut buf = [0u8; 16];
    loop {
        let Ok(Ok(n)) = timeout_at(deadline, stream.peek(&mut buf)).await else { return false };
        if n == 0 {
            return false;
        }
        match classify(&buf[..n]) {
            Some(hit) => return hit,
            None if Instant::now() + SNIFF_WAIT < deadline => tokio::time::sleep(SNIFF_WAIT).await,
            None => return false,
        }
    }
}

async fn read_head(stream: &mut TcpStream, deadline: Instant) -> Option<String> {
    let mut buf = Vec::with_capacity(1024);
    let mut chunk = [0u8; 1024];
    loop {
        let n = timeout_at(deadline, stream.read(&mut chunk)).await.ok()?.ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
        if buf.len() > MAX_HEAD {
            return None;
        }
    }
    String::from_utf8(buf).ok()
}

fn status_head(code: u16, text: &str) -> String {
    format!("HTTP/1.1 {code} {text}\r\nContent-Length: 0\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n")
}

pub fn file_head(len: u64) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: {MIME}\r\nContent-Length: {len}\r\nContent-Disposition: attachment; filename=\"{FILE}\"\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nConnection: close\r\n\r\n"
    )
}

async fn respond(stream: &mut TcpStream, reply: Reply) -> std::io::Result<()> {
    let (path, head_only) = match reply {
        Reply::File { path, head } => (path, head),
        Reply::Forbidden => return stream.write_all(status_head(403, "Forbidden").as_bytes()).await,
        Reply::Method => return stream.write_all(status_head(405, "Method Not Allowed").as_bytes()).await,
        Reply::NotFound => return stream.write_all(status_head(404, "Not Found").as_bytes()).await,
    };
    let Ok(file) = tokio::fs::File::open(&path).await else { return stream.write_all(status_head(404, "Not Found").as_bytes()).await };
    let len = file.metadata().await?.len();
    stream.write_all(file_head(len).as_bytes()).await?;
    if !head_only {
        let mut body = file.take(len);
        let _ = timeout(SEND_MAX, tokio::io::copy(&mut body, stream)).await;
    }
    Ok(())
}

pub async fn serve(tickets: &Tickets, mut stream: TcpStream, ip: IpAddr, deadline: Instant) {
    let Some(head) = read_head(&mut stream, deadline).await else { return };
    let reply = resolve(tickets, ip, &head, now_secs());
    let _ = respond(&mut stream, reply).await;
    let _ = stream.shutdown().await;
}
