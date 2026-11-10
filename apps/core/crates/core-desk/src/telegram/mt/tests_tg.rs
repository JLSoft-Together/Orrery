use super::store::{self, Saved};
use super::tests::{temp_file, Mock, MockConn};
use super::upload::PART_SIZE;
use super::{Connector, MtError, SignIn};
use crate::telegram::api::{Body, Transport};
use crate::telegram::base::{Deps, Tg};
use crate::telegram::html::Masker;
use crate::DeskError;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

const TOKEN: &str = "12345:ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
const HASH: &str = "0123456789abcdef0123456789abcdef";

struct NoHttp;

impl Transport for NoHttp {
    fn post(&self, _url: &str, _body: &Body, _t: u64) -> Result<(u16, Vec<u8>), &'static str> {
        Ok((200, br#"{"ok":true,"result":true}"#.to_vec()))
    }

    fn get(&self, _url: &str, _t: u64, _max: u64) -> Result<(u16, Vec<u8>), &'static str> {
        Ok((200, Vec::new()))
    }
}

struct World {
    tg: Arc<Tg>,
    dir: PathBuf,
    hosts: Arc<Mutex<Vec<String>>>,
}

fn world_in(dir: PathBuf, conn: Arc<MockConn>) -> World {
    let hosts: Arc<Mutex<Vec<String>>> = Arc::default();
    let h = hosts.clone();
    let host = Arc::new(move |m: &str, p: Value| -> Result<Value, DeskError> {
        h.lock().unwrap().push(m.to_string());
        match m {
            "keychain.available" => Ok(json!(true)),
            "keychain.seal" => Ok(json!(format!("S{}", p["plain"].as_str().unwrap_or("")))),
            "keychain.unseal" => p["sealed"].as_str().and_then(|s| s.strip_prefix('S')).map(|s| json!(s)).ok_or(DeskError::Host("bad".into())),
            "telegram.locale" => Ok(json!("en")),
            "telegram.notice" => Ok(Value::Bool(true)),
            _ => Err(DeskError::Host(format!("no host {m}"))),
        }
    });
    let mt: Arc<dyn Connector> = conn;
    let deps = Deps {
        file: Some(dir.join("telegram.json")),
        base: "http://127.0.0.1:9".into(),
        masker: Masker { home: "/Users/nobody".into(), platform: "darwin" },
        win: false,
        claude_root: dir.join("claude"),
        http: Arc::new(NoHttp),
        host,
        emit: Arc::new(|_e, _p| {}),
        clock: Arc::new(|| 1_800_000_000_000),
        sleep: Arc::new(|_ms| {}),
        local: Arc::new(crate::telegram::clock::local),
        album_ms: 0,
        mt: Some(mt),
    };
    let tg = Tg::new(deps);
    {
        let mut st = tg.lock();
        st.data.token = Some(format!("S{TOKEN}"));
        st.data.api_id = Some(12345);
        st.data.api_hash = Some(format!("S{HASH}"));
        st.data.big_files = true;
    }
    World { tg, dir, hosts }
}

fn world(conn: Arc<MockConn>) -> World {
    let dir = std::env::temp_dir().join(format!("ccm-mt-tg-{}", crate::util::uuid()));
    std::fs::create_dir_all(&dir).unwrap();
    world_in(dir, conn)
}

fn upload(w: &World, size: u64) -> Value {
    let file = temp_file(size);
    let req = json!({"chatId": "-1001234567", "threadId": 5, "file": file, "name": "big.apk", "size": size, "caption": "📝 <b>app</b>\nBuilt", "progress": 1});
    let creds = json!({"apiId": 12345, "apiHash": HASH, "token": TOKEN, "session": "gramjs-ignored"});
    let conn = w.tg.deps.mt.clone().unwrap();
    let out = w.tg.mt_upload(&conn, &req, &creds);
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
    out
}

fn saved(w: &World) -> Saved {
    store::read(Some(&w.dir.join(store::FILE)))
}

#[test]
fn upload_resumes_missing_parts_on_a_new_connection() {
    let first = Arc::new(Mock::default());
    *first.fail_from.lock().unwrap() = Some(3);
    *first.session.lock().unwrap() = Some("bot-s1".into());
    let second = Arc::new(Mock::default());
    *second.session.lock().unwrap() = Some("bot-s1".into());
    let conn = MockConn::with(vec![first.clone(), second.clone()]);
    let w = world(conn.clone());
    assert_eq!(upload(&w, PART_SIZE * 6), json!({"result": {"ok": true}}));
    assert_eq!(conn.sessions(), vec![None, Some("bot-s1".into())]);
    assert_eq!((first.did("bot"), second.did("bot")), (1, 0));
    let (a, b) = (first.part_ids(), second.part_ids());
    assert!(a.iter().all(|p| !b.contains(p)));
    let mut all = [a, b].concat();
    all.sort();
    assert_eq!(all, vec![0, 1, 2, 3, 4, 5]);
    let ids: Vec<i64> = first.parts.lock().unwrap().iter().chain(second.parts.lock().unwrap().iter()).map(|p| p.0).collect();
    assert!(ids.windows(2).all(|x| x[0] == x[1]));
    let sent = second.sent.lock().unwrap();
    assert_eq!(sent.len(), 1);
    assert_eq!(sent[0].thread, Some(5));
    assert_eq!(sent[0].peer, super::Peer::Channel { id: 1234567, hash: 2469134 });
    assert_eq!(sent[0].caption, "📝 app\nBuilt");
    assert_eq!(saved(&w).bot.map(|b| b.1), Some("Sbot-s1".into()));
}

#[test]
fn a_revoked_bot_session_signs_in_again_once() {
    let w0 = world(MockConn::with(vec![]));
    let fp = store::fingerprint(12345, HASH, TOKEN);
    assert!(store::write(Some(&w0.dir.join(store::FILE)), &Saved { bot: Some((fp, "Sold".into())), ..Saved::default() }));
    let old = Arc::new(Mock::default());
    *old.resolve_err.lock().unwrap() = Some(MtError::rpc(401, "SESSION_REVOKED", None));
    let fresh = Arc::new(Mock::default());
    *fresh.session.lock().unwrap() = Some("new-key".into());
    let conn = MockConn::with(vec![old.clone(), fresh.clone()]);
    let w = world_in(w0.dir.clone(), conn.clone());
    assert_eq!(upload(&w, 10), json!({"result": {"ok": true}}));
    assert_eq!(conn.sessions(), vec![Some("old".into()), None]);
    assert_eq!(fresh.did("bot"), 1);
    assert_eq!(saved(&w).bot.map(|b| b.1), Some("Snew-key".into()));
    let text = std::fs::read_to_string(w.dir.join(store::FILE)).unwrap();
    assert!(!text.contains(TOKEN) && !text.contains(HASH));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(w.dir.join(store::FILE)).unwrap().permissions().mode() & 0o777, 0o600);
    }
}

#[test]
fn refused_token_and_closed_chat_map_to_their_errors() {
    let bad = Arc::new(Mock::default());
    *bad.bot_err.lock().unwrap() = Some(MtError::rpc(400, "ACCESS_TOKEN_INVALID", None));
    let conn = MockConn::with(vec![bad]);
    let w = world(conn.clone());
    assert_eq!(upload(&w, 10), json!({"result": {"ok": false, "error": "auth"}}));
    assert_eq!(conn.sessions().len(), 1);
    let closed = Arc::new(Mock::default());
    *closed.resolve_err.lock().unwrap() = Some(MtError::rpc(400, "CHANNEL_PRIVATE", None));
    let w = world(MockConn::with(vec![closed]));
    assert_eq!(upload(&w, 10), json!({"result": {"ok": false, "error": "chat"}}));
    let w = world(MockConn::with(vec![]));
    let out = w.tg.mt_upload(&w.tg.deps.mt.clone().unwrap(), &json!({"chatId": "-1", "size": 0}), &json!({}));
    assert_eq!(out, json!({"result": {"ok": false, "error": "auth"}}));
}

#[test]
fn account_login_keeps_its_own_session_file() {
    let login = Arc::new(Mock::default());
    *login.session.lock().unwrap() = Some("user-key".into());
    login.sign.lock().unwrap().extend([Err(MtError::rpc(400, "PHONE_CODE_INVALID", None)), Ok(SignIn::Done("Jin Vu".into()))]);
    let out_link = Arc::new(Mock::default());
    let conn = MockConn::with(vec![login.clone(), out_link.clone()]);
    let w = world(conn.clone());
    let user = |v: Value| w.tg.user(&v);
    assert_eq!(user(json!({"action": "code", "phone": "+84 912"}))["next"], "code");
    assert_eq!(user(json!({"action": "signin", "code": "11111"})), json!({"ok": false, "error": "code"}));
    let done = user(json!({"action": "signin", "code": "12345"}));
    assert_eq!(done["ok"], true);
    assert_eq!(done["state"]["userAccount"], "Jin Vu");
    assert_eq!(saved(&w).user, Some((12345, "Suser-key".into())));
    let text = std::fs::read_to_string(w.dir.join(store::FILE)).unwrap();
    assert!(!text.contains("+84") && !text.contains("11111"));
    assert_eq!(w.tg.lock().data.user_session, None);
    let out = user(json!({"action": "logout"}));
    assert_eq!(out["ok"], true);
    assert_eq!(out["state"]["userAccount"], Value::Null);
    assert_eq!(conn.sessions(), vec![None, Some("user-key".into())]);
    assert_eq!(out_link.did("logout"), 1);
    assert_eq!(saved(&w).user, None);
}

#[test]
fn an_old_gramjs_account_asks_to_sign_in_again_once() {
    let w = world(MockConn::with(vec![]));
    w.tg.lock().data.user_session = Some("Sgramjs".into());
    w.tg.lock().data.user_name = Some("Jin".into());
    assert_eq!(w.tg.state()["userAccount"], Value::Null);
    w.tg.state();
    let w2 = world_in(w.dir.clone(), MockConn::with(vec![]));
    w2.tg.lock().data.user_session = Some("Sgramjs".into());
    w2.tg.state();
    let count = |w: &World| w.hosts.lock().unwrap().iter().filter(|m| *m == "telegram.notice").count();
    assert_eq!((count(&w), count(&w2)), (1, 0));
    assert!(saved(&w).noticed);
}

#[test]
fn a_revoked_account_is_dropped_on_scan() {
    let scan = Arc::new(Mock::default());
    *scan.chats.lock().unwrap() = vec![super::ForumChat { id: 9, hash: 1, title: "G".into() }];
    scan.pages.lock().unwrap().push_back(Err(MtError::rpc(401, "AUTH_KEY_UNREGISTERED", None)));
    let conn = MockConn::with(vec![scan]);
    let w = world(conn.clone());
    assert!(w.tg.mt.update(|s| s.user = Some((12345, "Suser-key".into()))));
    assert_eq!(w.tg.state()["userAccount"], "Telegram");
    let c: Arc<dyn Connector> = conn.clone();
    assert_eq!(w.tg.mt_topics(&c), None);
    assert_eq!(conn.sessions(), vec![Some("user-key".into())]);
    assert_eq!(saved(&w).user, None);
    assert_eq!(w.tg.state()["userAccount"], Value::Null);
}
