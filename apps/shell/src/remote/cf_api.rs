use serde_json::{json, Value};

use super::cf_http::{Http, Multipart, Req, Resp};

pub const SCRIPT: &str = "orrery-relay";
const SUBDOMAIN_TRIES: usize = 3;

pub type Res<T> = Result<T, &'static str>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub id: String,
    pub name: String,
}

pub struct Bundle<'a> {
    pub script: &'a str,
    pub meta: &'a Value,
}

pub fn valid_subdomain(s: &str) -> bool {
    let b = s.as_bytes();
    (1..=63).contains(&b.len())
        && b.iter().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'-')
        && b[0] != b'-'
        && b[b.len() - 1] != b'-'
}

pub fn valid_account(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|b| b.is_ascii_hexdigit())
}

fn fail(status: u16) -> &'static str {
    match status {
        401 => "token_invalid",
        403 => "token_scope",
        429 => "rate_limited",
        _ => "rejected",
    }
}

fn success(resp: &Resp) -> Option<Value> {
    if !(200..300).contains(&resp.status) {
        return None;
    }
    let v: Value = serde_json::from_str(&resp.body).ok()?;
    (v.get("success").and_then(Value::as_bool) == Some(true)).then_some(v)
}

fn send(http: &dyn Http, req: Req) -> Res<Resp> {
    http.call(req).ok_or("offline")
}

fn result_of(http: &dyn Http, req: Req) -> Res<Value> {
    let resp = send(http, req)?;
    success(&resp).map(|v| v["result"].clone()).ok_or_else(|| fail(resp.status))
}

pub fn verify(http: &dyn Http) -> Res<()> {
    let resp = send(http, Req::get("/user/tokens/verify"))?;
    if let Some(v) = success(&resp) {
        return match v["result"]["status"].as_str() {
            Some("active") => Ok(()),
            _ => Err("token_invalid"),
        };
    }
    let probe = send(http, Req::get("/accounts?per_page=1"))?;
    if success(&probe).is_some() {
        Ok(())
    } else {
        Err(fail(probe.status))
    }
}

const ACCOUNT_PAGES: u64 = 10;

pub fn accounts(http: &dyn Http) -> Res<Vec<Account>> {
    let mut out = Vec::new();
    let mut page = 1;
    loop {
        let resp = send(http, Req::get(format!("/accounts?per_page=50&page={page}")))?;
        let v = success(&resp).ok_or_else(|| fail(resp.status))?;
        let list = v["result"].as_array().ok_or("rejected")?;
        out.extend(list.iter().filter_map(|a| {
            let id = a["id"].as_str().filter(|i| valid_account(i))?;
            Some(Account { id: id.to_owned(), name: a["name"].as_str().unwrap_or(id).chars().filter(|c| !c.is_control()).take(80).collect() })
        }));
        let total = v["result_info"]["total_pages"].as_u64().unwrap_or(1).min(ACCOUNT_PAGES);
        if page >= total {
            return Ok(out);
        }
        page += 1;
    }
}

pub fn subdomain(http: &dyn Http, account: &str) -> Res<Option<String>> {
    let resp = send(http, Req::get(format!("/accounts/{account}/workers/subdomain")))?;
    if let Some(v) = success(&resp) {
        return match v["result"]["subdomain"].as_str().filter(|s| !s.is_empty()) {
            Some(s) if valid_subdomain(s) => Ok(Some(s.to_owned())),
            Some(_) => Err("subdomain"),
            None => Ok(None),
        };
    }
    if matches!(resp.status, 401 | 403 | 429) {
        return Err(fail(resp.status));
    }
    if resp.status == 404 || resp.body.contains("10007") {
        return Ok(None);
    }
    Err("rejected")
}

pub fn candidate_subdomain() -> Option<String> {
    let mut raw = [0u8; 3];
    getrandom::fill(&mut raw).ok()?;
    Some(format!("orrery-{}", raw.iter().map(|b| format!("{b:02x}")).collect::<String>()))
}

pub fn create_subdomain(http: &dyn Http, account: &str) -> Res<String> {
    for _ in 0..SUBDOMAIN_TRIES {
        let name = candidate_subdomain().ok_or("subdomain")?;
        let resp = send(http, Req::json("PUT", format!("/accounts/{account}/workers/subdomain"), &json!({ "subdomain": name })))?;
        if success(&resp).is_some() {
            return Ok(name);
        }
        if matches!(resp.status, 401 | 403 | 429) {
            return Err(fail(resp.status));
        }
    }
    Err("subdomain")
}

pub fn migration_tag(http: &dyn Http, account: &str) -> Res<Option<String>> {
    let result = result_of(http, Req::get(format!("/accounts/{account}/workers/scripts")))?;
    let list = result.as_array().ok_or("rejected")?;
    Ok(list.iter().find(|s| s["id"].as_str() == Some(SCRIPT)).and_then(|s| s["migration_tag"].as_str()).map(str::to_owned))
}

pub fn migrations(current: Option<&str>, bundle: &Value) -> Res<Option<Value>> {
    let list = bundle["migrations"].as_array().filter(|l| !l.is_empty()).ok_or("bundle")?;
    let tags: Vec<&str> = list.iter().filter_map(|m| m["tag"].as_str()).collect();
    let last = *tags.last().ok_or("bundle")?;
    let steps = |from: usize| -> Vec<Value> { list[from..].iter().map(|m| json!({ "new_sqlite_classes": m["new_sqlite_classes"] })).collect() };
    match current {
        Some(t) if t == last => Ok(None),
        Some(t) => {
            let at = tags.iter().position(|x| *x == t).ok_or("migration")?;
            Ok(Some(json!({ "old_tag": t, "new_tag": last, "steps": steps(at + 1) })))
        }
        None if list.len() == 1 => Ok(Some(json!({ "new_tag": last, "new_sqlite_classes": list[0]["new_sqlite_classes"] }))),
        None => Ok(Some(json!({ "new_tag": last, "steps": steps(0) }))),
    }
}

pub fn metadata(bundle: &Value, current: Option<&str>) -> Res<Value> {
    let mut meta = json!({
        "main_module": bundle["mainModule"],
        "compatibility_date": bundle["compatibilityDate"],
        "bindings": bundle["bindings"],
    });
    if let Some(m) = migrations(current, bundle)? {
        meta["migrations"] = m;
    }
    Ok(meta)
}

pub fn upload(http: &dyn Http, account: &str, bundle: &Bundle, current: Option<&str>) -> Res<()> {
    let meta = metadata(bundle.meta, current)?;
    let main = bundle.meta["mainModule"].as_str().unwrap_or("worker.js");
    let mut form = Multipart::new().ok_or("upload")?;
    form.part("metadata", None, "application/json", meta.to_string().as_bytes());
    form.part(main, Some(main), "application/javascript+module", bundle.script.as_bytes());
    let (content_type, body) = form.finish();
    let resp = send(http, Req { method: "PUT", path: format!("/accounts/{account}/workers/scripts/{SCRIPT}"), content_type, body })?;
    if success(&resp).is_some() {
        Ok(())
    } else if matches!(resp.status, 401 | 403 | 429) {
        Err(fail(resp.status))
    } else {
        Err("upload")
    }
}

pub fn enable_workers_dev(http: &dyn Http, account: &str) -> Res<()> {
    let body = json!({ "enabled": true, "previews_enabled": false });
    let resp = send(http, Req::json("POST", format!("/accounts/{account}/workers/scripts/{SCRIPT}/subdomain"), &body))?;
    if success(&resp).is_some() {
        Ok(())
    } else if matches!(resp.status, 401 | 403 | 429) {
        Err(fail(resp.status))
    } else {
        Err("enable")
    }
}

pub fn delete_script(http: &dyn Http, account: &str) -> Res<()> {
    let resp = send(http, Req { method: "DELETE", path: format!("/accounts/{account}/workers/scripts/{SCRIPT}?force=true"), content_type: String::new(), body: Vec::new() })?;
    if success(&resp).is_some() || resp.status == 404 {
        Ok(())
    } else {
        Err(fail(resp.status))
    }
}

pub fn worker_url(subdomain: &str) -> Res<String> {
    if valid_subdomain(subdomain) {
        Ok(format!("https://{SCRIPT}.{subdomain}.workers.dev"))
    } else {
        Err("subdomain")
    }
}
