use std::collections::HashMap;
use std::path::{Path, PathBuf};

use core_store::extra::{LimitWindow, PlanLimits};
use core_store::model::{ActivityData, ActivityKind};
use core_store::store::NewActivity;
use core_store::persist::localtime::{civil_from_days, Zone, DAY_MS};
use core_store::{JsNum, Store};
use core_transcript::codex::extract::{CodexSignal, Limits, Window};
use core_transcript::codex::meta::read_session_meta;
use core_transcript::codex::tail::{RolloutTailer, CHUNK_BYTES, MAX_LINE};

use super::monitor::{CodexMonitor, Targets, Thread, IDLE_END_MS};
use super::names::parse_thread_names;
use super::state::{mode_of, CodexMode, CodexThreadState};

const TAIL_BYTES: u64 = 8 * 1024 * 1024;
const LIMITS_TAIL_BYTES: u64 = 2 * 1024 * 1024;
const MAX_INDEX_BYTES: usize = 4 * 1024 * 1024;

pub fn file_id(path: &str) -> Option<String> {
    let base = path.rsplit(['\\', '/']).next().unwrap_or(path);
    let lower = base.to_ascii_lowercase();
    if !lower.ends_with(".jsonl") || base.len() < 6 + 36 {
        return None;
    }
    let start = base.len() - 6 - 36;
    let id = base.get(start..start + 36)?;
    let ok = id.bytes().enumerate().all(|(i, c)| if matches!(i, 8 | 13 | 18 | 23) { c == b'-' } else { c.is_ascii_hexdigit() });
    let prefix = lower.get(..start)?;
    (ok && prefix.contains("rollout-")).then(|| id.to_owned())
}

fn day_dir(sessions: &Path, ms: i64) -> PathBuf {
    let local = ms + Zone::Local.offset_ms(ms);
    let (y, m, d) = civil_from_days(local.div_euclid(DAY_MS));
    sessions.join(y.to_string()).join(format!("{m:02}")).join(format!("{d:02}"))
}

fn window_of(w: &Option<Window>) -> Option<LimitWindow> {
    w.as_ref().map(|w| LimitWindow { used_pct: JsNum(w.used_pct), resets_at: w.resets_at.clone(), forecast: None })
}

fn mtime_ms(path: &Path) -> Option<f64> {
    let m = std::fs::metadata(path).ok()?.modified().ok()?;
    let d = m.duration_since(std::time::UNIX_EPOCH).ok()?;
    Some(d.as_secs_f64() * 1000.0)
}

impl CodexMonitor {
    pub(super) fn load_names(&mut self) {
        self.names = match std::fs::read(&self.index_file) {
            Ok(bytes) => {
                let tail = if bytes.len() > MAX_INDEX_BYTES { &bytes[bytes.len() - MAX_INDEX_BYTES..] } else { &bytes[..] };
                parse_thread_names(&String::from_utf8_lossy(tail))
            }
            Err(_) => HashMap::new(),
        };
    }

    pub fn reload_names(&mut self, store: &mut Store) {
        self.load_names();
        let ids: Vec<String> = self.threads.keys().cloned().collect();
        for id in ids {
            self.publish(store, &id, Vec::new());
        }
    }

    fn list_recent_files(&self, now: i64) -> Vec<(PathBuf, f64)> {
        let mut dirs = vec![day_dir(&self.sessions_dir, now)];
        let older = day_dir(&self.sessions_dir, now - 86_400_000);
        if older != dirs[0] {
            dirs.push(older);
        }
        let mut out = Vec::new();
        for dir in dirs {
            let Ok(read) = std::fs::read_dir(&dir) else { continue };
            let mut names: Vec<String> = read.filter_map(|e| e.ok().map(|e| e.file_name().to_string_lossy().into_owned())).collect();
            names.sort();
            for name in names {
                if file_id(&name).is_none() {
                    continue;
                }
                let file = dir.join(&name);
                if let Some(m) = mtime_ms(&file) {
                    out.push((file, m));
                }
            }
        }
        out
    }

    pub(super) fn discover_initial(&mut self, store: &mut Store, targets: Targets) {
        let now = self.deps.now();
        let files = self.list_recent_files(now);
        for (file, mtime) in &files {
            if now as f64 - mtime > self.config.stale_ttl_ms as f64 {
                continue;
            }
            self.add_thread(store, file, true, Some(*mtime as i64));
        }
        self.scan_now(store, targets);
        let ids: Vec<String> = self.threads.keys().cloned().collect();
        for id in ids {
            let Some(t) = self.threads.get(&id) else { continue };
            let own_process = self.last_scan_ok && matches!(t.mode, CodexMode::Exec | CodexMode::Tui);
            let live = if t.proc_.is_some() { !t.proc_gone && !t.superseded } else { !own_process && t.state.turn_open == Some(true) && now - t.last_write_at < IDLE_END_MS };
            if !live {
                self.threads.shift_remove(&id);
            }
        }
        self.seed_limits(store, &files);
        self.booting = false;
        let ids: Vec<String> = self.threads.keys().cloned().collect();
        for id in ids {
            self.publish(store, &id, Vec::new());
        }
    }

    fn seed_limits(&mut self, store: &mut Store, files: &[(PathBuf, f64)]) {
        if self.limits.is_some() {
            return;
        }
        let mut newest: Option<&(PathBuf, f64)> = None;
        for f in files {
            if newest.is_none_or(|n| f.1 > n.1) {
                newest = Some(f);
            }
        }
        let Some((file, _)) = newest else { return };
        let mut state = CodexThreadState::default();
        let mut tailer = RolloutTailer::new(file.clone(), Some(LIMITS_TAIL_BYTES), CHUNK_BYTES, MAX_LINE);
        let mut records = Vec::new();
        tailer.read_all(&mut records);
        for signals in &records {
            state.apply(signals);
        }
        if let Some(l) = state.limits {
            self.apply_limits(store, &l);
        }
    }

    pub(super) fn add_thread(&mut self, store: &mut Store, file: &Path, initial: bool, mtime: Option<i64>) -> Option<String> {
        let id = file_id(&file.to_string_lossy())?;
        if self.threads.contains_key(&id) {
            return Some(id);
        }
        if self.skipped.contains(&id) {
            return None;
        }
        let meta = read_session_meta(file);
        let mode = meta.as_ref().and_then(mode_of);
        let meta = meta?;
        let Some(mode) = mode else {
            self.skipped.insert(id);
            return None;
        };
        if meta.id != id {
            self.skipped.insert(id);
            return None;
        }
        let state = CodexThreadState { meta: Some(meta.clone()), ..CodexThreadState::default() };
        let thread = Thread {
            id: id.clone(),
            meta,
            mode,
            state,
            tailer: RolloutTailer::new(file.to_path_buf(), Some(TAIL_BYTES), CHUNK_BYTES, MAX_LINE),
            proc_: None,
            proc_gone: false,
            superseded: false,
            link: None,
            awaiting_scan: !initial,
            initial,
            last_write_at: mtime.unwrap_or_else(|| self.deps.now()),
            ended_at: None,
            finished_at: None,
            published: None,
            last_status: None,
        };
        self.threads.insert(id.clone(), thread);
        self.read(store, &id, true);
        Some(id)
    }

    pub(super) fn read(&mut self, store: &mut Store, id: &str, catch_up: bool) -> (Vec<NewActivity>, bool) {
        let now = self.deps.now();
        let mut drafts = Vec::new();
        let mut turn_started = false;
        let Some(t) = self.threads.get_mut(id) else { return (drafts, turn_started) };
        let mut records = Vec::new();
        t.tailer.read_all(&mut records);
        if records.is_empty() {
            return (drafts, turn_started);
        }
        if !catch_up {
            t.last_write_at = now;
        }
        for signals in &records {
            for s in signals {
                if matches!(s, CodexSignal::TurnStart { .. }) {
                    turn_started = true;
                }
                if let (CodexSignal::ToolCall { name, at, .. }, false, Some(p)) = (s, catch_up, t.published.as_ref()) {
                    drafts.push(NewActivity {
                        kind: ActivityKind::ToolStarted,
                        terminal_id: Some(p.terminal_id.clone()),
                        agent_id: Some(p.agent_id.clone()),
                        data: Some(ActivityData { tool: Some(name.clone()), ..ActivityData::default() }),
                        at: at.clone(),
                    });
                }
            }
            t.state.apply(signals);
        }
        if let Some(l) = t.state.limits.clone() {
            self.apply_limits(store, &l);
        }
        (drafts, turn_started)
    }

    fn apply_limits(&mut self, store: &mut Store, limits: &Limits) {
        if self.limits.as_ref().is_some_and(|c| core_transcript::jsstr::utf16_cmp(&limits.at, &c.at) != std::cmp::Ordering::Greater) {
            return;
        }
        self.limits = Some(limits.clone());
        let plan = PlanLimits { updated_at: limits.at.clone(), five_hour: window_of(&limits.five_hour), seven_day: window_of(&limits.seven_day), plan: limits.plan.clone() };
        let out = match self.limit_history.as_mut() {
            Some(h) => h.apply(Some(&plan)),
            None => Some(plan),
        };
        if let Some(out) = out {
            store.set_codex_limits(out);
        }
    }
}
