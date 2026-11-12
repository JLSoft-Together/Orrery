use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine;
use ring::signature::{UnparsedPublicKey, RSA_PKCS1_2048_8192_SHA256};
use serde_json::{json, Value};

use super::fcm::{Fcm, FcmHttp, Outcome};
use super::fcm_files::{parse, pem_der, restore, stored, ServiceAccount, PACKAGE};
use super::fcm_jwt::{assertion, SCOPE, TOKEN_URL};
use crate::state::lock;

pub const KEY_A: &str = "MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQDBfrCq60xBFVm8n8YaF+wemuvwBmuBbiv7a7qsdqE3VO/MYunKzlQBa8SFSKIn696ZrESm5Ds7aTGZbv1PvFnAdux/TWRk95nsGU3UKDvz2gcqQWCEgn04yl1W0htrJbxRl2GjlanpNQN56ryW1pEzIHOF24GktGmNeCQw8H3qXny2VPv8NfLXL/lRonJ87Td+TjmbLNR6o8qoVlmsM58L4o5tUvZgAzHaYY0IFuipXlAPitc8C/W8F+StzOTc0KLALE958EcuQj1v64UaBAatVV8T6zv3rRHB25xZUKNRrW1UfOO6HkHa5YSud2UWmA4NBQaQDO3b/ISW9nQAkqVFAgMBAAECggEABoxtGjvAZut8EAAwSi3JFfgUSBlPhh/eaTHqLXAOYLsFKpumYfC9sxacng4pPsSi0fycjjFnN8mFnjW9aPds6jCcLGrIHQqsMOtiUO3TqZsVZv6zb4ijCEsBAbE2sFekmfZPnaOxDm124qdpHonAfER65GrbZmGcuOb6ApYBDO96o2qJBsTzJ9uC6nEkbX72Dodj29HEJIPE1cKGaAkVXjCJq4nTgm9kboPTIacJCVVoBOkoMMD215susOkrC2YJrhvfOhgSkekn3C1wgErkwvd+ZkX1PfFWei5tgaItXid4Dg/KI2pOyn7qHGnAwVAmEcsHQbuKop4wktWEn57bgQKBgQDjs42d+tP50CugRVZvSshbxd+zy7VUUGKcoH4fNGBhxTd48nM/XAaL3im/LXH+940QjPJ3ySSokb0cJ29DS0x4zh/mIGERwg0ODGETb+4txX+HP6vaCcHNMREx5Y+luPzK4DNLSwkq1oM52a2h59rPLEfqfGSPFHdZ4pvEDF338QKBgQDZitaNLT6pwbA7vKahtVSxiSx1SAICGEJot9FItyJBSMG456gxVHILkJ4bPs8TqiYKYXMUqt0w7/MaOyDs/Ls1s4r83BBm0vZsaD0I5QTf2M0k9MErdoIwE5Jbq+JYvUv2/s7/P7V/uaBRX06XM0Bu5ibxg8DL8pd/EsG0NVy2lQKBgDvkQH2rOhJdVPxFAKUpUGYDOAH8tOdGdYmpFUNz7V4KF7syWmnx9K7ZcrWcaH2d83bE/AnR2FiWbN146eUTazvjS7SmE6IuHmzxiJ8qM8XKSu7yKmuhYV5tZ45uSUhXzfY23ocRZ4ni/oU64lbRu1X+2bOtoG2Z/oKf/kSk1BqhAoGBAM+5PiOyZ1qLg2+XNPnGf2fDFzuqFZjQd2Es3N91V3VFNw4LzlPEUd3vKrnTYMJjP028PbonaWyYQubyPP9uWIjvtzC6JcM5gGqFonpCzUO97i1ytJ1+3NNxbNiyKUEZCFeCej/LItEifA4XVqEUJFuA0wDdERTtwXFqhKJCLhR9AoGBAJoaWmvFtHyjZPqeSjlOvIP/8aVJSOSZIG70tyZ3I4ZEprqjmF3s2RECnEEfmW8xGYoCNQ4+tGI7cTBPgmveGk16Jt8xjMXXVdbPJUgbQ5dl8uw9cAM15xPsQ8Fi4j7gg6yDeSM7kSXm4hEYHJGpU753KSBPcgToegkZ92r3hHqS";
pub const KEY_B: &str = "MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQDKsIWDpNPAHoURGLtG4KIHBvQ4A34f3QnAT5hh5lV8w44QdVIivwoRf/QL6b6pLGzlZoBsAJR6xaP7huIIj1Zst8ypW5Ps5HOkxbknjruATonaQpWBpluqvx5F4yRhUJx+kI9clO/LP96ra7TKWEnhWXVOC6SerNX2/4PewC+BUsluydWCbK/ltaTNYYcq+XlGOts1vyUIBZBe+9Rj74pv5N1TS9A9mbddW+q9jYgLUunJpZENDMk7JyAUz0bDan4mt0KrPxXTSQ2UE2XajZHXpxJ6PtYBHApWu0BzP1FZFO0Ffc05ECfY9Ix2sq4e9ShukUhr89u1EIkQLMy1H7ZBAgMBAAECggEAAmUgYfQ9crSDN51MzGmbGiCOG86IW8s6ckTk6BAKTCi2mDsxEHgLBmnpce3Kv8HNB5wbQfg/yObM9gvoX9frWlC4E/tt1z7COKnQpkcDFpQ9Zl08pwUjacEYYOFuOWtabgrUDT2bsV5WPNa1xcfrsSrKPnULKbs6smoShNWzugBBa8wYcjiX1Gvd3vcfO5yGs51b6wlqiQ0geLcKlF6CZ8VaNINbH6qWsqzl3aa+u7NhjkLJx7kwQpN7Pl9z4Ljgcddbr3X1L5WUAs6ZS4qb5HSTDtKOERX+W3MKLk7wmZXcN8VqBFUqTnpHk2QJeH2oZX7TiVQw41hlrNTO5A3uAQKBgQD6utrpS9JMITXvLXDnETCxvmuO1KPO/C0OLGIxyofG4srP1Nc6yuVCUeSUPduFtjcesxIJKV08+qzEotd4e6dzXGLH2dTQWdljSVVJrWGTfMdvMA/3KFOmWc3/hHE/qKugD/T+XG7sIaWB6SEklzUnFcY4toCxV31KAYkjT2Bo3QKBgQDO8yrgE1m2NYZbTNW3hxZLtyDoSBIW0T8lSpAVTneCAQQ+ET7A4PLh2Z3FQmcJ/JukdnZtJEqHTg0PuYoaY0fyFxbiWVWxqKeZxJczRUUKTPeZtDp6NIgLtZbDqSUsfZ1VJafz1o2VfKswWRGEVLt7A+SjOkzg/4fNMlByogm6tQKBgFIBaXYiiW7C8fv/dcztj2qjKFS38bfSPP5Ev/0fC9ed982QBLFDCUbut/9QWAtctJyC3yRF9ivqpTVPOYA7mWIe1mQdOADJUlOp+Y42ehKeHjCcnKwu8WUYPEsVS2ERzsh0T6IeI11yB8r8sA1is0zMT1/fOSpHvj/FTtBQtuy1AoGBAJ+nKvmJFBmke01uHiYi2gl5crKf/wyJSty7BCLb0QiDPkGfzjjAkb+4waVPT+x9a5laJoyWhcSSazMpYShwVS2nsGjYoGqZgQr4hejw1I1YyV9TkhV690WJRpEVqW8ETV9F2Kcda0ba0bpPzFq5f5UscLOYI30kxSCBY1y8MyNNAoGBAKF+97pRFRVJs3wmw0gvYWOdFV2LvaptoN8CGa8JJ/iXVDSuLVhilE8hsRuj895oC3FL8ZTgUBOMuP0eQumgrcjX0ISO9piY2iFWJnKb34o3xhHTg5NDZClzpjeHRV4m1ZMZXXpLlq+W1kHNR2LK0/j0qK+qSy0T+da11tQLM544";
const PUB_A: &str = "MIIBCgKCAQEAwX6wqutMQRVZvJ/GGhfsHprr8AZrgW4r+2u6rHahN1TvzGLpys5UAWvEhUiiJ+vemaxEpuQ7O2kxmW79T7xZwHbsf01kZPeZ7BlN1Cg789oHKkFghIJ9OMpdVtIbayW8UZdho5Wp6TUDeeq8ltaRMyBzhduBpLRpjXgkMPB96l58tlT7/DXy1y/5UaJyfO03fk45myzUeqPKqFZZrDOfC+KObVL2YAMx2mGNCBboqV5QD4rXPAv1vBfkrczk3NCiwCxPefBHLkI9b+uFGgQGrVVfE+s7960RwducWVCjUa1tVHzjuh5B2uWErndlFpgODQUGkAzt2/yElvZ0AJKlRQIDAQAB";

pub fn pem(b64: &str) -> String {
    let lines: Vec<&str> = b64.as_bytes().chunks(64).map(|c| std::str::from_utf8(c).unwrap()).collect();
    format!("-----BEGIN PRIVATE KEY-----\n{}\n-----END PRIVATE KEY-----\n", lines.join("\n"))
}

pub fn sa_json(project: &str, key_b64: &str) -> String {
    json!({
        "type": "service_account",
        "project_id": project,
        "private_key_id": "abc",
        "private_key": pem(key_b64),
        "client_email": format!("firebase-adminsdk-x@{project}.iam.gserviceaccount.com"),
        "token_uri": "https://evil.example/token",
    })
    .to_string()
}

pub fn gs_json(project: &str, package: &str) -> String {
    json!({
        "project_info": { "project_number": "123456789012", "project_id": project, "storage_bucket": "x" },
        "client": [
            { "client_info": { "mobilesdk_app_id": "1:123456789012:android:other", "android_client_info": { "package_name": "com.other.app" } }, "api_key": [{ "current_key": "OtherKeyOtherKeyOtherKey" }] },
            { "client_info": { "mobilesdk_app_id": "1:123456789012:android:feedbeef", "android_client_info": { "package_name": package } }, "api_key": [{ "current_key": "AIzaSyTestKey_0123456789-abc" }] }
        ],
        "configuration_version": "1"
    })
    .to_string()
}

pub fn account() -> ServiceAccount {
    parse(&gs_json("demo-proj", PACKAGE), &sa_json("demo-proj", KEY_A)).unwrap().account
}

pub type Call = (String, Option<String>, String, String);
pub type Handler = Box<dyn Fn(&Call) -> Option<(u16, String)> + Send + Sync>;

pub struct Fake {
    pub calls: Mutex<Vec<Call>>,
    pub handler: Handler,
}

impl Fake {
    pub fn new(handler: Handler) -> Arc<Fake> {
        Arc::new(Fake { calls: Mutex::new(Vec::new()), handler })
    }

    pub fn ok() -> Arc<Fake> {
        Fake::new(Box::new(|c| Some(happy(c))))
    }

    pub fn urls(&self) -> Vec<String> {
        lock(&self.calls).iter().map(|c| c.0.clone()).collect()
    }

    pub fn fcm_calls(&self) -> Vec<Call> {
        lock(&self.calls).iter().filter(|c| c.0.contains("fcm.googleapis.com")).cloned().collect()
    }
}

pub fn happy(c: &Call) -> (u16, String) {
    if c.0 == TOKEN_URL {
        (200, json!({ "access_token": "ya29.test-token", "expires_in": 3599, "token_type": "Bearer" }).to_string())
    } else {
        (200, json!({ "name": "projects/demo-proj/messages/1" }).to_string())
    }
}

impl FcmHttp for Fake {
    fn post(&self, url: &str, bearer: Option<&str>, content_type: &str, body: &[u8]) -> Option<(u16, String)> {
        let call = (url.to_owned(), bearer.map(str::to_owned), content_type.to_owned(), String::from_utf8_lossy(body).into_owned());
        lock(&self.calls).push(call.clone());
        (self.handler)(&call)
    }
}

pub type Reply = Option<(u16, String)>;

pub fn script(oauth: Vec<Reply>, fcm: Vec<Reply>) -> Arc<Fake> {
    let oauth = Mutex::new(VecDeque::from(oauth));
    let fcm = Mutex::new(VecDeque::from(fcm));
    Fake::new(Box::new(move |c| {
        let q = if c.0 == TOKEN_URL { &oauth } else { &fcm };
        lock(q).pop_front().unwrap_or_else(|| Some(happy(c)))
    }))
}

pub fn fast(http: Arc<Fake>) -> Fcm {
    let fcm = Fcm::default();
    fcm.set_http(http);
    *lock(&fcm.gap) = Duration::ZERO;
    fcm.set_account(Some(account()));
    fcm
}

pub fn msg() -> Value {
    json!({ "message": { "token": "t" } })
}

fn r(status: u16, body: &str) -> Reply {
    Some((status, body.to_owned()))
}

fn err_body(code: &str) -> String {
    json!({ "error": { "code": 404, "status": "NOT_FOUND", "details": [{ "@type": "type.googleapis.com/google.firebase.fcm.v1.FcmError", "errorCode": code }] } }).to_string()
}

#[test]
fn both_files_parse_into_the_options_the_phone_needs() {
    let p = parse(&gs_json("demo-proj", PACKAGE), &sa_json("demo-proj", KEY_A)).unwrap();
    assert_eq!(p.firebase.project_id, "demo-proj");
    assert_eq!(p.firebase.sender_id, "123456789012");
    assert_eq!(p.firebase.app_id, "1:123456789012:android:feedbeef");
    assert_eq!(p.firebase.api_key, "AIzaSyTestKey_0123456789-abc");
    assert!(p.firebase.is_set());
    assert_eq!(p.account.client_email, "firebase-adminsdk-x@demo-proj.iam.gserviceaccount.com");
}

#[test]
fn wrong_files_get_a_fixed_error_code() {
    let gs = gs_json("demo-proj", PACKAGE);
    let sa = sa_json("demo-proj", KEY_A);
    assert_eq!(parse(&gs, &sa_json("other-proj", KEY_A)).err(), Some("mismatch_project"));
    assert_eq!(parse(&gs_json("demo-proj", "com.other.pkg"), &sa).err(), Some("no_android_client"));
    assert_eq!(parse("not json", &sa).err(), Some("bad_json"));
    assert_eq!(parse("[1]", &sa).err(), Some("bad_json"));
    assert_eq!(parse("{}", &sa).err(), Some("bad_json"));
    assert_eq!(parse(&gs, "{").err(), Some("bad_service_account"));
    assert_eq!(parse(&sa, &gs).err(), Some("bad_json"));
    assert_eq!(parse(&gs, &gs).err(), Some("bad_service_account"));
    let mut v: Value = serde_json::from_str(&sa).unwrap();
    v["type"] = json!("authorized_user");
    assert_eq!(parse(&gs, &v.to_string()).err(), Some("bad_service_account"));
    let mut v: Value = serde_json::from_str(&sa).unwrap();
    v["private_key"] = json!("-----BEGIN RSA PRIVATE KEY-----\nAAAA\n-----END RSA PRIVATE KEY-----\n");
    assert_eq!(parse(&gs, &v.to_string()).err(), Some("bad_service_account"));
    v["private_key"] = json!(pem("AAAAAAAAAAAA"));
    assert_eq!(parse(&gs, &v.to_string()).err(), Some("bad_service_account"));
    let mut v: Value = serde_json::from_str(&sa).unwrap();
    v.as_object_mut().unwrap().remove("client_email");
    assert_eq!(parse(&gs, &v.to_string()).err(), Some("bad_service_account"));
    let big = format!("{{\"pad\":\"{}\"}}", "x".repeat(200_000));
    assert_eq!(parse(&big, &sa).err(), Some("bad_json"));
}

#[test]
fn ids_that_could_break_out_of_a_url_or_a_phone_field_are_refused() {
    for bad in ["a/../b", "demo proj", "Demo-Proj", "demo-proj?x=1", "abc", ""] {
        assert!(parse(&gs_json(bad, PACKAGE), &sa_json(bad, KEY_A)).is_err(), "{bad}");
    }
    let sa = sa_json("demo-proj", KEY_A);
    let mut v: Value = serde_json::from_str(&gs_json("demo-proj", PACKAGE)).unwrap();
    v["client"][1]["api_key"][0]["current_key"] = json!("key\"with\"quotes0123456789");
    assert_eq!(parse(&v.to_string(), &sa).err(), Some("no_android_client"));
    let mut v: Value = serde_json::from_str(&gs_json("demo-proj", PACKAGE)).unwrap();
    v["project_info"]["project_number"] = json!("12ab");
    assert_eq!(parse(&v.to_string(), &sa).err(), Some("bad_json"));
}

#[test]
fn the_saved_form_keeps_only_email_and_key() {
    let a = account();
    let text = stored(&a);
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v.as_object().unwrap().len(), 2);
    let back = restore(&text).unwrap();
    assert_eq!(back.client_email, a.client_email);
    assert_eq!(back.private_key.as_str(), a.private_key.as_str());
    assert!(restore("-").is_none());
    assert!(restore("{\"client_email\":\"a@b\",\"private_key\":\"x\"}").is_none());
    assert!(!format!("{a:?}").contains("PRIVATE"));
    assert!(pem_der(&pem(KEY_A)).is_some());
}

#[test]
fn the_jwt_is_rs256_signed_by_the_service_account_key() {
    let a = account();
    let jwt = assertion(&a, 1_700_000_000).unwrap();
    let parts: Vec<&str> = jwt.split('.').collect();
    assert_eq!(parts.len(), 3);
    let header: Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[0]).unwrap()).unwrap();
    assert_eq!(header, json!({ "alg": "RS256", "typ": "JWT" }));
    let claims: Value = serde_json::from_slice(&URL_SAFE_NO_PAD.decode(parts[1]).unwrap()).unwrap();
    assert_eq!(claims["iss"], a.client_email);
    assert_eq!(claims["scope"], SCOPE);
    assert_eq!(claims["scope"], "https://www.googleapis.com/auth/firebase.messaging");
    assert_eq!(claims["aud"], "https://oauth2.googleapis.com/token");
    assert_eq!(claims["iat"], 1_700_000_000u64);
    assert_eq!(claims["exp"], 1_700_003_600u64);
    let sig = URL_SAFE_NO_PAD.decode(parts[2]).unwrap();
    let public = STANDARD.decode(PUB_A).unwrap();
    let key = UnparsedPublicKey::new(&RSA_PKCS1_2048_8192_SHA256, public);
    let signed = format!("{}.{}", parts[0], parts[1]);
    assert!(key.verify(signed.as_bytes(), &sig).is_ok());
    assert!(key.verify(format!("{signed}x").as_bytes(), &sig).is_err());
    let other = ServiceAccount { client_email: a.client_email.clone(), private_key: zeroize::Zeroizing::new(pem(KEY_B)) };
    let forged = assertion(&other, 1_700_000_000).unwrap();
    let forged_sig = URL_SAFE_NO_PAD.decode(forged.split('.').nth(2).unwrap()).unwrap();
    assert!(key.verify(signed.as_bytes(), &forged_sig).is_err());
}

fn oauth_count(http: &Fake) -> usize {
    http.urls().iter().filter(|u| *u == TOKEN_URL).count()
}

#[test]
fn oauth_then_fcm_with_the_access_token_and_the_token_is_cached() {
    let http = Fake::ok();
    let fcm = fast(http.clone());
    assert_eq!(fcm.send("demo-proj", &msg()), Outcome::Sent);
    assert_eq!(fcm.send("demo-proj", &msg()), Outcome::Sent);
    let calls = lock(&http.calls).clone();
    assert_eq!(calls.len(), 3);
    assert_eq!(calls[0].0, TOKEN_URL);
    assert_eq!(calls[0].1, None);
    assert_eq!(calls[0].2, "application/x-www-form-urlencoded");
    assert!(calls[0].3.starts_with("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion="));
    assert_eq!(calls[1].0, "https://fcm.googleapis.com/v1/projects/demo-proj/messages:send");
    assert_eq!(calls[1].1.as_deref(), Some("ya29.test-token"));
    assert_eq!(calls[1].3, msg().to_string());
    assert_eq!(calls[2].0, calls[1].0);
    assert_eq!(fcm.last_error(), None);
    *lock(&fcm.ttl) = Duration::ZERO;
    assert_eq!(fcm.send("demo-proj", &msg()), Outcome::Sent);
    assert_eq!(oauth_count(&http), 2);
    fcm.set_account(Some(account()));
    *lock(&fcm.ttl) = Duration::from_secs(3000);
    assert_eq!(fcm.send("demo-proj", &msg()), Outcome::Sent);
    assert_eq!(oauth_count(&http), 3);
}

#[test]
fn nothing_is_sent_without_a_service_account() {
    let http = Fake::ok();
    let fcm = Fcm::default();
    fcm.set_http(http.clone());
    assert_eq!(fcm.send("demo-proj", &msg()), Outcome::Fail("not_configured"));
    assert!(lock(&http.calls).is_empty());
}

#[test]
fn a_401_refreshes_the_token_once_then_gives_up() {
    let http = script(vec![], vec![r(401, "{}"), r(200, "{}")]);
    let fcm = fast(http.clone());
    assert_eq!(fcm.send("demo-proj", &msg()), Outcome::Sent);
    assert_eq!(oauth_count(&http), 2);
    let http = script(vec![], vec![r(401, "{}"), r(401, "{}"), r(401, "{}")]);
    let fcm = fast(http.clone());
    assert_eq!(fcm.send("demo-proj", &msg()), Outcome::Fail("auth"));
    assert_eq!(fcm.last_error(), Some("auth"));
    assert_eq!(http.fcm_calls().len(), 2);
}

#[test]
fn oauth_failures_are_classified() {
    let http = script(vec![r(401, "{\"error\":\"invalid_grant\"}")], vec![]);
    assert_eq!(fast(http.clone()).send("demo-proj", &msg()), Outcome::Fail("auth"));
    assert_eq!(http.urls().len(), 1);
    assert_eq!(fast(script(vec![r(400, "{}")], vec![])).send("demo-proj", &msg()), Outcome::Fail("auth"));
    assert_eq!(fast(script(vec![r(200, "{}")], vec![])).send("demo-proj", &msg()), Outcome::Fail("auth"));
    let retry = json!({ "access_token": "tok" }).to_string();
    assert_eq!(fast(script(vec![r(503, "x"), r(200, &retry)], vec![])).send("demo-proj", &msg()), Outcome::Sent);
    assert_eq!(fast(script(vec![r(503, "x"), r(503, "x"), r(503, "x")], vec![])).send("demo-proj", &msg()), Outcome::Fail("unavailable"));
    assert_eq!(fast(script(vec![r(429, "x"), r(429, "x"), r(429, "x")], vec![])).send("demo-proj", &msg()), Outcome::Fail("quota"));
    assert_eq!(fast(script(vec![None, None, None], vec![])).send("demo-proj", &msg()), Outcome::Fail("offline"));
}

#[test]
fn unregistered_and_404_mean_the_token_is_dead() {
    for (status, body) in [(404, err_body("UNREGISTERED")), (404, "{}".to_owned()), (400, err_body("UNREGISTERED"))] {
        let http = script(vec![], vec![r(status, &body)]);
        assert_eq!(fast(http.clone()).send("demo-proj", &msg()), Outcome::Dead);
        assert_eq!(http.fcm_calls().len(), 1);
    }
    let invalid = json!({ "error": { "status": "INVALID_ARGUMENT" } }).to_string();
    assert_eq!(fast(script(vec![], vec![r(400, &invalid)])).send("demo-proj", &msg()), Outcome::Fail("rejected"));
    assert_eq!(fast(script(vec![], vec![r(403, "{}")])).send("demo-proj", &msg()), Outcome::Fail("denied"));
}

#[test]
fn quota_server_errors_and_lost_network_are_retried_then_reported() {
    let http = script(vec![], vec![r(429, "{}"), r(503, "{}"), r(200, "{}")]);
    assert_eq!(fast(http.clone()).send("demo-proj", &msg()), Outcome::Sent);
    assert_eq!(http.fcm_calls().len(), 3);
    let http = script(vec![], vec![r(500, "{}"), r(500, "{}"), r(500, "{}")]);
    let fcm = fast(http.clone());
    assert_eq!(fcm.send("demo-proj", &msg()), Outcome::Fail("unavailable"));
    assert_eq!(fcm.last_error(), Some("unavailable"));
    assert_eq!(http.fcm_calls().len(), 3);
    assert_eq!(fast(script(vec![], vec![None, None, None])).send("demo-proj", &msg()), Outcome::Fail("offline"));
    let fcm = fast(script(vec![], vec![None, r(200, "{}")]));
    assert_eq!(fcm.send("demo-proj", &msg()), Outcome::Sent);
    assert_eq!(fcm.last_error(), None);
}
