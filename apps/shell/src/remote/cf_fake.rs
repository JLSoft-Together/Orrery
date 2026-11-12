use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{json, Value};

use crate::remote::cf_http::{Http, Req, Resp, API_BASE};
use crate::remote::test_util::*;
use crate::state::lock;

pub const TOKEN: &str = "tok_abcdefghijklmnopqrstuvwx";
pub const ACCOUNT: &str = "80786e06fd9be997d29bc5bc4de835e6";
pub const OTHER: &str = "11111111111111111111111111111111";

pub struct Cf {
    pub verify_status: u16,
    pub user_verify: bool,
    pub accounts: Vec<(String, String)>,
    pub accounts_status: u16,
    pub page_size: usize,
    pub subdomain: Option<String>,
    pub tag: Option<String>,
    pub offline_on: Option<&'static str>,
    pub upload_status: u16,
    pub health_ok: bool,
    pub delete_status: u16,
    pub log: Vec<(String, String, String, Vec<u8>, String)>,
}

impl Default for Cf {
    fn default() -> Self {
        Cf {
            verify_status: 200,
            user_verify: true,
            accounts: vec![(ACCOUNT.into(), "Jin".into())],
            accounts_status: 200,
            page_size: 50,
            subdomain: None,
            tag: None,
            offline_on: None,
            upload_status: 200,
            health_ok: true,
            delete_status: 200,
            log: Vec::new(),
        }
    }
}

pub type Shared = Arc<Mutex<Cf>>;

struct Fake {
    cf: Shared,
    base: String,
    token: String,
}

pub fn ok(result: Value) -> Option<Resp> {
    Some(Resp { status: 200, body: json!({ "success": true, "errors": [], "result": result }).to_string() })
}

pub fn bad(status: u16) -> Option<Resp> {
    Some(Resp { status, body: json!({ "success": false, "errors": [{ "code": 10000, "message": "nope" }] }).to_string() })
}

impl Http for Fake {
    fn call(&self, req: Req) -> Option<Resp> {
        let mut cf = lock(&self.cf);
        cf.log.push((self.base.clone(), req.method.into(), req.path.clone(), req.body.clone(), self.token.clone()));
        if self.base != API_BASE {
            return Some(Resp { status: if cf.health_ok { 200 } else { 530 }, body: "ok".into() });
        }
        let key = match (req.method, req.path.as_str()) {
            ("PUT", p) if p.contains("/workers/scripts/") => "upload",
            ("POST", _) => "enable",
            _ => "",
        };
        if cf.offline_on == Some(key) && !key.is_empty() {
            return None;
        }
        match (req.method, req.path.as_str()) {
            ("GET", "/user/tokens/verify") if cf.user_verify => {
                if cf.verify_status == 200 {
                    ok(json!({ "id": "x", "status": "active" }))
                } else {
                    bad(cf.verify_status)
                }
            }
            ("GET", "/user/tokens/verify") => bad(400),
            ("GET", p) if p.starts_with("/accounts?") => {
                if cf.accounts_status != 200 {
                    return bad(cf.accounts_status);
                }
                let page: usize = p.split("&page=").nth(1).and_then(|s| s.parse().ok()).unwrap_or(1);
                let total = cf.accounts.len().div_ceil(cf.page_size).max(1);
                let rows: Vec<Value> = cf.accounts.iter().skip((page - 1) * cf.page_size).take(cf.page_size).map(|(id, name)| json!({ "id": id, "name": name })).collect();
                Some(Resp { status: 200, body: json!({ "success": true, "result": rows, "result_info": { "page": page, "total_pages": total } }).to_string() })
            }
            ("GET", p) if p.ends_with("/workers/subdomain") => match &cf.subdomain {
                Some(s) => ok(json!({ "subdomain": s })),
                None => bad(404),
            },
            ("PUT", p) if p.ends_with("/workers/subdomain") => {
                let name = serde_json::from_slice::<Value>(&req.body).ok()?["subdomain"].as_str()?.to_owned();
                cf.subdomain = Some(name.clone());
                ok(json!({ "subdomain": name }))
            }
            ("GET", p) if p.ends_with("/workers/scripts") => ok(match &cf.tag {
                Some(t) => json!([{ "id": "other" }, { "id": "orrery-relay", "migration_tag": t }]),
                None => json!([{ "id": "other" }]),
            }),
            ("PUT", p) if p.contains("/workers/scripts/orrery-relay") => {
                if cf.upload_status != 200 {
                    return bad(cf.upload_status);
                }
                cf.tag = Some("v1".into());
                ok(json!({ "id": "orrery-relay" }))
            }
            ("POST", p) if p.ends_with("/orrery-relay/subdomain") => ok(json!({ "enabled": true, "previews_enabled": false })),
            ("DELETE", _) => {
                if cf.delete_status == 200 {
                    ok(Value::Null)
                } else {
                    bad(cf.delete_status)
                }
            }
            _ => bad(404),
        }
    }
}

pub fn setup(tag: &str, cf: Cf) -> (Rig, Shared) {
    let rig = rig(tag, "", false);
    let shared: Shared = Arc::new(Mutex::new(cf));
    let s = shared.clone();
    {
        let mut env = lock(&rig.remote.setup.env);
        env.factory = Some(Box::new(move |base, token| Box::new(Fake { cf: s.clone(), base: base.into(), token: token.into() })));
        env.health_wait = Duration::from_millis(30);
        env.health_gap = Duration::from_millis(5);
    }
    (rig, shared)
}

pub fn calls(cf: &Shared) -> Vec<(String, String)> {
    lock(cf).log.iter().map(|(_, m, p, _, _)| (m.clone(), p.clone())).collect()
}

pub fn token_ok(rig: &Rig) {
    assert_eq!(rig.remote.relay_token(&json!(TOKEN))["ok"], true);
}

pub fn metadata_of(body: &[u8]) -> Value {
    let text = String::from_utf8_lossy(body).to_string();
    let at = text.find("name=\"metadata\"").unwrap();
    let rest = &text[at..];
    let start = rest.find("\r\n\r\n").unwrap() + 4;
    let end = rest[start..].find("\r\n--").unwrap();
    serde_json::from_str(&rest[start..start + end]).unwrap()
}

pub fn uploads(cf: &Shared) -> Vec<Vec<u8>> {
    lock(cf).log.iter().filter(|(_, m, p, _, _)| m == "PUT" && p.contains("/workers/scripts/")).map(|(_, _, _, b, _)| b.clone()).collect()
}
