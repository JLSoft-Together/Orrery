use super::net;
use super::pick::FeedAsset;
use super::semver::clean_version;
use crate::js;
use regex::Regex;
use serde_json::Value;
use std::io::Read;
use std::time::Duration;
use url::Url;

pub const DEFAULT_FEED: &str = "https://api.github.com/repos/JLSoft-Together/Orrery/releases/latest";
pub const RELEASES_PAGE: &str = "https://github.com/JLSoft-Together/Orrery/releases/latest";
const NOTES_MAX: usize = 40_000;
const FEED_LIMIT: u64 = 8 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct FeedRelease {
    pub version: String,
    pub name: String,
    pub date: Option<String>,
    pub notes: String,
    pub url: String,
    pub assets: Vec<FeedAsset>,
}

pub enum Feed {
    Release(Option<Box<FeedRelease>>),
    Error(&'static str),
}

pub fn parse_digest(raw: Option<&Value>) -> Option<String> {
    let s = raw?.as_str()?;
    let t = crate::claude::js_trim(s);
    let hex = t.get(..7).filter(|p| p.eq_ignore_ascii_case("sha256:")).and_then(|_| t.get(7..))?;
    (hex.len() == 64 && hex.bytes().all(|b| b.is_ascii_hexdigit())).then(|| hex.to_lowercase())
}

fn parse_url(raw: &str) -> Option<Url> {
    Url::parse(raw).ok()
}

fn is_local(u: &Url) -> bool {
    u.scheme() == "http" && matches!(u.host_str(), Some("127.0.0.1" | "localhost"))
}

pub fn is_allowed_feed(raw: &str, allow_local: bool) -> bool {
    parse_url(raw).is_some_and(|u| (u.scheme() == "https" && u.host_str() == Some("api.github.com")) || (allow_local && is_local(&u)))
}

pub fn is_allowed_download(raw: &str, allow_local: bool) -> bool {
    let Some(u) = parse_url(raw) else { return false };
    if allow_local && is_local(&u) {
        return true;
    }
    if u.scheme() != "https" || !u.username().is_empty() || u.password().is_some() {
        return false;
    }
    let host = u.host_str().unwrap_or("");
    host == "github.com" || host.ends_with(".githubusercontent.com")
}

pub fn is_allowed_page(raw: &str, allow_local: bool) -> bool {
    parse_url(raw).is_some_and(|u| (u.scheme() == "https" && u.host_str() == Some("github.com")) || (allow_local && is_local(&u)))
}

fn valid_date(s: &str) -> bool {
    let re = Regex::new(r"^\d{4}-(0[1-9]|1[0-2])-(0[1-9]|[12]\d|3[01])(?:[T ]([01]\d|2[0-3]):[0-5]\d(?::[0-5]\d(?:\.\d+)?)?(?:Z|[+-]\d{2}:?\d{2})?)?$").expect("regex");
    re.is_match(s)
}

pub fn parse_release(json: &Value, allow_local: bool) -> Option<FeedRelease> {
    let r = json.as_object()?;
    if r.get("draft") == Some(&Value::Bool(true)) || r.get("prerelease") == Some(&Value::Bool(true)) {
        return None;
    }
    let version = clean_version(r.get("tag_name")?.as_str()?)?;
    let url = match r.get("html_url").and_then(Value::as_str) {
        Some(u) if is_allowed_page(u, allow_local) => u.to_string(),
        _ => RELEASES_PAGE.to_string(),
    };
    let mut assets = Vec::new();
    if let Some(list) = r.get("assets").and_then(Value::as_array) {
        for raw in list {
            let Some(a) = raw.as_object() else { continue };
            let (Some(name), Some(dl), Some(size)) = (a.get("name").and_then(Value::as_str), a.get("browser_download_url").and_then(Value::as_str), a.get("size").and_then(Value::as_f64)) else { continue };
            if size.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) || !is_allowed_download(dl, allow_local) {
                continue;
            }
            assets.push(FeedAsset { name: name.to_string(), size, url: dl.to_string(), sha256: parse_digest(a.get("digest")) });
        }
    }
    let date = r.get("published_at").and_then(Value::as_str).filter(|d| valid_date(d)).map(str::to_string);
    let notes = r.get("body").and_then(Value::as_str).map(|b| js::slice(b, NOTES_MAX)).unwrap_or_default();
    let name = match r.get("name").and_then(Value::as_str).map(crate::claude::js_trim) {
        Some(n) if !n.is_empty() => js::slice(n, 200),
        _ => version.clone(),
    };
    Some(FeedRelease { version, name, date, notes, url, assets })
}

pub fn fetch_latest(feed: &str, allow_local: bool, timeout: Duration) -> Feed {
    if !is_allowed_feed(feed, allow_local) {
        return Feed::Error("feed");
    }
    let agent = net::agent(Some(timeout), 5);
    let mut res = match agent.get(feed).header("accept", "application/vnd.github+json").header("x-github-api-version", "2022-11-28").call() {
        Ok(r) => r,
        Err(_) => return Feed::Error("offline"),
    };
    let status = res.status().as_u16();
    if status == 404 {
        return Feed::Release(None);
    }
    if !(200..300).contains(&status) {
        return Feed::Error("feed");
    }
    let mut bytes = Vec::new();
    if res.body_mut().with_config().limit(FEED_LIMIT).reader().read_to_end(&mut bytes).is_err() {
        return Feed::Error("feed");
    }
    match serde_json::from_slice::<Value>(&bytes) {
        Ok(v) => Feed::Release(parse_release(&v, allow_local).map(Box::new)),
        Err(_) => Feed::Error("feed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn allow_lists() {
        assert!(is_allowed_feed(DEFAULT_FEED, false));
        assert!(!is_allowed_feed("https://evil.example/x", false));
        assert!(!is_allowed_feed("http://127.0.0.1:1/x", false));
        assert!(is_allowed_feed("http://127.0.0.1:1/x", true));
        assert!(is_allowed_download("https://github.com/a/b/releases/download/v1/x.exe", false));
        assert!(is_allowed_download("https://objects.githubusercontent.com/x", false));
        assert!(!is_allowed_download("https://user:pw@github.com/x", false));
        assert!(!is_allowed_download("http://github.com/x", false));
        assert!(is_allowed_page(RELEASES_PAGE, false));
        assert!(!is_allowed_page("https://github.com.evil.example/x", false));
    }

    #[test]
    fn parses_releases() {
        let digest = format!("sha256:{}", "A".repeat(64));
        let v = json!({"tag_name": "v2.5.0", "html_url": "https://github.com/x/y/releases/tag/v2.5.0", "name": "  Big  ", "body": "notes", "published_at": "2026-01-02T03:04:05Z",
            "assets": [{"name": "Orrery-Setup-2.5.0.exe", "size": 10, "browser_download_url": "https://github.com/x/y/releases/download/v2.5.0/a.exe", "digest": digest},
                {"name": "bad", "size": 0, "browser_download_url": "https://github.com/x"}, {"name": "evil", "size": 5, "browser_download_url": "https://evil.example/x"}]});
        let r = parse_release(&v, false).unwrap();
        assert_eq!((r.version.as_str(), r.name.as_str(), r.assets.len()), ("2.5.0", "Big", 1));
        assert_eq!(r.assets[0].sha256.as_deref(), Some("a".repeat(64).as_str()));
        assert_eq!(r.date.as_deref(), Some("2026-01-02T03:04:05Z"));
        assert!(parse_release(&json!({"tag_name": "v1.0.0", "draft": true}), false).is_none());
        assert!(parse_release(&json!({"tag_name": "nope"}), false).is_none());
        assert_eq!(parse_release(&json!({"tag_name": "v1.0.0", "html_url": "https://evil.example"}), false).unwrap().url, RELEASES_PAGE);
        assert_eq!(parse_digest(Some(&json!("sha256:xyz"))), None);
    }
}
