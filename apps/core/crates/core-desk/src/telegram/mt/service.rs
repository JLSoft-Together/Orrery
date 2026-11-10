use super::login::Logins;
use super::store::{self, Saved};
use super::upload::{self, Job, Parts};
use super::{chat_peer, classify_upload, html, random_i64, Connector, Doc, Link, MtError, Peer};
use crate::telegram::base::Tg;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

pub const IDLE_MS: u64 = 120_000;
pub const RECONNECTS: u32 = 2;

pub struct Service {
    pub file: Option<PathBuf>,
    saved: Mutex<Saved>,
    pub logins: Logins,
    bot: Mutex<Option<(String, Arc<dyn Link>)>>,
    gen: AtomicU64,
    queue: Mutex<()>,
    pub noticed: AtomicBool,
}

impl Service {
    pub fn new(telegram_json: Option<&Path>) -> Self {
        let file = store::path_for(telegram_json);
        let saved = store::read(file.as_deref());
        Service { file, saved: Mutex::new(saved), logins: Logins::default(), bot: Mutex::new(None), gen: AtomicU64::new(0), queue: Mutex::new(()), noticed: AtomicBool::new(false) }
    }

    pub fn saved(&self) -> MutexGuard<'_, Saved> {
        self.saved.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn update(&self, f: impl FnOnce(&mut Saved)) -> bool {
        let mut s = self.saved();
        f(&mut s);
        store::write(self.file.as_deref(), &s)
    }

    fn cached(&self) -> MutexGuard<'_, Option<(String, Arc<dyn Link>)>> {
        self.bot.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn drop_link(&self) {
        if let Some((_, link)) = self.cached().take() {
            link.close();
        }
    }

    pub fn has_link(&self) -> bool {
        self.cached().is_some()
    }
}

struct Up {
    api_id: i32,
    api_hash: String,
    token: String,
    fp: String,
    peer: Peer,
    thread: Option<i32>,
    file: PathBuf,
    size: u64,
    total: i32,
    name: String,
    caption: String,
    progress: u64,
}

fn parse_up(req: &Value, creds: &Value) -> Result<Up, &'static str> {
    let api_id = creds["apiId"].as_i64().and_then(|n| i32::try_from(n).ok()).filter(|n| *n > 0).ok_or("auth")?;
    let api_hash = creds["apiHash"].as_str().filter(|s| !s.is_empty()).ok_or("auth")?.to_string();
    let token = creds["token"].as_str().filter(|s| !s.is_empty()).ok_or("auth")?.to_string();
    let peer = req["chatId"].as_str().and_then(chat_peer).ok_or("chat")?;
    let size = req["size"].as_u64().unwrap_or(0);
    let total = upload::total_parts(size).ok_or("failed")?;
    let file = PathBuf::from(req["file"].as_str().filter(|s| !s.is_empty()).ok_or("failed")?);
    Ok(Up {
        fp: store::fingerprint(api_id as i64, &api_hash, &token),
        api_id,
        api_hash,
        token,
        peer,
        thread: req["threadId"].as_i64().and_then(|n| i32::try_from(n).ok()).filter(|n| *n > 0),
        file,
        size,
        total,
        name: req["name"].as_str().unwrap_or("file").to_string(),
        caption: req["caption"].as_str().unwrap_or("").to_string(),
        progress: req["progress"].as_u64().unwrap_or(0),
    })
}

impl Tg {
    pub fn mt_upload(self: &Arc<Self>, c: &Arc<dyn Connector>, req: &Value, creds: &Value) -> Value {
        let result = match parse_up(req, creds) {
            Ok(up) => self.mt_upload_up(c, &up),
            Err(e) => json!({"ok": false, "error": e}),
        };
        json!({"result": result})
    }

    fn mt_upload_up(self: &Arc<Self>, c: &Arc<dyn Connector>, up: &Up) -> Value {
        let turn = self.mt.queue.lock().unwrap_or_else(|e| e.into_inner());
        let first = self.mt_try(c, up);
        let res = match first {
            Err((e, false)) if classify_upload(&e) == "auth" => {
                self.mt_forget_bot();
                self.mt_try(c, up).map_err(|(e, _)| e)
            }
            r => r.map_err(|(e, _)| e),
        };
        drop(turn);
        self.mt_idle();
        match res {
            Ok(()) => json!({"ok": true}),
            Err(e) => json!({"ok": false, "error": classify_upload(&e)}),
        }
    }

    fn mt_try(&self, c: &Arc<dyn Connector>, up: &Up) -> Result<(), (MtError, bool)> {
        let (mut link, fresh) = self.mt_link(c, up).map_err(|e| (e, true))?;
        let peer = match up.peer {
            Peer::Channel { id, .. } => Peer::Channel { id, hash: link.resolve_channel(id).map_err(|e| (e, fresh))? },
            p => p,
        };
        let (caption, entities) = html::parse(&up.caption);
        let doc = Doc { peer, thread: up.thread, file_id: random_i64(), parts: up.total, name: up.name.clone(), caption, entities, random_id: random_i64() };
        let parts = Mutex::new(Parts::new(up.total));
        let sleep = |ms: u64| (self.deps.sleep)(ms);
        let progress = |f: f64| self.upload_progress(up.progress, f);
        let mut reconnects = 0;
        loop {
            let job = Job { link: &*link, file: &up.file, size: up.size, file_id: doc.file_id, sleep: &sleep, progress: &progress };
            match upload::run(&job, &parts, &doc) {
                Err(MtError::Net) if reconnects < RECONNECTS => {
                    reconnects += 1;
                    self.mt.drop_link();
                    link = self.mt_link(c, up).map_err(|e| (e, fresh))?.0;
                }
                r => {
                    self.mt_save_bot(&*link, &up.fp);
                    return r.map_err(|e| (e, fresh));
                }
            }
        }
    }

    fn mt_link(&self, c: &Arc<dyn Connector>, up: &Up) -> Result<(Arc<dyn Link>, bool), MtError> {
        if let Some((k, l)) = self.mt.cached().as_ref() {
            if *k == up.fp {
                return Ok((l.clone(), false));
            }
        }
        self.mt.drop_link();
        let sealed = self.mt.saved().bot.clone().filter(|(k, _)| *k == up.fp).map(|(_, s)| s);
        let session = self.unseal(sealed.as_deref());
        let link = c.connect(up.api_id, &up.api_hash, session.as_deref())?;
        let fresh = session.is_none();
        if fresh {
            if let Err(e) = link.bot_sign_in(&up.token) {
                link.close();
                return Err(e);
            }
            self.mt_save_bot(&*link, &up.fp);
        }
        *self.mt.cached() = Some((up.fp.clone(), link.clone()));
        Ok((link, fresh))
    }

    fn mt_save_bot(&self, link: &dyn Link, fp: &str) {
        let Some(session) = link.export() else { return };
        let old = self.mt.saved().bot.clone().filter(|(k, _)| k == fp).map(|(_, s)| s);
        if old.is_some() && self.unseal(old.as_deref()).as_deref() == Some(session.as_str()) {
            return;
        }
        if !self.keychain() {
            return;
        }
        if let Some(sealed) = self.seal(&session) {
            self.mt.update(|s| s.bot = Some((fp.to_string(), sealed)));
        }
    }

    pub fn mt_forget_bot(&self) {
        self.mt.drop_link();
        self.mt.update(|s| s.bot = None);
    }

    fn mt_idle(self: &Arc<Self>) {
        let gen = self.mt.gen.fetch_add(1, Ordering::SeqCst) + 1;
        self.spawn(move |tg| {
            (tg.deps.sleep)(IDLE_MS);
            if tg.mt.gen.load(Ordering::SeqCst) == gen && tg.mt.queue.try_lock().is_ok() {
                tg.mt.drop_link();
            }
        });
    }
}
