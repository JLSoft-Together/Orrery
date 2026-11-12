use std::sync::{Arc, Mutex};
use std::time::Duration;

use ring::digest::{digest, SHA256};
use ureq::tls::{RootCerts, TlsConfig};

use super::fcm::{Outcome, ATTEMPTS, RETRY_GAP};
use super::webpush::{b64, Vapid};
use crate::state::lock;

const HTTP_TIMEOUT: Duration = Duration::from_secs(20);
pub const ENDPOINT_MAX: usize = 2048;
pub const TTL_SECS: u32 = 3600;
pub const EXACT_HOSTS: [&str; 3] = ["web.push.apple.com", "fcm.googleapis.com", "updates.push.services.mozilla.com"];
pub const SUFFIX_HOSTS: [&str; 1] = [".notify.windows.com"];

pub fn valid_endpoint(endpoint: &str) -> bool {
    if endpoint.len() > ENDPOINT_MAX || !endpoint.bytes().all(|b| b.is_ascii_graphic() && !matches!(b, b'"' | b'\\' | b'<' | b'>' | b'`')) {
        return false;
    }
    let Some(rest) = endpoint.strip_prefix("https://") else { return false };
    let end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
    let (authority, path) = rest.split_at(end);
    if !path.starts_with('/') || path.len() < 2 || path.contains('#') {
        return false;
    }
    let host = authority.strip_suffix(":443").unwrap_or(authority);
    known_host(host)
}

pub fn known_host(host: &str) -> bool {
    if host.is_empty() || !host.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'.' || b == b'-') || host.contains("..") {
        return false;
    }
    EXACT_HOSTS.contains(&host) || SUFFIX_HOSTS.iter().any(|s| host.strip_suffix(s).is_some_and(|label| !label.is_empty() && !label.starts_with('.') && !label.ends_with('.')))
}

pub fn topic(machine: &str, pane: &str, auth: &str) -> String {
    let hash = digest(&SHA256, format!("{machine}\0{pane}\0{auth}").as_bytes());
    b64(&hash.as_ref()[..24])
}

pub trait PushHttp: Send + Sync {
    fn post(&self, url: &str, headers: &[(&str, String)], body: &[u8]) -> Option<u16>;
}

pub struct UreqPush;

impl PushHttp for UreqPush {
    fn post(&self, url: &str, headers: &[(&str, String)], body: &[u8]) -> Option<u16> {
        let tls = TlsConfig::builder().root_certs(RootCerts::PlatformVerifier).build();
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(HTTP_TIMEOUT)).http_status_as_error(false).max_redirects(0).tls_config(tls).build().into();
        let mut req = agent.post(url);
        for (k, v) in headers {
            req = req.header(*k, v);
        }
        let res = req.send(body).ok()?;
        Some(res.status().as_u16())
    }
}

pub struct Delivery<'a> {
    pub endpoint: &'a str,
    pub subject: &'a str,
    pub topic: &'a str,
    pub urgency: &'static str,
    pub body: &'a [u8],
}

pub struct WebPush {
    http: Mutex<Arc<dyn PushHttp>>,
    vapid: Mutex<Option<Arc<Vapid>>>,
    pub gap: Mutex<Duration>,
}

impl Default for WebPush {
    fn default() -> Self {
        WebPush { http: Mutex::new(Arc::new(UreqPush)), vapid: Mutex::new(None), gap: Mutex::new(RETRY_GAP) }
    }
}

pub fn status_outcome(status: u16) -> Result<Outcome, &'static str> {
    match status {
        200..=299 => Ok(Outcome::Sent),
        404 | 410 => Ok(Outcome::Dead),
        429 => Err("quota"),
        500..=599 => Err("unavailable"),
        401 | 403 => Ok(Outcome::Fail("denied")),
        _ => Ok(Outcome::Fail("rejected")),
    }
}

impl WebPush {
    #[cfg(test)]
    pub fn set_http(&self, http: Arc<dyn PushHttp>) {
        *lock(&self.http) = http;
    }

    pub fn set_vapid(&self, vapid: Option<Vapid>) {
        *lock(&self.vapid) = vapid.map(Arc::new);
    }

    pub fn vapid(&self) -> Option<Arc<Vapid>> {
        lock(&self.vapid).clone()
    }

    pub fn public_key(&self) -> Option<String> {
        self.vapid().map(|v| v.public_key())
    }

    pub fn send(&self, d: &Delivery, now: u64, wanted: &dyn Fn() -> bool) -> Outcome {
        let Some(vapid) = self.vapid() else { return Outcome::Fail("not_configured") };
        let http = lock(&self.http).clone();
        let mut last = "unavailable";
        for attempt in 0..ATTEMPTS {
            if attempt > 0 {
                std::thread::sleep(*lock(&self.gap) * (1 << (attempt - 1)));
            }
            if !wanted() {
                return Outcome::Fail("no_device");
            }
            let Some(auth) = vapid.authorization(d.endpoint, d.subject, now) else { return Outcome::Fail("rejected") };
            let headers = [
                ("authorization", auth),
                ("content-encoding", "aes128gcm".to_owned()),
                ("content-type", "application/octet-stream".to_owned()),
                ("ttl", TTL_SECS.to_string()),
                ("urgency", d.urgency.to_owned()),
                ("topic", d.topic.to_owned()),
            ];
            let Some(status) = http.post(d.endpoint, &headers, d.body) else {
                last = "offline";
                continue;
            };
            match status_outcome(status) {
                Ok(outcome) => return outcome,
                Err(e) => last = e,
            }
        }
        Outcome::Fail(last)
    }
}
