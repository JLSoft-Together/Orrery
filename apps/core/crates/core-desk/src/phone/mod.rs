pub mod link;
pub mod send;

use crate::claude::{js_trim, Sys};
use crate::{arg, js, keychain, util, Ctx, Desk, DeskEnv};
use send::{build_request, clean_discord_url, clean_ntfy_server, clean_topic, e2e_override, host_of, phone_text, test_text, Dest, RateLimiter, Text};
use serde_json::{json, Map, Value};
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const MODULE: &str = "phone";
pub const PHONE_FILE: &str = "phone.json";
pub const EVENT: &str = "ccm:phone-event";
const MAX_PER_HOUR: usize = 20;
const MAX_EPISODES: usize = 500;
const CHANNELS: [&str; 2] = ["ntfy", "discord"];
const IDLE_CHOICES: [i64; 4] = [0, 5, 10, 15];
const DEFAULT_IDLE: i64 = 5;
const REASONS: [&str; 8] = ["waiting", "question", "permission", "done", "job", "limit", "disk", "note"];

#[derive(Clone)]
struct Shape {
    enabled: bool,
    channel: String,
    idle_minutes: i64,
    ntfy_server: Option<String>,
    ntfy_topic: Option<String>,
    discord_url: Option<String>,
}

impl Shape {
    fn defaults() -> Self {
        Shape { enabled: false, channel: "ntfy".into(), idle_minutes: DEFAULT_IDLE, ntfy_server: None, ntfy_topic: None, discord_url: None }
    }

    fn read(file: &std::path::Path) -> Self {
        let Some(Value::Object(r)) = std::fs::read_to_string(file).ok().and_then(|t| js::parse(&t)) else { return Shape::defaults() };
        let s = |k: &str| r.get(k).and_then(Value::as_str).filter(|v| js::len(v) < 4096).map(str::to_string);
        Shape {
            enabled: r.get("enabled") == Some(&Value::Bool(true)),
            channel: r.get("channel").and_then(Value::as_str).filter(|c| CHANNELS.contains(c)).unwrap_or("ntfy").into(),
            idle_minutes: idle_choice(r.get("idleMinutes")).unwrap_or(DEFAULT_IDLE),
            ntfy_server: s("ntfyServer"),
            ntfy_topic: s("ntfyTopic"),
            discord_url: s("discordUrl"),
        }
    }

    fn to_value(&self) -> Value {
        let mut m = Map::new();
        m.insert("enabled".into(), json!(self.enabled));
        m.insert("channel".into(), json!(self.channel));
        m.insert("idleMinutes".into(), json!(self.idle_minutes));
        for (k, v) in [("ntfyServer", &self.ntfy_server), ("ntfyTopic", &self.ntfy_topic), ("discordUrl", &self.discord_url)] {
            if let Some(v) = v {
                m.insert(k.into(), json!(v));
            }
        }
        Value::Object(m)
    }
}

fn idle_choice(v: Option<&Value>) -> Option<i64> {
    let f = v?.as_f64()?;
    IDLE_CHOICES.iter().copied().find(|c| *c as f64 == f)
}

struct State {
    data: Shape,
    limiter: RateLimiter,
    sent: VecDeque<String>,
    seen: HashSet<String>,
    last_error: Option<&'static str>,
}

pub struct Phone {
    file: PathBuf,
    over: Option<String>,
    sys: Sys,
    state: Mutex<State>,
}

#[derive(Default)]
pub struct Patch {
    enabled: Option<bool>,
    channel: Option<String>,
    idle_minutes: Option<i64>,
    ntfy_server: Option<String>,
    ntfy_topic: Option<String>,
    discord_url: Option<String>,
}

pub fn sanitize_patch(raw: &Value) -> Option<Patch> {
    let r = raw.as_object()?;
    let mut out = Patch::default();
    let given = |k: &str| r.get(k);
    if let Some(v) = given("enabled") {
        out.enabled = Some(v.as_bool()?);
    }
    if let Some(v) = given("channel") {
        out.channel = Some(v.as_str().filter(|c| CHANNELS.contains(c))?.to_string());
    }
    if let Some(v) = given("idleMinutes") {
        out.idle_minutes = Some(idle_choice(Some(v))?);
    }
    for k in ["ntfyServer", "ntfyTopic", "discordUrl"] {
        let Some(v) = given(k) else { continue };
        let s = v.as_str().filter(|s| js::len(s) <= 500)?;
        let t = Some(js_trim(s).to_string());
        match k {
            "ntfyServer" => out.ntfy_server = t,
            "ntfyTopic" => out.ntfy_topic = t,
            _ => out.discord_url = t,
        }
    }
    Some(out)
}

fn locale_en(ctx: &Ctx) -> bool {
    matches!(ctx.host("phone.locale", Value::Null), Ok(Value::String(s)) if s == "en")
}

impl Phone {
    pub fn new(data_dir: &std::path::Path) -> Self {
        let file = data_dir.join(PHONE_FILE);
        let data = Shape::read(&file);
        let over = e2e_override(std::env::var("CCM_PHONE_URL_OVERRIDE").ok().as_deref());
        let state = State { data, limiter: RateLimiter::new(MAX_PER_HOUR, 3_600_000.0), sent: VecDeque::new(), seen: HashSet::new(), last_error: None };
        Phone { file, over, sys: Sys::from_env(), state: Mutex::new(state) }
    }

    fn destination(ctx: &Ctx, data: &Shape) -> Option<Dest> {
        let open = |s: &Option<String>| s.as_deref().filter(|v| !v.is_empty()).and_then(|v| keychain::unseal(ctx, v));
        if data.channel == "discord" {
            return open(&data.discord_url).map(|url| Dest { channel: "discord", url, topic: None });
        }
        let url = open(&data.ntfy_server)?;
        let topic = open(&data.ntfy_topic)?;
        Some(Dest { channel: "ntfy", url, topic: Some(topic) })
    }

    fn state_of(ctx: &Ctx, st: &State) -> Value {
        let dest = Self::destination(ctx, &st.data);
        json!({
            "enabled": st.data.enabled,
            "channel": st.data.channel,
            "idleMinutes": st.data.idle_minutes,
            "configured": dest.is_some(),
            "host": dest.as_ref().and_then(|d| host_of(&d.url)),
            "keychain": keychain::available(ctx),
            "lastError": st.last_error,
        })
    }

    pub fn state(&self, ctx: &Ctx) -> Value {
        let st = self.state.lock().unwrap();
        Self::state_of(ctx, &st)
    }

    pub fn set(&self, ctx: &Ctx, patch: Patch) -> Value {
        let mut st = self.state.lock().unwrap();
        let mut next = st.data.clone();
        let secrets = patch.ntfy_server.is_some() || patch.ntfy_topic.is_some() || patch.discord_url.is_some();
        if secrets && !keychain::available(ctx) {
            return json!({"ok": false, "error": "keychain"});
        }
        let seal = |v: String| -> Result<Option<String>, ()> {
            if v.is_empty() {
                return Ok(None);
            }
            keychain::seal(ctx, &v).map(Some).ok_or(())
        };
        if let Some(raw) = patch.ntfy_server {
            let v = if raw.is_empty() { Some(String::new()) } else { clean_ntfy_server(&raw) };
            let Some(v) = v else { return json!({"ok": false, "error": "invalid-url"}) };
            let Ok(s) = seal(v) else { return json!({"ok": false, "error": "failed"}) };
            next.ntfy_server = s;
        }
        if let Some(raw) = patch.ntfy_topic {
            let v = if raw.is_empty() { Some(String::new()) } else { clean_topic(&raw) };
            let Some(v) = v else { return json!({"ok": false, "error": "invalid-topic"}) };
            let Ok(s) = seal(v) else { return json!({"ok": false, "error": "failed"}) };
            next.ntfy_topic = s;
        }
        if let Some(raw) = patch.discord_url {
            let v = if raw.is_empty() { Some(String::new()) } else { clean_discord_url(&raw) };
            let Some(v) = v else { return json!({"ok": false, "error": "invalid-url"}) };
            let Ok(s) = seal(v) else { return json!({"ok": false, "error": "failed"}) };
            next.discord_url = s;
        }
        let channel_set = patch.channel.as_deref().is_some_and(|c| !c.is_empty());
        if let Some(c) = patch.channel.filter(|c| !c.is_empty()) {
            next.channel = c;
        }
        if let Some(n) = patch.idle_minutes {
            next.idle_minutes = n;
        }
        if let Some(e) = patch.enabled {
            next.enabled = e;
        }
        st.data = next;
        if secrets || channel_set {
            st.last_error = None;
        }
        let text = js::stringify_indent(&st.data.to_value(), "  ");
        if util::write_atomic(&self.file, &text).is_err() {
            return json!({"ok": false, "error": "failed"});
        }
        restrict(&self.file);
        let state = Self::state_of(ctx, &st);
        ctx.emit(EVENT, json!({"state": state}));
        json!({"ok": true, "state": state})
    }

    fn send(&self, ctx: &Ctx, st: &mut State, dest: &Dest, text: &Text) -> Option<&'static str> {
        if !st.limiter.take(util::now_ms()) {
            return Some(Self::fail(ctx, st, "rate"));
        }
        let (url, body) = build_request(dest, text, self.over.as_deref());
        match post(&url, &body) {
            Err(e) => Some(Self::fail(ctx, st, e)),
            Ok(()) => {
                if st.last_error.is_some() {
                    st.last_error = None;
                    ctx.emit(EVENT, json!({"state": Self::state_of(ctx, st)}));
                }
                None
            }
        }
    }

    fn fail(ctx: &Ctx, st: &mut State, error: &'static str) -> &'static str {
        let first = st.last_error.is_none();
        st.last_error = Some(error);
        if first {
            ctx.emit(EVENT, json!({"failed": error}));
            ctx.emit(EVENT, json!({"state": Self::state_of(ctx, st)}));
        }
        error
    }

    pub fn test(&self, ctx: &Ctx) -> Value {
        let mut st = self.state.lock().unwrap();
        let Some(dest) = Self::destination(ctx, &st.data) else { return json!({"ok": false, "error": "config"}) };
        match self.send(ctx, &mut st, &dest, &test_text(locale_en(ctx))) {
            Some(e) => json!({"ok": false, "error": e}),
            None => json!({"ok": true}),
        }
    }

    pub fn episode(&self, ctx: &Ctx, m: &Value) -> bool {
        let Some(key) = m.get("episode").and_then(Value::as_str).filter(|e| !e.is_empty()) else { return false };
        if !m.get("reason").and_then(Value::as_str).is_some_and(|r| REASONS.contains(&r)) || !m.get("name").is_some_and(Value::is_string) {
            return false;
        }
        let mut st = self.state.lock().unwrap();
        if !st.data.enabled || st.seen.contains(key) {
            return false;
        }
        let idle = ctx.host("phone.idle", Value::Null).ok().and_then(|v| v.as_f64()).unwrap_or(0.0);
        if idle < (st.data.idle_minutes * 60) as f64 {
            return false;
        }
        let Some(dest) = Self::destination(ctx, &st.data) else { return false };
        st.seen.insert(key.to_string());
        st.sent.push_back(key.to_string());
        if st.sent.len() > MAX_EPISODES {
            if let Some(old) = st.sent.pop_front() {
                st.seen.remove(&old);
            }
        }
        let en = locale_en(ctx);
        let mut text = phone_text(m, en, util::now_ms(), &self.sys.home, self.sys.platform());
        text.link = link::terminal_of(m).and_then(|t| link::from_host(ctx.host("remote.link", json!({"terminalId": t})).ok()));
        text.link_label = link::label(en);
        self.send(ctx, &mut st, &dest, &text).is_none()
    }
}

fn post(url: &str, body: &str) -> Result<(), &'static str> {
    let tls = ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build();
    let config = ureq::Agent::config_builder().timeout_global(Some(Duration::from_secs(10))).http_status_as_error(false).tls_config(tls).build();
    let agent: ureq::Agent = config.into();
    match agent.post(url).header("Content-Type", "application/json").send(body) {
        Ok(r) if r.status().is_success() => Ok(()),
        Ok(_) => Err("status"),
        Err(ureq::Error::Timeout(_)) => Err("timeout"),
        Err(ureq::Error::Io(e)) if e.kind() == std::io::ErrorKind::TimedOut => Err("timeout"),
        Err(_) => Err("network"),
    }
}

#[cfg(unix)]
fn restrict(file: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o600));
}

#[cfg(not(unix))]
fn restrict(_file: &std::path::Path) {}

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    let phone = Arc::new(Phone::new(&env.data_dir));
    let p = phone.clone();
    desk.on(MODULE, "ccm:phone-state", move |ctx, _| Ok(p.state(ctx)));
    let p = phone.clone();
    desk.on(MODULE, "ccm:phone-set", move |ctx, args| {
        Ok(match sanitize_patch(arg(args, 0)) {
            Some(patch) => p.set(ctx, patch),
            None => json!({"ok": false, "error": "failed"}),
        })
    });
    let p = phone.clone();
    desk.on(MODULE, "ccm:phone-test", move |ctx, _| Ok(p.test(ctx)));
    desk.on(MODULE, "desk:phone-episode", move |ctx, args| Ok(json!(phone.episode(ctx, arg(args, 0)))));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn patches() {
        assert!(sanitize_patch(&json!("x")).is_none());
        assert!(sanitize_patch(&json!({"enabled": 1})).is_none());
        assert!(sanitize_patch(&json!({"idleMinutes": 7})).is_none());
        assert!(sanitize_patch(&json!({"ntfyTopic": 5})).is_none());
        let p = sanitize_patch(&json!({"channel": "discord", "ntfyTopic": "  t  ", "idleMinutes": 10})).unwrap();
        assert_eq!(p.channel.as_deref(), Some("discord"));
        assert_eq!(p.ntfy_topic.as_deref(), Some("t"));
        assert_eq!(p.idle_minutes, Some(10));
    }
}
