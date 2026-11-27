use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use core_transcript::chat::page::{Live, Poll};
use core_transcript::chat::{PAGE, PAGE_MAX, WATCH_MAX};
use serde_json::{json, Value};
use tokio::sync::mpsc;

use super::chat_src::{agent_panes, items_events, last_event, page_of, remember, Ask, LastRow, Roots, ShellClean, Source, Target};
use super::dashboard::{error, Phone};
use crate::state::lock;

pub const POLL_EVERY: Duration = Duration::from_secs(5);
pub const BATCH: Duration = Duration::from_millis(core_transcript::chat::BATCH_MS);
pub const LAST_GAP: Duration = Duration::from_secs(1);
pub const MAX_JOBS: usize = 2;
const KEY_MAX: usize = 200;
const GEN_MAX: usize = 64;

pub struct Watch {
    pub pane: String,
    pub target: Target,
    pub gen: String,
    pub source: Option<Source>,
    pub live: Option<Arc<Mutex<Live>>>,
}

pub enum ChatMsg {
    Reply { id: String, result: Result<Value, &'static str>, watch: Option<Watch> },
    Polled { rows: Vec<PollRow> },
    Last { full: bool, rows: Vec<LastRow> },
}

pub enum PollRow {
    Items { pane: String, gen: String, items: Vec<Value> },
    Reset { pane: String, gen: String, source: Option<Source> },
}

pub struct ChatState {
    pub cap: bool,
    pub watches: Vec<Watch>,
    pub tx: mpsc::Sender<ChatMsg>,
    pub rx: mpsc::Receiver<ChatMsg>,
    pub due: Option<tokio::time::Instant>,
    jobs: usize,
    polling: bool,
    lasting: bool,
    last_dirty: bool,
    last_full: bool,
    last_at: Option<Instant>,
    last_poll: Option<Instant>,
    sent: HashMap<String, String>,
}

impl Default for ChatState {
    fn default() -> ChatState {
        let (tx, rx) = mpsc::channel(16);
        ChatState { cap: false, watches: Vec::new(), tx, rx, due: None, jobs: 0, polling: false, lasting: false, last_dirty: false, last_full: false, last_at: None, last_poll: None, sent: HashMap::new() }
    }
}

impl ChatState {
    pub fn can_watch(&self, pane: &str) -> bool {
        self.watches.iter().any(|w| w.pane == pane) || self.watches.len() < WATCH_MAX
    }

    pub fn put_watch(&mut self, w: Watch) {
        self.watches.retain(|x| x.pane != w.pane);
        if self.watches.len() < WATCH_MAX {
            self.watches.push(w);
        }
    }

    pub fn unwatch(&mut self, pane: &str) {
        self.watches.retain(|x| x.pane != pane);
    }

    pub fn wake(&mut self, after: Duration) {
        let at = tokio::time::Instant::now() + after;
        if self.due.is_none_or(|d| d > at) {
            self.due = Some(at);
        }
    }

    pub fn nudge(&mut self, turn_changed: bool) {
        if self.cap && turn_changed {
            self.last_dirty = true;
            self.wake(Duration::ZERO);
        }
        if !self.watches.is_empty() {
            let gap = self.last_poll.map_or(Duration::ZERO, |at| BATCH.saturating_sub(at.elapsed()));
            self.wake(gap);
        }
    }

    pub fn full_last(&mut self) {
        if self.cap {
            self.last_full = true;
            self.wake(Duration::ZERO);
        }
    }
}

pub fn parse_ask(t: &str, v: &Value) -> Result<(String, Option<String>, Option<u64>, usize), &'static str> {
    let pane = v["paneKey"].as_str().filter(|k| !k.is_empty() && k.len() <= KEY_MAX).ok_or("invalid")?;
    let limit = match v.get("limit") {
        None | Some(Value::Null) => PAGE,
        Some(l) => l.as_u64().filter(|n| (1..=PAGE_MAX as u64).contains(n)).ok_or("invalid")? as usize,
    };
    let before = match v.get("before") {
        Some(b) if t == "chat.page" && !b.is_null() => Some(b.as_u64().ok_or("invalid")?),
        _ => None,
    };
    let gen = match v.get("gen") {
        Some(g) if t == "chat.page" && !g.is_null() => Some(g.as_str().filter(|g| g.len() <= GEN_MAX).ok_or("invalid")?.to_owned()),
        _ => None,
    };
    Ok((pane.to_owned(), gen, before, limit))
}

impl Phone {
    pub(super) fn has_cap(&self, name: &str) -> bool {
        self.caps.iter().any(|c| c == name)
    }

    pub(super) fn set_chat_cap(&mut self) {
        self.x.chat.cap = self.has_cap("chat");
        self.x.chat.full_last();
    }

    fn terminal_of(&self, t: &Target) -> Option<Value> {
        let tap = self.ctx.host.tap()?;
        t.terminal.as_deref().and_then(|id| tap.terminal(id))
    }

    pub(super) async fn chat_command(&mut self, id: &str, t: &str, v: &Value) -> bool {
        if !self.ctx.viewer.take_rate(&self.device, Instant::now()) {
            return self.sec.json(&error(Value::from(id), "rate")).await;
        }
        let (pane, gen, before, limit) = match parse_ask(t, v) {
            Ok(a) => a,
            Err(code) => return self.sec.json(&error(Value::from(id), code)).await,
        };
        if t == "chat.unwatch" {
            self.x.chat.unwatch(&pane);
            return self.sec.json(&json!({ "re": id, "ok": true })).await;
        }
        let Some(summary) = self.ctx.summary() else { return self.sec.json(&error(Value::from(id), "offline")).await };
        let Some(target) = super::chat_src::target_of(&summary, &pane) else { return self.sec.json(&error(Value::from(id), "notFound")).await };
        if self.ctx.host.tap().is_none() {
            return self.sec.json(&error(Value::from(id), "offline")).await;
        }
        let watch = t == "chat.watch";
        if watch && !self.x.chat.can_watch(&pane) {
            return self.sec.json(&error(Value::from(id), "limit")).await;
        }
        if self.x.chat.jobs >= MAX_JOBS {
            return self.sec.json(&error(Value::from(id), "busy")).await;
        }
        self.x.chat.jobs += 1;
        let terminal = self.terminal_of(&target);
        let ctx = self.ctx.clone();
        let tx = self.x.chat.tx.clone();
        let id = id.to_owned();
        tokio::spawn(async move {
            let job = tokio::task::spawn_blocking(move || {
                let roots = Roots::from_env();
                let source = ctx.chat.source_of(&target, terminal.as_ref(), &roots);
                let ask = Ask { pane: &pane, target: &target, gen: gen.as_deref(), before, limit, live: watch };
                page_of(&ctx.chat, &ask, source).map(|p| {
                    let w = watch.then(|| Watch { pane: pane.clone(), target: target.clone(), gen: p.gen.clone(), source: p.source.clone(), live: p.live.map(|l| Arc::new(Mutex::new(l))) });
                    (p.reply, w)
                })
            })
            .await;
            let msg = match job {
                Ok(Ok((reply, watch))) => ChatMsg::Reply { id, result: Ok(reply), watch },
                Ok(Err(code)) => ChatMsg::Reply { id, result: Err(code), watch: None },
                Err(_) => ChatMsg::Reply { id, result: Err("internal"), watch: None },
            };
            let _ = tx.send(msg).await;
        });
        true
    }

    pub(super) async fn on_chat(&mut self, msg: ChatMsg) -> bool {
        match msg {
            ChatMsg::Reply { id, result, watch } => {
                self.x.chat.jobs = self.x.chat.jobs.saturating_sub(1);
                let reply = match result {
                    Ok(mut v) => {
                        v["re"] = Value::from(id);
                        v
                    }
                    Err(code) => error(Value::from(id), code),
                };
                if let Some(w) = watch {
                    self.x.chat.put_watch(w);
                    self.x.chat.wake(POLL_EVERY);
                }
                self.sec.json(&reply).await
            }
            ChatMsg::Polled { rows } => {
                self.x.chat.polling = false;
                for row in rows {
                    let event = match row {
                        PollRow::Items { pane, gen, items } => {
                            if !self.x.chat.watches.iter().any(|w| w.pane == pane && w.gen == gen) {
                                continue;
                            }
                            for e in items_events(&pane, &gen, items) {
                                if !self.sec.json(&e).await {
                                    return false;
                                }
                            }
                            continue;
                        }
                        PollRow::Reset { pane, gen, source } => {
                            let Some(w) = self.x.chat.watches.iter_mut().find(|w| w.pane == pane) else { continue };
                            w.gen = gen.clone();
                            w.source = source;
                            w.live = None;
                            json!({ "t": "chat.reset", "paneKey": pane, "gen": gen })
                        }
                    };
                    if !self.sec.json(&event).await {
                        return false;
                    }
                }
                if !self.x.chat.watches.is_empty() {
                    self.x.chat.wake(POLL_EVERY);
                }
                true
            }
            ChatMsg::Last { full, rows } => {
                self.x.chat.lasting = false;
                self.x.chat.last_at = Some(Instant::now());
                let event = last_event(full, &rows, &self.x.chat.sent);
                self.x.chat.sent = remember(&rows);
                if self.x.chat.last_dirty || self.x.chat.last_full {
                    self.x.chat.wake(LAST_GAP);
                }
                match event {
                    Some(e) => self.sec.json(&e).await,
                    None => true,
                }
            }
        }
    }

    pub(super) fn chat_tick(&mut self) {
        self.x.chat.due = None;
        if !self.x.chat.watches.is_empty() && !self.x.chat.polling {
            self.chat_poll();
        }
        if self.x.chat.cap && (self.x.chat.last_dirty || self.x.chat.last_full) && !self.x.chat.lasting {
            let wait = self.x.chat.last_at.map_or(Duration::ZERO, |at| LAST_GAP.saturating_sub(at.elapsed()));
            if wait.is_zero() || self.x.chat.last_full {
                self.chat_last();
            } else {
                self.x.chat.wake(wait);
            }
        }
    }

    fn chat_poll(&mut self) {
        let jobs: Vec<(String, Target, String, Option<Source>, Option<Arc<Mutex<Live>>>, Option<Value>)> =
            self.x.chat.watches.iter().map(|w| (w.pane.clone(), w.target.clone(), w.gen.clone(), w.source.clone(), w.live.clone(), self.terminal_of(&w.target))).collect();
        self.x.chat.polling = true;
        self.x.chat.last_poll = Some(Instant::now());
        let ctx = self.ctx.clone();
        let tx = self.x.chat.tx.clone();
        tokio::spawn(async move {
            let rows = tokio::task::spawn_blocking(move || {
                let roots = Roots::from_env();
                let mut rows = Vec::new();
                for (pane, target, gen, source, live, terminal) in jobs {
                    let now = ctx.chat.source_of(&target, terminal.as_ref(), &roots);
                    let moved = now.as_ref().map(|s| &s.path) != source.as_ref().map(|s| &s.path);
                    if moved {
                        let next = now.as_ref().map_or_else(String::new, |s| ctx.chat.gen(s));
                        if next != gen {
                            rows.push(PollRow::Reset { pane, gen: next, source: now });
                        }
                        continue;
                    }
                    let (Some(live), Some(src)) = (live, source) else { continue };
                    let polled = lock(&live).poll();
                    match polled {
                        Ok(Poll::Items(items)) if !items.is_empty() => rows.push(PollRow::Items { pane, gen, items: items.iter().map(|i| i.to_json(&ShellClean)).collect() }),
                        Ok(Poll::Items(_)) => {}
                        Ok(Poll::Reset) => rows.push(PollRow::Reset { pane, gen: ctx.chat.bump(&src), source: Some(src) }),
                        Err(_) => {}
                    }
                }
                rows
            })
            .await
            .unwrap_or_default();
            let _ = tx.send(ChatMsg::Polled { rows }).await;
        });
    }

    fn chat_last(&mut self) {
        let Some(summary) = self.ctx.summary() else {
            self.x.chat.last_dirty = false;
            return;
        };
        let full = std::mem::take(&mut self.x.chat.last_full);
        self.x.chat.last_dirty = false;
        self.x.chat.lasting = true;
        let panes: Vec<(String, Target, Option<Value>)> = agent_panes(&summary).into_iter().map(|(k, t)| {
            let term = self.terminal_of(&t);
            (k, t, term)
        }).collect();
        let ctx = self.ctx.clone();
        let tx = self.x.chat.tx.clone();
        tokio::spawn(async move {
            let rows = tokio::task::spawn_blocking(move || {
                let roots = Roots::from_env();
                panes
                    .into_iter()
                    .map(|(k, t, term)| {
                        let got = ctx.chat.source_of(&t, term.as_ref(), &roots).and_then(|s| ctx.chat.last(&s, t.agent));
                        (k, got.as_ref().map(|g| g.0.clone()), got.and_then(|g| g.1))
                    })
                    .collect::<Vec<LastRow>>()
            })
            .await
            .unwrap_or_default();
            let _ = tx.send(ChatMsg::Last { full, rows }).await;
        });
    }
}
