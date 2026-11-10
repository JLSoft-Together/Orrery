pub mod cache;
pub mod cds;
pub mod daemon;
pub mod download;
pub mod gradle_cache;
pub mod gradle_graph;
pub mod ipc;
pub mod ipc_render;
pub mod jdk;
pub mod model;
pub mod model_ipc;
pub mod model_load;
pub mod model_run;
pub mod model_watch;
pub mod proc;
pub mod project_model;
pub mod rpc;
pub mod source_model;
pub mod xml;
pub mod zip;

pub const MODULE: &str = "layout";
pub const EVENT: &str = "ccm:layout-event";
pub const LAYOUTLIB_VERSION: &str = "16.2.3";
pub const LAYOUTLIB_API_LEVEL: u64 = 36;

#[derive(Debug, Clone, PartialEq)]
pub struct LayoutError {
    pub code: String,
    pub detail: Option<String>,
}

impl LayoutError {
    pub fn new(code: &str) -> Self {
        LayoutError { code: code.to_string(), detail: None }
    }

    pub fn with(code: &str, detail: impl Into<String>) -> Self {
        LayoutError { code: code.to_string(), detail: Some(detail.into()) }
    }
}

impl std::fmt::Display for LayoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.code)
    }
}

pub fn tail_chars(s: &str, n: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    chars[chars.len().saturating_sub(n)..].iter().collect()
}
