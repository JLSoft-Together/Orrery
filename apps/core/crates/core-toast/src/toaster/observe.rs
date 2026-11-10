use std::collections::HashSet;

use core_store::extra::{BackgroundJob, JobState, PlanLimits};
use core_store::model::{LimitHit, TerminalSession, TerminalStatus};
use core_store::num::JsNum;
use core_store::persist::jsstr::{js_round, slice_units};
use serde_json::Value;

use super::{Draft, Toaster, LONG_JOB_MS, TOAST_THROTTLE_MS};
use crate::message::{HostMessage, PhoneEpisode, TelegramEpisode};
use crate::util::{date_ms, js_num, local_hhmm, name_of, phone_name, quiet_for, reminders_due, wait_reason, WaitReason, CONTEXT_CRITICAL};

type Lookup<'a, T> = Option<&'a dyn Fn(&str) -> Option<T>>;

#[derive(Default)]
pub struct ToastSnapshot<'a> {
    pub terminals: Vec<&'a TerminalSession>,
    pub jobs: Option<&'a [BackgroundJob]>,
    pub limits: Option<Option<&'a PlanLimits>>,
    pub context_pct: Lookup<'a, f64>,
    pub tool_of: Lookup<'a, String>,
    pub reply_of: Lookup<'a, String>,
    pub ask_of: Lookup<'a, Value>,
    pub snoozed: Option<&'a dyn Fn(&TerminalSession) -> bool>,
}

fn since_key(t: &TerminalSession) -> &str {
    t.status_since.as_deref().unwrap_or("")
}

fn ended(t: &TerminalSession) -> bool {
    t.ended_at.as_deref().is_some_and(|e| !e.is_empty())
}

fn wait_kind(t: &TerminalSession) -> &'static str {
    match wait_reason(t.waiting_for.as_deref()) {
        WaitReason::Question => "question",
        WaitReason::Permission => "permission",
        WaitReason::Other => "waiting",
    }
}

fn finite(v: f64) -> Option<JsNum> {
    v.is_finite().then_some(JsNum(v))
}

impl Toaster {
    pub fn observe(&mut self, snap: &ToastSnapshot) {
        let now = self.now();
        let snoozed = |t: &TerminalSession| snap.snoozed.is_some_and(|f| f(t));
        let tool_of = |id: &str| snap.tool_of.and_then(|f| f(id));
        let mut alive = HashSet::new();
        for &t in &snap.terminals {
            alive.insert(t.id.clone());
            self.terms.insert(t.id.clone(), t.clone());
            let before = self.prev.insert(t.id.clone(), t.status);
            let worked = self.track_work(t, before, now);
            self.watch_stuck(t, before, now);
            if let (Some(hit), false) = (&t.limit_hit, ended(t)) {
                self.watch_limit_hit(hit, before.is_some(), now);
            }
            let pct = snap.context_pct.and_then(|f| f(&t.id));
            self.watch_context(t, before, pct);
            if t.status != TerminalStatus::Idle || ended(t) {
                self.hide_key(&format!("done:{}", t.id));
            }
            if t.status != TerminalStatus::Waiting || ended(t) {
                self.hide_key(&format!("waiting:{}", t.id));
                if before == Some(TerminalStatus::Working) && t.status == TerminalStatus::Idle && !ended(t) {
                    self.show_done(t);
                    if let Some(w) = worked {
                        let reply = snap.reply_of.and_then(|f| f(&t.id));
                        self.telegram(t, "done", TelegramEpisode { worked: Some(JsNum(w)), reply, ..Default::default() });
                        if w >= LONG_JOB_MS {
                            let m = PhoneEpisode { episode: format!("d:{}|{}", t.id, since_key(t)), reason: "done", name: phone_name(t), since: Some(JsNum(now - w)), ..Default::default() };
                            self.phone(m, None);
                        }
                    }
                }
                continue;
            }
            if snoozed(t) {
                self.hide_key(&format!("waiting:{}", t.id));
            }
            let episode = format!("{}|{}", t.id, since_key(t));
            match before {
                None => {
                    self.reminded.insert(episode, reminders_due(t.status_since.as_deref(), now, self.zone));
                }
                Some(b) if b != TerminalStatus::Waiting => {
                    self.reminded.insert(episode, 0.0);
                    let ask = snap.ask_of.and_then(|f| f(&t.id));
                    self.show_waiting(t, now, tool_of(&t.id), ask, snoozed(t));
                }
                Some(_) => self.remind(t, &episode, now, tool_of(&t.id), snoozed(t)),
            }
        }
        let gone: Vec<String> = self.prev.keys().filter(|id| !alive.contains(*id)).cloned().collect();
        for id in gone {
            self.prev.shift_remove(&id);
            self.work_since.remove(&id);
            self.terms.shift_remove(&id);
            self.last_at.remove(&id);
            self.hide_for(&id);
        }
        let waiting: HashSet<String> = self.terms.values().filter(|t| t.status == TerminalStatus::Waiting).map(|t| format!("{}|{}", t.id, since_key(t))).collect();
        self.reminded.retain(|e, _| waiting.contains(e));
        let live: HashSet<String> = self.terms.values().map(|t| format!("{}|{}", t.id, t.last_activity_at.as_deref().unwrap_or(""))).collect();
        self.stuck_sent.retain(|e| live.contains(e));
        let terms = &self.terms;
        self.context_high.retain(|id| terms.contains_key(id));
        if let Some(jobs) = snap.jobs {
            self.observe_jobs(jobs);
        }
        if let Some(limits) = snap.limits {
            self.observe_limits(limits, now);
        }
    }

    fn track_work(&mut self, t: &TerminalSession, before: Option<TerminalStatus>, now: f64) -> Option<f64> {
        if t.status == TerminalStatus::Working {
            if before != Some(TerminalStatus::Working) || !self.work_since.contains_key(&t.id) {
                let since = date_ms(t.status_since.as_deref(), self.zone);
                let start = if before.is_none() { f64::INFINITY } else if since.is_finite() { since } else { now };
                self.work_since.insert(t.id.clone(), start);
            }
            return None;
        }
        let since = self.work_since.remove(&t.id)?;
        since.is_finite().then(|| (now - since).max(0.0))
    }

    fn telegram(&mut self, t: &TerminalSession, reason: &'static str, extra: TelegramEpisode) {
        let Some(cwd) = t.cwd.as_ref().filter(|c| !c.is_empty()) else { return };
        if !self.telegram_on {
            return;
        }
        let episode = format!("t{}:{}|{}", if reason == "done" { "d" } else { "w" }, t.id, since_key(t));
        self.outbox.push(HostMessage::Telegram(TelegramEpisode { episode, reason, terminal_id: t.id.clone(), name: phone_name(t), cwd: cwd.clone(), ..extra }));
    }

    fn phone_waiting(&mut self, t: &TerminalSession, now: f64, tool: Option<String>, snoozed: bool) {
        if snoozed {
            return;
        }
        let since = if t.status_since.is_some() { date_ms(t.status_since.as_deref(), self.zone) } else { now };
        let m = PhoneEpisode {
            episode: format!("w:{}|{}", t.id, since_key(t)),
            reason: wait_kind(t),
            name: phone_name(t),
            tool: tool.filter(|s| !s.is_empty()),
            since: Some(JsNum(if since.is_finite() { since } else { now })),
            ..Default::default()
        };
        self.phone(m, Some("waiting"));
    }

    fn show_waiting(&mut self, t: &TerminalSession, now: f64, tool: Option<String>, ask: Option<Value>, snoozed: bool) {
        let kind = wait_kind(t);
        let since = if t.status_since.is_some() { date_ms(t.status_since.as_deref(), self.zone) } else { now };
        let tool_opt = tool.clone().filter(|s| !s.is_empty());
        let ask = if kind == "question" { ask } else { None };
        self.telegram(t, kind, TelegramEpisode { since: Some(JsNum(if since.is_finite() { since } else { now })), tool: tool_opt.clone(), ask, ..Default::default() });
        self.phone_waiting(t, now, tool.clone(), snoozed);
        if !self.can("waiting") || snoozed {
            return;
        }
        if now - self.last_at.get(&t.id).copied().unwrap_or(f64::NEG_INFINITY) < TOAST_THROTTLE_MS {
            return;
        }
        self.last_at.insert(t.id.clone(), now);
        let text = self.text();
        let reason = t.waiting_for.as_deref().map(|w| slice_units(w, 120));
        let d = Draft { title: text.waiting_title(&name_of(t)), body: text.waiting_body(reason.as_deref(), tool.as_deref()), url: self.link_for(Some(&t.id)), open: text.open(), persistent: true, snooze: true };
        self.show(&format!("waiting:{}", t.id), d);
    }

    fn remind(&mut self, t: &TerminalSession, episode: &str, now: f64, tool: Option<String>, snoozed: bool) {
        let due = reminders_due(t.status_since.as_deref(), now, self.zone);
        let Some(sent) = self.reminded.get(episode).copied() else {
            self.reminded.insert(episode.to_string(), due);
            return;
        };
        if due <= sent {
            return;
        }
        self.reminded.insert(episode.to_string(), due);
        self.phone_waiting(t, now, tool.clone(), snoozed);
        if !self.can("waiting") || snoozed {
            return;
        }
        let text = self.text();
        let since = if t.status_since.is_some() { date_ms(t.status_since.as_deref(), self.zone) } else { now };
        self.last_at.insert(t.id.clone(), now);
        let minutes = js_round((now - since) / 60_000.0).max(1.0);
        let reason = t.waiting_for.as_deref().map(|w| slice_units(w, 120));
        let d = Draft { title: text.reminder_title(&name_of(t), &js_num(minutes)), body: text.waiting_body(reason.as_deref(), tool.as_deref()), url: self.link_for(Some(&t.id)), open: text.open(), persistent: true, snooze: true };
        self.show(&format!("waiting:{}", t.id), d);
    }

    fn watch_stuck(&mut self, t: &TerminalSession, before: Option<TerminalStatus>, now: f64) {
        let quiet = if ended(t) { None } else { quiet_for(t, now, self.prefs().stuck_minutes, self.zone) };
        let Some(quiet) = quiet else {
            self.hide_key(&format!("stuck:{}", t.id));
            return;
        };
        let episode = format!("{}|{}", t.id, t.last_activity_at.as_deref().unwrap_or(""));
        if !self.stuck_sent.insert(episode) || before.is_none() || !self.can("stuck") {
            return;
        }
        let text = self.text();
        let d = Draft { title: text.stuck_title(&name_of(t)), body: text.stuck_body(&js_num(js_round(quiet / 60_000.0))), url: self.link_for(Some(&t.id)), open: text.open(), persistent: false, snooze: false };
        self.show(&format!("stuck:{}", t.id), d);
    }

    fn watch_context(&mut self, t: &TerminalSession, before: Option<TerminalStatus>, pct: Option<f64>) {
        let Some(pct) = pct.filter(|p| *p >= CONTEXT_CRITICAL && !ended(t)) else {
            self.context_high.remove(&t.id);
            return;
        };
        if !self.context_high.insert(t.id.clone()) || before.is_none() || !self.can("context") {
            return;
        }
        let text = self.text();
        let d = Draft { title: text.context_title(&name_of(t), &js_num(js_round(pct))), body: text.context_body(), url: self.link_for(Some(&t.id)), open: text.open(), persistent: false, snooze: false };
        self.show(&format!("context:{}", t.id), d);
    }

    fn show_done(&mut self, t: &TerminalSession) {
        if !self.can("done") {
            return;
        }
        let text = self.text();
        let d = Draft { title: text.done_title(&name_of(t)), body: text.done_body(), url: self.link_for(Some(&t.id)), open: text.open(), persistent: false, snooze: false };
        self.show(&format!("done:{}", t.id), d);
    }

    fn observe_jobs(&mut self, jobs: &[BackgroundJob]) {
        let mut alive = HashSet::new();
        for job in jobs {
            alive.insert(job.id.clone());
            let before = self.job_prev.insert(job.id.clone(), job.state);
            if job.state != JobState::Blocked {
                self.hide_key(&format!("job:{}", job.id));
                continue;
            }
            if before.is_none() || before == Some(JobState::Blocked) {
                continue;
            }
            let owner = job.session_id.as_ref().filter(|s| !s.is_empty()).and_then(|sid| self.terms.values().find(|t| t.claude_session_id.as_ref() == Some(sid))).cloned();
            let since = date_ms(job.state_since.as_deref(), self.zone);
            let m = PhoneEpisode {
                episode: format!("j:{}|{}", job.id, job.state_since.as_deref().unwrap_or("")),
                reason: "job",
                name: owner.as_ref().map_or_else(|| "Claude".to_string(), phone_name),
                since: finite(since),
                ..Default::default()
            };
            self.phone(m, Some("job"));
            if !self.can("job") {
                continue;
            }
            let text = self.text();
            let name = slice_units(job.name.as_deref().unwrap_or(&job.id), 80);
            let open = if owner.is_some() { text.open() } else { text.open_dashboard() };
            let d = Draft { title: text.job_title(&name), body: text.job_body(), url: self.link_for(owner.as_ref().map(|o| o.id.as_str())), open, persistent: true, snooze: false };
            self.show(&format!("job:{}", job.id), d);
        }
        let gone: Vec<String> = self.job_prev.keys().filter(|id| !alive.contains(*id)).cloned().collect();
        for id in gone {
            self.job_prev.shift_remove(&id);
            self.hide_key(&format!("job:{id}"));
        }
    }

    fn watch_limit_hit(&mut self, hit: &LimitHit, seen_before: bool, now: f64) {
        let window = hit.window.as_deref().filter(|w| *w == "five_hour" || *w == "seven_day").map(String::from);
        let key = format!("hit:{}|{}", hit.window.as_deref().unwrap_or("?"), hit.resets_at.as_deref().unwrap_or(&hit.at));
        if !self.limit_sent.insert(key.clone()) {
            return;
        }
        let resets_at = date_ms(hit.resets_at.as_deref(), self.zone);
        if !seen_before || (resets_at.is_finite() && resets_at <= now) {
            return;
        }
        let m = PhoneEpisode { episode: format!("lh:{key}"), reason: "limit", name: "Claude".into(), used_pct: Some(JsNum(100.0)), hit: Some(true), window, resets_at: finite(resets_at), ..Default::default() };
        self.phone(m, Some("limit"));
    }

    fn observe_limits(&mut self, limits: Option<&PlanLimits>, now: f64) {
        let w = limits.and_then(|l| l.five_hour.as_ref());
        let f = w.and_then(|w| w.forecast.as_ref());
        let full = date_ms(f.and_then(|f| f.full_at.as_deref()), self.zone);
        let primed = self.limits_primed;
        if limits.is_some() {
            self.limits_primed = true;
        }
        let (Some(w), Some(true)) = (w, f.and_then(|f| f.before_reset)) else { return };
        if !full.is_finite() || w.used_pct.0 >= 100.0 || full - now > 60.0 * 60_000.0 {
            return;
        }
        let key = w.resets_at.clone().unwrap_or_else(|| "unknown".into());
        if !self.limit_sent.insert(key.clone()) || !primed {
            return;
        }
        let resets_at = date_ms(w.resets_at.as_deref(), self.zone);
        let pct = js_round(w.used_pct.0);
        let m = PhoneEpisode { episode: format!("l:{key}"), reason: "limit", name: "Claude".into(), window: Some("five_hour".into()), used_pct: Some(JsNum(pct)), resets_at: finite(resets_at), ..Default::default() };
        self.phone(m, Some("limit"));
        if !self.can("limit") {
            return;
        }
        let text = self.text();
        let d = Draft { title: text.limit_title(&js_num(pct)), body: text.limit_body(&local_hhmm(full, self.zone)), url: self.link_for(None), open: text.open_dashboard(), persistent: false, snooze: false };
        self.show("limit:5h", d);
    }
}
