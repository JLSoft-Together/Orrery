use super::api::{scan_updates, Seen};
use super::base::{Entry, Tg};
use super::html::{cell, markup};
use super::message::{command_of, read_message, Command, TgFile, TgMessage};
use super::shape::Known;
use super::text::esc;
use super::text_act::act;
use super::text_more::{backlog_head, backlog_skipped};
use serde_json::{json, Map, Value};
use std::sync::Arc;

const POLL_TIMEOUT_S: u64 = 50;
const POLL_MIN_GAP_MS: i64 = 1000;
const BACKLOG_PAGE: usize = 100;
const BACKLOG_PAGES: usize = 5;
const BACKLOG_MS: f64 = 60.0 * 60_000.0;
const BACKLOG_PER_TOPIC: usize = 8;
const BACKLOG_LINE: usize = 80;
const BACKLOGS_MAX: usize = 20;
const ALBUM_MAX: usize = 10;
const HEAVY_AT_ONCE: usize = 2;
const KNOWN_TOPICS_MAX: usize = 200;
const ALLOWED: [&str; 3] = ["message", "my_chat_member", "callback_query"];

type Group = (String, Option<i64>, Vec<(Value, String)>);

fn update_id(u: &Value) -> Option<i64> {
    u.get("update_id").and_then(Value::as_f64).map(|f| f as i64)
}

pub fn replace_ci(text: &str, needle: &str, with: &str) -> String {
    let n: Vec<char> = needle.to_lowercase().chars().collect();
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        if !n.is_empty() && i + n.len() <= chars.len() && chars[i..i + n.len()].iter().zip(n.iter()).all(|(a, b)| a.to_lowercase().eq(b.to_lowercase())) {
            out.push_str(with);
            i += n.len();
            continue;
        }
        out.push(chars[i]);
        i += 1;
    }
    out
}

impl Tg {
    fn running(&self, run: u64) -> bool {
        self.lock().poll_run == run
    }

    fn back_off(&self, backoff: &mut u64) {
        (self.deps.sleep)(*backoff);
        *backoff = (*backoff * 2).min(60_000);
    }

    pub fn poll_loop(self: &Arc<Self>, run: u64) {
        let mut offset: Option<i64> = None;
        let mut backoff: u64 = 2000;
        self.lock().poll_seen = Some(Seen::default());
        while self.running(run) {
            if !self.want_updates() {
                self.lock().poll_run = 0;
                self.emit_state();
                break;
            }
            if self.lock().bot_id.is_none() && !self.who_am_i().get("ok").and_then(Value::as_bool).unwrap_or(false) {
                self.back_off(&mut backoff);
                continue;
            }
            let Some(off) = offset else {
                let first = self.call_json("getUpdates", json!({"offset": 0, "limit": BACKLOG_PAGE, "timeout": 0, "allowed_updates": ALLOWED}));
                if let Some(e) = first.error() {
                    if self.poll_failed(e, run) {
                        break;
                    }
                    self.back_off(&mut backoff);
                    continue;
                }
                self.poll_ok();
                let mut waiting: Vec<Value> = Vec::new();
                let mut next = 0;
                let mut list = first.result().as_array().cloned().unwrap_or_default();
                for page in 0.. {
                    for u in &list {
                        waiting.push(u.clone());
                        if let Some(id) = update_id(u).filter(|id| *id >= next) {
                            next = id + 1;
                        }
                    }
                    if list.len() < BACKLOG_PAGE || page >= BACKLOG_PAGES - 1 {
                        break;
                    }
                    let more = self.call_json("getUpdates", json!({"offset": next, "limit": BACKLOG_PAGE, "timeout": 0, "allowed_updates": ALLOWED}));
                    if !more.ok() {
                        break;
                    }
                    list = more.result().as_array().cloned().unwrap_or_default();
                }
                offset = Some(next);
                if self.running(run) {
                    self.offer_backlog(&waiting);
                }
                (self.deps.sleep)(POLL_MIN_GAP_MS as u64);
                continue;
            };
            let started = self.now();
            let body = json!({"offset": off, "timeout": POLL_TIMEOUT_S, "allowed_updates": ALLOWED});
            let res = self.call("getUpdates", super::api::Body::Json(body.as_object().cloned().unwrap_or_default()), (POLL_TIMEOUT_S + 15) * 1000);
            if !self.running(run) {
                break;
            }
            let quick = self.now() - started;
            if quick < POLL_MIN_GAP_MS {
                (self.deps.sleep)((POLL_MIN_GAP_MS - quick) as u64);
            }
            if !self.running(run) {
                break;
            }
            if let Some(e) = res.error() {
                if self.poll_failed(e, run) {
                    break;
                }
                self.back_off(&mut backoff);
                continue;
            }
            self.poll_ok();
            backoff = 2000;
            let updates = res.result();
            let seen = {
                let mut st = self.lock();
                let mut seen = st.poll_seen.take().unwrap_or_default();
                scan_updates(&updates, &mut seen);
                st.poll_seen = Some(seen.clone());
                seen
            };
            self.learn(&seen);
            let list = updates.as_array().cloned().unwrap_or_default();
            let mut off = off;
            for u in &list {
                if !self.running(run) {
                    break;
                }
                if let Some(id) = update_id(u).filter(|id| *id >= off) {
                    off = id + 1;
                }
                self.dispatch(u.clone());
            }
            offset = Some(off);
            let mut st = self.lock();
            if !list.is_empty() && Some(off - 1) != st.data.last_update {
                st.data.last_update = Some(off - 1);
                Tg::save_st(&self.deps, &st);
            }
        }
    }

    fn offer_backlog(self: &Arc<Self>, updates: &[Value]) {
        let (bot, owner, receiving, last, bot_id) = {
            let st = self.lock();
            (st.data.bot.clone(), st.data.owner.clone(), st.receiving, st.data.last_update, st.bot_id)
        };
        let (Some(bot), Some((owner, _)), true) = (bot, owner, receiving) else { return };
        let now = self.now() as f64;
        let mut groups: Vec<(String, Group)> = Vec::new();
        for update in updates {
            if let (Some(uid), Some(last)) = (update_id(update), last) {
                if uid <= last {
                    continue;
                }
            }
            let Some(msg) = read_message(update, bot_id) else { continue };
            if msg.from_bot || msg.from_id != owner || msg.chat_type == "private" || msg.date.is_none_or(|d| now - d > BACKLOG_MS) {
                continue;
            }
            let Some(cmd) = command_of(&msg, &bot) else { continue };
            if matches!(cmd, Command::Pair(_) | Command::Help) {
                continue;
            }
            let known = {
                let st = self.lock();
                st.data.session_routes.iter().any(|r| r.chat_id == msg.chat_id && r.thread_id == msg.thread_id) || st.data.routes.iter().any(|r| r.chat_id == msg.chat_id && r.thread_id == msg.thread_id)
            };
            if !known {
                continue;
            }
            let key = format!("{}:{}", msg.chat_id, msg.thread_id.map(|t| t.to_string()).unwrap_or_default());
            let line = self.backlog_line(&msg, &bot);
            match groups.iter_mut().find(|(k, _)| *k == key) {
                Some((_, g)) => {
                    if g.2.len() < BACKLOG_PER_TOPIC {
                        g.2.push((update.clone(), line));
                    }
                }
                None => groups.push((key, (msg.chat_id.clone(), msg.thread_id, vec![(update.clone(), line)]))),
            }
        }
        let vi = self.vi();
        for (_, (chat, thread, items)) in groups {
            let key = self.add_action(Entry { kind: "bg".into(), chat_id: chat.clone(), thread_id: thread, ..Entry::default() });
            {
                let mut st = self.lock();
                st.backlogs.push((key.clone(), items.iter().map(|(u, _)| u.clone()).collect()));
                if st.backlogs.len() > BACKLOGS_MAX {
                    st.backlogs.remove(0);
                }
            }
            let mut lines = vec![backlog_head(vi, items.len())];
            lines.extend(items.iter().enumerate().map(|(n, (_, l))| format!("{}. {l}", n + 1)));
            let mut extra = Map::new();
            extra.insert("reply_markup".into(), markup(vi, &[vec![cell("bg", &key), cell("bx", &key)]]));
            self.post(&chat, thread, &lines.join("\n"), extra);
        }
    }

    fn backlog_line(&self, msg: &TgMessage, bot: &str) -> String {
        let flat = replace_ci(&msg.text, &format!("@{bot}"), " ");
        let flat = super::jsx::collapse(&flat, core_transcript::jsstr::js_space, " ");
        let raw = super::jsx::trim(&flat).to_string();
        let raw = if raw.is_empty() { msg.file.as_ref().map(|f| f.name.clone()).unwrap_or_default() } else { raw };
        let text = self.deps.masker.clean(&raw);
        esc(&if super::jsx::len(&text) > BACKLOG_LINE { format!("{}…", super::jsx::slice(&text, BACKLOG_LINE - 1)) } else { text })
    }

    pub fn on_backlog(self: &Arc<Self>, id: &str, kind: &str, key: &str, entry: &Entry, cb: &Value) {
        let items = {
            let mut st = self.lock();
            let items = super::jsx::js_remove(&mut st.backlogs, key);
            if items.is_some() {
                super::jsx::js_remove(&mut st.actions, key);
            }
            items
        };
        let Some(items) = items else {
            self.answer(id, Some(act(self.vi(), "expired")));
            return;
        };
        self.answer(id, None);
        self.clear_markup(&entry.chat_id, cb["message"]["message_id"].as_i64());
        if kind == "bx" {
            self.post(&entry.chat_id, entry.thread_id, &backlog_skipped(self.vi(), items.len()), Map::new());
            return;
        }
        for u in items {
            self.on_update(&u);
        }
    }

    pub fn dispatch(self: &Arc<Self>, update: Value) {
        let m = update.get("message").filter(|m| m.is_object());
        let heavy = m.is_some_and(|m| m.get("photo").is_some_and(Value::is_array) || ["document", "video", "audio", "voice"].iter().any(|k| m.get(*k).is_some_and(|v| !v.is_null() && v != &Value::Bool(false))) || m.get("text").and_then(Value::as_str).is_some_and(mentions_screen));
        if update.get("callback_query").is_some_and(|c| !c.is_null()) || heavy {
            self.spawn(move |tg| tg.on_update(&update));
            return;
        }
        let mut light = self.light.lock().unwrap_or_else(|e| e.into_inner());
        if light.is_none() {
            let (tx, rx) = std::sync::mpsc::channel::<Value>();
            let me = Arc::downgrade(self);
            std::thread::spawn(move || {
                for u in rx {
                    let Some(tg) = me.upgrade() else { break };
                    tg.on_update(&u);
                }
            });
            *light = Some(tx);
        }
        if let Some(tx) = light.as_ref() {
            let _ = tx.send(update);
        }
    }

    pub fn heavy_slot<T>(&self, work: impl FnOnce() -> T) -> T {
        {
            let mut st = self.lock();
            while st.heavy_running >= HEAVY_AT_ONCE {
                st = self.heavy.wait(st).unwrap_or_else(|e| e.into_inner());
            }
            st.heavy_running += 1;
        }
        let out = work();
        self.lock().heavy_running -= 1;
        self.heavy.notify_one();
        out
    }

    pub fn on_update(self: &Arc<Self>, update: &Value) {
        let need = {
            let st = self.lock();
            st.bot_id.is_none() || st.data.bot.is_none()
        };
        if need {
            self.who_am_i();
        }
        if let Some(cb) = update.get("callback_query").filter(|c| c.is_object()) {
            self.on_callback(cb);
            return;
        }
        let (bot_id, bot) = {
            let st = self.lock();
            (st.bot_id, st.data.bot.clone())
        };
        let Some(msg) = read_message(update, bot_id) else { return };
        if msg.from_bot || bot.is_none() {
            return;
        }
        if msg.group.is_some() {
            self.collect_album(msg);
            return;
        }
        self.on_message(&msg, Vec::new());
    }

    fn collect_album(self: &Arc<Self>, msg: TgMessage) {
        let key = format!("{}:{}", msg.chat_id, msg.group.clone().unwrap_or_default());
        let gen = {
            let mut st = self.lock();
            st.timer_gen += 1;
            let gen = st.timer_gen;
            let album = st.albums.entry(key.clone()).or_insert_with(|| (Vec::new(), 0));
            if album.0.len() < ALBUM_MAX {
                album.0.push(msg);
            }
            album.1 = gen;
            gen
        };
        let wait = self.deps.album_ms;
        self.spawn(move |tg| {
            (tg.deps.sleep)(wait);
            let (items, bot) = {
                let mut st = tg.lock();
                if st.albums.get(&key).map(|a| a.1) != Some(gen) {
                    return;
                }
                let items = st.albums.remove(&key).map(|a| a.0).unwrap_or_default();
                (items, st.data.bot.clone())
            };
            let Some(bot) = bot else { return };
            let Some(head) = items.iter().find(|m| matches!(command_of(m, &bot), Some(Command::File(..)))).cloned() else { return };
            let more: Vec<TgFile> = items.iter().filter(|m| **m != head && m.from_id == head.from_id).filter_map(|m| m.file.clone()).collect();
            tg.on_message(&head, more);
        });
    }

    pub fn learn(&self, seen: &Seen) {
        let changed = {
            let mut st = self.lock();
            let mut changed = false;
            for (id, (title, forum)) in &seen.chats {
                let old = super::jsx::js_get(&st.data.known, id).cloned();
                if old.is_none() && st.data.known.len() >= 50 {
                    continue;
                }
                let mut k = old.clone().unwrap_or(Known { title: title.clone(), is_forum: *forum, ..Known::default() });
                if old.is_none() || (title != id && k.title != *title) {
                    changed = true;
                }
                if title != id {
                    k.title = title.clone();
                }
                for (t, n) in seen.topics(id).cloned().unwrap_or_default() {
                    let ts = t.to_string();
                    if super::jsx::js_get(&k.topics, &ts) == Some(&n) || k.topics.len() >= KNOWN_TOPICS_MAX {
                        continue;
                    }
                    super::jsx::js_insert(&mut k.topics, &ts, n);
                    changed = true;
                }
                super::jsx::js_insert(&mut st.data.known, id, k);
            }
            changed && Tg::save_st(&self.deps, &st)
        };
        if changed {
            self.emit_state();
        }
    }
}

fn mentions_screen(text: &str) -> bool {
    let low = text.to_lowercase();
    let b = low.as_bytes();
    let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
    let mut i = 0;
    while let Some(at) = low[i..].find("screen") {
        let s = i + at;
        let mut e = s + 6;
        if low[e..].starts_with("shot") && (e + 4 == b.len() || !word(b[e + 4])) {
            e += 4;
        }
        if (s == 0 || !word(b[s - 1])) && (e == b.len() || !word(b[e])) {
            return true;
        }
        i = s + 1;
    }
    false
}
