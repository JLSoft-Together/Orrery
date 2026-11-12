use std::fmt;

use serde_json::Value;
use zeroize::Zeroizing;

use super::config::FirebaseConfig;

pub const PACKAGE: &str = "dev.ccm.orrery.remote";
pub const MAX_FILE: usize = 100 * 1024;
const PEM_BEGIN: &str = "-----BEGIN PRIVATE KEY-----";
const PEM_END: &str = "-----END PRIVATE KEY-----";

#[derive(Clone)]
pub struct ServiceAccount {
    pub client_email: String,
    pub private_key: Zeroizing<String>,
}

impl fmt::Debug for ServiceAccount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ServiceAccount(..)")
    }
}

pub struct Parsed {
    pub firebase: FirebaseConfig,
    pub account: ServiceAccount,
}

pub fn pem_der(pem: &str) -> Option<Vec<u8>> {
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    let start = pem.find(PEM_BEGIN)? + PEM_BEGIN.len();
    let end = pem.find(PEM_END)?;
    if end < start {
        return None;
    }
    let body: String = pem[start..end].chars().filter(|c| !c.is_whitespace()).collect();
    STANDARD.decode(body).ok().filter(|d| !d.is_empty())
}

fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or("")
}

fn safe(s: &str, min: usize, max: usize, extra: &[u8]) -> bool {
    (min..=max).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric() || extra.contains(&b))
}

fn project_id_ok(s: &str) -> bool {
    (4..=64).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn json(raw: &str) -> Option<Value> {
    (raw.len() <= MAX_FILE).then(|| serde_json::from_str::<Value>(raw).ok()).flatten().filter(Value::is_object)
}

fn service_account(raw: &str) -> Result<(ServiceAccount, String), &'static str> {
    let v = json(raw).ok_or("bad_service_account")?;
    if text(&v, "type") != "service_account" {
        return Err("bad_service_account");
    }
    let email = text(&v, "client_email");
    let key = text(&v, "private_key");
    let project = text(&v, "project_id");
    if !safe(email, 8, 200, b"@._-") || !email.contains('@') || !project_id_ok(project) {
        return Err("bad_service_account");
    }
    let der = Zeroizing::new(pem_der(key).ok_or("bad_service_account")?);
    ring::signature::RsaKeyPair::from_pkcs8(&der).map_err(|_| "bad_service_account")?;
    Ok((ServiceAccount { client_email: email.to_owned(), private_key: Zeroizing::new(key.to_owned()) }, project.to_owned()))
}

fn android_client(v: &Value) -> Result<(String, String), &'static str> {
    let client = v
        .get("client")
        .and_then(Value::as_array)
        .and_then(|list| list.iter().find(|c| c["client_info"]["android_client_info"]["package_name"].as_str() == Some(PACKAGE)))
        .ok_or("no_android_client")?;
    let app_id = text(&client["client_info"], "mobilesdk_app_id");
    let key = client["api_key"].as_array().and_then(|k| k.iter().map(|k| text(k, "current_key")).find(|k| !k.is_empty())).unwrap_or("");
    if !safe(app_id, 8, 128, b":_-") || !safe(key, 16, 128, b"_-") {
        return Err("no_android_client");
    }
    Ok((app_id.to_owned(), key.to_owned()))
}

pub fn parse(google_services: &str, service_account_json: &str) -> Result<Parsed, &'static str> {
    let gs = json(google_services).ok_or("bad_json")?;
    let info = &gs["project_info"];
    let project = text(info, "project_id");
    let number = text(info, "project_number");
    if !project_id_ok(project) || !safe(number, 4, 24, b"") || !number.bytes().all(|b| b.is_ascii_digit()) {
        return Err("bad_json");
    }
    let (account, sa_project) = service_account(service_account_json)?;
    if sa_project != project {
        return Err("mismatch_project");
    }
    let (app_id, api_key) = android_client(&gs)?;
    let firebase = FirebaseConfig { project_id: project.to_owned(), api_key, app_id, sender_id: number.to_owned(), client_email: account.client_email.clone() };
    Ok(Parsed { firebase, account })
}

pub fn stored(account: &ServiceAccount) -> String {
    serde_json::json!({ "client_email": account.client_email, "private_key": account.private_key.as_str() }).to_string()
}

pub fn restore(raw: &str) -> Option<ServiceAccount> {
    let v: Value = serde_json::from_str(raw).ok()?;
    let (email, key) = (text(&v, "client_email"), text(&v, "private_key"));
    (!email.is_empty() && pem_der(key).is_some()).then(|| ServiceAccount { client_email: email.to_owned(), private_key: Zeroizing::new(key.to_owned()) })
}
