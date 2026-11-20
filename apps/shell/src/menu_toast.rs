use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::sync::Mutex;
use std::time::Duration;

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};

use crate::config::LINK_SCHEME;
use crate::state::{lock, MAIN};
use crate::toast::{js_len, tag_of, Answer, Toast, MAX_ANSWERS};

pub const CHANNEL: &str = "ccm:menu-toast";
pub const GRACE: Duration = Duration::from_millis(2000);
const MAX_ENTRIES: usize = 50;
const MAX_ID: usize = 200;
const MAX_TEXT: usize = 200;
const MAX_LABEL: usize = 40;

#[derive(Debug, Clone, PartialEq)]
pub struct Opt {
    pub index: u8,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub pane_key: String,
    pub sig: String,
    pub terminal_id: Option<String>,
    pub title: String,
    pub place: String,
    pub options: Option<Vec<Opt>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sync {
    pub enabled: bool,
    pub open: String,
    pub dismiss: String,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Content {
    pub tag: String,
    pub title: String,
    pub body: String,
    pub persistent: bool,
    pub open: Answer,
    pub dismiss: String,
    pub answers: Vec<Answer>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Op {
    Show(Content),
    Hide(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Open { pane_key: String, sig: String },
    Answer { pane_key: String, sig: String, index: u8 },
}

fn js_slice(s: &str, max: usize) -> String {
    let units: Vec<u16> = s.encode_utf16().take(max).collect();
    String::from_utf16_lossy(&units)
}

fn id_str(v: &Value) -> Option<&str> {
    v.as_str().filter(|s| !s.is_empty() && js_len(s) <= MAX_ID)
}

fn parse_option(v: &Value) -> Option<Opt> {
    let index = v.get("index")?.as_f64().filter(|n| n.fract() == 0.0 && (0.0..=99.0).contains(n))? as u8;
    let label = v.get("label")?.as_str()?;
    Some(Opt { index, label: js_slice(label, MAX_LABEL) })
}

fn parse_entry(v: &Value) -> Option<Entry> {
    let pane_key = id_str(v.get("paneKey")?)?;
    let sig = id_str(v.get("sig")?)?;
    let (title, place) = (v.get("title")?.as_str()?, v.get("place")?.as_str()?);
    let terminal_id = match v.get("terminalId") {
        None => None,
        Some(t) => Some(t.as_str().filter(|s| js_len(s) <= MAX_ID)?).filter(|s| !s.is_empty()),
    };
    let options = v.get("options").and_then(Value::as_array).and_then(|raw| {
        let parsed: Vec<Opt> = raw.iter().map(parse_option).collect::<Option<_>>()?;
        let unique: HashSet<u8> = parsed.iter().map(|o| o.index).collect();
        (!parsed.is_empty() && parsed.len() <= MAX_ANSWERS && unique.len() == parsed.len()).then_some(parsed)
    });
    Some(Entry { pane_key: pane_key.into(), sig: sig.into(), terminal_id: terminal_id.map(str::to_owned), title: js_slice(title, MAX_TEXT), place: js_slice(place, MAX_TEXT), options })
}

pub fn parse_sync(v: &Value) -> Option<Sync> {
    let enabled = v.get("enabled")?.as_bool()?;
    let label = |k: &str| v.get(k).and_then(Value::as_str).filter(|s| js_len(s) <= MAX_LABEL).map(str::to_owned);
    let (open, dismiss) = (label("open")?, label("dismiss")?);
    let raw = v.get("entries")?.as_array().filter(|a| a.len() <= MAX_ENTRIES)?;
    let entries = raw.iter().map(parse_entry).collect::<Option<Vec<_>>>()?;
    Some(Sync { enabled, open, dismiss, entries })
}

pub fn tag_for(e: &Entry) -> String {
    match &e.terminal_id {
        Some(t) => tag_of(&format!("waiting:{t}")),
        None => tag_of(&format!("menu:{}", e.pane_key)),
    }
}

fn same_key(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

struct Known {
    entry: Entry,
    tag: String,
    shown: bool,
    gone: Option<u64>,
}

pub struct Core {
    key: String,
    known: HashMap<String, Known>,
    labels: (String, String),
    timers: u64,
}

impl Core {
    pub fn new(key: String) -> Core {
        Core { key, known: HashMap::new(), labels: (String::new(), String::new()), timers: 0 }
    }

    pub fn link(&self, pane_key: &str, sig: &str, index: Option<u8>) -> String {
        let mut url = tauri::Url::parse(&format!("{LINK_SCHEME}://menu")).expect("menu url");
        {
            let mut q = url.query_pairs_mut();
            q.append_pair("k", &self.key).append_pair("p", pane_key).append_pair("s", sig);
            if let Some(i) = index {
                q.append_pair("i", &i.to_string());
            }
        }
        url.into()
    }

    fn hide(k: &mut Known, ops: &mut Vec<Op>) {
        if k.shown {
            k.shown = false;
            ops.push(Op::Hide(k.tag.clone()));
        }
    }

    fn show(&self, k: &mut Known, ops: &mut Vec<Op>) {
        let e = &k.entry;
        let answers = e.options.iter().flatten().map(|o| Answer { label: o.label.clone(), url: self.link(&e.pane_key, &e.sig, Some(o.index)) }).collect();
        let content = Content {
            tag: k.tag.clone(),
            title: e.title.clone(),
            body: e.place.clone(),
            persistent: true,
            open: Answer { label: self.labels.0.clone(), url: self.link(&e.pane_key, &e.sig, None) },
            dismiss: self.labels.1.clone(),
            answers,
        };
        k.shown = true;
        ops.push(Op::Show(content));
    }

    fn drop_pane(&mut self, pane_key: &str, ops: &mut Vec<Op>) {
        if let Some(mut k) = self.known.remove(pane_key) {
            Self::hide(&mut k, ops);
        }
    }

    pub fn clear(&mut self) -> Vec<Op> {
        let mut ops = Vec::new();
        let keys: Vec<String> = self.known.keys().cloned().collect();
        for k in keys {
            self.drop_pane(&k, &mut ops);
        }
        ops
    }

    pub fn settle(&mut self) -> Vec<Op> {
        let mut ops = Vec::new();
        for k in self.known.values_mut() {
            Self::hide(k, &mut ops);
        }
        ops
    }

    pub fn sync(&mut self, data: &Value, focused: bool) -> (Vec<Op>, Vec<(String, u64)>) {
        let Some(parsed) = parse_sync(data) else { return (Vec::new(), Vec::new()) };
        if !parsed.enabled {
            return (self.clear(), Vec::new());
        }
        self.labels = (parsed.open.clone(), parsed.dismiss.clone());
        let alive: HashSet<&str> = parsed.entries.iter().map(|e| e.pane_key.as_str()).collect();
        let mut timers = Vec::new();
        for (pane, k) in self.known.iter_mut() {
            if alive.contains(pane.as_str()) || k.gone.is_some() {
                continue;
            }
            self.timers += 1;
            k.gone = Some(self.timers);
            timers.push((pane.clone(), self.timers));
        }
        let mut ops = Vec::new();
        for entry in parsed.entries {
            if let Some(cur) = self.known.get_mut(&entry.pane_key) {
                cur.gone = None;
                if cur.entry.sig == entry.sig {
                    cur.entry = entry;
                    continue;
                }
            }
            let pane = entry.pane_key.clone();
            self.drop_pane(&pane, &mut ops);
            let mut next = Known { tag: tag_for(&entry), entry, shown: false, gone: None };
            if !focused {
                self.show(&mut next, &mut ops);
            }
            self.known.insert(pane, next);
        }
        (ops, timers)
    }

    pub fn expire(&mut self, pane_key: &str, timer: u64) -> Vec<Op> {
        let mut ops = Vec::new();
        if self.known.get(pane_key).is_some_and(|k| k.gone == Some(timer)) {
            self.drop_pane(pane_key, &mut ops);
        }
        ops
    }

    pub fn activate(&mut self, raw: &str) -> Option<(Action, Vec<Op>)> {
        let url = tauri::Url::parse(raw).ok()?;
        if url.scheme() != LINK_SCHEME || url.host_str() != Some("menu") {
            return None;
        }
        let get = |k: &str| url.query_pairs().find(|(n, _)| n == k).map(|(_, v)| v.into_owned());
        let (k, p, s) = (get("k").unwrap_or_default(), get("p").unwrap_or_default(), get("s").unwrap_or_default());
        if k.is_empty() || !same_key(&k, &self.key) || p.is_empty() || s.is_empty() {
            return None;
        }
        let cur = self.known.get_mut(&p)?;
        if cur.gone.is_some() || cur.entry.sig != s {
            return None;
        }
        let Some(i) = get("i") else { return Some((Action::Open { pane_key: p, sig: s }, Vec::new())) };
        if i.is_empty() || i.len() > 2 || !i.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let index: u8 = i.parse().ok()?;
        if !cur.entry.options.iter().flatten().any(|o| o.index == index) {
            return None;
        }
        let mut ops = Vec::new();
        Self::hide(cur, &mut ops);
        Some((Action::Answer { pane_key: p, sig: s, index }, ops))
    }
}

pub fn action_json(a: &Action) -> Value {
    match a {
        Action::Open { pane_key, sig } => json!({ "type": "open", "paneKey": pane_key, "sig": sig }),
        Action::Answer { pane_key, sig, index } => json!({ "type": "answer", "paneKey": pane_key, "sig": sig, "index": index }),
    }
}

pub fn content_json(c: &Content) -> Value {
    let answer = |a: &Answer| json!({ "label": a.label, "url": a.url });
    json!({ "tag": c.tag, "title": c.title, "body": c.body, "persistent": c.persistent, "open": answer(&c.open), "dismiss": c.dismiss, "answers": c.answers.iter().map(answer).collect::<Vec<_>>() })
}

pub fn toast_of(c: &Content) -> Toast {
    Toast {
        tag: c.tag.clone(),
        title: c.title.clone(),
        body: c.body.clone(),
        url: c.open.url.clone(),
        open: c.open.label.clone(),
        dismiss: c.dismiss.clone(),
        persistent: c.persistent,
        snooze: None,
        answers: c.answers.iter().take(MAX_ANSWERS).cloned().collect(),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Drawer {
    pub log: Option<String>,
    pub real: bool,
    pub grace: Duration,
}

pub fn drawer_from(get: &dyn Fn(&str) -> Option<String>, test_build: bool, windows: bool) -> Option<Drawer> {
    let log = test_build.then(|| get("CCM_E2E_MENU_TOAST_LOG")).flatten().filter(|v| !v.is_empty());
    let toast_off = get("CCM_TOAST").as_deref() == Some("0");
    if toast_off && log.is_none() {
        return None;
    }
    let real = windows && (!toast_off || (test_build && get("CCM_E2E_MENU_TOAST_REAL").as_deref() == Some("1")));
    let grace = log.as_ref().and_then(|_| get("CCM_E2E_MENU_TOAST_GRACE_MS")).and_then(|v| v.trim().parse::<u64>().ok()).filter(|ms| *ms > 0).map_or(GRACE, Duration::from_millis);
    Some(Drawer { log, real, grace })
}

pub struct MenuToasts {
    core: Mutex<Core>,
    drawer: Drawer,
}

pub fn install(app: &AppHandle) {
    let get = |k: &str| std::env::var(k).ok();
    let test_build = cfg!(debug_assertions) || crate::config::e2e_isolated();
    let Some(drawer) = drawer_from(&get, test_build, cfg!(windows)) else { return };
    app.manage(MenuToasts { core: Mutex::new(Core::new(crate::config::random_hex(16))), drawer });
}

fn apply(app: &AppHandle, ops: Vec<Op>) {
    let Some(m) = app.try_state::<MenuToasts>() else { return };
    for op in ops {
        if let Some(file) = &m.drawer.log {
            let line = match &op {
                Op::Show(c) => {
                    let mut v = json!({ "op": "show" });
                    if let (Some(o), Value::Object(body)) = (v.as_object_mut(), content_json(c)) {
                        o.extend(body);
                    }
                    v
                }
                Op::Hide(tag) => json!({ "op": "hide", "tag": tag }),
            };
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(file) {
                let _ = writeln!(f, "{line}");
            }
        }
        if m.drawer.real {
            match op {
                Op::Show(c) => crate::toast::show_on_worker(app, toast_of(&c)),
                Op::Hide(tag) => crate::toast::hide_on_worker(app, tag),
            }
        }
    }
}

fn focused(app: &AppHandle) -> bool {
    app.get_webview_window(MAIN).is_some_and(|w| w.is_visible().unwrap_or(false) && !w.is_minimized().unwrap_or(false) && w.is_focused().unwrap_or(false))
}

pub fn sync(app: &AppHandle, data: &Value) {
    let Some(m) = app.try_state::<MenuToasts>() else { return };
    let away = !focused(app);
    let (ops, timers) = lock(&m.core).sync(data, !away);
    apply(app, ops);
    let grace = m.drawer.grace;
    for (pane, timer) in timers {
        let handle = app.clone();
        let _ = std::thread::Builder::new().name("menu-toast-grace".into()).spawn(move || {
            std::thread::sleep(grace);
            let Some(m) = handle.try_state::<MenuToasts>() else { return };
            let ops = lock(&m.core).expire(&pane, timer);
            apply(&handle, ops);
        });
    }
}

pub fn settle(app: &AppHandle) {
    let Some(m) = app.try_state::<MenuToasts>() else { return };
    let ops = lock(&m.core).settle();
    apply(app, ops);
}

pub fn activate(app: &AppHandle, raw: &str) {
    let Some(m) = app.try_state::<MenuToasts>() else { return };
    let Some((action, ops)) = lock(&m.core).activate(raw) else { return };
    apply(app, ops);
    if app.get_webview_window(MAIN).is_none() {
        return;
    }
    if matches!(action, Action::Open { .. }) {
        crate::window::show_main(app);
    }
    let _ = app.emit_to(MAIN, CHANNEL, action_json(&action));
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "0123456789abcdef0123456789abcdef";

    fn entry(pane: &str, sig: &str, options: Option<Value>) -> Value {
        let mut e = json!({ "paneKey": pane, "sig": sig, "title": "Do you want to proceed?", "place": "repo - Bash" });
        if let Some(o) = options {
            e["options"] = o;
        }
        e
    }

    fn two() -> Value {
        json!([{ "index": 0, "label": "1. Yes" }, { "index": 1, "label": "2. No" }])
    }

    fn sync_of(entries: Vec<Value>) -> Value {
        json!({ "enabled": true, "open": "Open", "dismiss": "Dismiss", "entries": entries })
    }

    fn shows(ops: &[Op]) -> Vec<&Content> {
        ops.iter().filter_map(|o| if let Op::Show(c) = o { Some(c) } else { None }).collect()
    }

    #[test]
    fn the_sync_payload_is_checked_like_menu_toast_ts() {
        let ok = parse_sync(&sync_of(vec![entry("p1", "aabbccdd", Some(two())), entry("p2", "s", Some(json!([{ "index": 0, "label": "a" }, { "index": 0, "label": "b" }])))])).unwrap();
        assert_eq!(ok.entries[0].options.as_ref().map(Vec::len), Some(2));
        assert_eq!(ok.entries[1].options, None);
        assert_eq!(parse_sync(&json!({ "enabled": true, "open": "o".repeat(41), "dismiss": "d", "entries": [] })), None);
        assert_eq!(parse_sync(&sync_of(vec![json!({ "paneKey": "", "sig": "s", "title": "", "place": "" })])), None);
        assert_eq!(parse_sync(&sync_of((0..51).map(|i| entry(&format!("p{i}"), "s", None)).collect())), None);
        let long = parse_sync(&sync_of(vec![entry("p", "s", Some(json!([{ "index": 3, "label": "x".repeat(60) }])))])).unwrap();
        assert_eq!(long.entries[0].options.as_ref().unwrap()[0].label.len(), MAX_LABEL);
        assert_eq!(parse_sync(&sync_of(vec![entry("p", "s", Some(json!([{ "index": 100, "label": "x" }])))])).unwrap().entries[0].options, None);
    }

    #[test]
    fn an_away_app_toasts_each_menu_once_with_answer_links() {
        let mut c = Core::new(KEY.into());
        let (ops, timers) = c.sync(&sync_of(vec![entry("p1", "aabbccdd", Some(two()))]), false);
        assert!(timers.is_empty());
        let s = shows(&ops);
        assert_eq!(s.len(), 1);
        assert_eq!(s[0].tag, tag_of("menu:p1"));
        assert_eq!(s[0].answers.len(), 2);
        assert_eq!(s[0].answers[1].url, format!("agent-monitor://menu?k={KEY}&p=p1&s=aabbccdd&i=1"));
        assert_eq!(s[0].open.url, format!("agent-monitor://menu?k={KEY}&p=p1&s=aabbccdd"));
        let (again, _) = c.sync(&sync_of(vec![entry("p1", "aabbccdd", Some(two()))]), false);
        assert!(again.is_empty());
        let (focused, _) = c.sync(&sync_of(vec![entry("p1", "aabbccdd", Some(two())), entry("p2", "s2", None)]), true);
        assert!(focused.is_empty());
    }

    #[test]
    fn a_terminal_menu_shares_the_waiting_toast_tag() {
        let mut c = Core::new(KEY.into());
        let mut e = entry("p1", "s", None);
        e["terminalId"] = json!("t9");
        let (ops, _) = c.sync(&sync_of(vec![e]), false);
        assert_eq!(shows(&ops)[0].tag, tag_of("waiting:t9"));
        assert!(shows(&ops)[0].answers.is_empty());
    }

    #[test]
    fn answers_need_the_key_the_live_sig_and_a_known_option() {
        let mut c = Core::new(KEY.into());
        let (ops, _) = c.sync(&sync_of(vec![entry("p1", "aabbccdd", Some(two()))]), false);
        let s = shows(&ops)[0].clone();
        let bad_key = s.answers[1].url.replace(KEY, &"0".repeat(32));
        assert_eq!(c.activate(&bad_key), None);
        assert_eq!(c.activate(&s.answers[1].url.replace("s=aabbccdd", "s=other")), None);
        assert_eq!(c.activate(&s.answers[1].url.replace("i=1", "i=7")), None);
        assert_eq!(c.activate(&s.answers[1].url.replace("i=1", "i=x")), None);
        assert_eq!(c.activate("other://menu?k=a"), None);
        let (action, hide) = c.activate(&s.answers[1].url).unwrap();
        assert_eq!(action, Action::Answer { pane_key: "p1".into(), sig: "aabbccdd".into(), index: 1 });
        assert_eq!(hide, vec![Op::Hide(s.tag.clone())]);
        assert_eq!(action_json(&action), json!({ "type": "answer", "paneKey": "p1", "sig": "aabbccdd", "index": 1 }));
        let (open, none) = c.activate(&s.open.url).unwrap();
        assert_eq!(open, Action::Open { pane_key: "p1".into(), sig: "aabbccdd".into() });
        assert!(none.is_empty());
    }

    #[test]
    fn a_gone_menu_waits_the_grace_then_hides() {
        let mut c = Core::new(KEY.into());
        let (ops, _) = c.sync(&sync_of(vec![entry("p1", "s1", Some(two()))]), false);
        let tag = shows(&ops)[0].tag.clone();
        let (none, timers) = c.sync(&sync_of(vec![]), false);
        assert!(none.is_empty());
        assert_eq!(timers.len(), 1);
        let (_, twice) = c.sync(&sync_of(vec![]), false);
        assert!(twice.is_empty());
        assert_eq!(c.activate(&c.link("p1", "s1", Some(0))), None);
        assert_eq!(c.expire("p1", timers[0].1 + 1), vec![]);
        assert_eq!(c.expire("p1", timers[0].1), vec![Op::Hide(tag)]);
        let (back, _) = c.sync(&sync_of(vec![entry("p2", "s", None)]), true);
        let (_, t) = c.sync(&sync_of(vec![]), false);
        assert!(back.is_empty() && t.len() == 1);
        c.sync(&sync_of(vec![entry("p2", "s", None)]), false);
        assert!(c.expire("p2", t[0].1).is_empty());
    }

    #[test]
    fn a_new_sig_replaces_the_toast_and_settle_or_off_hides_all() {
        let mut c = Core::new(KEY.into());
        let (first, _) = c.sync(&sync_of(vec![entry("p1", "s1", None)]), false);
        let tag = shows(&first)[0].tag.clone();
        let (next, _) = c.sync(&sync_of(vec![entry("p1", "s2", None)]), false);
        assert_eq!(next.first(), Some(&Op::Hide(tag.clone())));
        assert_eq!(shows(&next).len(), 1);
        assert_eq!(c.settle(), vec![Op::Hide(tag.clone())]);
        assert!(c.settle().is_empty());
        let (shown, _) = c.sync(&sync_of(vec![entry("p3", "s", None)]), false);
        assert_eq!(shows(&shown).len(), 1);
        let (off, _) = c.sync(&json!({ "enabled": false, "open": "", "dismiss": "", "entries": [] }), false);
        assert_eq!(off, vec![Op::Hide(tag_of("menu:p3"))]);
    }

    #[test]
    fn the_drawer_follows_ccm_toast_and_the_e2e_log() {
        let env = |pairs: &'static [(&'static str, &'static str)]| move |k: &str| pairs.iter().find(|(n, _)| *n == k).map(|(_, v)| (*v).to_owned());
        assert_eq!(drawer_from(&env(&[]), false, true), Some(Drawer { log: None, real: true, grace: GRACE }));
        assert_eq!(drawer_from(&env(&[]), false, false), Some(Drawer { log: None, real: false, grace: GRACE }));
        assert_eq!(drawer_from(&env(&[("CCM_TOAST", "0")]), true, true), None);
        assert_eq!(drawer_from(&env(&[("CCM_TOAST", "0"), ("CCM_E2E_MENU_TOAST_LOG", "/l")]), false, true), None);
        assert_eq!(
            drawer_from(&env(&[("CCM_TOAST", "0"), ("CCM_E2E_MENU_TOAST_LOG", "/l"), ("CCM_E2E_MENU_TOAST_GRACE_MS", "60000")]), true, true),
            Some(Drawer { log: Some("/l".into()), real: false, grace: Duration::from_secs(60) })
        );
        assert_eq!(drawer_from(&env(&[("CCM_TOAST", "0"), ("CCM_E2E_MENU_TOAST_LOG", "/l"), ("CCM_E2E_MENU_TOAST_REAL", "1")]), true, true).map(|d| d.real), Some(true));
        assert_eq!(drawer_from(&env(&[("CCM_E2E_MENU_TOAST_GRACE_MS", "5")]), true, true).map(|d| d.grace), Some(GRACE));
    }

    #[test]
    fn the_windows_toast_carries_open_and_up_to_three_answers() {
        let mut c = Core::new(KEY.into());
        let (ops, _) = c.sync(&sync_of(vec![entry("p1", "s", Some(two()))]), false);
        let t = toast_of(shows(&ops)[0]);
        assert_eq!((t.open.as_str(), t.dismiss.as_str(), t.persistent, t.answers.len()), ("Open", "Dismiss", true, 2));
        assert!(t.url.starts_with("agent-monitor://menu?k="));
        assert_eq!(content_json(shows(&ops)[0])["open"]["label"], "Open");
    }
}
