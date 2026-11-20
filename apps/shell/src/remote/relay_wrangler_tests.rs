use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};
use zeroize::Zeroizing;

use super::fake::*;
use crate::remote::config::Config;
use crate::remote::relay_setup::wrangler::{deployed_url, parse_whoami, project, tail, Call, Run, Who, Wrangler, WRANGLER};
use crate::remote::relay_setup::{bundle_meta, clip_token, TOKEN_SECRET};
use crate::remote::test_util::*;
use crate::state::lock;

const URL: &str = "https://orrery-relay.jin.workers.dev";

#[derive(Clone)]
struct Seen {
    args: Vec<String>,
    account: Option<String>,
    cwd: PathBuf,
    config: Option<Value>,
    script: bool,
}

struct State {
    missing: bool,
    logged_in: bool,
    login: Run,
    accounts: Vec<(String, String)>,
    deploy: Run,
    cancel_in_deploy: bool,
    calls: Vec<Seen>,
}

impl Default for State {
    fn default() -> Self {
        State {
            missing: false,
            logged_in: true,
            login: Run { code: Some(0), ..Run::default() },
            accounts: vec![(ACCOUNT.into(), "Jin".into())],
            deploy: Run { code: Some(0), out: format!("Uploaded orrery-relay (2.1 sec)\nDeployed orrery-relay triggers (0.4 sec)\n  {URL}\nCurrent Version ID: 1234"), ..Run::default() },
            cancel_in_deploy: false,
            calls: Vec::new(),
        }
    }
}

#[derive(Clone, Default)]
struct Fake(Arc<Mutex<State>>);

impl Wrangler for Fake {
    fn run(&self, call: &Call, cancel: &Arc<AtomicBool>) -> Run {
        let mut s = lock(&self.0);
        let config = std::fs::read(call.cwd.join("wrangler.json")).ok().and_then(|b| serde_json::from_slice(&b).ok());
        let script = call.cwd.join("worker.js").is_file();
        s.calls.push(Seen { args: call.args.iter().map(|a| (*a).to_owned()).collect(), account: call.account.map(str::to_owned), cwd: call.cwd.to_owned(), config, script });
        if s.missing {
            return Run { missing: true, ..Run::default() };
        }
        match call.args.first().copied() {
            Some("whoami") if s.logged_in => {
                let accounts: Vec<Value> = s.accounts.iter().map(|(id, name)| json!({ "id": id, "name": name })).collect();
                Run { code: Some(0), out: format!("npm warn exec\n{}", json!({ "loggedIn": true, "authType": "OAuth Token", "email": "me@example.com", "accounts": accounts })), ..Run::default() }
            }
            Some("whoami") => Run { code: Some(1), out: "{\n  \"loggedIn\": false\n}".into(), ..Run::default() },
            Some("login") => {
                let out = s.login.clone();
                if out.ok() {
                    s.logged_in = true;
                }
                out
            }
            Some("deploy") if s.cancel_in_deploy => {
                cancel.store(true, Ordering::SeqCst);
                Run { cancelled: true, ..Run::default() }
            }
            Some("deploy") => s.deploy.clone(),
            Some("delete") => Run { code: Some(0), ..Run::default() },
            _ => Run { code: Some(1), ..Run::default() },
        }
    }
}

fn quick_rig(tag: &str, state: State) -> (Rig, Fake) {
    let (rig, _) = setup(tag, Cf::default());
    let fake = Fake(Arc::new(Mutex::new(state)));
    lock(&rig.remote.setup.env).wrangler = Some(Arc::new(fake.clone()));
    (rig, fake)
}

fn verbs(fake: &Fake) -> Vec<String> {
    lock(&fake.0).calls.iter().map(|c| c.args[0].clone()).collect()
}

#[test]
fn a_logged_in_wrangler_deploys_the_bundle_in_one_tap() {
    let (rig, fake) = quick_rig("wr-one", State::default());
    let steps = Mutex::new(Vec::new());
    let out = rig.remote.relay_quick(&Value::Null, &|s| lock(&steps).push(s.to_owned()));
    assert_eq!(out, json!({ "ok": true, "url": URL }));
    assert_eq!(*lock(&steps), ["login", "deploy", "health", "done"]);
    assert_eq!(verbs(&fake), ["whoami", "deploy"]);
    let calls = lock(&fake.0).calls.clone();
    assert_eq!(calls[0].args, ["whoami", "--json"]);
    assert_eq!(calls[0].account, None);
    let deploy = &calls[1];
    assert_eq!(deploy.args, ["deploy", "--no-bundle"]);
    assert_eq!(deploy.account.as_deref(), Some(ACCOUNT));
    assert!(deploy.script);
    let cfg = deploy.config.clone().unwrap();
    assert_eq!(cfg["name"], "orrery-relay");
    assert_eq!(cfg["main"], "worker.js");
    assert_eq!(cfg["no_bundle"], true);
    assert_eq!(cfg["workers_dev"], true);
    assert_eq!(cfg["durable_objects"]["bindings"], json!([{ "name": "ROOM", "class_name": "Room" }]));
    assert_eq!(cfg["migrations"], json!([{ "tag": "v1", "new_sqlite_classes": ["Room"] }]));
    assert_eq!(cfg["ratelimits"][0]["name"], "LIMITER");
    assert!(cfg["vars"]["ANDROID_CERT_SHA256"].is_string());
    assert!(!deploy.cwd.exists());
    let saved = Config::load(&rig.dir).relay;
    assert_eq!((saved.url.as_str(), saved.account_id.as_str(), saved.via.as_str()), (URL, ACCOUNT, "wrangler"));
    let status = rig.remote.relay_status();
    assert_eq!((status["state"].as_str(), status["via"].as_str(), status["account"].as_str()), (Some("ready"), Some("wrangler"), Some("Jin")));
    assert!(lock(&rig.secrets.0).get(TOKEN_SECRET).is_none());
}

#[test]
fn a_logged_out_wrangler_logs_in_first_then_deploys() {
    let (rig, fake) = quick_rig("wr-login", State { logged_in: false, ..State::default() });
    assert_eq!(rig.remote.relay_quick(&Value::Null, &|_| {})["ok"], true);
    assert_eq!(verbs(&fake), ["whoami", "login", "whoami", "deploy"]);
    assert_eq!(lock(&fake.0).calls[1].args, ["login"]);
}

#[test]
fn a_cancelled_or_slow_login_stops_before_any_deploy() {
    let denied = Run { code: Some(1), err: "Error: Consent denied. You must grant consent to Wrangler in order to login.".into(), ..Run::default() };
    let (rig, fake) = quick_rig("wr-deny", State { logged_in: false, login: denied, ..State::default() });
    let out = rig.remote.relay_quick(&Value::Null, &|_| {});
    assert_eq!(out["error"], "login_cancelled");
    assert!(out["detail"].as_str().unwrap().contains("Consent denied"));
    assert!(!verbs(&fake).contains(&"deploy".to_owned()));
    assert_eq!(rig.remote.relay_status()["state"], "error");
    assert!(Config::load(&rig.dir).relay.url.is_empty());

    let (rig, _) = quick_rig("wr-slow", State { logged_in: false, login: Run { timed_out: true, ..Run::default() }, ..State::default() });
    assert_eq!(rig.remote.relay_quick(&Value::Null, &|_| {})["error"], "login_timeout");
}

#[test]
fn several_accounts_ask_for_a_pick_and_the_pick_is_used() {
    let accounts = vec![(ACCOUNT.into(), "Jin".into()), (OTHER.into(), "Work".into()), ("nope".into(), "Bad".into())];
    let (rig, fake) = quick_rig("wr-many", State { accounts, ..State::default() });
    let out = rig.remote.relay_quick(&Value::Null, &|_| {});
    assert_eq!(out, json!({ "ok": false, "error": "pick_account", "accounts": [{ "id": ACCOUNT, "name": "Jin" }, { "id": OTHER, "name": "Work" }] }));
    assert_eq!(rig.remote.relay_status()["state"], "none");
    assert_eq!(verbs(&fake), ["whoami"]);
    assert_eq!(rig.remote.relay_quick(&json!(OTHER), &|_| {})["ok"], true);
    let last = lock(&fake.0).calls.last().cloned().unwrap();
    assert_eq!((last.args[0].as_str(), last.account.as_deref()), ("deploy", Some(OTHER)));
    assert_eq!(Config::load(&rig.dir).relay.account_name, "Work");
    assert_eq!(rig.remote.relay_quick(&Value::Null, &|_| {})["ok"], true);
    assert_eq!(lock(&fake.0).calls.last().unwrap().account.as_deref(), Some(OTHER));
}

#[test]
fn no_node_is_reported_without_touching_the_config() {
    let (rig, _) = quick_rig("wr-node", State { missing: true, ..State::default() });
    assert_eq!(rig.remote.relay_quick(&Value::Null, &|_| {}), json!({ "ok": false, "error": "no_node" }));
    assert_eq!(rig.remote.relay_status()["error"], "no_node");
    assert!(Config::load(&rig.dir).relay.url.is_empty());
}

#[test]
fn a_failed_deploy_returns_a_short_masked_tail() {
    let secret = "AbCdEfGhIjKlMnOpQrStUvWxYz0123456789abcd";
    let err = format!("{}\n\u{1b}[31m✘ [ERROR]\u{1b}[0m A request to the Cloudflare API failed.\nAuthentication error [code: 10000]\nuser jin.vu@example.com token {secret}\n", "noise\n".repeat(20));
    let (rig, _) = quick_rig("wr-fail", State { deploy: Run { code: Some(1), err, ..Run::default() }, ..State::default() });
    let out = rig.remote.relay_quick(&Value::Null, &|_| {});
    assert_eq!(out["error"], "deploy_failed");
    let detail = out["detail"].as_str().unwrap();
    assert!(detail.contains("Authentication error [code: 10000]"));
    assert!(!detail.contains(secret) && !detail.contains("jin.vu@example.com") && !detail.contains('\u{1b}'));
    assert!(detail.lines().count() <= 6);
    assert_eq!(rig.remote.relay_status()["error"], "deploy_failed");
}

#[test]
fn a_missing_workers_dev_subdomain_links_to_the_onboarding_page() {
    let err = "✘ [ERROR] You need to register a workers.dev subdomain before publishing to workers.dev".to_owned();
    let (rig, _) = quick_rig("wr-sub", State { deploy: Run { code: Some(1), err, ..Run::default() }, ..State::default() });
    let out = rig.remote.relay_quick(&Value::Null, &|_| {});
    assert_eq!(out["error"], "subdomain_missing");
    assert_eq!(out["link"], format!("https://dash.cloudflare.com/{ACCOUNT}/workers/onboarding"));
}

#[test]
fn a_deploy_without_a_workers_dev_url_or_offline_is_an_error() {
    let (rig, _) = quick_rig("wr-nourl", State { deploy: Run { code: Some(0), out: "Deployed https://orrery-relay.EVIL..workers.dev".into(), ..Run::default() }, ..State::default() });
    assert_eq!(rig.remote.relay_quick(&Value::Null, &|_| {})["error"], "deploy_failed");
    let (rig, _) = quick_rig("wr-off", State { deploy: Run { code: Some(1), err: "TypeError: fetch failed".into(), ..Run::default() }, ..State::default() });
    assert_eq!(rig.remote.relay_quick(&Value::Null, &|_| {})["error"], "offline");
}

#[test]
fn cancel_kills_the_run_and_keeps_the_old_config() {
    let (rig, _) = quick_rig("wr-cancel", State { cancel_in_deploy: true, ..State::default() });
    let out = rig.remote.relay_quick(&Value::Null, &|_| {});
    assert_eq!(out, json!({ "ok": false, "error": "cancelled" }));
    assert_eq!(rig.remote.relay_status()["state"], "none");
    assert!(rig.remote.relay_status().get("error").is_none());
    assert!(Config::load(&rig.dir).relay.url.is_empty());
}

#[test]
fn cancel_only_raises_the_flag_while_busy() {
    let (rig, _) = quick_rig("wr-flag", State::default());
    assert_eq!(rig.remote.relay_cancel(), json!({ "ok": false }));
    assert!(!rig.remote.setup.cancel.load(Ordering::SeqCst));
    *lock(&rig.remote.setup.busy) = true;
    assert_eq!(rig.remote.relay_cancel(), json!({ "ok": true }));
    assert!(rig.remote.setup.cancel.load(Ordering::SeqCst));
    assert_eq!(rig.remote.relay_quick(&Value::Null, &|_| {})["error"], "busy");
}

#[test]
fn removing_a_wrangler_relay_with_its_worker_uses_wrangler_delete() {
    let (rig, fake) = quick_rig("wr-rm", State::default());
    assert_eq!(rig.remote.relay_quick(&Value::Null, &|_| {})["ok"], true);
    rig.remote.set_delete_confirm(Box::new(|| true));
    assert_eq!(rig.remote.relay_remove(&json!({ "deleteWorker": true })), json!({ "ok": true, "workerDeleted": true }));
    let last = lock(&fake.0).calls.last().cloned().unwrap();
    assert_eq!(last.args, ["delete", "--name", "orrery-relay", "--force"]);
    assert_eq!(last.account.as_deref(), Some(ACCOUNT));
    let cfg = Config::load(&rig.dir).relay;
    assert!(cfg.url.is_empty() && cfg.via.is_empty());
}

#[test]
fn the_clipboard_is_used_only_for_a_cloudflare_shaped_token() {
    let (rig, cf) = setup("wr-clip", Cf::default());
    let clip = Arc::new(Mutex::new(String::new()));
    let c = clip.clone();
    lock(&rig.remote.setup.env).clipboard = Some(Box::new(move || Some(Zeroizing::new(lock(&c).clone()))));
    for junk in ["", "0123456789abcdef0123456789abcdef01234567", "ghp_abcdefghijklmnopqrstuvwxyz0123456789", "cfk_abcdefghijklmnopqrstuvwxyz0123456789ABCD", "cfut_ has spaces in it but is long enough"] {
        *lock(&clip) = junk.into();
        assert_eq!(rig.remote.relay_clipboard(), json!({ "ok": false, "error": "none" }), "{junk}");
    }
    assert!(calls(&cf).is_empty());
    let token = "cfut_abcdefghijklmnopqrstuvwxyz0123456789ABCDEFGH";
    *lock(&clip) = format!("  {token}\n");
    let out = rig.remote.relay_clipboard();
    assert_eq!(out, json!({ "ok": true, "accounts": [{ "id": ACCOUNT, "name": "Jin" }] }));
    assert!(!out.to_string().contains(token));
    assert_eq!(lock(&rig.secrets.0).get(TOKEN_SECRET).map(String::as_str), Some(token));
    assert_eq!(clip_token("cfat_abcdefghijklmnopqrstuvwxyz0123456789ABCD"), Some("cfat_abcdefghijklmnopqrstuvwxyz0123456789ABCD"));
}

#[test]
fn the_parsers_accept_real_shapes_and_refuse_junk() {
    match parse_whoami("Getting User settings...\n{\"loggedIn\":true,\"accounts\":[{\"id\":\"80786e06fd9be997d29bc5bc4de835e6\",\"name\":\"a\\u0007b\"},{\"id\":\"80786e06fd9be997d29bc5bc4de835e6\",\"name\":\"dup\"}]}\n") {
        Some(Who::In(list)) => assert_eq!((list.len(), list[0].name.as_str()), (1, "ab")),
        _ => panic!("logged in"),
    }
    assert!(matches!(parse_whoami("{\"loggedIn\": false}"), Some(Who::Out)));
    assert!(parse_whoami("You are not authenticated.").is_none());
    assert_eq!(deployed_url("x https://orrery-relay.my-sub.workers.dev y").as_deref(), Some("https://orrery-relay.my-sub.workers.dev"));
    assert_eq!(deployed_url("https://orrery-relay.-bad.workers.dev"), None);
    assert_eq!(deployed_url("https://evil.example/orrery-relay"), None);
    let mut meta = bundle_meta();
    assert!(project(&meta).is_ok());
    meta["bindings"].as_array_mut().unwrap().push(json!({ "type": "secret_text", "name": "X" }));
    assert_eq!(project(&meta).err(), Some("bundle"));
    let mut meta = bundle_meta();
    meta["mainModule"] = json!("../worker.js");
    assert_eq!(project(&meta).err(), Some("bundle"));
    assert_eq!(tail("\n\n"), "");
    assert!(WRANGLER.starts_with("wrangler@4."));
}
