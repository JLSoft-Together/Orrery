use crate::extract::REPLY_MAX;
use crate::jsstr::{js_slice, later, num, put, put_num, put_opt};
use crate::omap::OMap;
use crate::origin::Ask;
use crate::signal::Signal;
use serde_json::{Map, Value};

const MAX_CODEX_CALLS: usize = 20;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LimitHit {
    pub at: String,
    pub resets_at: Option<String>,
    pub window: Option<String>,
    pub auto_resume: Option<bool>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Totals {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
}

#[derive(Default)]
pub struct TranscriptState {
    pub open_tools: OMap<String>,
    open_asks: OMap<Ask>,
    pub codex_calls: Vec<String>,
    pub device_call_at: Option<String>,
    pub port_conflict: Option<(u32, String)>,
    usage: OMap<Totals>,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub cache_ttl: Option<&'static str>,
    pub cache_at: Option<String>,
    pub last_at: Option<String>,
    pub context_tokens: Option<f64>,
    pub first_context: Option<f64>,
    pub peak_context: f64,
    pub compactions: u64,
    pub permission_mode: Option<String>,
    pub git_branch: Option<String>,
    pub ended: bool,
    pub last_end_at: Option<String>,
    pub limit_hit: Option<LimitHit>,
    reply_id: Option<String>,
    reply_text: String,
    pub last_reply: Option<String>,
}

fn truthy(v: &Option<String>) -> Option<&String> {
    v.as_ref().filter(|s| !s.is_empty())
}

impl TranscriptState {
    pub fn apply(&mut self, signals: &[Signal]) {
        for s in signals {
            self.apply_one(s);
        }
    }

    fn apply_one(&mut self, s: &Signal) {
        match s {
            Signal::Seen { at } => self.last_at = Some(at.clone()),
            Signal::Model { model, effort } => {
                self.model = Some(model.clone());
                self.effort = effort.clone();
                self.limit_hit = None;
            }
            Signal::LimitHit { resets_at, window, at } => {
                if let Some(at) = truthy(at) {
                    let auto_resume = self.limit_hit.as_ref().and_then(|l| l.auto_resume);
                    self.limit_hit = Some(LimitHit { at: at.clone(), resets_at: resets_at.clone(), window: window.clone(), auto_resume });
                }
            }
            Signal::LimitResume { state } => {
                if *state == "reset" {
                    self.limit_hit = None;
                } else if let Some(l) = self.limit_hit.as_mut() {
                    l.auto_resume = Some(*state == "armed");
                }
            }
            Signal::Usage { message_id, input, output, cache_read, ttl, at } => {
                self.usage.insert(message_id, Totals { input: *input, output: *output, cache_read: *cache_read });
                let ctx = input + cache_read;
                self.context_tokens = Some(ctx);
                self.first_context.get_or_insert(ctx);
                self.peak_context = self.peak_context.max(ctx);
                if ttl.is_some() {
                    self.cache_ttl = *ttl;
                }
                if let Some(at) = truthy(at) {
                    self.cache_at = Some(at.clone());
                }
            }
            Signal::Compact { post_tokens, .. } => {
                self.compactions += 1;
                if post_tokens.is_some() {
                    self.context_tokens = *post_tokens;
                }
            }
            Signal::PermissionMode { mode } => self.permission_mode = Some(mode.clone()),
            Signal::Branch { branch } => self.git_branch = Some(branch.clone()),
            Signal::ToolUse { id, name, at, codex, device, ask, .. } => {
                self.open_tools.insert(id, name.clone());
                if let Some(a) = ask {
                    self.open_asks.insert(id, a.clone());
                }
                self.ended = false;
                if let (true, Some(at)) = (*codex, truthy(at)) {
                    self.codex_calls.push(at.clone());
                    if self.codex_calls.len() > MAX_CODEX_CALLS {
                        self.codex_calls.remove(0);
                    }
                }
                if let (true, Some(at)) = (*device, truthy(at)) {
                    if self.device_call_at.as_ref().is_none_or(|d| d.is_empty() || later(at, d)) {
                        self.device_call_at = Some(at.clone());
                    }
                }
            }
            Signal::ToolResult { tool_use_id, at, port_conflict, .. } => {
                self.open_tools.remove(tool_use_id);
                self.open_asks.remove(tool_use_id);
                self.ended = false;
                if let (Some(port), Some(at)) = (port_conflict.filter(|p| *p != 0), truthy(at)) {
                    if self.port_conflict.as_ref().is_none_or(|(_, prev)| later(at, prev)) {
                        self.port_conflict = Some((port, at.clone()));
                    }
                }
            }
            Signal::TurnEnd { .. } => {
                self.open_tools.clear();
                self.open_asks.clear();
            }
            Signal::ReplyText { message_id, text } => {
                if self.reply_id.as_deref() == Some(message_id.as_str()) {
                    let joined = format!("{}\n{}", self.reply_text, text);
                    self.reply_text = js_slice(&joined, REPLY_MAX).to_owned();
                } else {
                    self.reply_id = Some(message_id.clone());
                    self.reply_text = text.clone();
                }
            }
            Signal::AssistantEnd { message_id, at } => {
                self.ended = true;
                self.last_end_at = at.clone();
                let matches = message_id.as_ref().is_none_or(|m| m.is_empty() || self.reply_id.as_ref() == Some(m));
                self.last_reply = (!self.reply_text.is_empty() && matches).then(|| self.reply_text.clone());
            }
            Signal::Prompt { .. } => {
                self.reply_id = None;
                self.reply_text.clear();
                self.last_reply = None;
                self.open_asks.clear();
            }
            _ => {}
        }
    }

    pub fn pending_ask(&self) -> Option<&Ask> {
        self.open_asks.iter().last().map(|(_, a)| a)
    }

    pub fn current_tool(&self) -> Option<&str> {
        self.open_tools.iter().last().map(|(_, n)| n.as_str())
    }

    pub fn tokens(&self) -> Option<Totals> {
        if self.usage.is_empty() {
            return None;
        }
        let mut t = Totals::default();
        for (_, u) in self.usage.iter() {
            t.input += u.input;
            t.output += u.output;
            t.cache_read += u.cache_read;
        }
        Some(t)
    }

    pub fn to_json(&self) -> Value {
        let mut m = Map::new();
        let tools: Vec<Value> = self.open_tools.iter().map(|(k, v)| Value::from(vec![k, v.as_str()])).collect();
        put(&mut m, "openTools", tools);
        put(&mut m, "codexCalls", self.codex_calls.clone());
        put_opt(&mut m, "deviceCallAt", &self.device_call_at);
        if let Some((port, at)) = &self.port_conflict {
            let mut p = Map::new();
            put(&mut p, "port", *port);
            put(&mut p, "at", at.as_str());
            put(&mut m, "portConflict", p);
        }
        put_opt(&mut m, "model", &self.model);
        put_opt(&mut m, "effort", &self.effort);
        if let Some(t) = self.cache_ttl {
            put(&mut m, "cacheTtl", t);
        }
        put_opt(&mut m, "cacheAt", &self.cache_at);
        put_opt(&mut m, "lastAt", &self.last_at);
        put_num(&mut m, "contextTokens", self.context_tokens);
        put_num(&mut m, "firstContext", self.first_context);
        put(&mut m, "peakContext", num(self.peak_context));
        put(&mut m, "compactions", self.compactions);
        put_opt(&mut m, "permissionMode", &self.permission_mode);
        put_opt(&mut m, "gitBranch", &self.git_branch);
        put(&mut m, "ended", self.ended);
        put_opt(&mut m, "lastEndAt", &self.last_end_at);
        if let Some(l) = &self.limit_hit {
            let mut o = Map::new();
            put(&mut o, "at", l.at.as_str());
            put_opt(&mut o, "resetsAt", &l.resets_at);
            put_opt(&mut o, "window", &l.window);
            if let Some(a) = l.auto_resume {
                put(&mut o, "autoResume", a);
            }
            put(&mut m, "limitHit", o);
        }
        put_opt(&mut m, "lastReply", &self.last_reply);
        if let Some(t) = self.tokens() {
            let mut o = Map::new();
            put(&mut o, "input", num(t.input));
            put(&mut o, "output", num(t.output));
            put(&mut o, "cacheRead", num(t.cache_read));
            put(&mut m, "tokens", o);
        }
        if let Some(t) = self.current_tool() {
            put(&mut m, "currentTool", t);
        }
        if let Some(a) = self.pending_ask() {
            put(&mut m, "pendingAsk", a.to_json());
        }
        Value::Object(m)
    }
}
