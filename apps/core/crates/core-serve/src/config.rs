use std::path::PathBuf;

use core_monitor::MonitorConfig;
use core_store::persist::datadir::{default_data_dir, Platform};

pub const DEFAULT_PORT: u16 = 4317;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastBackend {
    Powershell,
    Electron,
}

#[derive(Debug, Clone)]
pub struct ServeConfig {
    pub port: u16,
    pub web_dist: PathBuf,
    pub app_key: Option<String>,
    pub dev: bool,
    pub monitor: MonitorConfig,
    pub status_line_command: String,
    pub activity_limit: usize,
    pub batch_ms: i64,
    pub host_rpc: bool,
    pub dev_origin_port: u16,
    pub toast: Option<ToastBackend>,
    pub toast_link: Option<String>,
    pub res_fixture: Option<PathBuf>,
    pub stats: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    NoAppKey,
    BadPort(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::NoAppKey => write!(f, "Orrery runs inside its desktop app: npm run desktop (UI work: npm run dev)."),
            ConfigError::BadPort(v) => write!(f, "CCM_PORT must be an integer 0..65535 (0 = any free port), got {v:?}"),
        }
    }
}

pub fn parse_port(raw: Option<&str>) -> Result<u16, ConfigError> {
    let Some(text) = raw.map(str::trim).filter(|s| !s.is_empty()) else { return Ok(DEFAULT_PORT) };
    text.parse::<u16>().map_err(|_| ConfigError::BadPort(text.chars().take(40).collect()))
}

fn positive(raw: Option<String>, fallback: i64) -> i64 {
    raw.and_then(|v| v.trim().parse::<i64>().ok()).filter(|n| *n > 0).unwrap_or(fallback)
}

pub fn toast_backend(toast: Option<&str>, backend: Option<&str>, windows: bool) -> Option<ToastBackend> {
    if toast == Some("0") {
        None
    } else if windows {
        Some(ToastBackend::Powershell)
    } else if backend == Some("electron") {
        Some(ToastBackend::Electron)
    } else {
        None
    }
}

impl ServeConfig {
    pub fn from_env(dev: bool) -> Result<ServeConfig, ConfigError> {
        Self::from_vars(&|k| std::env::var(k).ok().filter(|v| !v.is_empty()), dev)
    }

    pub fn from_vars(get: &dyn Fn(&str) -> Option<String>, dev: bool) -> Result<ServeConfig, ConfigError> {
        let app_key = get("CCM_APP_KEY");
        if app_key.is_none() && !dev {
            return Err(ConfigError::NoAppKey);
        }
        let port = parse_port(get("CCM_PORT").as_deref())?;
        let home = get("USERPROFILE").or_else(|| get("HOME")).unwrap_or_default();
        let home_dir = PathBuf::from(&home);
        let claude_root = get("CLAUDE_CONFIG_DIR").map_or_else(|| home_dir.join(".claude"), PathBuf::from);
        let data_dir = get("CCM_DATA_DIR").map_or_else(|| PathBuf::from(default_data_dir(get("LOCALAPPDATA").as_deref(), Platform::HOST, &home)), PathBuf::from);
        let mut monitor = MonitorConfig::new(claude_root, data_dir);
        monitor.stale_ttl_ms = positive(get("CCM_STALE_TTL_MIN"), 120) * 60_000;
        monitor.peer_ttl_ms = positive(get("CCM_PEER_TTL_MIN"), 60) * 60_000;
        monitor.verify_processes = cfg!(any(windows, target_os = "macos")) && get("CCM_VERIFY_PROCESSES").as_deref() != Some("0");
        if get("CCM_CODEX").as_deref() != Some("0") {
            monitor.codex_home = Some(get("CODEX_HOME").map_or_else(|| home_dir.join(".codex"), PathBuf::from));
        }
        if get("CCM_DEEPSEEK").as_deref() != Some("0") {
            monitor.codewhale_home = Some(core_monitor::deepseek::config::codewhale_home(get("CODEWHALE_HOME"), |p| p.exists(), &home_dir));
            monitor.home_dir = home_dir;
        }
        let bridge = get("CCM_STATUSLINE_BRIDGE").unwrap_or_default();
        Ok(ServeConfig {
            port,
            web_dist: PathBuf::from(get("CCM_WEB_DIST").unwrap_or_else(|| "apps/web/dist".into())),
            app_key,
            dev,
            monitor,
            status_line_command: get("CCM_STATUSLINE_COMMAND").unwrap_or_else(|| format!("node \"{}\"", bridge.replace('\\', "/"))),
            activity_limit: 1000,
            batch_ms: 50,
            host_rpc: get("CCM_HOST_RPC").as_deref() == Some("1"),
            dev_origin_port: u16::try_from(positive(get("CCM_DEV_ORIGIN_PORT"), 5173)).unwrap_or(5173),
            toast: toast_backend(get("CCM_TOAST").as_deref(), get("CCM_TOAST_BACKEND").as_deref(), cfg!(windows)),
            toast_link: get("CCM_TOAST_LINK"),
            res_fixture: get("CCM_RES_FIXTURE").map(PathBuf::from),
            stats: get("CCM_SERVE_STATS").as_deref() == Some("1"),
        })
    }

    pub fn toast_supported(&self) -> bool {
        self.host_rpc && self.toast.is_some()
    }

    pub fn toast_full(&self) -> bool {
        self.toast == Some(ToastBackend::Powershell)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> {
        let map: HashMap<String, String> = pairs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect();
        move |k| map.get(k).cloned().filter(|v| !v.is_empty())
    }

    #[test]
    fn without_an_app_key_only_dev_starts() {
        assert_eq!(ServeConfig::from_vars(&vars(&[]), false).unwrap_err(), ConfigError::NoAppKey);
        assert!(ServeConfig::from_vars(&vars(&[]), true).unwrap().app_key.is_none());
        assert_eq!(ServeConfig::from_vars(&vars(&[("CCM_APP_KEY", "k")]), false).unwrap().app_key.as_deref(), Some("k"));
    }

    #[test]
    fn port_zero_means_any_port_and_a_bad_port_is_an_error_not_4317() {
        assert_eq!(parse_port(None), Ok(4317));
        assert_eq!(parse_port(Some(" ")), Ok(4317));
        assert_eq!(parse_port(Some("0")), Ok(0));
        assert_eq!(parse_port(Some("4399")), Ok(4399));
        for bad in ["-1", "65536", "abc", "43a", "1.5", "99999999999"] {
            assert!(matches!(parse_port(Some(bad)), Err(ConfigError::BadPort(_))), "{bad}");
        }
        let err = ServeConfig::from_vars(&vars(&[("CCM_APP_KEY", "k"), ("CCM_PORT", "nope")]), false).unwrap_err();
        assert!(matches!(err, ConfigError::BadPort(_)));
        assert_eq!(ServeConfig::from_vars(&vars(&[("CCM_APP_KEY", "k"), ("CCM_PORT", "0")]), false).unwrap().port, 0);
    }

    #[test]
    fn reads_the_collector_env() {
        let c = ServeConfig::from_vars(
            &vars(&[
                ("CCM_APP_KEY", "k"),
                ("HOME", "/h"),
                ("CLAUDE_CONFIG_DIR", "/c"),
                ("CCM_DATA_DIR", "/d"),
                ("CODEX_HOME", "/x"),
                ("CCM_DEEPSEEK", "0"),
                ("CCM_STALE_TTL_MIN", "5"),
                ("CCM_PEER_TTL_MIN", "zz"),
                ("CCM_WEB_DIST", "/w"),
                ("CCM_HOST_RPC", "1"),
                ("CCM_TOAST_LINK", "agent-monitor://"),
                ("CCM_RES_FIXTURE", "/f.json"),
                ("CCM_STATUSLINE_BRIDGE", "C:\\b\\s.mjs"),
            ]),
            false,
        )
        .unwrap();
        assert_eq!(c.monitor.claude_root, PathBuf::from("/c"));
        assert_eq!(c.monitor.data_dir, PathBuf::from("/d"));
        assert_eq!(c.monitor.codex_home, Some(PathBuf::from("/x")));
        assert_eq!(c.monitor.codewhale_home, None);
        assert_eq!(c.monitor.stale_ttl_ms, 5 * 60_000);
        assert_eq!(c.monitor.peer_ttl_ms, 60 * 60_000);
        assert_eq!(c.web_dist, PathBuf::from("/w"));
        assert!(c.host_rpc);
        assert_eq!(c.toast_link.as_deref(), Some("agent-monitor://"));
        assert_eq!(c.res_fixture, Some(PathBuf::from("/f.json")));
        assert_eq!(c.status_line_command, "node \"C:/b/s.mjs\"");
        let off = ServeConfig::from_vars(&vars(&[("CCM_APP_KEY", "k"), ("CCM_CODEX", "0")]), false).unwrap();
        assert_eq!(off.monitor.codex_home, None);
        assert!(!off.host_rpc);
    }

    #[test]
    fn toast_backend_follows_the_collector() {
        assert_eq!(toast_backend(Some("0"), None, true), None);
        assert_eq!(toast_backend(None, None, true), Some(ToastBackend::Powershell));
        assert_eq!(toast_backend(None, Some("electron"), false), Some(ToastBackend::Electron));
        assert_eq!(toast_backend(None, None, false), None);
    }
}
