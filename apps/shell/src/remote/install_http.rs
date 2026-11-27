use std::net::IpAddr;
use std::path::Path;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, timeout_at, Instant};

use super::config::now_secs;
use super::install::{resolve, Reply, Tickets, FILE, MAX_HEAD, MIME};

const SEND_MAX: Duration = Duration::from_secs(900);
const SNIFF_WAIT: Duration = Duration::from_millis(20);
const NEEDLES: [&[u8]; 2] = [b"GET /install/", b"HEAD /install/"];
pub const PAGE_CSP: &str = "default-src 'none'; style-src 'unsafe-inline'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'";

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

pub fn html_head(len: usize) -> String {
    format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {len}\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nContent-Security-Policy: {PAGE_CSP}\r\nX-Content-Type-Options: nosniff\r\nX-Frame-Options: DENY\r\nConnection: close\r\n\r\n"
    )
}

pub fn redirect_head(location: &str) -> String {
    format!("HTTP/1.1 302 Found\r\nLocation: {location}\r\nContent-Length: 0\r\nCache-Control: no-store\r\nReferrer-Policy: no-referrer\r\nConnection: close\r\n\r\n")
}

pub fn servable(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|m| m.is_file() && !m.file_type().is_symlink())
}

async fn respond(stream: &mut TcpStream, reply: Reply) -> std::io::Result<()> {
    let (path, head_only) = match reply {
        Reply::File { path, head, .. } => (path, head),
        Reply::Html { body, head } => {
            stream.write_all(html_head(body.len()).as_bytes()).await?;
            return if head { Ok(()) } else { stream.write_all(body.as_bytes()).await };
        }
        Reply::Redirect(location) => return stream.write_all(redirect_head(&location).as_bytes()).await,
        Reply::Forbidden | Reply::Pair { .. } => return stream.write_all(status_head(403, "Forbidden").as_bytes()).await,
        Reply::Method => return stream.write_all(status_head(405, "Method Not Allowed").as_bytes()).await,
        Reply::NotFound => return stream.write_all(status_head(404, "Not Found").as_bytes()).await,
    };
    let opened = if servable(&path) { tokio::fs::File::open(&path).await.ok() } else { None };
    let Some(file) = opened else { return stream.write_all(status_head(404, "Not Found").as_bytes()).await };
    let len = file.metadata().await?.len();
    stream.write_all(file_head(len).as_bytes()).await?;
    if !head_only {
        let mut body = file.take(len);
        let _ = timeout(SEND_MAX, tokio::io::copy(&mut body, stream)).await;
    }
    Ok(())
}

pub async fn serve_with(tickets: &Tickets, mut stream: TcpStream, ip: IpAddr, deadline: Instant, after: impl FnOnce(Reply) -> Reply) {
    let Some(head) = read_head(&mut stream, deadline).await else { return };
    let reply = after(resolve(tickets, ip, &head, now_secs()));
    let _ = respond(&mut stream, reply).await;
    let _ = stream.shutdown().await;
}

pub async fn serve(tickets: &Tickets, stream: TcpStream, ip: IpAddr, deadline: Instant) {
    serve_with(tickets, stream, ip, deadline, |r| r).await;
}
