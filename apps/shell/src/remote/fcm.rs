use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde_json::Value;
use ureq::tls::{RootCerts, TlsConfig};
use zeroize::Zeroizing;

use super::config::now_secs;
use super::fcm_files::ServiceAccount;
use super::fcm_jwt::{assertion, TOKEN_URL};
use crate::state::lock;

const HTTP_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_BODY: u64 = 256 * 1024;
pub const ATTEMPTS: usize = 3;
pub const TOKEN_TTL: Duration = Duration::from_secs(55 * 60);
pub const RETRY_GAP: Duration = Duration::from_secs(2);

pub trait FcmHttp: Send + Sync {
    fn post(&self, url: &str, bearer: Option<&str>, content_type: &str, body: &[u8]) -> Option<(u16, String)>;
}

pub struct UreqFcm;

impl FcmHttp for UreqFcm {
    fn post(&self, url: &str, bearer: Option<&str>, content_type: &str, body: &[u8]) -> Option<(u16, String)> {
        let tls = TlsConfig::builder().root_certs(RootCerts::PlatformVerifier).build();
        let agent: ureq::Agent = ureq::Agent::config_builder().timeout_global(Some(HTTP_TIMEOUT)).http_status_as_error(false).tls_config(tls).build().into();
        let mut req = agent.post(url).header("content-type", content_type);
        if let Some(t) = bearer {
            req = req.header("authorization", &format!("Bearer {t}"));
        }
        let mut res = req.send(body).ok()?;
        let status = res.status().as_u16();
        let text = res.body_mut().with_config().limit(MAX_BODY).read_to_string().unwrap_or_default();
        Some((status, text))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Sent,
    Dead,
    Fail(&'static str),
}

pub struct Fcm {
    http: Mutex<Arc<dyn FcmHttp>>,
    account: Mutex<Option<ServiceAccount>>,
    token: Mutex<Option<(Zeroizing<String>, Instant)>>,
    last_error: Mutex<Option<&'static str>>,
    pub ttl: Mutex<Duration>,
    pub gap: Mutex<Duration>,
}

impl Default for Fcm {
    fn default() -> Self {
        Fcm {
            http: Mutex::new(Arc::new(UreqFcm)),
            account: Mutex::new(None),
            token: Mutex::new(None),
            last_error: Mutex::new(None),
            ttl: Mutex::new(TOKEN_TTL),
            gap: Mutex::new(RETRY_GAP),
        }
    }
}

fn error_code(body: &str) -> String {
    let v: Value = serde_json::from_str(body).unwrap_or(Value::Null);
    let details = v["error"]["details"].as_array().into_iter().flatten();
    details.filter_map(|d| d["errorCode"].as_str()).next().or_else(|| v["error"]["status"].as_str()).unwrap_or("").to_owned()
}

impl Fcm {
    pub fn set_http(&self, http: Arc<dyn FcmHttp>) {
        *lock(&self.http) = http;
    }

    pub fn set_account(&self, account: Option<ServiceAccount>) {
        *lock(&self.account) = account;
        *lock(&self.token) = None;
        self.set_error(None);
    }

    pub fn has_account(&self) -> bool {
        lock(&self.account).is_some()
    }

    pub fn last_error(&self) -> Option<&'static str> {
        *lock(&self.last_error)
    }

    pub fn set_error(&self, e: Option<&'static str>) {
        *lock(&self.last_error) = e;
    }

    fn http(&self) -> Arc<dyn FcmHttp> {
        lock(&self.http).clone()
    }

    fn access_token(&self, force: bool) -> Result<Zeroizing<String>, (&'static str, bool)> {
        {
            let mut slot = lock(&self.token);
            if force {
                *slot = None;
            }
            if let Some((t, at)) = slot.as_ref() {
                if at.elapsed() < *lock(&self.ttl) {
                    return Ok(t.clone());
                }
            }
        }
        let account = lock(&self.account).clone().ok_or(("not_configured", false))?;
        let jwt = Zeroizing::new(assertion(&account, now_secs()).ok_or(("auth", false))?);
        let form = format!("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion={}", jwt.as_str());
        let (status, body) = self.http().post(TOKEN_URL, None, "application/x-www-form-urlencoded", form.as_bytes()).ok_or(("offline", true))?;
        match status {
            200 => {
                let v: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
                let token = v["access_token"].as_str().filter(|t| !t.is_empty()).ok_or(("auth", false))?;
                let token = Zeroizing::new(token.to_owned());
                if lock(&self.account).as_ref().is_some_and(|a| a.client_email == account.client_email) {
                    *lock(&self.token) = Some((token.clone(), Instant::now()));
                }
                Ok(token)
            }
            429 => Err(("quota", true)),
            500..=599 => Err(("unavailable", true)),
            _ => Err(("auth", false)),
        }
    }

    pub fn send(&self, project_id: &str, message: &Value) -> Outcome {
        let url = format!("https://fcm.googleapis.com/v1/projects/{project_id}/messages:send");
        let body = message.to_string();
        let mut refreshed = false;
        let mut last = "unavailable";
        for attempt in 0..ATTEMPTS {
            if attempt > 0 {
                std::thread::sleep(*lock(&self.gap) * (1 << (attempt - 1)));
            }
            let token = match self.access_token(false) {
                Ok(t) => t,
                Err((code, retry)) => {
                    last = code;
                    if retry {
                        continue;
                    }
                    break;
                }
            };
            let Some((status, text)) = self.http().post(&url, Some(token.as_str()), "application/json; charset=UTF-8", body.as_bytes()) else {
                last = "offline";
                continue;
            };
            match status {
                200..=299 => {
                    self.set_error(None);
                    return Outcome::Sent;
                }
                401 if !refreshed => {
                    refreshed = true;
                    *lock(&self.token) = None;
                    last = "auth";
                    continue;
                }
                401 => {
                    last = "auth";
                    break;
                }
                404 => return Outcome::Dead,
                400 | 403 => {
                    if error_code(&text) == "UNREGISTERED" {
                        return Outcome::Dead;
                    }
                    last = if status == 403 { "denied" } else { "rejected" };
                    break;
                }
                429 => last = "quota",
                500..=599 => last = "unavailable",
                _ => {
                    last = "rejected";
                    break;
                }
            }
        }
        self.set_error(Some(last));
        Outcome::Fail(last)
    }
}
