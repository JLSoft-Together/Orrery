use super::html::{parse, Kind};
use super::login::{self, Logins, Outcome, LOGIN_TTL_MS};
use super::upload::{self, backoff_ms, part_range, total_parts, Job, Parts, PART_SIZE};
use super::*;
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicUsize};
use std::sync::Mutex;

pub type PartSeen = (i64, i32, i32, usize, u8);

#[derive(Default)]
pub struct Mock {
    pub parts: Mutex<Vec<PartSeen>>,
    pub part_errors: Mutex<HashMap<i32, VecDeque<MtError>>>,
    pub fail_from: Mutex<Option<i32>>,
    pub send_errors: Mutex<VecDeque<MtError>>,
    pub sent: Mutex<Vec<Doc>>,
    pub calls: Mutex<Vec<String>>,
    pub bot_err: Mutex<Option<MtError>>,
    pub resolve_err: Mutex<Option<MtError>>,
    pub code: Mutex<VecDeque<Result<Sent, MtError>>>,
    pub sign: Mutex<VecDeque<Result<SignIn, MtError>>>,
    pub password: Mutex<VecDeque<Result<String, MtError>>>,
    pub chats: Mutex<Vec<ForumChat>>,
    pub pages: Mutex<VecDeque<Result<TopicPage, MtError>>>,
    pub offsets: Mutex<Vec<Offset>>,
    pub session: Mutex<Option<String>>,
    pub closed: AtomicBool,
}

impl Mock {
    pub fn call(&self, what: &str) {
        self.calls.lock().unwrap().push(what.to_string());
    }

    pub fn did(&self, what: &str) -> usize {
        self.calls.lock().unwrap().iter().filter(|c| *c == what).count()
    }

    pub fn part_ids(&self) -> Vec<i32> {
        let mut v: Vec<i32> = self.parts.lock().unwrap().iter().map(|p| p.1).collect();
        v.sort();
        v
    }
}

impl Link for Mock {
    fn bot_sign_in(&self, _token: &str) -> Result<(), MtError> {
        self.call("bot");
        self.bot_err.lock().unwrap().clone().map_or(Ok(()), Err)
    }

    fn save_part(&self, file_id: i64, part: i32, total: i32, bytes: Vec<u8>) -> Result<(), MtError> {
        if let Some(e) = self.part_errors.lock().unwrap().get_mut(&part).and_then(|q| q.pop_front()) {
            return Err(e);
        }
        if self.fail_from.lock().unwrap().is_some_and(|n| part >= n) {
            return Err(MtError::Net);
        }
        self.parts.lock().unwrap().push((file_id, part, total, bytes.len(), bytes.first().copied().unwrap_or(0)));
        Ok(())
    }

    fn resolve_channel(&self, id: i64) -> Result<i64, MtError> {
        self.call("resolve");
        self.resolve_err.lock().unwrap().clone().map_or(Ok(id * 2), Err)
    }

    fn send_document(&self, doc: &Doc) -> Result<(), MtError> {
        self.sent.lock().unwrap().push(doc.clone());
        self.send_errors.lock().unwrap().pop_front().map_or(Ok(()), Err)
    }

    fn send_code(&self, _phone: &str) -> Result<Sent, MtError> {
        self.call("code");
        self.code.lock().unwrap().pop_front().unwrap_or(Ok(Sent::Code("hash".into())))
    }

    fn sign_in(&self, _phone: &str, hash: &str, _code: &str) -> Result<SignIn, MtError> {
        self.call(&format!("sign:{hash}"));
        self.sign.lock().unwrap().pop_front().unwrap_or(Ok(SignIn::Done("Jin".into())))
    }

    fn check_password(&self, _password: &str) -> Result<String, MtError> {
        self.call("password");
        self.password.lock().unwrap().pop_front().unwrap_or(Ok("Jin".into()))
    }

    fn forum_chats(&self, _max: usize) -> Result<Vec<ForumChat>, MtError> {
        Ok(self.chats.lock().unwrap().clone())
    }

    fn forum_topics(&self, _chat: &ForumChat, at: Offset, _limit: i32) -> Result<TopicPage, MtError> {
        self.offsets.lock().unwrap().push(at);
        self.pages.lock().unwrap().pop_front().unwrap_or(Ok(TopicPage::default()))
    }

    fn log_out(&self) -> Result<(), MtError> {
        self.call("logout");
        Ok(())
    }

    fn export(&self) -> Option<String> {
        self.session.lock().unwrap().clone()
    }

    fn close(&self) {
        self.closed.store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

#[derive(Default)]
pub struct MockConn {
    pub links: Mutex<VecDeque<Arc<Mock>>>,
    pub made: Mutex<Vec<(Option<String>, Arc<Mock>)>>,
}

impl MockConn {
    pub fn with(links: Vec<Arc<Mock>>) -> Arc<MockConn> {
        Arc::new(MockConn { links: Mutex::new(links.into()), made: Mutex::default() })
    }

    pub fn sessions(&self) -> Vec<Option<String>> {
        self.made.lock().unwrap().iter().map(|m| m.0.clone()).collect()
    }
}

impl Connector for MockConn {
    fn connect(&self, _id: i32, _hash: &str, session: Option<&str>) -> Result<Arc<dyn Link>, MtError> {
        let link = self.links.lock().unwrap().pop_front().unwrap_or_default();
        self.made.lock().unwrap().push((session.map(str::to_string), link.clone()));
        Ok(link)
    }
}

pub fn temp_file(size: u64) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("ccm-mt-{}", crate::util::uuid()));
    std::fs::create_dir_all(&dir).unwrap();
    let file = dir.join("big.apk");
    let bytes: Vec<u8> = (0..size).map(|i| (i / PART_SIZE) as u8).collect();
    std::fs::write(&file, bytes).unwrap();
    file
}

fn doc(parts: i32) -> Doc {
    Doc { peer: Peer::Channel { id: 1, hash: 2 }, thread: Some(5), file_id: 42, parts, name: "big.apk".into(), caption: String::new(), entities: Vec::new(), random_id: 9 }
}

fn run_with(link: &Mock, size: u64, sleeps: &Mutex<Vec<u64>>) -> (Result<(), MtError>, f64) {
    let file = temp_file(size);
    let last = Mutex::new(0.0);
    let sleep = |ms: u64| sleeps.lock().unwrap().push(ms);
    let progress = |f: f64| *last.lock().unwrap() = f;
    let total = total_parts(size).unwrap();
    let job = Job { link, file: &file, size, file_id: 42, sleep: &sleep, progress: &progress };
    let parts = Mutex::new(Parts::new(total));
    let r = upload::run(&job, &parts, &doc(total));
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
    let f = *last.lock().unwrap();
    (r, f)
}

#[test]
fn chunks_cover_the_file_in_512k_parts() {
    assert_eq!(total_parts(0), None);
    assert_eq!(total_parts(1), Some(1));
    assert_eq!(total_parts(PART_SIZE), Some(1));
    assert_eq!(total_parts(PART_SIZE + 1), Some(2));
    assert_eq!(total_parts(51 * 1024 * 1024), Some(102));
    assert_eq!(total_parts(2000 * 1024 * 1024), Some(4000));
    assert_eq!(total_parts(4000 * PART_SIZE + 1), None);
    assert_eq!(part_range(PART_SIZE * 2 + 10, 2), (PART_SIZE * 2, 10));
    assert_eq!(part_range(PART_SIZE * 2 + 10, 0), (0, PART_SIZE as usize));
    assert_eq!((backoff_ms(1), backoff_ms(2), backoff_ms(9)), (1000, 2000, 8000));
}

#[test]
fn every_part_goes_up_once_with_its_own_bytes() {
    let link = Mock::default();
    let sleeps = Mutex::new(Vec::new());
    let (r, last) = run_with(&link, PART_SIZE * 3 + 7, &sleeps);
    assert_eq!(r, Ok(()));
    assert_eq!(last, 1.0);
    let mut parts = link.parts.lock().unwrap().clone();
    parts.sort_by_key(|p| p.1);
    assert_eq!(parts.iter().map(|p| (p.0, p.1, p.2, p.3, p.4)).collect::<Vec<_>>(), vec![(42, 0, 4, PART_SIZE as usize, 0), (42, 1, 4, PART_SIZE as usize, 1), (42, 2, 4, PART_SIZE as usize, 2), (42, 3, 4, 7, 3)]);
    assert_eq!(link.sent.lock().unwrap().len(), 1);
    assert!(sleeps.lock().unwrap().is_empty());
}

#[test]
fn flood_wait_sleeps_then_retries_the_part() {
    let link = Mock::default();
    link.part_errors.lock().unwrap().insert(1, VecDeque::from([MtError::rpc(420, "FLOOD_WAIT", Some(3))]));
    link.part_errors.lock().unwrap().insert(2, VecDeque::from([MtError::Net, MtError::Net]));
    let sleeps = Mutex::new(Vec::new());
    let (r, _) = run_with(&link, PART_SIZE * 3, &sleeps);
    assert_eq!(r, Ok(()));
    assert_eq!(link.part_ids(), vec![0, 1, 2]);
    let mut s = sleeps.lock().unwrap().clone();
    s.sort();
    assert_eq!(s, vec![1000, 2000, 3000]);
}

#[test]
fn a_long_flood_wait_or_endless_network_error_gives_up() {
    let link = Mock::default();
    link.part_errors.lock().unwrap().insert(0, VecDeque::from([MtError::rpc(420, "FLOOD_WAIT", Some(600))]));
    let (r, _) = run_with(&link, 10, &Mutex::new(Vec::new()));
    let e = r.unwrap_err();
    assert_eq!((classify_upload(&e), classify_user(&e)), ("failed", "flood"));
    let link = Mock::default();
    *link.fail_from.lock().unwrap() = Some(0);
    let sleeps = Mutex::new(Vec::new());
    let (r, _) = run_with(&link, 10, &sleeps);
    assert_eq!(r, Err(MtError::Net));
    assert_eq!(sleeps.lock().unwrap().len(), upload::PART_TRIES as usize - 1);
    assert!(link.sent.lock().unwrap().is_empty());
}

#[test]
fn a_missing_part_is_sent_again_before_the_document() {
    let link = Mock::default();
    link.send_errors.lock().unwrap().extend([MtError::rpc(400, "FILE_PART_MISSING", Some(2)), MtError::rpc(400, "RANDOM_ID_DUPLICATE", None)]);
    let (r, _) = run_with(&link, PART_SIZE * 3, &Mutex::new(Vec::new()));
    assert_eq!(r, Ok(()));
    assert_eq!(link.part_ids(), vec![0, 1, 2, 2]);
    let sent = link.sent.lock().unwrap();
    assert_eq!(sent.len(), 2);
    assert_eq!(sent[0].random_id, sent[1].random_id);
}

#[test]
fn errors_map_like_the_gramjs_classifiers() {
    let r = |name: &str| MtError::rpc(400, name, None);
    let up: Vec<&str> = [r("AUTH_KEY_UNREGISTERED"), r("SESSION_REVOKED"), r("ACCESS_TOKEN_INVALID"), r("CHANNEL_PRIVATE"), r("CHAT_WRITE_FORBIDDEN"), r("TOPIC_CLOSED"), MtError::Net, MtError::rpc(500, "RPC_CALL_FAIL", None), r("FLOOD_WAIT"), MtError::rpc(401, "UNAUTHORIZED", None), r("WHATEVER")].iter().map(classify_upload).collect();
    assert_eq!(up, vec!["auth", "auth", "auth", "chat", "chat", "chat", "network", "network", "failed", "auth", "failed"]);
    let user: Vec<&str> = [r("FLOOD_WAIT"), r("PHONE_PASSWORD_FLOOD"), r("PHONE_NUMBER_INVALID"), r("PHONE_CODE_INVALID"), r("PHONE_CODE_EXPIRED"), r("PASSWORD_HASH_INVALID"), r("SRP_ID_INVALID"), r("SESSION_REVOKED"), r("SESSION_EXPIRED"), MtError::Net, r("OTHER")].iter().map(classify_user).collect();
    assert_eq!(user, vec!["flood", "flood", "phone", "code", "code", "password", "password", "auth", "auth", "network", "failed"]);
    assert_eq!(MtError::rpc(420, "FLOOD_WAIT", Some(31)).flood_wait(), Some(31));
    assert_eq!(MtError::rpc(420, "SLOWMODE_WAIT", Some(5)).flood_wait(), Some(5));
    assert_eq!(MtError::Net.code(), "NETWORK");
}

#[test]
fn caption_html_becomes_text_and_utf16_entities() {
    let (text, ents) = parse("📝 <b>a&lt;b</b>\nx &amp; <i>y</i> <a href=\"https://e.x/?a=1&amp;b=2\">l</a> <blockquote expandable>q</blockquote> <x>z</x> 1 < 2");
    assert_eq!(text, "📝 a<b\nx & y l q z 1 < 2");
    let got: Vec<(Kind, i32, i32)> = ents.iter().map(|e| (e.kind, e.offset, e.length)).collect();
    assert_eq!(got, vec![(Kind::Bold, 3, 3), (Kind::Italic, 11, 1), (Kind::Url, 13, 1), (Kind::Quote, 15, 1)]);
    assert_eq!(ents[2].url.as_deref(), Some("https://e.x/?a=1&b=2"));
    assert!(ents[3].collapsed);
    assert_eq!(parse("<b></b>plain").1, vec![]);
}

#[test]
fn session_blob_round_trips_and_needs_the_home_key() {
    let mut data = grammers_session::SessionData::default();
    let mut dc = data.dc_options.get(&2).cloned().unwrap();
    dc.auth_key = Some([7u8; 256]);
    data.dc_options.insert(2, dc.clone());
    let text = blob::encode(2, &[dc.clone()]);
    let back = blob::decode(&text).unwrap();
    assert_eq!(back.home_dc, 2);
    assert_eq!(back.dc_options.get(&2), Some(&dc));
    assert!(blob::decode(&blob::encode(4, &[dc])).is_none());
    assert!(blob::decode("1BQANOTEBu2gGramJS").is_none());
}

#[test]
fn chat_ids_map_to_peers() {
    assert_eq!(chat_peer("-1001234567"), Some(Peer::Channel { id: 1234567, hash: 0 }));
    assert_eq!(chat_peer("-42"), Some(Peer::Chat(42)));
    assert_eq!(chat_peer("42"), None);
    assert_eq!(chat_peer("-100x"), None);
    assert_eq!(display_name(Some("Jin"), Some("Vu"), None), "Jin Vu");
    assert_eq!(display_name(None, None, Some("jv")), "@jv");
    assert_eq!(display_name(Some("a\n\tb"), None, None), "a b");
}

#[test]
fn login_goes_phone_code_password_and_expires() {
    let link = Arc::new(Mock::default());
    *link.session.lock().unwrap() = Some("user-session".into());
    link.sign.lock().unwrap().extend([Err(MtError::rpc(400, "PHONE_CODE_INVALID", None)), Err(MtError::rpc(401, "SESSION_PASSWORD_NEEDED", None))]);
    link.password.lock().unwrap().extend([Err(MtError::rpc(400, "PASSWORD_HASH_INVALID", None)), Ok("Jin Vu".into())]);
    let conn = MockConn::with(vec![link.clone()]);
    let logins = Logins::default();
    assert_eq!(logins.sign_in("1", 0), Outcome::Fail("expired"));
    assert_eq!(logins.send_code(&*conn, 1, "h", "+84", 0), Outcome::Next("code"));
    assert_eq!(logins.sign_in("1", 10), Outcome::Fail("code"));
    assert_eq!(logins.sign_in("2", 20), Outcome::Next("password"));
    assert_eq!(logins.password("bad", 30), Outcome::Fail("password"));
    assert_eq!(logins.password("good", 40), Outcome::Done { session: "user-session".into(), name: "Jin Vu".into() });
    assert_eq!(link.did("sign:hash"), 2);
    assert!(link.closed.load(std::sync::atomic::Ordering::SeqCst));
    assert_eq!(logins.password("good", 50), Outcome::Fail("expired"));
    let slow = Arc::new(Mock::default());
    let conn = MockConn::with(vec![slow.clone()]);
    assert_eq!(logins.send_code(&*conn, 1, "h", "+84", 0), Outcome::Next("code"));
    assert_eq!(logins.sign_in("1", LOGIN_TTL_MS + 1), Outcome::Fail("expired"));
    assert!(slow.closed.load(std::sync::atomic::Ordering::SeqCst));
    let signup = Arc::new(Mock::default());
    signup.sign.lock().unwrap().push_back(Ok(SignIn::SignUp));
    let conn = MockConn::with(vec![signup]);
    logins.send_code(&*conn, 1, "h", "+84", 0);
    assert_eq!(logins.sign_in("1", 1), Outcome::Fail("phone"));
    let bad = Arc::new(Mock::default());
    bad.code.lock().unwrap().push_back(Err(MtError::rpc(400, "PHONE_NUMBER_INVALID", None)));
    assert_eq!(logins.send_code(&*MockConn::with(vec![bad]), 1, "h", "x", 0), Outcome::Fail("phone"));
    assert!(!logins.active(0));
}

#[test]
fn topic_scan_pages_with_offsets_and_skips_general() {
    let link = Arc::new(Mock::default());
    *link.chats.lock().unwrap() = vec![ForumChat { id: 55, hash: 1, title: "Group".into() }];
    let page = |from: i32, to: i32| TopicPage { topics: (from..=to).map(|id| Topic { id, title: format!("t{id}"), top_message: id * 10 }).collect(), raw_len: (to - from + 1) as usize, dates: vec![(1000, 777)] };
    link.pages.lock().unwrap().extend([Ok(page(1, 100)), Ok(page(101, 130))]);
    let chats = login::topics(&*MockConn::with(vec![link.clone()]), 1, "h", "s").unwrap();
    assert_eq!(chats.len(), 1);
    assert_eq!(chats[0]["chatId"], "-10055");
    let topics = chats[0]["topics"].as_array().unwrap();
    assert_eq!(topics.len(), 129);
    assert_eq!(topics[0]["threadId"], 2);
    assert_eq!(*link.offsets.lock().unwrap(), vec![Offset::default(), Offset { topic: 100, id: 1000, date: 777 }]);
    let broken = Arc::new(Mock::default());
    *broken.chats.lock().unwrap() = vec![ForumChat { id: 1, hash: 1, title: "G".into() }];
    broken.pages.lock().unwrap().push_back(Err(MtError::rpc(401, "AUTH_KEY_UNREGISTERED", None)));
    assert_eq!(login::topics(&*MockConn::with(vec![broken]), 1, "h", "s"), Err("auth"));
}

#[test]
fn upload_counts_parts_across_workers() {
    let link = Mock::default();
    let counter = AtomicUsize::new(0);
    let file = temp_file(PART_SIZE * 9);
    let progress = |_f: f64| {
        counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    };
    let job = Job { link: &link, file: &file, size: PART_SIZE * 9, file_id: 1, sleep: &|_| {}, progress: &progress };
    let parts = Mutex::new(Parts::new(9));
    upload::upload_parts(&job, &parts, vec![0, 4, 8]).unwrap();
    assert_eq!(parts.lock().unwrap().missing(), vec![1, 2, 3, 5, 6, 7]);
    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 3);
    let _ = std::fs::remove_dir_all(file.parent().unwrap());
}
