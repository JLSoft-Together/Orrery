use super::{Meta, Sub, Tracker};
use crate::jsstr::{num, put, put_num, put_opt};
use crate::state::TranscriptState;
use serde_json::{Map, Value};
use std::collections::HashSet;

fn state_view(s: &TranscriptState, main: bool) -> Value {
    let mut m = Map::new();
    put_opt(&mut m, "model", &s.model);
    put_opt(&mut m, "effort", &s.effort);
    if let Some(t) = s.current_tool() {
        put(&mut m, "currentTool", t);
    }
    put_opt(&mut m, "lastAt", &s.last_at);
    if let Some(t) = s.tokens() {
        let mut o = Map::new();
        put(&mut o, "input", num(t.input));
        put(&mut o, "output", num(t.output));
        put(&mut o, "cacheRead", num(t.cache_read));
        put(&mut m, "tokens", o);
    }
    put_num(&mut m, "contextTokens", s.context_tokens);
    put(&mut m, "peakContext", num(s.peak_context));
    put_opt(&mut m, "cacheAt", &s.cache_at);
    put(&mut m, "codexCalls", s.codex_calls.clone());
    put_opt(&mut m, "deviceCallAt", &s.device_call_at);
    if let Some((port, at)) = &s.port_conflict {
        let mut p = Map::new();
        put(&mut p, "port", *port);
        put(&mut p, "at", at.as_str());
        put(&mut m, "portConflict", p);
    }
    if main {
        put_num(&mut m, "firstContext", s.first_context);
        if let Some(t) = s.cache_ttl {
            put(&mut m, "cacheTtl", t);
        }
        put_opt(&mut m, "permissionMode", &s.permission_mode);
        put_opt(&mut m, "gitBranch", &s.git_branch);
        put(&mut m, "compactions", s.compactions);
        if let Some(l) = &s.limit_hit {
            let mut o = Map::new();
            put(&mut o, "at", l.at.as_str());
            put_opt(&mut o, "resetsAt", &l.resets_at);
            put_opt(&mut o, "window", &l.window);
            if let Some(a) = l.auto_resume {
                put(&mut o, "autoResume", a);
            }
            put(&mut m, "limitHit", o);
        }
        put_opt(&mut m, "lastReply", &s.last_reply);
        if let Some(a) = s.pending_ask() {
            put(&mut m, "pendingAsk", a.to_json());
        }
    }
    Value::Object(m)
}

fn meta_view(meta: &Meta) -> Value {
    let mut m = Map::new();
    put_opt(&mut m, "agentType", &meta.agent_type);
    put_opt(&mut m, "description", &meta.description);
    put_opt(&mut m, "toolUseId", &meta.tool_use_id);
    put_num(&mut m, "spawnDepth", meta.spawn_depth);
    put_opt(&mut m, "parentAgentId", &meta.parent_agent_id);
    put_opt(&mut m, "model", &meta.model);
    Value::Object(m)
}

impl Tracker {
    fn sub_view(&self, e: &Sub) -> Value {
        let mut m = Map::new();
        put(&mut m, "agentId", e.agent_id.as_str());
        put(&mut m, "meta", meta_view(&e.meta));
        put_opt(&mut m, "startedAt", &e.started_at);
        put(&mut m, "status", e.status);
        put(&mut m, "resolved", e.resolved);
        put_opt(&mut m, "completedAt", &e.completed_at);
        if let Some(n) = self.links.msg_in.get(&e.agent_id).map(|s| s.len()).filter(|n| *n > 0) {
            put(&mut m, "msgIn", n);
        }
        if let Some(n) = self.links.msg_out.get(&e.agent_id).map(|s| s.len()).filter(|n| *n > 0) {
            put(&mut m, "msgOut", n);
        }
        put(&mut m, "state", state_view(&e.state, false));
        Value::Object(m)
    }

    pub fn view(&self) -> Value {
        self.view_of(true, true, &HashSet::new())
    }

    pub fn view_of(&self, full: bool, main: bool, dirty: &HashSet<String>) -> Value {
        let mut m = Map::new();
        if full {
            put(&mut m, "full", true);
        }
        if main {
            put(&mut m, "main", state_view(&self.main, true));
        }
        let subs = self.subs.iter().filter(|(k, _)| full || dirty.contains(*k)).map(|(_, e)| self.sub_view(e)).collect::<Vec<_>>();
        put(&mut m, "subs", subs);
        Value::Object(m)
    }
}
