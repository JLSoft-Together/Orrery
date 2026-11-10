use super::api::{Seen, Transport};
use super::buttons::ReplyButtons;
use super::clock::Local;
use super::html::{DayCounts, Masker};
use super::message::TgMessage;
use super::shape::{Route, Shape};
use crate::phone::send::RateLimiter;
use crate::DeskError;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex, MutexGuard};

pub const EVENT: &str = "ccm:telegram-event";
pub const TIMEOUT_MS: u64 = 15_000;
pub const UPLOAD_TIMEOUT_MS: u64 = 180_000;
pub const PER_CHAT_MIN: usize = 20;
pub const PER_HOUR: usize = 120;
pub const INBOUND_PER_CHAT_MIN: usize = 10;
pub const HOLD_MAX_S: f64 = 3600.0;
pub const HOLD_DEFAULT_S: f64 = 5.0;
pub const TOPIC_HEALTH_MAX: usize = 200;
pub const TOPIC_PUSH_MS: i64 = 30_000;
pub const ACTIONS_MAX: usize = 500;
pub const IGNORED_STATE_MS: f64 = 1000.0;
pub const NOT_OWNER_PER_MIN: usize = 5;
pub const STRANGERS_SEEN_MAX: usize = 20;
pub const SEND_COMMAND: &str = "bot-tele-send-notify";
pub const RECEIVE_COMMAND: &str = "bot-tele-receive";

pub type HostFn = Arc<dyn Fn(&str, Value) -> Result<Value, DeskError> + Send + Sync>;
pub type EmitFn = Arc<dyn Fn(&str, Value) + Send + Sync>;

#[derive(Clone)]
pub struct Deps {
    pub file: Option<PathBuf>,
    pub base: String,
    pub masker: Masker,
    pub win: bool,
    pub claude_root: PathBuf,
    pub http: Arc<dyn Transport>,
    pub host: HostFn,
    pub emit: EmitFn,
    pub clock: Arc<dyn Fn() -> i64 + Send + Sync>,
    pub sleep: Arc<dyn Fn(u64) + Send + Sync>,
    pub local: Arc<dyn Fn(i64) -> Local + Send + Sync>,
    pub album_ms: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Terminal(String),
    Pane(String),
}

impl Target {
    pub fn value(&self) -> Value {
        match self {
            Target::Terminal(t) => json!({"kind": "terminal", "terminalId": t}),
            Target::Pane(p) => json!({"kind": "pane", "sessionId": p}),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Entry {
    pub kind: String,
    pub terminal_id: String,
    pub project: String,
    pub cwd: String,
    pub name: String,
    pub chat_id: String,
    pub thread_id: Option<i64>,
    pub extra: Option<String>,
    pub label: Option<String>,
    pub since: Option<f64>,
    pub tool: Option<String>,
    pub at: Option<i64>,
    pub by: Option<i64>,
    pub group: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Held {
    pub route: Route,
    pub html: String,
    pub target: Option<Target>,
    pub keys: Option<Value>,
    pub important: bool,
}

#[derive(Clone, Default)]
pub struct Health5 {
    pub last_sent: Option<i64>,
    pub last_received: Option<i64>,
    pub last_poll: Option<i64>,
    pub sent_today: i64,
    pub errors_today: i64,
}

pub struct RunWatch {
    pub project: String,
    pub chat_id: String,
    pub thread_id: Option<i64>,
    pub config: String,
    pub shot: bool,
}

pub struct DigestMsg {
    pub chat_id: String,
    pub message_id: i64,
    pub day: String,
    pub n: i64,
}

pub struct St {
    pub data: Shape,
    pub total: RateLimiter,
    pub per_chat: HashMap<String, RateLimiter>,
    pub sent: VecDeque<String>,
    pub sent_set: HashSet<String>,
    pub reported_at: HashMap<String, i64>,
    pub last_error: Option<&'static str>,
    pub bot_id: Option<i64>,
    pub receiving: bool,
    pub poll_run: u64,
    pub poll_conflict: bool,
    pub commands_busy: bool,
    pub conflict_runs: u32,
    pub backlogs: Vec<(String, Vec<Value>)>,
    pub poll_seen: Option<Seen>,
    pub pair_code: Option<String>,
    pub pair_until: Option<i64>,
    pub ignored: i64,
    pub inbound_limit: HashMap<String, RateLimiter>,
    pub sent_to: Vec<(String, Target)>,
    pub actions: Vec<(String, Entry)>,
    pub counts: Vec<(String, DayCounts)>,
    pub counts_day: String,
    pub started_at: i64,
    pub pair_tries: HashMap<i64, u32>,
    pub pair_tries_total: u32,
    pub ignored_at: f64,
    pub not_owner_limit: RateLimiter,
    pub stranger_limit: RateLimiter,
    pub read_only_limit: RateLimiter,
    pub strangers: Vec<(i64, Value)>,
    pub run_failed: VecDeque<String>,
    pub last_replies: Vec<(String, (String, String))>,
    pub run_watch: Vec<(String, RunWatch)>,
    pub pin_text: HashMap<String, String>,
    pub digest_msgs: Vec<(String, DigestMsg)>,
    pub pin_busy: bool,
    pub pin_at: HashMap<String, i64>,
    pub hold_until: HashMap<String, i64>,
    pub held: Vec<(String, Vec<Held>)>,
    pub held_timers: HashMap<String, u64>,
    pub timer_gen: u64,
    pub flushing: bool,
    pub topic_saved_at: i64,
    pub limit_sent: HashSet<String>,
    pub icon_busy: bool,
    pub icon_ids: Option<Vec<(&'static str, String)>>,
    pub icon_set: Vec<(String, (String, i64, String))>,
    pub icon_at: HashMap<String, i64>,
    pub icon_off: HashSet<String>,
    pub health: Health5,
    pub health_day: String,
    pub agent_of_pane: HashMap<String, String>,
    pub agent_of_terminal: HashMap<String, String>,
    pub terminal_of_agent: HashMap<String, String>,
    pub open_sessions: Vec<(String, String, String)>,
    pub ended_topics: HashMap<String, String>,
    pub panes_seen: bool,
    pub swept: bool,
    pub albums: HashMap<String, (Vec<TgMessage>, u64)>,
    pub heavy_running: usize,
    pub reply_buttons: ReplyButtons,
    pub unsealed: HashMap<String, Option<String>>,
    pub uploads: HashMap<u64, Arc<dyn Fn(f64) + Send + Sync>>,
    pub upload_next: u64,
}

pub struct Tg {
    pub deps: Deps,
    pub st: Mutex<St>,
    pub heavy: Condvar,
    pub light: Mutex<Option<std::sync::mpsc::Sender<Value>>>,
}

pub fn topic_key(chat: &str, thread: Option<i64>) -> String {
    match thread {
        Some(t) => format!("{chat}:{t}"),
        None => format!("{chat}:null"),
    }
}

pub fn outgoing(method: &str) -> bool {
    (method.starts_with("send") && !method.starts_with("sendChatAction")) || method.starts_with("editMessage") || method == "pinChatMessage" || method == "unpinChatMessage" || method == "editForumTopic"
}

impl Tg {
    pub fn new(deps: Deps) -> Arc<Tg> {
        let data = super::shape::read(deps.file.as_deref());
        let now = (deps.clock)();
        let st = St {
            data,
            total: RateLimiter::new(PER_HOUR, 3_600_000.0),
            per_chat: HashMap::new(),
            sent: VecDeque::new(),
            sent_set: HashSet::new(),
            reported_at: HashMap::new(),
            last_error: None,
            bot_id: None,
            receiving: false,
            poll_run: 0,
            poll_conflict: false,
            commands_busy: false,
            conflict_runs: 0,
            backlogs: Vec::new(),
            poll_seen: None,
            pair_code: None,
            pair_until: None,
            ignored: 0,
            inbound_limit: HashMap::new(),
            sent_to: Vec::new(),
            actions: Vec::new(),
            counts: Vec::new(),
            counts_day: String::new(),
            started_at: now,
            pair_tries: HashMap::new(),
            pair_tries_total: 0,
            ignored_at: f64::NEG_INFINITY,
            not_owner_limit: RateLimiter::new(NOT_OWNER_PER_MIN, 60_000.0),
            stranger_limit: RateLimiter::new(NOT_OWNER_PER_MIN, 60_000.0),
            read_only_limit: RateLimiter::new(NOT_OWNER_PER_MIN, 60_000.0),
            strangers: Vec::new(),
            run_failed: VecDeque::new(),
            last_replies: Vec::new(),
            run_watch: Vec::new(),
            pin_text: HashMap::new(),
            digest_msgs: Vec::new(),
            pin_busy: false,
            pin_at: HashMap::new(),
            hold_until: HashMap::new(),
            held: Vec::new(),
            held_timers: HashMap::new(),
            timer_gen: 0,
            flushing: false,
            topic_saved_at: 0,
            limit_sent: HashSet::new(),
            icon_busy: false,
            icon_ids: None,
            icon_set: Vec::new(),
            icon_at: HashMap::new(),
            icon_off: HashSet::new(),
            health: Health5::default(),
            health_day: String::new(),
            agent_of_pane: HashMap::new(),
            agent_of_terminal: HashMap::new(),
            terminal_of_agent: HashMap::new(),
            open_sessions: Vec::new(),
            ended_topics: HashMap::new(),
            panes_seen: false,
            swept: false,
            albums: HashMap::new(),
            heavy_running: 0,
            reply_buttons: ReplyButtons::default(),
            unsealed: HashMap::new(),
            uploads: HashMap::new(),
            upload_next: 1,
        };
        Arc::new(Tg { deps, st: Mutex::new(st), heavy: Condvar::new(), light: Mutex::new(None) })
    }

    pub fn lock(&self) -> MutexGuard<'_, St> {
        self.st.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn now(&self) -> i64 {
        (self.deps.clock)()
    }

    pub fn local(&self, ms: i64) -> Local {
        (self.deps.local)(ms)
    }

    pub fn day_key(&self, ms: i64) -> String {
        let l = self.local(ms);
        format!("{:04}-{:02}-{:02}", l.y, l.mo, l.d)
    }

    pub fn host(&self, method: &str, params: Value) -> Option<Value> {
        (self.deps.host)(method, params).ok()
    }

    pub fn vi(&self) -> bool {
        !matches!(self.host("telegram.locale", Value::Null), Some(Value::String(s)) if s == "en")
    }

    pub fn keychain(&self) -> bool {
        matches!(self.host("keychain.available", Value::Null), Some(Value::Bool(true)))
    }

    pub fn seal(&self, plain: &str) -> Option<String> {
        match self.host("keychain.seal", json!({"plain": plain})) {
            Some(Value::String(s)) if !s.is_empty() => {
                self.lock().unsealed.insert(s.clone(), Some(plain.to_string()));
                Some(s)
            }
            _ => None,
        }
    }

    pub fn unseal(&self, sealed: Option<&str>) -> Option<String> {
        let sealed = sealed.filter(|s| !s.is_empty())?;
        if let Some(hit) = self.lock().unsealed.get(sealed) {
            return hit.clone();
        }
        let plain = match self.host("keychain.unseal", json!({"sealed": sealed})) {
            Some(Value::String(s)) => Some(s),
            _ => None,
        };
        self.lock().unsealed.insert(sealed.to_string(), plain.clone());
        plain
    }

    pub fn token(&self) -> Option<String> {
        let sealed = self.lock().data.token.clone();
        self.unseal(sealed.as_deref())
    }

    pub fn emit_state(&self) {
        let state = self.state();
        (self.deps.emit)(super::base::EVENT, json!({"state": state}));
    }

}

pub fn strip_tags(t: &str) -> String {
    let mut out = String::new();
    let mut rest = t;
    while let Some(at) = rest.find('<') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        match after.find('>') {
            Some(end) if end > 0 => rest = &after[end + 1..],
            _ => {
                out.push('<');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}
