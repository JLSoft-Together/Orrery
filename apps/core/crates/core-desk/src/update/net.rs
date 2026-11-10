use std::time::Duration;
use ureq::tls::{RootCerts, TlsConfig};

pub fn agent(global: Option<Duration>, follow: u32) -> ureq::Agent {
    let tls = TlsConfig::builder().root_certs(RootCerts::PlatformVerifier).build();
    let config = ureq::Agent::config_builder()
        .timeout_global(global)
        .timeout_connect(Some(Duration::from_secs(15)))
        .timeout_recv_response(Some(Duration::from_secs(30)))
        .max_redirects(follow)
        .max_redirects_will_error(false)
        .http_status_as_error(false)
        .tls_config(tls)
        .build();
    config.into()
}

pub fn is_timeout(e: &ureq::Error) -> bool {
    matches!(e, ureq::Error::Timeout(_))
}
