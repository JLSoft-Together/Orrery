use super::{Meta, Sub, Tracker};
use crate::jsstr::{num, put, put_num, put_opt};
use crate::state::TranscriptState;
use serde_json::{Map, Value};
use std::collections::HashSet;

type Obj = Map<String, Value>;

fn state_view(s: &TranscriptState, main: bool) -> Obj {
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
    m
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

fn removed(sent: &Obj, next: &Obj, d: &mut Obj) {
    for k in sent.keys() {
        if !next.contains_key(k) {
            d.insert(k.clone(), Value::Null);
        }
    }
}

pub fn delta(sent: &Obj, next: &Obj) -> Obj {
    let mut d = Map::new();
    for (k, v) in next {
        if sent.get(k) != Some(v) {
            d.insert(k.clone(), v.clone());
        }
    }
    removed(sent, next, &mut d);
    d
}

fn sub_delta(sent: &Obj, next: &Obj) -> Obj {
    let mut d = Map::new();
    for (k, v) in next {
        match (sent.get(k), v) {
            (Some(Value::Object(a)), Value::Object(b)) if k == "state" => {
                let s = delta(a, b);
                if !s.is_empty() {
                    d.insert(k.clone(), Value::Object(s));
                }
            }
            (old, _) if old != Some(v) => {
                d.insert(k.clone(), v.clone());
            }
            _ => {}
        }
    }
    removed(sent, next, &mut d);
    d
}

impl Tracker {
    fn sub_view(&self, e: &Sub) -> Obj {
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
        m
    }

    pub fn view(&self) -> Value {
        let mut m = Map::new();
        put(&mut m, "full", true);
        put(&mut m, "main", state_view(&self.main, true));
        put(&mut m, "subs", self.subs.iter().map(|(_, e)| Value::Object(self.sub_view(e))).collect::<Vec<_>>());
        Value::Object(m)
    }

    pub fn view_of(&mut self, full: bool, main: bool, dirty: &HashSet<String>) -> Option<Value> {
        if full {
            self.sent_main = state_view(&self.main, true);
            let subs: Vec<(String, Obj)> = self.subs.iter().map(|(k, e)| (k.to_owned(), self.sub_view(e))).collect();
            let list = subs.iter().map(|(_, v)| Value::Object(v.clone())).collect::<Vec<_>>();
            self.sent_subs = subs.into_iter().collect();
            let mut m = Map::new();
            put(&mut m, "full", true);
            put(&mut m, "main", self.sent_main.clone());
            put(&mut m, "subs", list);
            return Some(Value::Object(m));
        }
        let mut m = Map::new();
        if main {
            let next = state_view(&self.main, true);
            let d = delta(&self.sent_main, &next);
            self.sent_main = next;
            if !d.is_empty() {
                put(&mut m, "main", d);
            }
        }
        let views: Vec<(String, Obj)> = self.subs.iter().filter(|(k, _)| dirty.contains(*k)).map(|(k, e)| (k.to_owned(), self.sub_view(e))).collect();
        let mut subs = Vec::new();
        for (k, next) in views {
            let d = match self.sent_subs.get(&k) {
                Some(sent) => sub_delta(sent, &next),
                None => next.clone(),
            };
            if !d.is_empty() {
                let mut s = Map::new();
                put(&mut s, "agentId", k.as_str());
                s.extend(d);
                subs.push(Value::Object(s));
            }
            self.sent_subs.insert(k, next);
        }
        if m.is_empty() && subs.is_empty() {
            return None;
        }
        put(&mut m, "subs", subs);
        Some(Value::Object(m))
    }
}
