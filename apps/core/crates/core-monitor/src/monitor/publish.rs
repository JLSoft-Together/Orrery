use std::sync::Arc;

use core_store::extra::SessionRecord;
use core_store::model::{ActivityData, ActivityKind, AgentRole, PeerLink, Provider, TerminalSession};
use core_store::store::NewActivity;
use core_store::time::now_ms;
use core_store::{JsNum, Store};
use core_transcript::time::parse_iso;
use indexmap::IndexMap;

use super::Monitor;
use crate::gitstat::same_diff;
use crate::hints::ContextHints;
use crate::peers::{PeerMessage, PeerSink};
use crate::sidefiles::COMPACT_STALE_MS;
use crate::tracker::MonTracker;
use core_store::persist::aliases::AliasStore;

struct PeerCtx<'a> {
    store: &'a mut Store,
    trackers: &'a IndexMap<String, MonTracker>,
    aliases: &'a AliasStore,
    hints: &'a ContextHints,
}

pub(super) fn title_of(aliases: &AliasStore, hints: &ContextHints, t: &MonTracker) -> String {
    aliases.get(&t.terminal_id).map_or_else(|| t.build_terminal(hints).title, str::to_owned)
}

impl PeerSink for PeerCtx<'_> {
    fn upsert_link(&mut self, link: PeerLink) {
        self.store.upsert_link(link);
    }

    fn remove_link(&mut self, id: &str) {
        self.store.remove_link(id);
    }

    fn sender_of(&self, from_key: &str) -> Option<String> {
        let mut ended = None;
        for t in self.trackers.values() {
            if t.record.sock_key.as_deref() != Some(from_key) {
                continue;
            }
            if !t.ended() {
                return Some(t.terminal_id.clone());
            }
            ended.get_or_insert_with(|| t.terminal_id.clone());
        }
        ended
    }

    fn on_message(&mut self, m: PeerMessage) {
        let title = |id: &str| self.trackers.get(id).map(|t| title_of(self.aliases, self.hints, t));
        let data = ActivityData { title: title(&m.from), to: title(&m.to), ..ActivityData::default() };
        self.store.push_activity(NewActivity { kind: ActivityKind::PeerMessage, terminal_id: Some(m.from), agent_id: Some(m.agent_id), data: Some(data), at: Some(m.at) });
    }
}

impl Monitor {
    pub(super) fn title_of(&self, id: &str) -> Option<String> {
        self.trackers.get(id).map(|t| title_of(&self.stores.aliases, &self.hints, t))
    }

    pub(super) fn drain_peers(&mut self, id: &str) {
        let Some(t) = self.trackers.get_mut(id) else { return };
        let (inbound, sent) = t.drain_peers();
        let now = self.deps.now();
        if !sent.is_empty() {
            self.peers.sent(sent, now);
        }
        if !inbound.is_empty() {
            self.peers.inbound(inbound, now_ms());
        }
    }

    pub(super) fn flush_peers(&mut self, store: &mut Store, final_pass: bool) {
        let now = self.deps.now();
        let mut ctx = PeerCtx { store, trackers: &self.trackers, aliases: &self.stores.aliases, hints: &self.hints };
        self.peers.flush(&mut ctx, now, now_ms(), final_pass);
    }

    pub(super) fn prune_peers(&mut self, store: &mut Store) {
        let now = self.deps.now();
        let mut ctx = PeerCtx { store, trackers: &self.trackers, aliases: &self.stores.aliases, hints: &self.hints };
        self.peers.prune(&mut ctx, now);
    }

    pub(super) fn drop_peer_terminal(&mut self, store: &mut Store, id: &str) {
        let mut ctx = PeerCtx { store, trackers: &self.trackers, aliases: &self.stores.aliases, hints: &self.hints };
        self.peers.drop_terminal(&mut ctx, id);
    }

    fn hook_fields(&self, t: &mut TerminalSession, session_id: Option<&str>) {
        let Some(h) = session_id.and_then(|s| self.hooks.get(s)) else { return };
        let since = h.compacting_since.as_deref().filter(|s| !s.is_empty());
        let compacting = since.and_then(parse_iso).is_some_and(|at| self.deps.now() - at < COMPACT_STALE_MS);
        if compacting {
            t.compacting_since = since.map(str::to_owned);
            t.compact_trigger = h.compact_trigger;
        }
        if h.denied_count != 0.0 {
            t.denied_count = Some(h.denied_count as u64);
            t.denied_tools = Some(Arc::clone(&h.denied_tools));
        }
    }

    pub(super) fn publish(&mut self, store: &mut Store, id: &str) {
        let Some(tracker) = self.trackers.get(id) else { return };
        let sid = tracker.session_id().map(str::to_owned);
        let mut terminal = tracker.build_terminal(&self.hints);
        terminal.alias = self.stores.aliases.get(id).map(str::to_owned);
        terminal.note = self.stores.notes.get(id).map(str::to_owned);
        terminal.cost_usd = sid.as_deref().and_then(|s| self.costs.get(s)).map(|c| JsNum(*c));
        terminal.diff = self.diffs.get(id).cloned();
        self.hook_fields(&mut terminal, sid.as_deref());
        let agents = tracker.build_agents(&self.hints, &terminal);
        store.upsert_terminal(terminal.clone());
        let title = terminal.alias.clone().unwrap_or_else(|| terminal.title.clone());
        self.stores.timeline.observe(&terminal.id, &title, terminal.status, terminal.status_since.as_deref());
        let ids: Vec<&str> = agents.iter().map(|a| a.id.as_str()).collect();
        let stale: Vec<String> = store.agents_of(id).iter().filter(|a| !ids.contains(&a.id.as_str()) && a.provider != Some(Provider::Codex)).map(|a| a.id.clone()).collect();
        for a in stale {
            store.remove_agent(&a);
        }
        for a in agents {
            store.upsert_agent(a);
        }
    }

    pub(super) fn record_of(&self, store: &Store, id: &str) -> Option<SessionRecord> {
        let terminal = store.get_terminal(id)?;
        let ended_at = terminal.ended_at.clone()?;
        let agents: Vec<_> = store.agents_of(id).into_iter().filter(|a| a.provider != Some(Provider::Codex)).collect();
        let main = agents.iter().find(|a| a.role == AgentRole::Main);
        let totals = self.stores.timeline.totals(id);
        let (mut input, mut output, mut cache) = (0u64, 0u64, 0u64);
        for a in &agents {
            input += a.input_tokens.unwrap_or(0);
            output += a.output_tokens.unwrap_or(0);
            cache += a.cache_read_tokens.unwrap_or(0);
        }
        Some(SessionRecord {
            id: terminal.id.clone(),
            title: terminal.alias.clone().unwrap_or_else(|| terminal.title.clone()),
            ended_at,
            input_tokens: input,
            output_tokens: output,
            cache_read_tokens: cache,
            subagents: (agents.len() - usize::from(main.is_some())) as u64,
            session_id: terminal.claude_session_id.clone(),
            cwd: terminal.cwd.clone(),
            git_branch: terminal.git_branch.clone(),
            model: main.and_then(|m| m.model.clone()),
            started_at: terminal.started_at.clone(),
            compactions: terminal.compactions,
            cost_usd: terminal.cost_usd,
            working_ms: (totals.working != 0.0).then_some(JsNum(totals.working)),
            waiting_ms: (totals.waiting != 0.0).then_some(JsNum(totals.waiting)),
        })
    }

    pub(super) fn schedule_diff(&mut self, id: &str) {
        if self.diff_reader.is_none() || self.stopped {
            return;
        }
        let Some(t) = self.trackers.get(id) else { return };
        if t.record.cwd.as_deref().is_none_or(str::is_empty) {
            return;
        }
        self.diff_due.insert(id.to_owned(), now_ms() + self.config.diff_delay_ms);
    }

    pub(super) fn run_diffs(&mut self, store: &mut Store, wall: i64) {
        let Some(reader) = self.diff_reader.as_mut() else { return };
        let due: Vec<String> = self.diff_due.iter().filter(|(_, d)| **d <= wall).map(|(k, _)| k.clone()).collect();
        for id in due {
            self.diff_due.shift_remove(&id);
            let Some(cwd) = self.trackers.get(&id).and_then(|t| t.record.cwd.clone()) else { continue };
            reader.request(&id, &cwd);
        }
        let ready = reader.poll();
        self.apply_diffs(store, ready);
    }

    pub(super) fn apply_diffs(&mut self, store: &mut Store, ready: Vec<(String, Option<core_store::extra::GitDiffStat>)>) {
        for (id, stat) in ready {
            if !self.trackers.contains_key(&id) {
                continue;
            }
            let prev = self.diffs.get(&id);
            match (&stat, prev) {
                (None, None) => continue,
                (Some(s), Some(p)) if same_diff(s, p) => continue,
                _ => {}
            }
            match stat {
                Some(s) => self.diffs.insert(id.clone(), Arc::new(s)),
                None => self.diffs.remove(&id),
            };
            self.publish(store, &id);
        }
    }
}
