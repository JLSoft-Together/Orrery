mod validate;

use serde::Serialize;
use serde_json::{Map, Number, Value};

use crate::js::{date_parse_ok, len16};
use validate::{is_absolute_path, is_rule, is_search_id, is_session_id, sanitize_globs, sanitize_groups, sanitize_idle, sanitize_ignore, sanitize_kinds, sanitize_quiet, sanitize_stuck};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LaunchMode {
    New,
    Continue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum TerminalApp {
    Terminal,
    Iterm,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum OpenApp {
    Explorer,
    Vscode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Locale {
    En,
    Vi,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", rename_all_fields = "camelCase")]
pub enum ClientMessage {
    #[serde(rename = "resync")]
    Resync,
    #[serde(rename = "office.enable")]
    OfficeEnable { on: bool },
    #[serde(rename = "office.subscribe")]
    OfficeSubscribe { on: bool },
    #[serde(rename = "toolSpans.get")]
    ToolSpansGet {
        #[serde(skip_serializing_if = "Option::is_none")]
        terminal_id: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        since: Option<String>,
    },
    #[serde(rename = "terminal.alias")]
    TerminalAlias { terminal_id: String, alias: Option<String> },
    #[serde(rename = "favorite.toggle")]
    FavoriteToggle { terminal_id: String },
    #[serde(rename = "favorite.remove")]
    FavoriteRemove { dir: String },
    #[serde(rename = "favorite.reorder")]
    FavoriteReorder { dirs: Vec<String> },
    #[serde(rename = "favorite.add")]
    FavoriteAdd {
        dir: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        label: Option<Option<String>>,
    },
    #[serde(rename = "folder.pick")]
    FolderPick { request_id: String },
    #[serde(rename = "favorite.open")]
    FavoriteOpen {
        dir: String,
        mode: LaunchMode,
        #[serde(skip_serializing_if = "Option::is_none")]
        terminal_app: Option<TerminalApp>,
    },
    #[serde(rename = "favorite.rename")]
    FavoriteRename { dir: String, label: Option<String> },
    #[serde(rename = "terminal.dismiss")]
    TerminalDismiss {
        #[serde(skip_serializing_if = "Option::is_none")]
        terminal_id: Option<String>,
    },
    #[serde(rename = "terminal.focusWindow")]
    TerminalFocusWindow { terminal_id: String },
    #[serde(rename = "history.get")]
    HistoryGet,
    #[serde(rename = "timeline.get")]
    TimelineGet,
    #[serde(rename = "heatmap.get")]
    HeatmapGet,
    #[serde(rename = "toolstats.get")]
    ToolstatsGet { days: u8, project: Option<String> },
    #[serde(rename = "promptSearch.start")]
    PromptSearchStart { search_id: String, query: String, days: u8, project: Option<String> },
    #[serde(rename = "promptSearch.cancel")]
    PromptSearchCancel { search_id: String },
    #[serde(rename = "promptSearch.open")]
    PromptSearchOpen { search_id: String, session_id: String },
    #[serde(rename = "terminal.open")]
    TerminalOpen { terminal_id: String, app: OpenApp },
    #[serde(rename = "diagnostics.get")]
    DiagnosticsGet,
    #[serde(rename = "job.dismiss")]
    JobDismiss {
        #[serde(skip_serializing_if = "Option::is_none")]
        job_id: Option<String>,
    },
    #[serde(rename = "job.stop")]
    JobStop { job_id: String },
    #[serde(rename = "client.presence")]
    ClientPresence {
        attentive: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        locale: Option<Locale>,
    },
    #[serde(rename = "notify.update")]
    NotifyUpdate {
        #[serde(skip_serializing_if = "Option::is_none")]
        toast: Option<bool>,
        #[serde(skip_serializing_if = "Option::is_none")]
        kinds: Option<Map<String, Value>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        quiet: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        stuck_minutes: Option<u64>,
        #[serde(skip_serializing_if = "Option::is_none")]
        idle_minutes: Option<u64>,
    },
    #[serde(rename = "notify.test")]
    NotifyTest,
    #[serde(rename = "terminal.note")]
    TerminalNote { terminal_id: String, note: Option<String> },
    #[serde(rename = "terminal.snooze")]
    TerminalSnooze { terminal_id: String, minutes: Option<Number> },
    #[serde(rename = "sensitive.update")]
    SensitiveUpdate {
        #[serde(skip_serializing_if = "Option::is_none")]
        groups: Option<Map<String, Value>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        globs: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        ignore: Option<Map<String, Value>>,
    },
    #[serde(rename = "sensitive.ignore")]
    SensitiveIgnore { project: String, rule: String, on: bool },
}

struct Fields<'a>(&'a Map<String, Value>);

impl<'a> Fields<'a> {
    fn str(&self, k: &str) -> Option<&'a str> {
        self.0.get(k).and_then(Value::as_str)
    }

    fn string(&self, k: &str) -> Option<String> {
        self.str(k).map(str::to_owned)
    }

    fn string_max(&self, k: &str, max: usize) -> Option<String> {
        self.str(k).filter(|s| len16(s) <= max).map(str::to_owned)
    }

    fn bool(&self, k: &str) -> Option<bool> {
        self.0.get(k).and_then(Value::as_bool)
    }

    fn undefined(&self, k: &str) -> bool {
        !self.0.contains_key(k)
    }

    fn optional_string(&self, k: &str) -> Option<Option<String>> {
        if self.undefined(k) {
            Some(None)
        } else {
            self.string(k).map(Some)
        }
    }

    fn string_or_null(&self, k: &str) -> Option<Option<String>> {
        match self.0.get(k)? {
            Value::Null => Some(None),
            Value::String(s) => Some(Some(s.clone())),
            _ => None,
        }
    }

    fn string_or_null_max(&self, k: &str, max: usize) -> Option<Option<String>> {
        self.string_or_null(k).filter(|v| v.as_deref().is_none_or(|s| len16(s) <= max))
    }

    fn days(&self, allowed: &[u8]) -> Option<u8> {
        let Value::Number(n) = self.0.get("days")? else { return None };
        let f = n.as_f64()?;
        allowed.iter().copied().find(|a| f64::from(*a) == f)
    }
}

pub fn parse_client_message(raw: &Value) -> Option<ClientMessage> {
    let obj = raw.as_object()?;
    let f = Fields(obj);
    let kind = f.str("type")?;
    let terminal_id = || f.string("terminalId");
    Some(match kind {
        "resync" => ClientMessage::Resync,
        "office.enable" => ClientMessage::OfficeEnable { on: f.bool("on")? },
        "office.subscribe" => ClientMessage::OfficeSubscribe { on: f.bool("on")? },
        "terminal.alias" => ClientMessage::TerminalAlias { terminal_id: terminal_id()?, alias: f.string_or_null("alias")? },
        "favorite.toggle" => ClientMessage::FavoriteToggle { terminal_id: terminal_id()? },
        "favorite.rename" => ClientMessage::FavoriteRename { dir: f.string("dir")?, label: f.string_or_null("label")? },
        "favorite.remove" => ClientMessage::FavoriteRemove { dir: f.string("dir")? },
        "favorite.reorder" => {
            let items = obj.get("dirs")?.as_array().filter(|a| a.len() <= 100)?;
            let dirs = items.iter().map(|d| d.as_str().filter(|s| len16(s) <= 1024).map(str::to_owned)).collect::<Option<Vec<_>>>()?;
            ClientMessage::FavoriteReorder { dirs }
        }
        "favorite.add" => {
            let label = match obj.get("label") {
                None => None,
                Some(Value::Null) => Some(None),
                Some(Value::String(s)) => Some(Some(s.clone())),
                Some(_) => return None,
            };
            ClientMessage::FavoriteAdd { dir: f.string("dir")?, label }
        }
        "folder.pick" => ClientMessage::FolderPick { request_id: f.string_max("requestId", 64)? },
        "terminal.open" => {
            let app = match f.str("app")? {
                "explorer" => OpenApp::Explorer,
                "vscode" => OpenApp::Vscode,
                _ => return None,
            };
            ClientMessage::TerminalOpen { terminal_id: terminal_id()?, app }
        }
        "history.get" => ClientMessage::HistoryGet,
        "timeline.get" => ClientMessage::TimelineGet,
        "heatmap.get" => ClientMessage::HeatmapGet,
        "diagnostics.get" => ClientMessage::DiagnosticsGet,
        "toolstats.get" => {
            let days = f.days(&[7, 30])?;
            let project = match obj.get("project")? {
                Value::Null => None,
                Value::String(s) if !s.is_empty() && len16(s) <= 1024 => Some(s.clone()),
                _ => return None,
            };
            ClientMessage::ToolstatsGet { days, project }
        }
        "promptSearch.start" => {
            let search_id = f.string("searchId").filter(|s| is_search_id(s))?;
            let query = f.string("query").filter(|q| !crate::js::trim(q).is_empty() && len16(q) <= 200)?;
            let days = f.days(&[7, 30, 90, 0])?;
            let project = match obj.get("project")? {
                Value::Null => None,
                Value::String(p) if is_absolute_path(p) && len16(p) <= 1024 && !p.chars().any(|c| c <= '\u{1f}') => Some(p.clone()),
                _ => return None,
            };
            ClientMessage::PromptSearchStart { search_id, query, days, project }
        }
        "promptSearch.cancel" => ClientMessage::PromptSearchCancel { search_id: f.string("searchId").filter(|s| is_search_id(s))? },
        "promptSearch.open" => ClientMessage::PromptSearchOpen {
            search_id: f.string("searchId").filter(|s| is_search_id(s))?,
            session_id: f.string("sessionId").filter(|s| is_session_id(s))?,
        },
        "toolSpans.get" => {
            let since = f.optional_string("since")?;
            if since.as_deref().is_some_and(|s| len16(s) > 40 || !date_parse_ok(s)) {
                return None;
            }
            ClientMessage::ToolSpansGet { terminal_id: f.optional_string("terminalId")?, since }
        }
        "terminal.focusWindow" => ClientMessage::TerminalFocusWindow { terminal_id: terminal_id()? },
        "terminal.dismiss" => ClientMessage::TerminalDismiss { terminal_id: f.optional_string("terminalId")? },
        "job.dismiss" => {
            let job_id = f.optional_string("jobId")?;
            if job_id.as_deref().is_some_and(|j| len16(j) > 64) {
                return None;
            }
            ClientMessage::JobDismiss { job_id }
        }
        "job.stop" => ClientMessage::JobStop { job_id: f.string_max("jobId", 64)? },
        "client.presence" => {
            let locale = match f.str("locale") {
                Some("en") => Some(Locale::En),
                Some("vi") => Some(Locale::Vi),
                _ => None,
            };
            ClientMessage::ClientPresence { attentive: f.bool("attentive")?, locale }
        }
        "notify.update" => {
            let toast = match obj.get("toast") {
                None => None,
                Some(Value::Bool(b)) => Some(*b),
                Some(_) => return None,
            };
            let kinds = obj.get("kinds").map(sanitize_kinds);
            let quiet = obj.get("quiet").map(sanitize_quiet);
            let stuck = obj.get("stuckMinutes").map(sanitize_stuck);
            let idle = obj.get("idleMinutes").map(sanitize_idle);
            if kinds.as_ref().is_some_and(Option::is_none) || quiet.as_ref().is_some_and(Option::is_none) || stuck.is_some_and(|v| v.is_none()) || idle.is_some_and(|v| v.is_none()) {
                return None;
            }
            ClientMessage::NotifyUpdate { toast, kinds: kinds.flatten(), quiet: quiet.flatten(), stuck_minutes: stuck.flatten(), idle_minutes: idle.flatten() }
        }
        "notify.test" => ClientMessage::NotifyTest,
        "terminal.note" => ClientMessage::TerminalNote { terminal_id: terminal_id()?, note: f.string_or_null_max("note", 1000)? },
        "terminal.snooze" => {
            let minutes = match obj.get("minutes")? {
                Value::Null => None,
                Value::Number(n) => Some(n.clone()),
                _ => return None,
            };
            ClientMessage::TerminalSnooze { terminal_id: terminal_id()?, minutes }
        }
        "sensitive.update" => {
            let groups = obj.get("groups").map(sanitize_groups);
            let globs = obj.get("globs").map(sanitize_globs);
            let ignore = obj.get("ignore").map(sanitize_ignore);
            if groups.as_ref().is_some_and(Option::is_none) || globs.as_ref().is_some_and(Option::is_none) || ignore.as_ref().is_some_and(Option::is_none) {
                return None;
            }
            ClientMessage::SensitiveUpdate { groups: groups.flatten(), globs: globs.flatten(), ignore: ignore.flatten() }
        }
        "sensitive.ignore" => ClientMessage::SensitiveIgnore {
            project: f.string_max("project", 1024)?,
            rule: f.string("rule").filter(|r| is_rule(r))?,
            on: f.bool("on")?,
        },
        "favorite.open" => {
            let mode = match f.str("mode")? {
                "new" => LaunchMode::New,
                "continue" => LaunchMode::Continue,
                _ => return None,
            };
            let terminal_app = match f.str("terminalApp") {
                Some("terminal") => Some(TerminalApp::Terminal),
                Some("iterm") => Some(TerminalApp::Iterm),
                _ => None,
            };
            ClientMessage::FavoriteOpen { dir: f.string("dir")?, mode, terminal_app }
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests;
