pub mod appearance;
pub mod backup;
pub mod bridge;
pub mod build_clean;
pub mod claude;
pub mod claude_hooks;
pub mod editors;
pub mod extensions;
pub mod fs_tree;
pub mod git;
pub mod js;
pub mod layouts;
pub mod libs;
pub mod pages;
pub mod pages_style;
pub mod preview;
pub mod restore;
pub mod stdio;
pub mod util;

use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum DeskError {
    UnknownChannel(String),
    Host(String),
    Failed(String),
}

impl fmt::Display for DeskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DeskError::UnknownChannel(c) => write!(f, "unknown channel {c}"),
            DeskError::Host(m) => write!(f, "host: {m}"),
            DeskError::Failed(m) => write!(f, "{m}"),
        }
    }
}

impl std::error::Error for DeskError {}

pub trait DeskIo: Send + Sync {
    fn emit(&self, event: &str, payload: Value);
    fn host(&self, req: u64, method: &str, params: Value) -> Result<Value, DeskError>;
}

pub struct NoHost;

impl DeskIo for NoHost {
    fn emit(&self, _event: &str, _payload: Value) {}
    fn host(&self, _req: u64, method: &str, _params: Value) -> Result<Value, DeskError> {
        Err(DeskError::Host(format!("no host for {method}")))
    }
}

pub struct Ctx<'a> {
    io: &'a dyn DeskIo,
    req: u64,
}

impl<'a> Ctx<'a> {
    pub fn new(io: &'a dyn DeskIo, req: u64) -> Self {
        Ctx { io, req }
    }

    pub fn emit(&self, event: &str, payload: Value) {
        self.io.emit(event, payload)
    }

    pub fn host(&self, method: &str, params: Value) -> Result<Value, DeskError> {
        self.io.host(self.req, method, params)
    }
}

pub type Handler = Box<dyn Fn(&Ctx, &[Value]) -> Result<Value, DeskError> + Send + Sync>;

#[derive(Debug, Clone, Default)]
pub struct DeskEnv {
    pub data_dir: PathBuf,
    pub app_version: String,
    pub documents_dir: PathBuf,
    pub user_data: Option<PathBuf>,
    pub libs_registry: Option<String>,
    pub osv_url: Option<String>,
}

impl DeskEnv {
    pub fn from_env() -> Self {
        let var = |k: &str| std::env::var(k).ok().map(|v| v.trim().to_string()).filter(|v| !v.is_empty());
        let home = var("USERPROFILE").or_else(|| var("HOME")).map(PathBuf::from).unwrap_or_default();
        let data_dir = var("CCM_DATA_DIR").map(PathBuf::from).unwrap_or_else(|| util::default_data_dir(&home));
        DeskEnv {
            data_dir,
            app_version: var("CCM_APP_VERSION").unwrap_or_else(|| "0.0.0".into()),
            documents_dir: var("CCM_DOCUMENTS_DIR").map(PathBuf::from).unwrap_or_else(|| home.join("Documents")),
            user_data: var("CCM_USER_DATA").map(PathBuf::from),
            libs_registry: var("CCM_LIBS_REGISTRY").filter(|v| libs::registry::is_allowed_override(v)),
            osv_url: var("CCM_OSV_URL").filter(|v| libs::registry::is_allowed_override(v)),
        }
    }
}

#[derive(Default)]
pub struct Desk {
    handlers: HashMap<String, Handler>,
    modules: Vec<(String, Vec<String>)>,
}

impl Desk {
    pub fn new() -> Self {
        Desk::default()
    }

    pub fn standard(env: &DeskEnv) -> Self {
        let mut desk = Desk::new();
        backup::register(&mut desk, env);
        build_clean::register(&mut desk, env);
        libs::register(&mut desk, env);
        claude_hooks::register(&mut desk, env);
        extensions::register(&mut desk, env);
        bridge::register(&mut desk, env);
        layouts::register(&mut desk, env);
        restore::register(&mut desk, env);
        appearance::register(&mut desk, env);
        pages::register(&mut desk, env);
        editors::register(&mut desk, env);
        preview::register(&mut desk, env);
        desk
    }

    pub fn on<F>(&mut self, module: &str, channel: &str, handler: F)
    where
        F: Fn(&Ctx, &[Value]) -> Result<Value, DeskError> + Send + Sync + 'static,
    {
        self.handlers.insert(channel.to_string(), Box::new(handler));
        match self.modules.iter_mut().find(|(m, _)| m == module) {
            Some((_, list)) => list.push(channel.to_string()),
            None => self.modules.push((module.to_string(), vec![channel.to_string()])),
        }
    }

    pub fn channels(&self) -> Vec<String> {
        self.modules.iter().flat_map(|(_, c)| c.iter().cloned()).collect()
    }

    pub fn modules(&self) -> Vec<(String, Vec<String>)> {
        self.modules.clone()
    }

    pub fn dispatch(&self, channel: &str, args: Value) -> Result<Value, DeskError> {
        self.dispatch_with(&Ctx::new(&NoHost, 0), channel, args)
    }

    pub fn dispatch_with(&self, ctx: &Ctx, channel: &str, args: Value) -> Result<Value, DeskError> {
        let handler = self.handlers.get(channel).ok_or_else(|| DeskError::UnknownChannel(channel.to_string()))?;
        let list = match args {
            Value::Array(a) => a,
            Value::Null => Vec::new(),
            other => vec![other],
        };
        handler(ctx, &list)
    }
}

pub fn arg(args: &[Value], i: usize) -> &Value {
    args.get(i).unwrap_or(&Value::Null)
}
