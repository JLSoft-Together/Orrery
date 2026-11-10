use core_transcript::codex::extract::{CodexSignal, Limits, Meta, Usage};
use core_transcript::jsstr::utf16_cmp;
use indexmap::IndexMap;
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexMode {
    Exec,
    Tui,
    Plugin,
    Subagent,
    AppServer,
}

impl CodexMode {
    pub fn as_str(self) -> &'static str {
        match self {
            CodexMode::Exec => "exec",
            CodexMode::Tui => "tui",
            CodexMode::Plugin => "plugin",
            CodexMode::Subagent => "subagent",
            CodexMode::AppServer => "app-server",
        }
    }
}

pub fn mode_of(meta: &Meta) -> Option<CodexMode> {
    if meta.parent_thread_id.is_some() || meta.source.as_deref() == Some("subagent") {
        return Some(CodexMode::Subagent);
    }
    if meta.originator.as_deref() == Some("Claude Code") {
        return Some(CodexMode::Plugin);
    }
    match meta.source.as_deref() {
        Some("exec") => Some(CodexMode::Exec),
        Some("cli") => Some(CodexMode::Tui),
        Some("vscode") => None,
        _ => Some(CodexMode::AppServer),
    }
}

#[derive(Debug, Default)]
pub struct CodexThreadState {
    pub meta: Option<Meta>,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub turn_open: Option<bool>,
    pub aborted: bool,
    pub turn_started_at: Option<String>,
    pub turn_ended_at: Option<String>,
    pub open_calls: IndexMap<String, String>,
    pub call_info: IndexMap<String, (Option<String>, Option<String>)>,
    pub total: Option<Usage>,
    pub last: Option<Usage>,
    pub context_window: Option<f64>,
    pub limits: Option<Limits>,
    pub last_at: Option<String>,
    pub compactions: u64,
}

impl CodexThreadState {
    pub fn apply(&mut self, signals: &[CodexSignal]) {
        for s in signals {
            match s {
                CodexSignal::Seen { at } => {
                    if self.last_at.as_deref().is_none_or(|l| utf16_cmp(at, l) == Ordering::Greater) {
                        self.last_at = Some(at.clone());
                    }
                }
                CodexSignal::Meta(m) => {
                    if self.meta.is_none() {
                        self.meta = Some(m.clone());
                    }
                }
                CodexSignal::Context { model, effort } => {
                    if model.is_some() {
                        self.model.clone_from(model);
                    }
                    if effort.is_some() {
                        self.effort.clone_from(effort);
                    }
                }
                CodexSignal::TurnStart { at, context_window } => {
                    self.turn_open = Some(true);
                    self.aborted = false;
                    self.turn_started_at.clone_from(at);
                    self.open_calls.clear();
                    self.call_info.clear();
                    if context_window.is_some() {
                        self.context_window = *context_window;
                    }
                }
                CodexSignal::TurnEnd { at, aborted } => {
                    self.turn_open = Some(false);
                    self.aborted = *aborted;
                    self.turn_ended_at.clone_from(at);
                    self.open_calls.clear();
                    self.call_info.clear();
                }
                CodexSignal::ToolCall { id, name, summary, at } => {
                    self.open_calls.insert(id.clone(), name.clone());
                    self.call_info.insert(id.clone(), (summary.clone(), at.clone().filter(|a| !a.is_empty())));
                    self.turn_open.get_or_insert(true);
                }
                CodexSignal::ToolOutput { id } => {
                    self.open_calls.shift_remove(id);
                    self.call_info.shift_remove(id);
                }
                CodexSignal::Tokens { total, last, context_window } => {
                    self.total = Some(total.clone());
                    if last.is_some() {
                        self.last.clone_from(last);
                    }
                    if context_window.is_some() {
                        self.context_window = *context_window;
                    }
                    self.turn_open.get_or_insert(true);
                }
                CodexSignal::Limits(l) => {
                    if self.limits.as_ref().is_none_or(|cur| utf16_cmp(&l.at, &cur.at) != Ordering::Less) {
                        self.limits = Some(l.clone());
                    }
                }
                CodexSignal::Compact => self.compactions += 1,
            }
        }
    }

    pub fn current_tool(&self) -> Option<&str> {
        self.open_calls.values().last().map(String::as_str)
    }

    fn current_call(&self) -> Option<&(Option<String>, Option<String>)> {
        self.call_info.get(self.open_calls.keys().last()?)
    }

    pub fn current_tool_summary(&self) -> Option<&str> {
        self.current_call()?.0.as_deref()
    }

    pub fn current_tool_started_at(&self) -> Option<&str> {
        self.current_call()?.1.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_transcript::codex::extract::extract_codex_signals;
    use core_transcript::codex::value::parse_record;

    const TID: &str = "01a125e6-f212-7711-a885-025fd33ab05a";

    fn sig(json: serde_json::Value) -> Vec<CodexSignal> {
        extract_codex_signals(&parse_record(&json.to_string()).expect("record"))
    }

    fn rec(kind: &str, at: &str, payload: serde_json::Value) -> Vec<CodexSignal> {
        sig(serde_json::json!({ "timestamp": at, "type": kind, "payload": payload }))
    }

    #[test]
    fn folds_a_turn_without_reading_content() {
        let mut st = CodexThreadState::default();
        let at = |s: u32| format!("2026-10-11T07:00:0{s}.000Z");
        st.apply(&rec("session_meta", &at(0), serde_json::json!({ "id": TID, "timestamp": at(0), "cwd": "C:\\Temp", "originator": "codex_exec", "source": "exec" })));
        st.apply(&rec("event_msg", &at(0), serde_json::json!({ "type": "task_started", "model_context_window": 258_400 })));
        st.apply(&rec("turn_context", &at(0), serde_json::json!({ "model": "gpt-6-luna", "effort": "medium" })));
        st.apply(&rec("response_item", &at(1), serde_json::json!({ "type": "custom_tool_call", "call_id": "c1", "name": "exec" })));
        let usage = |i: u32, c: u32, o: u32| serde_json::json!({ "input_tokens": i, "cached_input_tokens": c, "output_tokens": o, "total_tokens": i + o });
        st.apply(&rec("event_msg", &at(2), serde_json::json!({ "type": "token_count", "info": { "total_token_usage": usage(18_000, 12_000, 2_000), "last_token_usage": usage(18_000, 12_000, 2_000), "model_context_window": 258_400 }, "rate_limits": { "limit_id": "codex", "primary": { "used_percent": 7, "window_minutes": 300, "resets_at": 1_791_650_000 }, "plan_type": "plus" } })));
        assert_eq!(st.meta.as_ref().map(|m| m.id.as_str()), Some(TID));
        assert_eq!((st.model.as_deref(), st.effort.as_deref(), st.turn_open, st.context_window), (Some("gpt-6-luna"), Some("medium"), Some(true), Some(258_400.0)));
        assert_eq!(st.current_tool(), Some("exec"));
        assert_eq!(st.total.as_ref().map(|t| t.total), Some(20_000.0));
        assert_eq!(st.limits.as_ref().map(|l| (l.plan.as_deref(), l.five_hour.as_ref().map(|w| w.used_pct))), Some((Some("plus"), Some(7.0))));
        st.apply(&rec("response_item", &at(2), serde_json::json!({ "type": "custom_tool_call_output", "call_id": "c1" })));
        st.apply(&rec("event_msg", &at(3), serde_json::json!({ "type": "task_complete" })));
        assert_eq!(st.turn_open, Some(false));
        assert_eq!(st.current_tool(), None);
    }

    #[test]
    fn keeps_the_call_summary_until_its_output() {
        let mut st = CodexThreadState::default();
        let at = "2026-11-10T10:00:01.000Z";
        let args = serde_json::json!({ "cmd": "cargo test -j 4" }).to_string();
        st.apply(&rec("response_item", at, serde_json::json!({ "type": "function_call", "call_id": "c1", "name": "exec_command", "arguments": args })));
        assert_eq!((st.current_tool(), st.current_tool_summary(), st.current_tool_started_at()), (Some("exec_command"), Some("cargo test -j 4"), Some(at)));
        st.apply(&rec("response_item", at, serde_json::json!({ "type": "function_call_output", "call_id": "c1" })));
        assert_eq!((st.current_tool(), st.current_tool_summary(), st.current_tool_started_at()), (None, None, None));
    }

    #[test]
    fn maps_modes_from_meta() {
        let m = |source: Option<&str>, originator: &str, parent: Option<&str>| Meta {
            id: TID.into(),
            created_at: 0,
            cwd: None,
            originator: Some(originator.into()),
            source: source.map(str::to_owned),
            parent_thread_id: parent.map(str::to_owned),
            version: None,
        };
        assert_eq!(mode_of(&m(Some("exec"), "codex_exec", None)), Some(CodexMode::Exec));
        assert_eq!(mode_of(&m(Some("cli"), "codex-tui", None)), Some(CodexMode::Tui));
        assert_eq!(mode_of(&m(Some("vscode"), "Claude Code", None)), Some(CodexMode::Plugin));
        assert_eq!(mode_of(&m(Some("vscode"), "Codex Desktop", None)), None);
        assert_eq!(mode_of(&m(Some("subagent"), "x", Some("p1"))), Some(CodexMode::Subagent));
        assert_eq!(mode_of(&m(None, "x", None)), Some(CodexMode::AppServer));
    }
}
