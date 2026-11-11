use crate::jsstr::{put, put_num, put_opt};
use crate::origin::Ask;
use crate::rules::{FileAccess, Touch};
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum Signal {
    ToolUse { id: String, name: String, summary: Option<String>, at: Option<String>, codex: bool, device: bool, touch: Option<Touch>, ask: Option<Ask>, access: Option<FileAccess> },
    ToolResult { tool_use_id: String, is_error: bool, async_launched: bool, at: Option<String>, port_conflict: Option<u32> },
    TurnEnd { at: Option<String> },
    AssistantEnd { message_id: Option<String>, at: Option<String> },
    Prompt { at: Option<String> },
    ReplyText { message_id: String, text: String },
    Usage { message_id: String, input: f64, output: f64, cache_read: f64, ttl: Option<&'static str>, at: Option<String> },
    Model { model: String, effort: Option<String> },
    TaskNotification { tool_use_id: Option<String>, task_id: Option<String>, status: String, at: Option<String> },
    Seen { at: String },
    Compact { trigger: Option<String>, pre_tokens: Option<f64>, post_tokens: Option<f64>, at: Option<String> },
    PermissionMode { mode: String },
    Branch { branch: String },
    CoordinatorMsg { key: String, at: Option<String> },
    Handback { from: String, key: String, at: Option<String> },
    SendMain { key: String, at: Option<String> },
    PeerIn { from_key: String, msg_id: Option<String>, dedupe_key: String, at: Option<String> },
    PeerSent { msg_id: String, at: Option<String> },
    LimitHit { resets_at: Option<String>, window: Option<String>, at: Option<String> },
    LimitResume { state: &'static str },
}

impl Signal {
    pub fn kind(&self) -> &'static str {
        match self {
            Signal::ToolUse { .. } => "tool_use",
            Signal::ToolResult { .. } => "tool_result",
            Signal::TurnEnd { .. } => "turn_end",
            Signal::AssistantEnd { .. } => "assistant_end",
            Signal::Prompt { .. } => "prompt",
            Signal::ReplyText { .. } => "reply_text",
            Signal::Usage { .. } => "usage",
            Signal::Model { .. } => "model",
            Signal::TaskNotification { .. } => "task_notification",
            Signal::Seen { .. } => "seen",
            Signal::Compact { .. } => "compact",
            Signal::PermissionMode { .. } => "permission_mode",
            Signal::Branch { .. } => "branch",
            Signal::CoordinatorMsg { .. } => "coordinator_msg",
            Signal::Handback { .. } => "handback",
            Signal::SendMain { .. } => "send_main",
            Signal::PeerIn { .. } => "peer_in",
            Signal::PeerSent { .. } => "peer_sent",
            Signal::LimitHit { .. } => "limit_hit",
            Signal::LimitResume { .. } => "limit_resume",
        }
    }

    pub fn to_json(&self) -> Value {
        let mut m = Map::new();
        put(&mut m, "k", self.kind());
        match self {
            Signal::ToolUse { id, name, summary, at, codex, device, touch, ask, access } => {
                put(&mut m, "id", id.as_str());
                put(&mut m, "name", name.as_str());
                put_opt(&mut m, "summary", summary);
                put_opt(&mut m, "at", at);
                if *codex {
                    put(&mut m, "codex", true);
                }
                if *device {
                    put(&mut m, "device", true);
                }
                if let Some(t) = touch {
                    put(&mut m, "touch", t.to_json());
                }
                if let Some(a) = ask {
                    put(&mut m, "ask", a.to_json());
                }
                if let Some((kind, path)) = access {
                    let mut f = Map::new();
                    put(&mut f, "kind", *kind);
                    put(&mut f, "path", path.as_str());
                    put(&mut m, "file", Value::Object(f));
                }
            }
            Signal::ToolResult { tool_use_id, is_error, async_launched, at, port_conflict } => {
                put(&mut m, "toolUseId", tool_use_id.as_str());
                put(&mut m, "isError", *is_error);
                put(&mut m, "asyncLaunched", *async_launched);
                put_opt(&mut m, "at", at);
                if let Some(p) = port_conflict {
                    put(&mut m, "portConflict", *p);
                }
            }
            Signal::TurnEnd { at } | Signal::Prompt { at } => put_opt(&mut m, "at", at),
            Signal::AssistantEnd { message_id, at } => {
                put_opt(&mut m, "messageId", message_id);
                put_opt(&mut m, "at", at);
            }
            Signal::ReplyText { message_id, text } => {
                put(&mut m, "messageId", message_id.as_str());
                put(&mut m, "text", text.as_str());
            }
            Signal::Usage { message_id, input, output, cache_read, ttl, at } => {
                put(&mut m, "messageId", message_id.as_str());
                put_num(&mut m, "input", Some(*input));
                put_num(&mut m, "output", Some(*output));
                put_num(&mut m, "cacheRead", Some(*cache_read));
                if let Some(t) = ttl {
                    put(&mut m, "ttl", *t);
                }
                put_opt(&mut m, "at", at);
            }
            Signal::Model { model, effort } => {
                put(&mut m, "model", model.as_str());
                put_opt(&mut m, "effort", effort);
            }
            Signal::TaskNotification { tool_use_id, task_id, status, at } => {
                put_opt(&mut m, "toolUseId", tool_use_id);
                put_opt(&mut m, "taskId", task_id);
                put(&mut m, "status", status.as_str());
                put_opt(&mut m, "at", at);
            }
            Signal::Seen { at } => put(&mut m, "at", at.as_str()),
            Signal::Compact { trigger, pre_tokens, post_tokens, at } => {
                put_opt(&mut m, "trigger", trigger);
                put_num(&mut m, "preTokens", *pre_tokens);
                put_num(&mut m, "postTokens", *post_tokens);
                put_opt(&mut m, "at", at);
            }
            Signal::PermissionMode { mode } => put(&mut m, "mode", mode.as_str()),
            Signal::Branch { branch } => put(&mut m, "branch", branch.as_str()),
            Signal::CoordinatorMsg { key, at } | Signal::SendMain { key, at } => {
                put(&mut m, "key", key.as_str());
                put_opt(&mut m, "at", at);
            }
            Signal::Handback { from, key, at } => {
                put(&mut m, "from", from.as_str());
                put(&mut m, "key", key.as_str());
                put_opt(&mut m, "at", at);
            }
            Signal::PeerIn { from_key, msg_id, dedupe_key, at } => {
                put(&mut m, "fromKey", from_key.as_str());
                put_opt(&mut m, "msgId", msg_id);
                put(&mut m, "dedupeKey", dedupe_key.as_str());
                put_opt(&mut m, "at", at);
            }
            Signal::PeerSent { msg_id, at } => {
                put(&mut m, "msgId", msg_id.as_str());
                put_opt(&mut m, "at", at);
            }
            Signal::LimitHit { resets_at, window, at } => {
                put_opt(&mut m, "resetsAt", resets_at);
                put_opt(&mut m, "window", window);
                put_opt(&mut m, "at", at);
            }
            Signal::LimitResume { state } => put(&mut m, "state", *state),
        }
        Value::Object(m)
    }
}
