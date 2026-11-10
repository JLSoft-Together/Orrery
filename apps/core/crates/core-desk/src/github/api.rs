use std::io::Read;
use std::time::Duration;
use ureq::tls::{RootCerts, TlsConfig};

pub const API: &str = "https://api.github.com";

pub struct ApiRes {
    pub status: u16,
    pub body: serde_json::Value,
    pub scopes: Option<String>,
}

pub fn api_base() -> String {
    std::env::var("CCM_GH_API")
        .ok()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| crate::libs::registry::is_allowed_override(v))
        .unwrap_or_else(|| API.into())
}

pub fn get(base: &str, token: &str, route: &str, timeout: Duration) -> Option<ApiRes> {
    let tls = TlsConfig::builder().root_certs(RootCerts::PlatformVerifier).build();
    let config = ureq::Agent::config_builder().timeout_global(Some(timeout)).http_status_as_error(false).tls_config(tls).build();
    let agent: ureq::Agent = config.into();
    let mut res = agent
        .get(&format!("{base}{route}"))
        .header("Accept", "application/vnd.github+json")
        .header("Authorization", &format!("Bearer {token}"))
        .header("User-Agent", "Orrery")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .call()
        .ok()?;
    let status = res.status().as_u16();
    let scopes = res.headers().get("x-oauth-scopes").and_then(|v| v.to_str().ok()).map(str::to_string);
    let body = if status == 200 {
        let mut bytes = Vec::new();
        match res.body_mut().with_config().limit(8 * 1024 * 1024).reader().read_to_end(&mut bytes) {
            Ok(_) => serde_json::from_slice(&bytes).unwrap_or(serde_json::Value::Null),
            Err(_) => return None,
        }
    } else {
        serde_json::Value::Null
    };
    Some(ApiRes { status, body, scopes })
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

pub fn github_repo_of(url: &str) -> Option<(String, String, bool)> {
    use std::sync::OnceLock;
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
    Some((caps[1].to_string(), caps[2].to_string(), is_https))
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
        assert_eq!(encode_component("a b/é"), "a%20b%2F%C3%A9");
    }
}
