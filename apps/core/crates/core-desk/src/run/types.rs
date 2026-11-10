use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;

pub type Env = BTreeMap<String, String>;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct EnvIssue {
    pub code: String,
    pub detail: Option<String>,
}

impl EnvIssue {
    pub fn new(code: &str) -> Self {
        EnvIssue { code: code.to_string(), detail: None }
    }

    pub fn with(code: &str, detail: impl Into<String>) -> Self {
        EnvIssue { code: code.to_string(), detail: Some(detail.into()) }
    }

    pub fn to_value(&self) -> Value {
        match &self.detail {
            Some(d) => json!({"code": self.code, "detail": d}),
            None => json!({"code": self.code}),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum RunResult {
    Ok(Option<String>),
    Err(EnvIssue),
}

impl RunResult {
    pub fn ok() -> Self {
        RunResult::Ok(None)
    }

    pub fn err(code: &str) -> Self {
        RunResult::Err(EnvIssue::new(code))
    }

    pub fn to_value(&self) -> Value {
        match self {
            RunResult::Ok(None) => json!({"ok": true}),
            RunResult::Ok(Some(id)) => json!({"ok": true, "sessionId": id}),
            RunResult::Err(e) => json!({"ok": false, "error": e.to_value()}),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct RunSettings {
    pub flutter_sdk: Option<String>,
    pub android_sdk: Option<String>,
    pub jdk: Option<String>,
    pub flutter_sdk_projects: BTreeMap<String, String>,
    pub layout_auto_compile: bool,
}

impl RunSettings {
    pub fn new() -> Self {
        RunSettings { layout_auto_compile: true, ..Default::default() }
    }

    pub fn to_value(&self) -> Value {
        json!({
            "flutterSdk": self.flutter_sdk,
            "androidSdk": self.android_sdk,
            "jdk": self.jdk,
            "flutterSdkProjects": self.flutter_sdk_projects,
            "layoutAutoCompile": self.layout_auto_compile,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunDevice {
    pub id: String,
    pub name: String,
    pub platform: String,
    pub emulator: bool,
    pub state: String,
    pub source: String,
}

impl RunDevice {
    pub fn to_value(&self) -> Value {
        json!({"id": self.id, "name": self.name, "platform": self.platform, "emulator": self.emulator, "state": self.state, "source": self.source})
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunEmulator {
    pub id: String,
    pub name: String,
    pub source: String,
}

impl RunEmulator {
    pub fn to_value(&self) -> Value {
        json!({"id": self.id, "name": self.name, "source": self.source})
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BuildError {
    pub file: String,
    pub line: u32,
    pub col: u32,
    pub message: String,
    pub severity: String,
}

impl BuildError {
    pub fn to_value(&self) -> Value {
        json!({"file": self.file, "line": self.line, "col": self.col, "message": self.message, "severity": self.severity})
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SessionPatch {
    pub app_id: Option<Option<String>>,
    pub vm_service_uri: Option<Option<String>>,
    pub devtools_uri: Option<Option<String>>,
    pub progress: Option<Option<String>>,
    pub failure: Option<Option<EnvIssue>>,
}

pub trait Hooks: Send + Sync {
    fn state(&self, state: &str, patch: SessionPatch);
    fn fail(&self, failure: EnvIssue);
    fn log(&self, stream: &str, text: &str, level: Option<&str>, tag: Option<&str>);
    fn error(&self, error: BuildError);
    fn confirm(&self, code: &str, detail: Option<&str>) -> bool;
}

pub type AdbUse = Arc<dyn Fn(&str) + Send + Sync>;

#[derive(Clone)]
pub struct RunContext {
    pub root: String,
    pub config: Value,
    pub device: Option<RunDevice>,
    pub settings: RunSettings,
    pub env: Env,
    pub win: bool,
    pub home: String,
    pub hooks: Arc<dyn Hooks>,
    pub on_adb_use: Option<AdbUse>,
}

impl RunContext {
    pub fn cfg_str(&self, key: &str) -> String {
        self.config.get(key).and_then(Value::as_str).unwrap_or("").to_string()
    }

    pub fn cfg_env(&self) -> Env {
        self.config.get("env").and_then(Value::as_object).map(|o| o.iter().filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string()))).collect()).unwrap_or_default()
    }

    pub fn cfg_before(&self) -> Vec<String> {
        self.config.get("beforeLaunch").and_then(Value::as_array).map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default()
    }
}

pub trait Runner: Send + Sync {
    fn start(&self);
    fn stop(&self);
    fn kill(&self);
    fn reload(&self, _full: bool) -> RunResult {
        RunResult::err("reload-unsupported")
    }
}

pub type RunnerFactory = fn(RunContext) -> Arc<dyn Runner>;

pub type DevicesCb = Arc<dyn Fn(Vec<RunDevice>) + Send + Sync>;

pub trait DeviceService: Send + Sync {
    fn list(&self) -> Vec<RunDevice>;
    fn on_change(&self, cb: DevicesCb);
    fn emulators(&self) -> Vec<RunEmulator>;
    fn launch_emulator(&self, id: &str) -> RunResult;
    fn stop(&self, sync: bool);
}
