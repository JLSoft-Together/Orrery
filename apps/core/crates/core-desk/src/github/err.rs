use super::api::{scrub_token, ApiRes, NetErr};
use serde_json::{json, Map, Value};

const DETAIL_MAX: usize = 300;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GhErr {
    pub code: &'static str,
    pub detail: Option<String>,
    pub permission: Option<String>,
}

impl GhErr {
    pub fn new(code: &'static str) -> Self {
        GhErr { code, detail: None, permission: None }
    }

    pub fn with_detail(code: &'static str, detail: impl Into<String>) -> Self {
        GhErr { code, detail: Some(detail.into()), permission: None }
    }

    pub fn value(&self) -> Value {
        let mut m = Map::new();
        m.insert("ok".into(), json!(false));
        m.insert("error".into(), json!(self.code));
        if let Some(d) = &self.detail {
            m.insert("detail".into(), json!(d));
        }
        if let Some(p) = &self.permission {
            m.insert("permission".into(), json!(p));
        }
        Value::Object(m)
    }
}

impl From<NetErr> for GhErr {
    fn from(e: NetErr) -> Self {
        GhErr::new(e.code())
    }
}

pub fn clean(text: &str, token: &str) -> String {
    let sys = crate::claude::Sys::from_env();
    let masked = core_transcript::mask::mask_secrets(&scrub_token(text, token), &sys.home, sys.platform());
    let flat: String = masked.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
    crate::js::slice(flat.trim(), DETAIL_MAX)
}

pub fn clean_line(line: &str, token: &str) -> String {
    let sys = crate::claude::Sys::from_env();
    let masked = core_transcript::mask::mask_secrets(&scrub_token(line, token), &sys.home, sys.platform());
    masked.chars().filter(|c| *c == '\t' || !c.is_control()).collect()
}

pub fn message_of(res: &ApiRes, token: &str) -> Option<String> {
    let msg = res.body.get("message").and_then(Value::as_str).filter(|m| !m.trim().is_empty())?;
    let extra = res.body.get("errors").and_then(Value::as_array).and_then(|a| a.first()).and_then(|e| e.get("message").and_then(Value::as_str).or_else(|| e.get("code").and_then(Value::as_str)));
    let text = match extra {
        Some(x) if !x.is_empty() => format!("{msg}: {x}"),
        _ => msg.to_string(),
    };
    Some(clean(&text, token))
}

pub fn from_status(res: &ApiRes, token: &str, need: &str) -> GhErr {
    let detail = message_of(res, token);
    let code = match res.status {
        401 => "gh.auth",
        403 | 429 if detail.as_deref().is_some_and(|d| d.to_lowercase().contains("rate limit")) => "gh.rateLimit",
        403 => "gh.scope",
        404 => "gh.notFound",
        405 => "gh.notMergeable",
        409 => "gh.conflict",
        422 => "gh.invalid",
        _ => "gh.http",
    };
    let permission = (code == "gh.scope").then(|| res.accepted.as_deref().map(|a| clean(a, token)).filter(|a| !a.is_empty()).unwrap_or_else(|| need.to_string()));
    let detail = detail.or_else(|| (code == "gh.http").then(|| format!("HTTP {}", res.status)));
    GhErr { code, detail, permission }
}

pub fn ok_or(res: ApiRes, token: &str, need: &str) -> Result<ApiRes, GhErr> {
    if (200..300).contains(&res.status) {
        Ok(res)
    } else {
        Err(from_status(&res, token, need))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn res(status: u16, body: Value, accepted: Option<&str>) -> ApiRes {
        ApiRes { status, body, scopes: None, accepted: accepted.map(str::to_string) }
    }

    #[test]
    fn status_codes_map_and_hide_the_token() {
        let tok = "ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ0123";
        let e = from_status(&res(403, json!({"message": format!("Resource not accessible by {tok}")}), Some("pull_requests=write")), tok, "Pull requests");
        assert_eq!(e.code, "gh.scope");
        assert_eq!(e.permission.as_deref(), Some("pull_requests=write"));
        assert!(!e.value().to_string().contains(tok));
        let e = from_status(&res(403, json!({"message": "x"}), None), tok, "Contents");
        assert_eq!(e.permission.as_deref(), Some("Contents"));
        assert_eq!(from_status(&res(403, json!({"message": "API rate limit exceeded"}), None), tok, "").code, "gh.rateLimit");
        assert_eq!(from_status(&res(405, json!({"message": "Pull Request is not mergeable"}), None), tok, "").code, "gh.notMergeable");
        assert_eq!(from_status(&res(409, json!({"message": "Head branch was modified"}), None), tok, "").code, "gh.conflict");
        let inv = from_status(&res(422, json!({"message": "Validation Failed", "errors": [{"message": "A pull request already exists"}]}), None), tok, "");
        assert_eq!(inv.detail.as_deref(), Some("Validation Failed: A pull request already exists"));
        assert_eq!(from_status(&res(500, Value::Null, None), tok, "").detail.as_deref(), Some("HTTP 500"));
    }
}
