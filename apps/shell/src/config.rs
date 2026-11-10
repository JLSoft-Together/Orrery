use std::collections::HashMap;
use std::path::{Path, PathBuf};

use core_serve::config::{parse_port, ServeConfig};

pub const DEV_PORT: u16 = 4419;
pub const LINK_SCHEME: &str = "agent-monitor";

pub struct ShellConfig {
    pub serve: ServeConfig,
    pub port: u16,
    pub key: String,
    pub data_dir: PathBuf,
    pub webview_dir: PathBuf,
    pub core_bin: PathBuf,
    pub hidden: bool,
}

pub fn random_hex(bytes: usize) -> String {
    let mut buf = vec![0u8; bytes];
    if getrandom::fill(&mut buf).is_err() {
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos());
        for (i, b) in buf.iter_mut().enumerate() {
            *b = (seed >> ((i % 16) * 8)) as u8 ^ (i as u8).wrapping_mul(151);
        }
    }
    buf.iter().map(|b| format!("{b:02x}")).collect()
}

pub fn uuid() -> String {
    let h = random_hex(16);
    let v = format!("4{}", &h[13..16]);
    let variant = ["8", "9", "a", "b"][usize::from(h.as_bytes()[16] % 4)];
    format!("{}-{}-{}-{}{}-{}", &h[0..8], &h[8..12], v, variant, &h[17..20], &h[20..32])
}

fn free_port() -> u16 {
    std::net::TcpListener::bind(("127.0.0.1", 0)).and_then(|l| l.local_addr()).map_or(DEV_PORT, |a| a.port())
}

fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe().ok()?.parent().map(Path::to_path_buf)
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn core_name() -> &'static str {
    if cfg!(windows) {
        "orrery-core.exe"
    } else {
        "orrery-core"
    }
}

pub fn pick_core_bin(env: Option<String>, exe: Option<&Path>, repo: &Path) -> PathBuf {
    if let Some(p) = env.filter(|p| !p.trim().is_empty()) {
        return PathBuf::from(p.trim());
    }
    if let Some(next) = exe.map(|d| d.join(core_name())).filter(|p| p.is_file()) {
        return next;
    }
    repo.join("apps").join("core").join("target").join("release").join(core_name())
}

pub fn pick_web_dist(env: Option<String>, exe: Option<&Path>, repo: &Path) -> PathBuf {
    if let Some(p) = env.filter(|p| !p.trim().is_empty()) {
        return PathBuf::from(p);
    }
    if let Some(web) = exe.map(|d| d.join("web")).filter(|p| p.join("index.html").is_file()) {
        return web;
    }
    repo.join("apps").join("web").join("dist")
}

impl ShellConfig {
    pub fn from_env(args: &[String]) -> Result<ShellConfig, String> {
        let var = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
        let dev = cfg!(debug_assertions);
        let port = match var("CCM_PORT") {
            None if dev => DEV_PORT,
            raw => parse_port(raw.as_deref()).map_err(|e| e.to_string())?,
        };
        let port = if port == 0 { free_port() } else { port };
        let key = random_hex(32);
        let exe = exe_dir();
        let repo = repo_root();
        let mut over: HashMap<&str, String> = HashMap::new();
        over.insert("CCM_APP_KEY", key.clone());
        over.insert("CCM_PORT", port.to_string());
        over.insert("CCM_WEB_DIST", pick_web_dist(var("CCM_WEB_DIST"), exe.as_deref(), &repo).to_string_lossy().into_owned());
        if dev && var("CCM_DATA_DIR").is_none() {
            over.insert("CCM_DATA_DIR", std::env::temp_dir().join("orrery-shell-dev").to_string_lossy().into_owned());
        }
        if var("CCM_STATUSLINE_COMMAND").is_none() {
            over.insert("CCM_STATUSLINE_COMMAND", String::new());
        }
        let get = |k: &str| match k {
            "CCM_HOST_RPC" => None,
            _ => over.get(k).cloned().or_else(|| var(k)),
        };
        let serve = ServeConfig::from_vars(&get, false).map_err(|e| e.to_string())?;
        let data_dir = serve.monitor.data_dir.clone();
        let webview_dir = var("CCM_SHELL_WEBVIEW_DIR").map_or_else(|| data_dir.join("shell-webview"), PathBuf::from);
        Ok(ShellConfig {
            serve,
            port,
            key,
            data_dir,
            webview_dir,
            core_bin: pick_core_bin(var("CCM_CORE_BIN"), exe.as_deref(), &repo),
            hidden: args.iter().any(|a| a == "--hidden"),
        })
    }

    pub fn origin(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_has_the_v4_shape() {
        let id = uuid();
        assert_eq!(id.len(), 36);
        let parts: Vec<&str> = id.split('-').collect();
        assert_eq!(parts.iter().map(|p| p.len()).collect::<Vec<_>>(), [8, 4, 4, 4, 12]);
        assert!(parts[2].starts_with('4'));
        assert!(matches!(&parts[3][..1], "8" | "9" | "a" | "b"));
        assert_ne!(uuid(), id);
    }

    #[test]
    fn random_hex_is_long_and_fresh() {
        let a = random_hex(32);
        assert_eq!(a.len(), 64);
        assert!(a.bytes().all(|b| b.is_ascii_hexdigit()));
        assert_ne!(a, random_hex(32));
    }

    #[test]
    fn the_core_binary_comes_from_env_then_next_to_the_exe_then_the_repo() {
        let repo = PathBuf::from("/repo");
        assert_eq!(pick_core_bin(Some(" /x/core ".into()), None, &repo), PathBuf::from("/x/core"));
        let fallback = pick_core_bin(None, Some(Path::new("/definitely/missing")), &repo);
        assert_eq!(fallback, repo.join("apps").join("core").join("target").join("release").join(core_name()));
        assert_eq!(pick_web_dist(None, None, &repo), repo.join("apps").join("web").join("dist"));
        assert_eq!(pick_web_dist(Some("/w".into()), None, &repo), PathBuf::from("/w"));
    }
}
