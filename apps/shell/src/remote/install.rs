use std::net::IpAddr;
use std::path::{Path, PathBuf};

use super::guard_ip::norm;
use super::lan::is_private_v4;

pub use super::install_http::{classify, file_head, serve, sniff};
pub use super::install_tickets::{TicketKind, Tickets};

pub const PREFIX: &str = "/install/";
pub const FILE: &str = "orrery.apk";
pub const PAIR: &str = "pair";
pub const TTL_SECS: u64 = 600;
pub const MAX_HEAD: usize = 8192;
pub const MAX_APK: u64 = 1 << 30;
pub const MAX_URL: usize = 2048;
pub const MIME: &str = "application/vnd.android.package-archive";
pub const MAX_SLOTS: usize = 4;
const MAX_TOKEN: usize = 64;


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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Custom,
    Bundled,
    Dev,
}

impl Source {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Custom => "custom",
            Self::Bundled => "bundled",
            Self::Dev => "dev",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApkSource {
    pub path: PathBuf,
    pub source: Source,
    pub version: String,
}

pub fn pick_source(custom: &str, bundled: Option<(PathBuf, String)>, dev: Option<PathBuf>) -> Option<ApkSource> {
    let custom_path = Some(PathBuf::from(custom)).filter(|p| !custom.is_empty() && is_apk(p));
    if let Some(path) = custom_path {
        return Some(ApkSource { path, source: Source::Custom, version: String::new() });
    }
    if let Some((path, version)) = bundled.filter(|(p, _)| is_apk(p)) {
        return Some(ApkSource { path, source: Source::Bundled, version });
    }
    dev.filter(|p| is_apk(p)).map(|path| ApkSource { path, source: Source::Dev, version: String::new() })
}

pub fn save_name(version: &str) -> String {
    let v: String = version.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+')).collect();
    if v.is_empty() {
        "orrery-remote.apk".to_owned()
    } else {
        format!("orrery-remote-{v}.apk")
    }
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
    File { path: PathBuf, head: bool, first: bool },
    Html { body: String, head: bool },
    Pair { token: String },
    Redirect(String),
    NotFound,
    Forbidden,
    Method,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Route<'a> {
    ApkFile(&'a str),
    Page(&'a str),
    Pair(&'a str),
}

pub fn allowed_peer(ip: IpAddr) -> bool {
    matches!(norm(ip), IpAddr::V4(v4) if is_private_v4(v4))
}

fn valid_token(token: &str) -> bool {
    !token.is_empty() && token.len() <= MAX_TOKEN && token.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

pub fn page_path(token: &str) -> String {
    format!("{PREFIX}{token}/")
}

pub fn route(target: &str) -> Option<Route<'_>> {
    let (token, leaf) = target.strip_prefix(PREFIX)?.split_once('/')?;
    if !valid_token(token) {
        return None;
    }
    match leaf {
        FILE => Some(Route::ApkFile(token)),
        "" => Some(Route::Page(token)),
        PAIR => Some(Route::Pair(token)),
        _ => None,
    }
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
    let Some(route) = route(target) else { return Reply::NotFound };
    let (Route::ApkFile(token) | Route::Page(token) | Route::Pair(token)) = route;
    let Some(hit) = tickets.hit(token, now).filter(|h| is_apk(&h.path)) else { return Reply::NotFound };
    match route {
        Route::ApkFile(_) => {
            let first = !head_only && tickets.note_fetch(token, ip, now);
            Reply::File { path: hit.path, head: head_only, first }
        }
        _ if hit.kind != TicketKind::ApkPage => Reply::NotFound,
        Route::Page(_) => {
            let size = std::fs::metadata(&hit.path).map_or(0, |m| m.len());
            let body = super::install_page::render(super::install_page::is_vi(head), &hit.version, size);
            Reply::Html { body, head: head_only }
        }
        Route::Pair(_) if head_only => Reply::Method,
        Route::Pair(token) => Reply::Pair { token: token.to_owned() },
    }
}
