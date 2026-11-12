use super::api::{Body, Part, Transport};
use super::base::{Deps, Tg};
use super::html::Masker;
use crate::DeskError;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

type Reply = Box<dyn Fn(&str, &Value) -> (u16, Value) + Send + Sync>;

struct Fake {
    calls: Mutex<Vec<(String, Value)>>,
    reply: Mutex<Reply>,
}

fn body_value(b: &Body) -> Value {
    match b {
        Body::Json(m) => Value::Object(m.clone()),
        Body::Form(parts) => Value::Object(
            parts
                .iter()
                .map(|(k, p)| {
                    let v = match p {
                        Part::Text(t) => json!(t),
                        Part::File { name, bytes, .. } => json!({"file": name, "size": bytes.len()}),
                    };
                    (k.clone(), v)
                })
                .collect(),
        ),
    }
}

impl Transport for Fake {
    fn post(&self, url: &str, body: &Body, _t: u64) -> Result<(u16, Vec<u8>), &'static str> {
        let method = url.rsplit('/').next().unwrap_or("").to_string();
        let b = body_value(body);
        self.calls.lock().unwrap().push((method.clone(), b.clone()));
        let (status, v) = (self.reply.lock().unwrap())(&method, &b);
        Ok((status, serde_json::to_vec(&v).unwrap()))
    }

    fn get(&self, _url: &str, _t: u64, _max: u64) -> Result<(u16, Vec<u8>), &'static str> {
        Ok((200, b"file".to_vec()))
    }
}

struct World {
    tg: Arc<Tg>,
    http: Arc<Fake>,
    asks: Arc<Mutex<Vec<Value>>>,
    answer: Arc<Mutex<Value>>,
    clock: Arc<AtomicI64>,
    events: Arc<Mutex<Vec<Value>>>,
    keychain: Arc<Mutex<bool>>,
    link: Arc<Mutex<Value>>,
    _dir: std::path::PathBuf,
}

const TOKEN: &str = "12345:ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
const OWNER: i64 = 7;
const CHAT: &str = "-1001234567";

fn proj(p: &str) -> String {
    if cfg!(windows) {
        format!("C:{}", p.replace('/', "\\"))
    } else {
        p.to_string()
    }
}

fn ok(result: Value) -> (u16, Value) {
    (200, json!({"ok": true, "result": result}))
}

fn default_reply(method: &str, _b: &Value) -> (u16, Value) {
    match method {
        "getMe" => ok(json!({"id": 99, "username": "OrreryBot"})),
        "getUpdates" => ok(json!([])),
        "sendMessage" => ok(json!({"message_id": 500})),
        _ => ok(json!(true)),
    }
}

fn world() -> World {
    let dir = std::env::temp_dir().join(format!("ccm-tg-test-{}", crate::util::uuid()));
    std::fs::create_dir_all(&dir).unwrap();
    let http = Arc::new(Fake { calls: Mutex::new(Vec::new()), reply: Mutex::new(Box::new(default_reply)) });
    let asks: Arc<Mutex<Vec<Value>>> = Arc::default();
    let answer = Arc::new(Mutex::new(json!({"ok": true, "action": "send", "name": "app", "queued": false, "terminalId": "t1"})));
    let clock = Arc::new(AtomicI64::new(1_800_000_000_000));
    let events: Arc<Mutex<Vec<Value>>> = Arc::default();
    let keychain = Arc::new(Mutex::new(true));
    let link: Arc<Mutex<Value>> = Arc::new(Mutex::new(Value::Null));
    let lk = link.clone();
    let (a, ans, kc) = (asks.clone(), answer.clone(), keychain.clone());
    let host = Arc::new(move |m: &str, p: Value| -> Result<Value, DeskError> {
        match m {
            "keychain.available" => Ok(json!(*kc.lock().unwrap())),
            "keychain.seal" => Ok(json!(format!("S{}", p["plain"].as_str().unwrap_or("")))),
            "keychain.unseal" => p["sealed"].as_str().and_then(|s| s.strip_prefix('S')).map(|s| json!(s)).ok_or(DeskError::Host("bad".into())),
            "telegram.locale" => Ok(json!("en")),
            "remote.link" => Ok(lk.lock().unwrap().clone()),
            "telegram.ask" => {
                a.lock().unwrap().push(p["req"].clone());
                Ok(ans.lock().unwrap().clone())
            }
            _ => Err(DeskError::Host(format!("no host {m}"))),
        }
    });
    let ev = events.clone();
    let c = clock.clone();
    let deps = Deps {
        file: Some(dir.join("telegram.json")),
        base: "http://127.0.0.1:9".into(),
        masker: Masker { home: "/Users/nobody".into(), platform: "darwin" },
        win: false,
        claude_root: dir.join("claude"),
        http: http.clone(),
        host,
        emit: Arc::new(move |_e, p| ev.lock().unwrap().push(p)),
        clock: Arc::new(move || c.load(Ordering::SeqCst)),
        sleep: Arc::new(|ms| std::thread::sleep(std::time::Duration::from_millis(ms.min(5)))),
        local: Arc::new(super::clock::local),
        album_ms: 0,
        mt: None,
    };
    World { tg: Tg::new(deps), http, asks, answer, clock, events, keychain, link, _dir: dir }
}

fn msg(from: i64, text: &str, reply_to: Option<i64>) -> Value {
    let mut m = json!({"message_id": 10, "chat": {"id": -1001234567i64, "type": "supergroup"}, "from": {"id": from, "first_name": if from == OWNER { "Jin" } else { "Eve" }}, "text": text, "is_topic_message": true, "message_thread_id": 5, "date": 1_800_000_000});
    if let Some(r) = reply_to {
        m["reply_to_message"] = json!({"message_id": r, "from": {"id": 99}});
    }
    json!({"update_id": 1, "message": m})
}

fn sent_texts(w: &World) -> Vec<String> {
    w.http.calls.lock().unwrap().iter().filter(|(m, _)| m == "sendMessage").map(|(_, b)| b["text"].as_str().unwrap_or("").to_string()).collect()
}

fn setup(w: &World, receiving: bool) {
    let p = super::sanitize::sanitize_patch(&json!({"token": TOKEN, "enabled": true})).unwrap();
    assert_eq!(w.tg.set(&p)["ok"], true);
    w.tg.who_am_i();
    w.tg.lock().data.owner = Some((OWNER, "Jin".into()));
    let mut input = serde_json::Map::new();
    input.insert("project".into(), json!(proj("/p/app")));
    input.insert("chatId".into(), json!(CHAT));
    input.insert("threadId".into(), json!(5));
    assert_eq!(w.tg.set_route(&input)["ok"], true);
    if receiving {
        w.tg.lock().receiving = true;
    }
}

#[test]
fn settings_validate_token_and_keychain() {
    let w = world();
    let bad = super::sanitize::sanitize_patch(&json!({"token": "nope"})).unwrap();
    assert_eq!(w.tg.set(&bad), json!({"ok": false, "error": "invalid-token"}));
    *w.keychain.lock().unwrap() = false;
    let good = super::sanitize::sanitize_patch(&json!({"token": TOKEN})).unwrap();
    assert_eq!(w.tg.set(&good), json!({"ok": false, "error": "keychain"}));
    *w.keychain.lock().unwrap() = true;
    let res = w.tg.set(&good);
    assert_eq!(res["state"]["configured"], true);
    assert!(!res.to_string().contains(TOKEN));
    let file = std::fs::read_to_string(w._dir.join("telegram.json")).unwrap();
    assert!(file.contains("\"token\": \"S12345"));
    assert!(!w.events.lock().unwrap().is_empty());
    w.tg.stop();
}

#[test]
fn pairing_needs_the_code_and_caps_tries() {
    let w = world();
    setup(&w, false);
    w.tg.lock().data.owner = None;
    let res = w.tg.pair(true);
    let code = res["state"]["pairCode"].as_str().unwrap().to_string();
    let wrong = if code == "000000" { "111111" } else { "000000" };
    for _ in 0..5 {
        w.tg.on_update(&msg(8, &format!("@OrreryBot pair {wrong}"), None));
    }
    w.tg.on_update(&msg(8, &format!("@OrreryBot pair {code}"), None));
    assert!(w.tg.lock().data.owner.is_none());
    w.tg.on_update(&msg(OWNER, &format!("@OrreryBot pair {code}"), None));
    assert_eq!(w.tg.lock().data.owner, Some((OWNER, "Jin".into())));
    assert!(sent_texts(&w).iter().any(|t| t.starts_with("🔗 Paired with Jin.")));
    assert!(w.tg.state()["pairCode"].is_null());
    w.tg.stop();
}

#[test]
fn strangers_and_receive_off_never_reach_a_pane() {
    let w = world();
    setup(&w, false);
    w.tg.on_update(&msg(OWNER, "@OrreryBot fix the bug", None));
    assert!(w.asks.lock().unwrap().is_empty());
    w.tg.lock().receiving = true;
    for _ in 0..7 {
        w.tg.on_update(&msg(8, "@OrreryBot rm -rf /", None));
    }
    assert!(w.asks.lock().unwrap().is_empty());
    assert_eq!(sent_texts(&w).iter().filter(|t| t.contains("I only answer my owner Jin")).count(), 5);
    assert_eq!(w.tg.state()["ignored"], 7);
    w.tg.on_update(&msg(OWNER, "@OrreryBot fix the bug", None));
    let asks = w.asks.lock().unwrap().clone();
    assert_eq!(asks.len(), 1);
    assert_eq!(asks[0]["action"], "send");
    assert!(asks[0]["text"].as_str().unwrap().starts_with("fix the bug\n\n(Sent from Telegram."));
    assert_eq!(asks[0]["from"], "Jin");
    w.tg.stop();
}

#[test]
fn read_members_only_view_and_close_is_owner_only() {
    let w = world();
    setup(&w, true);
    w.tg.lock().data.members = vec![json!({"id": 20, "name": "Reader", "level": "read"}), json!({"id": 21, "name": "Full", "level": "full"})];
    w.tg.on_update(&msg(20, "@OrreryBot do things", None));
    w.tg.on_update(&msg(21, "@OrreryBot close", None));
    assert!(w.asks.lock().unwrap().is_empty());
    let texts = sent_texts(&w);
    assert!(texts.iter().any(|t| t.starts_with("You can only view here")));
    assert!(texts.iter().any(|t| t == "Your access does not cover this button."));
    w.tg.on_update(&msg(21, "@OrreryBot go on", None));
    assert_eq!(w.asks.lock().unwrap()[0]["from"], "Full");
    w.tg.stop();
}

#[test]
fn reply_buttons_answer_once_and_only_for_the_owner() {
    let w = world();
    setup(&w, false);
    let res = w.tg.report(&proj("/p/app"), "Pick one", None, Some("pane-1"), Some(vec![("Yes".into(), "yes please".into())]));
    assert_eq!(res["buttons"], 1);
    let keys = w.http.calls.lock().unwrap().iter().rev().find(|(m, _)| m == "sendMessage").unwrap().1["reply_markup"].clone();
    let data = keys["inline_keyboard"][0][0]["callback_data"].as_str().unwrap().to_string();
    *w.answer.lock().unwrap() = json!({"ok": true, "action": "answer", "name": "app", "submitted": true});
    w.tg.on_callback(&json!({"id": "c1", "from": {"id": 8}, "data": data, "message": {"message_id": 500}}));
    assert!(w.asks.lock().unwrap().is_empty());
    w.tg.on_callback(&json!({"id": "c2", "from": {"id": OWNER}, "data": data, "message": {"message_id": 500}}));
    let asks = w.asks.lock().unwrap().clone();
    assert_eq!(asks.len(), 1);
    assert_eq!(asks[0]["action"], "answer");
    assert_eq!(asks[0]["target"], json!({"kind": "pane", "sessionId": "pane-1"}));
    assert_eq!(asks[0]["text"], "yes please");
    w.tg.on_callback(&json!({"id": "c3", "from": {"id": OWNER}, "data": data}));
    assert_eq!(w.asks.lock().unwrap().len(), 1);
    let answers: Vec<String> = w.http.calls.lock().unwrap().iter().filter(|(m, _)| m == "answerCallbackQuery").filter_map(|(_, b)| b["text"].as_str().map(str::to_string)).collect();
    assert_eq!(answers, vec!["Only the paired account can use these buttons.", "✓ Yes — sent to the session.", "Already answered: Yes"]);
    w.tg.lock().data.owner = None;
    let none = w.tg.report(&proj("/p/app"), "x", None, Some("pane-1"), Some(vec![("A".into(), "a".into())]));
    assert_eq!(none, json!({"ok": false, "error": "not-paired"}));
    w.tg.stop();
}

#[test]
fn a_429_holds_the_chat_and_queues_the_message() {
    let w = world();
    setup(&w, false);
    *w.http.reply.lock().unwrap() = Box::new(|m, b| if m == "sendMessage" && b["text"] == "first" { (429, json!({"ok": false, "parameters": {"retry_after": 30}})) } else { default_reply(m, b) });
    let route = w.tg.lock().data.routes[0].clone();
    assert_eq!(w.tg.send_text(&route, "first", None, None, None), None);
    assert!(w.tg.hold_left(CHAT) > 0);
    let before = w.http.calls.lock().unwrap().len();
    assert_eq!(w.tg.send_text(&route, "second", None, None, None), None);
    assert_eq!(w.http.calls.lock().unwrap().len(), before);
    assert_eq!(w.tg.lock().held[0].1.len(), 2);
    w.clock.fetch_add(31_000, Ordering::SeqCst);
    *w.http.reply.lock().unwrap() = Box::new(default_reply);
    w.tg.flush_held();
    let texts = sent_texts(&w);
    assert_eq!(&texts[texts.len() - 2..], &["first".to_string(), "second".to_string()]);
    w.tg.stop();
}

#[test]
fn episodes_go_to_the_topic_with_buttons_only_when_receiving() {
    let w = world();
    setup(&w, false);
    let e = json!({"type": "telegram", "episode": "e1", "reason": "permission", "terminalId": "t1", "name": "app", "cwd": proj("/p/app/src"), "since": 5, "tool": "Bash"});
    assert!(w.tg.handle(&e));
    assert!(!w.tg.handle(&e));
    let last = w.http.calls.lock().unwrap().last().unwrap().1.clone();
    assert_eq!(last["text"], "🔐 <b>app</b> needs approval: Bash");
    assert!(last.get("reply_markup").is_none());
    w.tg.lock().receiving = true;
    assert!(w.tg.handle(&json!({"type": "telegram", "episode": "e2", "reason": "permission", "terminalId": "t1", "name": "app", "cwd": proj("/p/app"), "since": 5, "tool": "ExitPlanMode"})));
    let last = w.http.calls.lock().unwrap().last().unwrap().1.clone();
    assert_eq!(last["reply_markup"]["inline_keyboard"][0][0]["text"], "✅ Approve plan");
    assert_eq!(last["message_thread_id"], 5);
    w.tg.stop();
}

#[test]
fn pane_link_button_only_when_the_shell_returns_a_valid_link() {
    let w = world();
    setup(&w, false);
    let ep = |id: &str| json!({"type": "telegram", "episode": id, "reason": "done", "terminalId": "t1", "name": "app", "cwd": proj("/p/app"), "worked": 600_000});
    assert!(w.tg.handle(&ep("l0")));
    let last = w.http.calls.lock().unwrap().last().unwrap().1.clone();
    assert!(last.get("reply_markup").is_none());
    *w.link.lock().unwrap() = json!({"url": "https://evil.dev/x"});
    assert!(w.tg.handle(&ep("l1")));
    let last = w.http.calls.lock().unwrap().last().unwrap().1.clone();
    assert!(last.get("reply_markup").is_none());
    *w.link.lock().unwrap() = json!({"url": "https://r.dev/o/abcdefghijklmnop/p1"});
    assert!(w.tg.handle(&ep("l2")));
    let last = w.http.calls.lock().unwrap().last().unwrap().1.clone();
    assert_eq!(last["reply_markup"]["inline_keyboard"], json!([[{"text": "Open on phone", "url": "https://r.dev/o/abcdefghijklmnop/p1"}]]));
    w.tg.lock().receiving = true;
    assert!(w.tg.handle(&ep("l3")));
    let last = w.http.calls.lock().unwrap().last().unwrap().1.clone();
    let rows = last["reply_markup"]["inline_keyboard"].as_array().unwrap();
    assert!(rows.len() >= 2);
    assert_eq!(rows[rows.len() - 1][0]["url"], "https://r.dev/o/abcdefghijklmnop/p1");
    assert!(rows[0][0].get("callback_data").is_some());
    w.tg.stop();
}

#[test]
fn remote_alerts_post_escaped_text_to_every_routed_chat() {
    let w = world();
    setup(&w, false);
    w.tg.lock().data.alerts = true;
    assert!(!w.tg.alert("remote", &json!({ "text": "  " })));
    assert!(w.tg.alert("remote", &json!({ "text": "Orrery Remote: IP blocked\n<b>203.0.113.9</b>\u{7}" })));
    assert_eq!(sent_texts(&w).last().map(String::as_str), Some("🛡 Orrery Remote: IP blocked\n&lt;b&gt;203.0.113.9&lt;/b&gt;"));
    w.tg.lock().data.alerts = false;
    assert!(!w.tg.alert("remote", &json!({ "text": "x" })));
    w.tg.stop();
}
