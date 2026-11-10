mod extra;
mod observe;

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use core_store::extra::{JobState, NotifySettings, QuietHours, ToastHealth, ToastState, ToastTestResult};
use core_store::model::{TerminalSession, TerminalStatus};
use core_store::num::JsNum;
use core_store::persist::localtime::Zone;
use core_store::persist::notify_file::{NotifyFile, NotifyPrefs, ToastLocale, IDLE_CHOICES, STUCK_CHOICES, TOAST_KINDS};
use indexmap::IndexMap;
use serde_json::{Map, Value};

use crate::message::{HostMessage, PhoneEpisode, ToastContent};
use crate::text::Text;
use crate::util::{encode_uri_component, in_quiet_hours, random_key, same_key, toast_tag};

pub use extra::{error_loops, ErrorLoop};
pub use observe::ToastSnapshot;

pub const LONG_JOB_MS: f64 = 5.0 * 60_000.0;
pub const TOAST_THROTTLE_MS: f64 = 30_000.0;
pub const TOAST_SNOOZE_MIN: u64 = 15;

pub type ToastClock = Arc<dyn Fn() -> i64 + Send + Sync>;

#[derive(Debug, Clone, Default)]
pub struct ToastOptions {
    pub supported: bool,
    pub port: u16,
    pub file: Option<PathBuf>,
    pub link: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct NotifyPatch {
    pub toast: Option<bool>,
    pub kinds: IndexMap<String, bool>,
    pub quiet: Option<QuietHours>,
    pub stuck_minutes: Option<f64>,
    pub idle_minutes: Option<f64>,
}

impl NotifyPatch {
    pub fn from_client(toast: Option<bool>, kinds: Option<&Map<String, Value>>, quiet: Option<QuietHours>, stuck_minutes: Option<u64>, idle_minutes: Option<u64>) -> Self {
        let kinds = kinds.map(|k| k.iter().filter_map(|(n, v)| v.as_bool().map(|b| (n.clone(), b))).collect()).unwrap_or_default();
        Self { toast, kinds, quiet, stuck_minutes: stuck_minutes.map(|v| v as f64), idle_minutes: idle_minutes.map(|v| v as f64) }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TestOutcome {
    pub result: ToastTestResult,
    pub health: Option<ToastHealth>,
}

pub(crate) struct Draft {
    pub title: String,
    pub body: String,
    pub url: String,
    pub open: String,
    pub persistent: bool,
    pub snooze: bool,
}

pub struct Toaster {
    options: ToastOptions,
    file: NotifyFile,
    clock: ToastClock,
    pub zone: Zone,
    pub phone_on: bool,
    pub telegram_on: bool,
    pub health_probe: Option<Box<dyn Fn() -> ToastHealth + Send>>,
    health: Option<ToastHealth>,
    idle_sent: HashSet<String>,
    idle_primed: bool,
    stuck_sent: HashSet<String>,
    context_high: HashSet<String>,
    prev: IndexMap<String, TerminalStatus>,
    terms: IndexMap<String, TerminalSession>,
    shown: IndexMap<String, String>,
    last_at: HashMap<String, f64>,
    reminded: HashMap<String, f64>,
    job_prev: IndexMap<String, JobState>,
    loop_sent: HashSet<String>,
    limit_sent: HashSet<String>,
    limits_primed: bool,
    presence: IndexMap<u64, bool>,
    work_since: HashMap<String, f64>,
    outbox: Vec<HostMessage>,
}

impl Toaster {
    pub fn new(options: ToastOptions, clock: ToastClock) -> Self {
        let c = clock.clone();
        let file = NotifyFile::with_clock(options.file.clone(), random_key, Box::new(move || c()));
        Self {
            options,
            file,
            clock,
            zone: Zone::Local,
            phone_on: false,
            telegram_on: false,
            health_probe: None,
            health: None,
            idle_sent: HashSet::new(),
            idle_primed: false,
            stuck_sent: HashSet::new(),
            context_high: HashSet::new(),
            prev: IndexMap::new(),
            terms: IndexMap::new(),
            shown: IndexMap::new(),
            last_at: HashMap::new(),
            reminded: HashMap::new(),
            job_prev: IndexMap::new(),
            loop_sent: HashSet::new(),
            limit_sent: HashSet::new(),
            limits_primed: false,
            presence: IndexMap::new(),
            work_since: HashMap::new(),
            outbox: Vec::new(),
        }
    }

    pub fn system(options: ToastOptions) -> Self {
        Self::new(options, Arc::new(core_store::time::now_ms))
    }

    pub fn load(&mut self) {
        self.file.load();
    }

    pub fn drain(&mut self) -> Vec<HostMessage> {
        std::mem::take(&mut self.outbox)
    }

    pub(crate) fn now(&self) -> f64 {
        (self.clock)() as f64
    }

    pub(crate) fn prefs(&self) -> &NotifyPrefs {
        self.file.prefs()
    }

    pub(crate) fn text(&self) -> Text {
        Text(self.prefs().locale)
    }

    pub(crate) fn kind_on(&self, kind: &str) -> bool {
        self.prefs().kinds.get(kind).copied().unwrap_or(false)
    }

    pub fn refresh_health(&mut self) -> Option<ToastHealth> {
        if !self.options.supported {
            return self.health;
        }
        if let Some(probe) = &self.health_probe {
            self.health = Some(probe());
        }
        self.health
    }

    pub fn set_health(&mut self, health: ToastHealth) {
        self.health = Some(health);
    }

    pub fn test(&mut self) -> TestOutcome {
        if !self.options.supported {
            return TestOutcome { result: ToastTestResult::Unsupported, health: None };
        }
        let text = self.text();
        let content = ToastContent { tag: toast_tag("test"), title: text.test_title(), body: text.test_body(), url: self.link_for(None), open: text.open_dashboard(), dismiss: text.dismiss(), persistent: false, snooze_url: None, snooze_label: None };
        self.outbox.push(HostMessage::Show(content));
        let health = self.refresh_health();
        TestOutcome { result: ToastTestResult::Ok, health }
    }

    pub fn snooze_link(&self, terminal_id: &str, minutes: u64) -> String {
        format!("{}snooze?k={}&t={}&m={minutes}", self.link_base(), self.key(), encode_uri_component(terminal_id))
    }

    pub fn key(&self) -> &str {
        &self.prefs().key
    }

    pub fn state(&self) -> ToastState {
        match (self.options.supported, self.prefs().enabled) {
            (false, _) => ToastState::Unsupported,
            (true, true) => ToastState::On,
            (true, false) => ToastState::Off,
        }
    }

    pub fn locale(&self) -> ToastLocale {
        self.prefs().locale
    }

    pub fn settings(&self) -> NotifySettings {
        let p = self.prefs();
        NotifySettings { toast: self.state(), kinds: p.kinds.clone(), quiet: p.quiet.clone(), stuck_minutes: JsNum(p.stuck_minutes), idle_minutes: JsNum(p.idle_minutes), health: self.health }
    }

    pub fn update(&mut self, patch: &NotifyPatch) {
        if let Some(on) = patch.toast.filter(|_| self.options.supported) {
            self.file.prefs_mut().enabled = on;
            if !on {
                self.hide_all();
            }
        }
        for kind in TOAST_KINDS {
            let Some(&on) = patch.kinds.get(kind) else { continue };
            self.file.prefs_mut().kinds.insert(kind.to_string(), on);
            if !on {
                self.hide_kind(kind);
            }
        }
        if let Some(q) = &patch.quiet {
            self.file.prefs_mut().quiet = q.clone();
        }
        if let Some(s) = patch.stuck_minutes.filter(|v| STUCK_CHOICES.contains(v)) {
            self.file.prefs_mut().stuck_minutes = s;
        }
        if let Some(i) = patch.idle_minutes.filter(|v| IDLE_CHOICES.contains(v)) {
            self.file.prefs_mut().idle_minutes = i;
            if i == 0.0 {
                self.hide_key("idle:all");
            }
        }
        self.file.save();
    }

    pub fn idle_minutes(&self) -> f64 {
        self.prefs().idle_minutes
    }

    pub fn idle_watch(&self) -> bool {
        self.state() == ToastState::On && self.kind_on("idle") && self.idle_minutes() > 0.0
    }

    pub fn set_enabled(&mut self, on: bool) {
        self.update(&NotifyPatch { toast: Some(on), ..Default::default() });
    }

    pub fn set_presence(&mut self, client_id: u64, attentive: bool, locale: Option<ToastLocale>) {
        self.presence.insert(client_id, attentive);
        if let Some(l) = locale.filter(|l| *l != self.prefs().locale) {
            self.file.prefs_mut().locale = l;
            self.file.save();
        }
        if attentive {
            self.hide_all();
        }
    }

    pub fn drop_client(&mut self, client_id: u64) {
        self.presence.shift_remove(&client_id);
    }

    pub fn attentive(&self) -> bool {
        self.presence.values().any(|on| *on)
    }

    pub fn link_for(&self, terminal_id: Option<&str>) -> String {
        let base = format!("{}focus?k={}", self.link_base(), self.key());
        match terminal_id {
            Some(id) => format!("{base}&t={}", encode_uri_component(id)),
            None => base,
        }
    }

    fn link_base(&self) -> String {
        self.options.link.clone().unwrap_or_else(|| format!("http://127.0.0.1:{}/", self.options.port))
    }

    pub fn accepts(&self, key: Option<&str>) -> bool {
        key.is_some_and(|k| same_key(k, self.key()))
    }

    pub fn next_flush_at(&self) -> Option<i64> {
        self.file.next_flush_at()
    }

    pub fn poll(&mut self, now: i64) -> bool {
        self.file.poll(now)
    }

    pub fn flush(&mut self) -> bool {
        self.file.flush()
    }

    pub fn hide_for(&mut self, terminal_id: &str) {
        for kind in ["waiting", "done", "loop", "stuck", "context", "sensitive", "note"] {
            self.hide_key(&format!("{kind}:{terminal_id}"));
        }
    }

    pub fn clear(&mut self) {
        let tags: Vec<String> = self.shown.drain(..).map(|(_, t)| t).collect();
        self.outbox.extend(tags.into_iter().map(HostMessage::Hide));
    }

    pub(crate) fn quiet_now(&self) -> bool {
        in_quiet_hours(&self.prefs().quiet, self.now(), self.zone)
    }

    pub(crate) fn can(&self, kind: &str) -> bool {
        self.state() == ToastState::On && self.kind_on(kind) && !self.attentive() && !self.quiet_now()
    }

    pub(crate) fn phone(&mut self, m: PhoneEpisode, kind: Option<&str>) {
        if !self.phone_on || kind.is_some_and(|k| !self.kind_on(k)) || self.quiet_now() {
            return;
        }
        self.outbox.push(HostMessage::Phone(m));
    }

    pub(crate) fn show(&mut self, key: &str, d: Draft) {
        let text = self.text();
        let (snooze_url, snooze_label) = match d.snooze {
            true => {
                let id = key.split_once(':').map_or("", |(_, id)| id);
                (Some(self.snooze_link(id, TOAST_SNOOZE_MIN)), Some(text.snooze(TOAST_SNOOZE_MIN)))
            }
            false => (None, None),
        };
        let content = ToastContent { tag: toast_tag(key), title: d.title, body: d.body, url: d.url, open: d.open, dismiss: text.dismiss(), persistent: d.persistent, snooze_url, snooze_label };
        self.shown.insert(key.to_string(), content.tag.clone());
        self.outbox.push(HostMessage::Show(content));
    }

    pub(crate) fn hide_key(&mut self, key: &str) {
        if let Some(tag) = self.shown.shift_remove(key) {
            self.outbox.push(HostMessage::Hide(tag));
        }
    }

    fn hide_kind(&mut self, kind: &str) {
        let prefix = format!("{kind}:");
        let keys: Vec<String> = self.shown.keys().filter(|k| k.starts_with(&prefix)).cloned().collect();
        for k in keys {
            self.hide_key(&k);
        }
    }

    fn hide_all(&mut self) {
        let keys: Vec<String> = self.shown.keys().cloned().collect();
        for k in keys {
            self.hide_key(&k);
        }
    }
}
