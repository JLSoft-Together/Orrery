use std::collections::VecDeque;
use std::io::Read;
use std::sync::OnceLock;
use std::time::Duration;
use ureq::tls::{RootCerts, TlsConfig};

pub const API: &str = "https://api.github.com";
const JSON_MAX: u64 = 8 * 1024 * 1024;

pub struct ApiRes {
    pub status: u16,
    pub body: serde_json::Value,
    pub scopes: Option<String>,
    pub accepted: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Post,
    Put,
    Patch,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetErr {
    Timeout,
    Network,
}

impl NetErr {
    pub fn code(self) -> &'static str {
        match self {
            NetErr::Timeout => "timeout",
            NetErr::Network => "network",
        }
    }
}

pub fn api_base() -> String {
    std::env::var("CCM_GH_API")
        .ok()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| crate::libs::registry::is_allowed_override(v))
        .unwrap_or_else(|| API.into())
}

pub fn agent() -> &'static ureq::Agent {
    static AGENT: OnceLock<ureq::Agent> = OnceLock::new();
    AGENT.get_or_init(|| {
        let tls = TlsConfig::builder().root_certs(RootCerts::PlatformVerifier).build();
        ureq::Agent::config_builder().http_status_as_error(false).tls_config(tls).max_redirects(5).build().into()
    })
}

fn net_err(e: &ureq::Error) -> NetErr {
    match e {
        ureq::Error::Timeout(_) => NetErr::Timeout,
        ureq::Error::Io(io) if io.kind() == std::io::ErrorKind::TimedOut => NetErr::Timeout,
        _ => NetErr::Network,
    }
}

fn io_timeout(e: &std::io::Error) -> bool {
    e.kind() == std::io::ErrorKind::TimedOut || e.get_ref().and_then(|i| i.downcast_ref::<ureq::Error>()).is_some_and(|u| matches!(u, ureq::Error::Timeout(_)))
}

fn headers<B>(req: ureq::RequestBuilder<B>, token: &str, accept: &str) -> ureq::RequestBuilder<B> {
    req.header("Accept", accept)
        .header("Authorization", &format!("Bearer {token}"))
        .header("User-Agent", "Orrery")
        .header("X-GitHub-Api-Version", "2022-11-28")
}

fn header(res: &ureq::http::Response<ureq::Body>, name: &str) -> Option<String> {
    res.headers().get(name).and_then(|v| v.to_str().ok()).map(str::to_string)
}

fn read_json(res: &mut ureq::http::Response<ureq::Body>) -> Result<serde_json::Value, NetErr> {
    let mut bytes = Vec::new();
    match res.body_mut().with_config().limit(JSON_MAX).reader().read_to_end(&mut bytes) {
        Ok(_) if bytes.is_empty() => Ok(serde_json::Value::Null),
        Ok(_) => Ok(serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null)),
        Err(e) if io_timeout(&e) => Err(NetErr::Timeout),
        Err(_) => Err(NetErr::Network),
    }
}

pub fn get(base: &str, token: &str, route: &str, timeout: Duration) -> Option<ApiRes> {
    let req = agent().get(&format!("{base}{route}")).config().timeout_global(Some(timeout)).build();
    let mut res = headers(req, token, "application/vnd.github+json").call().ok()?;
    let status = res.status().as_u16();
    let scopes = header(&res, "x-oauth-scopes");
    let accepted = header(&res, "x-accepted-github-permissions");
    let body = if status == 200 { read_json(&mut res).ok()? } else { serde_json::Value::Null };
    Some(ApiRes { status, body, scopes, accepted })
}

pub fn fetch(base: &str, token: &str, route: &str, timeout: Duration) -> Result<ApiRes, NetErr> {
    let req = agent().get(&format!("{base}{route}")).config().timeout_global(Some(timeout)).build();
    let mut res = headers(req, token, "application/vnd.github+json").call().map_err(|e| net_err(&e))?;
    let status = res.status().as_u16();
    let scopes = header(&res, "x-oauth-scopes");
    let accepted = header(&res, "x-accepted-github-permissions");
    let body = read_json(&mut res)?;
    Ok(ApiRes { status, body, scopes, accepted })
}

pub fn send(method: Method, base: &str, token: &str, route: &str, body: Option<&serde_json::Value>, timeout: Duration) -> Result<ApiRes, NetErr> {
    let url = format!("{base}{route}");
    let a = agent();
    let accept = "application/vnd.github+json";
    let call = match method {
        Method::Delete => headers(a.delete(&url).config().timeout_global(Some(timeout)).build(), token, accept).call(),
        m => {
            let req = match m {
                Method::Post => a.post(&url),
                Method::Put => a.put(&url),
                _ => a.patch(&url),
            };
            let text = body.map(|b| b.to_string()).unwrap_or_else(|| "{}".into());
            headers(req.config().timeout_global(Some(timeout)).build(), token, accept).header("Content-Type", "application/json").send(text.as_bytes())
        }
    };
    let mut res = call.map_err(|e| net_err(&e))?;
    let status = res.status().as_u16();
    let scopes = header(&res, "x-oauth-scopes");
    let accepted = header(&res, "x-accepted-github-permissions");
    let body = read_json(&mut res)?;
    Ok(ApiRes { status, body, scopes, accepted })
}

pub struct TextRes {
    pub status: u16,
    pub tail: Vec<u8>,
    pub cut: bool,
    pub redirected: bool,
}

pub fn fetch_tail(base: &str, token: &str, route: &str, timeout: Duration, keep: usize, read_max: u64) -> Result<TextRes, NetErr> {
    use ureq::ResponseExt;
    let url = format!("{base}{route}");
    let req = agent().get(&url).config().timeout_global(Some(timeout)).build();
    let mut res = headers(req, token, "application/vnd.github+json").call().map_err(|e| net_err(&e))?;
    let status = res.status().as_u16();
    let redirected = res.get_uri().to_string() != url;
    if !(200..300).contains(&status) {
        return Ok(TextRes { status, tail: Vec::new(), cut: false, redirected });
    }
    let mut reader = res.body_mut().with_config().limit(read_max).reader();
    let mut tail: VecDeque<u8> = VecDeque::with_capacity(keep.min(1 << 20) + 64 * 1024);
    let mut cut = false;
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                tail.extend(&buf[..n]);
                if tail.len() > keep {
                    let drop = tail.len() - keep;
                    tail.drain(..drop);
                    cut = true;
                }
            }
            Err(_) if !tail.is_empty() => {
                cut = true;
                break;
            }
            Err(e) if io_timeout(&e) => return Err(NetErr::Timeout),
            Err(_) => return Err(NetErr::Network),
        }
    }
    Ok(TextRes { status, tail: Vec::from(tail), cut, redirected })
}

pub fn scrub_token(text: &str, token: &str) -> String {
    if token.len() < 8 {
        return text.to_string();
    }
    text.replace(token, "[token]")
}

pub fn encode_component(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub fn encode_path(s: &str) -> String {
    s.split('/').map(encode_component).collect::<Vec<_>>().join("/")
}

pub fn name_ok(s: &str) -> bool {
    !s.is_empty() && s.len() <= 100 && s != "." && s != ".." && s.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

pub fn github_repo_of(url: &str) -> Option<(String, String, bool)> {
    static RE: OnceLock<(regex::Regex, regex::Regex)> = OnceLock::new();
    let (https, ssh) = RE.get_or_init(|| {
        (
            regex::Regex::new(r"(?i)^https://(?:[^@/\s]+@)?github\.com(?::443)?/([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+?)(?:\.git)?/?$").unwrap(),
            regex::Regex::new(r"(?i)^(?:ssh://)?(?:[A-Za-z0-9_.-]+@)?(?:ssh\.)?github\.com[:/](?:443/)?([A-Za-z0-9_.-]+)/([A-Za-z0-9_.-]+?)(?:\.git)?/?$").unwrap(),
        )
    });
    let s = crate::claude::js_trim(url);
    let caps = https.captures(s).or_else(|| ssh.captures(s))?;
    let is_https = s.get(..6).is_some_and(|h| h.eq_ignore_ascii_case("https:"));
    let (owner, repo) = (&caps[1], &caps[2]);
    (name_ok(owner) && name_ok(repo)).then(|| (owner.to_string(), repo.to_string(), is_https))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_urls() {
        assert_eq!(github_repo_of(" https://github.com/o/r.git "), Some(("o".into(), "r".into(), true)));
        assert_eq!(github_repo_of("https://u@GitHub.com:443/o/r/"), Some(("o".into(), "r".into(), true)));
        assert_eq!(github_repo_of("git@github.com:o/r.git"), Some(("o".into(), "r".into(), false)));
        assert_eq!(github_repo_of("ssh://git@ssh.github.com:443/o/r"), Some(("o".into(), "r".into(), false)));
        assert_eq!(github_repo_of("ssh://git@ssh.github.com/443/o/r"), Some(("o".into(), "r".into(), false)));
        assert_eq!(github_repo_of("https://gitlab.com/o/r"), None);
        assert_eq!(github_repo_of("https://github.com/../r"), None);
        assert_eq!(github_repo_of("https://github.com/o/.."), None);
        assert_eq!(github_repo_of("git@github.com:./r.git"), None);
        assert!(name_ok("a.b_c-1") && !name_ok("") && !name_ok(".") && !name_ok("..") && !name_ok("a/b") && !name_ok("a b") && !name_ok("é"));
        assert_eq!(encode_component("a b/é"), "a%20b%2F%C3%A9");
        assert_eq!(encode_path("feat/a b"), "feat/a%20b");
        assert_eq!(scrub_token("bad ghp_abcdefghij here", "ghp_abcdefghij"), "bad [token] here");
    }
}
