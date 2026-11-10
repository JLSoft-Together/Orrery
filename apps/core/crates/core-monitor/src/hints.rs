use std::collections::HashMap;

use indexmap::IndexMap;

use crate::shared::autocompact_buffer_pct;

#[derive(Debug, Clone, PartialEq)]
pub struct SessionContext {
    pub pct: f64,
    pub buffer_pct: Option<f64>,
    pub at: String,
}

#[derive(Debug, Clone)]
pub struct ContextHints {
    pub settings_model: Option<String>,
    pub model_windows: IndexMap<String, f64>,
    pub session_windows: HashMap<String, f64>,
    pub session_context: HashMap<String, SessionContext>,
    pub buffer_pct: f64,
}

impl ContextHints {
    pub fn new(auto_compact_window: Option<f64>) -> Self {
        ContextHints {
            settings_model: None,
            model_windows: IndexMap::new(),
            session_windows: HashMap::new(),
            session_context: HashMap::new(),
            buffer_pct: autocompact_buffer_pct(auto_compact_window),
        }
    }

    pub fn from_env() -> Self {
        let raw = std::env::var("CLAUDE_CODE_AUTO_COMPACT_WINDOW").ok();
        let n = raw.as_deref().map(str::trim).filter(|s| !s.is_empty()).and_then(|s| s.parse::<f64>().ok());
        Self::new(n)
    }
}
