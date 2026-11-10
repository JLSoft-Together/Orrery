use super::grammers_tl::{doc_request, entity_user, forum_chats_page, topic_page};
use super::{blob, Connector, Doc, ForumChat, Link, MtError, Offset, Sent, SignIn, TopicPage};
use grammers_mtsender::{ConnectionParams, InvocationError, NoRetries, SenderPool, SenderPoolHandle};
use grammers_session::storages::MemorySession;
use grammers_session::Session;
use grammers_tl_types as tl;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;
use tokio::runtime::{Handle, Runtime};
use tokio::task::JoinHandle;

pub const CALL_TIMEOUT: Duration = Duration::from_secs(60);
const DC_IDS: [i32; 5] = [1, 2, 3, 4, 5];

#[derive(Default)]
pub struct Grammers {
    rt: OnceLock<Option<Runtime>>,
}

impl Grammers {
    fn handle(&self) -> Option<Handle> {
        let rt = self.rt.get_or_init(|| tokio::runtime::Builder::new_multi_thread().worker_threads(1).thread_name("orrery-mtproto").enable_all().build().ok());
        rt.as_ref().map(|r| r.handle().clone())
    }
}

pub fn map_err(e: InvocationError) -> MtError {
    match e {
        InvocationError::Rpc(r) => MtError::Rpc { code: r.code, name: r.name, value: r.value },
        InvocationError::Transport(grammers_mtproto::transport::Error::BadStatus { status: 404 }) => MtError::rpc(401, "AUTH_KEY_UNREGISTERED", None),
        InvocationError::Io(_) | InvocationError::Transport(_) | InvocationError::Dropped(_) | InvocationError::Authentication(_) => MtError::Net,
        InvocationError::Deserialize(_) => MtError::Other("decode"),
        InvocationError::Session(_) | InvocationError::InvalidDc => MtError::Other("session"),
    }
}

impl Connector for Grammers {
    fn connect(&self, api_id: i32, api_hash: &str, session: Option<&str>) -> Result<Arc<dyn Link>, MtError> {
        let rt = self.handle().ok_or(MtError::Other("runtime"))?;
        let data = match session {
            Some(s) => blob::decode(s).ok_or_else(|| MtError::rpc(401, "AUTH_KEY_UNREGISTERED", None))?,
            None => Default::default(),
        };
        let session = Arc::new(MemorySession::from(data));
        let params = ConnectionParams { retry_policy: Box::new(NoRetries), app_version: env!("CARGO_PKG_VERSION").to_string(), ..Default::default() };
        let SenderPool { runner, handle, updates } = SenderPool::with_configuration(session.clone(), api_id, params);
        let task = rt.spawn(runner.run());
        Ok(Arc::new(GLink { rt, pool: handle.thin.clone(), session, api_id, api_hash: api_hash.to_string(), task: Mutex::new(Some(task)), _updates: Mutex::new(Some(updates)) }))
    }
}

type Updates = tokio::sync::mpsc::Receiver<grammers_session::updates::UpdatesLike>;

struct GLink {
    rt: Handle,
    pool: SenderPoolHandle,
    session: Arc<MemorySession>,
    api_id: i32,
    api_hash: String,
    task: Mutex<Option<JoinHandle<()>>>,
    _updates: Mutex<Option<Updates>>,
}

impl GLink {
    fn call<R: tl::RemoteCall>(&self, req: &R) -> Result<R::Return, MtError> {
        let dc = self.session.home_dc_id().map_err(|_| MtError::Other("session"))?;
        self.rt.block_on(async {
            match tokio::time::timeout(CALL_TIMEOUT, self.pool.invoke_in_dc(dc, req)).await {
                Ok(r) => r.map_err(map_err),
                Err(_) => Err(MtError::Net),
            }
        })
    }

    fn call_home<R: tl::RemoteCall>(&self, req: &R) -> Result<R::Return, MtError> {
        match self.call(req) {
            Err(MtError::Rpc { code: 303, value: Some(dc), .. }) => {
                let old = self.session.home_dc_id().map_err(|_| MtError::Other("session"))?;
                self.pool.disconnect_from_dc(old);
                self.rt.block_on(self.session.set_home_dc_id(dc as i32)).map_err(|_| MtError::Other("session"))?;
                self.call(req)
            }
            r => r,
        }
    }

    fn authorized(&self, auth: tl::enums::auth::Authorization) -> Result<String, MtError> {
        match auth {
            tl::enums::auth::Authorization::Authorization(a) => Ok(entity_user(&a.user)),
            tl::enums::auth::Authorization::SignUpRequired(_) => Err(MtError::rpc(400, "PHONE_NUMBER_UNOCCUPIED", None)),
        }
    }
}

impl Link for GLink {
    fn bot_sign_in(&self, token: &str) -> Result<(), MtError> {
        let req = tl::functions::auth::ImportBotAuthorization { flags: 0, api_id: self.api_id, api_hash: self.api_hash.clone(), bot_auth_token: token.to_string() };
        self.call_home(&req).and_then(|a| self.authorized(a)).map(|_| ())
    }

    fn save_part(&self, file_id: i64, part: i32, total: i32, bytes: Vec<u8>) -> Result<(), MtError> {
        let ok = self.call(&tl::functions::upload::SaveBigFilePart { file_id, file_part: part, file_total_parts: total, bytes })?;
        if ok { Ok(()) } else { Err(MtError::Other("part")) }
    }

    fn resolve_channel(&self, id: i64) -> Result<i64, MtError> {
        let req = tl::functions::channels::GetChannels { id: vec![tl::enums::InputChannel::Channel(tl::types::InputChannel { channel_id: id, access_hash: 0 })] };
        let chats = match self.call(&req)? {
            tl::enums::messages::Chats::Chats(c) => c.chats,
            tl::enums::messages::Chats::Slice(c) => c.chats,
        };
        chats
            .into_iter()
            .find_map(|c| match c {
                tl::enums::Chat::Channel(ch) if ch.id == id => Some(ch.access_hash.unwrap_or(0)),
                _ => None,
            })
            .ok_or_else(|| MtError::rpc(400, "CHANNEL_INVALID", None))
    }

    fn send_document(&self, doc: &Doc) -> Result<(), MtError> {
        self.call(&doc_request(doc)).map(|_| ())
    }

    fn send_code(&self, phone: &str) -> Result<Sent, MtError> {
        let settings = tl::types::CodeSettings { allow_flashcall: false, current_number: false, allow_app_hash: false, allow_missed_call: false, allow_firebase: false, unknown_number: false, logout_tokens: None, token: None, app_sandbox: None };
        let req = tl::functions::auth::SendCode { phone_number: phone.to_string(), api_id: self.api_id, api_hash: self.api_hash.clone(), settings: settings.into() };
        match self.call_home(&req)? {
            tl::enums::auth::SentCode::Code(c) => Ok(Sent::Code(c.phone_code_hash)),
            tl::enums::auth::SentCode::Success(s) => self.authorized(s.authorization).map(Sent::Done),
            tl::enums::auth::SentCode::PaymentRequired(_) => Err(MtError::Other("payment")),
        }
    }

    fn sign_in(&self, phone: &str, hash: &str, code: &str) -> Result<SignIn, MtError> {
        let req = tl::functions::auth::SignIn { phone_number: phone.to_string(), phone_code_hash: hash.to_string(), phone_code: Some(code.to_string()), email_verification: None };
        match self.call_home(&req)? {
            tl::enums::auth::Authorization::SignUpRequired(_) => Ok(SignIn::SignUp),
            a => self.authorized(a).map(SignIn::Done),
        }
    }

    fn check_password(&self, password: &str) -> Result<String, MtError> {
        let tl::enums::account::Password::Password(p) = self.call(&tl::functions::account::GetPassword {})?;
        let bad = || MtError::rpc(400, "PASSWORD_HASH_INVALID", None);
        let Some(tl::enums::PasswordKdfAlgo::Sha256Sha256Pbkdf2Hmacsha512iter100000Sha256ModPow(algo)) = p.current_algo.as_ref() else { return Err(bad()) };
        let (Some(g_b), Some(srp_id)) = (p.srp_b.clone(), p.srp_id) else { return Err(bad()) };
        if !grammers_crypto::two_factor_auth::check_p_and_g(&algo.p, &algo.g) {
            return Err(MtError::Other("srp"));
        }
        let (m1, g_a) = grammers_crypto::two_factor_auth::calculate_2fa(&algo.salt1, &algo.salt2, &algo.p, &algo.g, g_b, p.secure_random.clone(), password);
        let srp = tl::enums::InputCheckPasswordSrp::Srp(tl::types::InputCheckPasswordSrp { srp_id, a: g_a.to_vec(), m1: m1.to_vec() });
        let auth = self.call(&tl::functions::auth::CheckPassword { password: srp })?;
        self.authorized(auth)
    }

    fn forum_chats(&self, max_dialogs: usize) -> Result<Vec<ForumChat>, MtError> {
        let mut out = Vec::new();
        let mut offset = (0, 0, tl::enums::InputPeer::Empty);
        let mut seen = 0usize;
        while seen < max_dialogs {
            let limit = (max_dialogs - seen).min(100) as i32;
            let req = tl::functions::messages::GetDialogs { exclude_pinned: false, folder_id: None, offset_date: offset.0, offset_id: offset.1, offset_peer: offset.2.clone(), limit, hash: 0 };
            let page = forum_chats_page(self.call(&req)?);
            seen += page.count;
            out.extend(page.chats);
            match page.next {
                Some(next) if page.count >= limit as usize => offset = next,
                _ => break,
            }
        }
        Ok(out)
    }

    fn forum_topics(&self, chat: &ForumChat, at: Offset, limit: i32) -> Result<TopicPage, MtError> {
        let peer = tl::enums::InputPeer::Channel(tl::types::InputPeerChannel { channel_id: chat.id, access_hash: chat.hash });
        let req = tl::functions::messages::GetForumTopics { peer, q: None, offset_date: at.date, offset_id: at.id, offset_topic: at.topic, limit };
        Ok(topic_page(self.call(&req)?))
    }

    fn log_out(&self) -> Result<(), MtError> {
        self.call(&tl::functions::auth::LogOut {}).map(|_| ())
    }

    fn export(&self) -> Option<String> {
        let home = self.session.home_dc_id().ok()?;
        let dcs: Vec<_> = DC_IDS.iter().filter_map(|id| self.session.dc_option(*id).ok().flatten()).collect();
        dcs.iter().any(|d| d.id == home && d.auth_key.is_some()).then(|| blob::encode(home, &dcs))
    }

    fn close(&self) {
        self.pool.quit();
        if let Some(t) = self.task.lock().unwrap_or_else(|e| e.into_inner()).take() {
            t.abort();
        }
    }
}

impl Drop for GLink {
    fn drop(&mut self) {
        self.close();
    }
}
