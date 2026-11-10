use super::project::Entry;
use crate::run::types::BuildError;
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct FwpError {
    pub code: String,
    pub detail: Option<String>,
    pub build: Option<Box<BuildError>>,
    pub entry: Option<String>,
}

impl FwpError {
    pub fn new(code: &str) -> Self {
        FwpError { code: code.to_string(), detail: None, build: None, entry: None }
    }

    pub fn with(code: &str, detail: impl Into<String>) -> Self {
        FwpError { detail: Some(detail.into()), ..FwpError::new(code) }
    }

    pub fn to_value(&self) -> Value {
        let mut v = json!({"code": self.code});
        if let Some(d) = &self.detail {
            v["detail"] = json!(d);
        }
        if let Some(b) = &self.build {
            v["build"] = b.to_value();
        }
        if let Some(e) = &self.entry {
            v["entry"] = json!(e);
        }
        v
    }
}

pub type Res = Result<(), FwpError>;

pub fn res_value(r: &Res) -> Value {
    match r {
        Ok(()) => json!({"ok": true}),
        Err(e) => json!({"ok": false, "error": e.to_value()}),
    }
}

#[derive(Debug, Clone)]
pub struct Snap {
    pub state: &'static str,
    pub root: Option<String>,
    pub device_id: Option<String>,
    pub app_id: Option<String>,
    pub current: Option<String>,
    pub progress: Option<String>,
    pub error: Option<FwpError>,
    pub entries: Vec<Entry>,
    pub log: Vec<String>,
}

impl Snap {
    pub fn new() -> Self {
        Snap { state: "idle", root: None, device_id: None, app_id: None, current: None, progress: None, error: None, entries: Vec::new(), log: Vec::new() }
    }

    pub fn to_value(&self) -> Value {
        json!({
            "state": self.state,
            "root": self.root,
            "deviceId": self.device_id,
            "appId": self.app_id,
            "current": self.current,
            "progress": self.progress,
            "error": self.error.as_ref().map(FwpError::to_value),
            "entries": self.entries.iter().map(Entry::to_value).collect::<Vec<_>>(),
            "log": self.log,
        })
    }
}

impl Default for Snap {
    fn default() -> Self {
        Snap::new()
    }
}
