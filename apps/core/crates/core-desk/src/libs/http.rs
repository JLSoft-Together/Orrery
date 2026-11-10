use std::io::Read;
use std::time::Duration;
use ureq::tls::{RootCerts, TlsConfig};

#[derive(Debug, Clone, PartialEq)]
pub enum Fetched {
    Text(String),
    Status(u16),
    Error(&'static str),
}

pub trait Fetch: Send + Sync {
    fn fetch(&self, url: &str, accept: &str, body: Option<&str>) -> Fetched;
}

const READ_LIMIT: u64 = 128 * 1024 * 1024;

pub struct UreqFetch {
    agent: ureq::Agent,
}

impl UreqFetch {
    pub fn new(timeout: Duration) -> Self {
        let tls = TlsConfig::builder().root_certs(RootCerts::PlatformVerifier).build();
        let config = ureq::Agent::config_builder().timeout_global(Some(timeout)).http_status_as_error(false).tls_config(tls).build();
        UreqFetch { agent: config.into() }
    }
}

fn error_of(e: &ureq::Error) -> &'static str {
    match e {
        ureq::Error::Timeout(_) => "timeout",
        ureq::Error::Io(io) if io.kind() == std::io::ErrorKind::TimedOut => "timeout",
        _ => "network",
    }
}

impl Fetch for UreqFetch {
    fn fetch(&self, url: &str, accept: &str, body: Option<&str>) -> Fetched {
        let res = match body {
            None => self.agent.get(url).header("accept", accept).call(),
            Some(b) => self.agent.post(url).header("accept", accept).header("content-type", "application/json").send(b),
        };
        let mut res = match res {
            Ok(r) => r,
            Err(e) => return Fetched::Error(error_of(&e)),
        };
        let status = res.status().as_u16();
        if !(200..300).contains(&status) {
            return Fetched::Status(status);
        }
        let mut bytes = Vec::new();
        let mut reader = res.body_mut().with_config().limit(READ_LIMIT).reader();
        match reader.read_to_end(&mut bytes) {
            Ok(_) => Fetched::Text(String::from_utf8_lossy(&bytes).into_owned()),
            Err(e) if e.kind() == std::io::ErrorKind::TimedOut => Fetched::Error("timeout"),
            Err(e) if e.to_string().to_lowercase().contains("limit") => Fetched::Status(413),
            Err(e) if e.to_string().to_lowercase().contains("timeout") => Fetched::Error("timeout"),
            Err(_) => Fetched::Error("network"),
        }
    }
}
