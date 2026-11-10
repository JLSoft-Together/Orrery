use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use core_desk::mcp::registry::{Bridge, Registry};
use core_desk::mcp::server::McpServer;
use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use crate::config::uuid;
use crate::state::{lock, Shell};

pub const MCP_CONFIG_JSON: &str = r#"{"mcpServers":{"orrery":{"type":"http","url":"${ORRERY_API}/mcp","headers":{"Authorization":"Bearer ${ORRERY_TOKEN}"}}}}"#;

pub struct McpLink {
    pub reg: Arc<Registry>,
    pub port: u16,
    pub config: Option<PathBuf>,
    _server: McpServer,
}

static LINK: OnceLock<McpLink> = OnceLock::new();

pub fn link() -> Option<&'static McpLink> {
    LINK.get()
}

struct ShellBridge {
    app: AppHandle,
}

impl Bridge for ShellBridge {
    fn ask(&self, action: &str, args: Value, pane: &str, timeout_ms: u64) -> Value {
        let id = uuid();
        let req = json!({ "id": id, "action": action, "args": args, "caller": { "source": "mcp", "pane": pane } });
        let shell = self.app.state::<Shell>();
        shell.ask(&self.app, &id, "ccm:orrery", req, Duration::from_millis(timeout_ms)).unwrap_or_else(|| json!({ "ok": false, "error": "unavailable" }))
    }

    fn desk(&self, channel: &str, call: Value) -> Value {
        crate::desk::call_inside(&self.app, channel, json!([call, { "deps": "native" }])).unwrap_or_else(|_| json!({ "ok": false, "error": "failed" }))
    }

    fn note(&self, note: Value) {
        if let Some(serve) = lock(&self.app.state::<Shell>().serve).as_mut() {
            serve.post(&note);
        }
    }
}

pub fn write_config(file: &Path) -> bool {
    if let Some(dir) = file.parent() {
        if std::fs::create_dir_all(dir).is_err() {
            return false;
        }
    }
    if std::fs::write(file, MCP_CONFIG_JSON).is_err() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o600));
    }
    true
}

fn home() -> String {
    std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).unwrap_or_default()
}

pub fn start(app: &AppHandle) {
    if LINK.get().is_some() {
        return;
    }
    let shell = app.state::<Shell>();
    let file = shell.data_dir.join("orrery-mcp.json");
    let config = write_config(&file).then_some(file);
    let reg = Registry::new(Arc::new(ShellBridge { app: app.clone() }), false, home(), Arc::new(core_desk::util::now_ms));
    let callers_reg = reg.clone();
    let callers: core_desk::mcp::server::Callers = Arc::new(move |token: &str| callers_reg.caller(token));
    match McpServer::start(callers, &app.package_info().version.to_string(), false) {
        Ok(server) => {
            let port = server.port();
            let _ = LINK.set(McpLink { reg, port, config, _server: server });
        }
        Err(e) => eprintln!("[shell] mcp server failed, orrery MCP off: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_config_names_the_env_not_the_token() {
        let v: Value = serde_json::from_str(MCP_CONFIG_JSON).unwrap();
        assert_eq!(v["mcpServers"]["orrery"]["url"], "${ORRERY_API}/mcp");
        assert_eq!(v["mcpServers"]["orrery"]["headers"]["Authorization"], "Bearer ${ORRERY_TOKEN}");
    }
}
