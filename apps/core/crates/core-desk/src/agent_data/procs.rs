use super::rules::Agent;
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct AgentProc {
    pub pid: f64,
    pub name: String,
}

impl AgentProc {
    pub fn to_value(&self) -> Value {
        json!({"pid": self.pid, "name": self.name})
    }
}

fn base_name(name: &str) -> String {
    let unix = name.replace('\\', "/");
    let trimmed = unix.trim_end_matches('/');
    trimmed.rsplit('/').next().unwrap_or("").to_string()
}

pub fn proc_agent(name: &str) -> Option<Agent> {
    let lower = base_name(name).to_lowercase();
    let base = lower.strip_suffix(".exe").unwrap_or(&lower);
    match base {
        "claude" => Some(Agent::Claude),
        "codex" => Some(Agent::Codex),
        _ => None,
    }
}

fn os_base(name: &str) -> String {
    if cfg!(windows) {
        base_name(name)
    } else {
        let trimmed = name.trim_end_matches('/');
        trimmed.rsplit('/').next().unwrap_or("").to_string()
    }
}

fn fixture(file: &str) -> Vec<AgentProc> {
    let rows: Value = std::fs::read_to_string(file).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(Value::Array(Vec::new()));
    rows.as_array()
        .map(|a| {
            a.iter()
                .filter_map(|r| Some(AgentProc { pid: r.get("pid")?.as_f64()?, name: r.get("name")?.as_str()?.to_string() }))
                .collect()
        })
        .unwrap_or_default()
}

pub fn agent_procs(agent: Agent) -> Vec<AgentProc> {
    let own = std::process::id() as f64;
    let rows: Vec<AgentProc> = match std::env::var("CCM_E2E_AGENT_PROCS").ok().filter(|v| !v.is_empty()) {
        Some(file) => fixture(&file),
        None => core_procs::table()
            .unwrap_or_default()
            .into_iter()
            .map(|p| AgentProc { pid: p.pid as f64, name: if cfg!(windows) { p.name } else { os_base(&p.name) } })
            .collect(),
    };
    rows.into_iter().filter(|p| p.pid != own && proc_agent(&p.name) == Some(agent)).collect()
}

#[cfg(target_os = "macos")]
pub fn alive(pid: f64) -> bool {
    if !(pid.is_finite() && pid >= 1.0 && pid <= i32::MAX as f64 && pid.fract() == 0.0) {
        return false;
    }
    if unsafe { libc::kill(pid as i32, 0) } == 0 {
        return true;
    }
    std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}

#[cfg(not(target_os = "macos"))]
pub fn alive(pid: f64) -> bool {
    if !(pid.is_finite() && pid >= 1.0 && pid <= u32::MAX as f64 && pid.fract() == 0.0) {
        return false;
    }
    core_procs::table().is_some_and(|t| t.iter().any(|p| p.pid as f64 == pid))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_map_to_agents() {
        assert_eq!(proc_agent("claude"), Some(Agent::Claude));
        assert_eq!(proc_agent("C:\\Program Files\\Claude\\Claude.exe"), Some(Agent::Claude));
        assert_eq!(proc_agent("/usr/local/bin/codex"), Some(Agent::Codex));
        assert_eq!(proc_agent("codex-helper"), None);
        assert!(alive(std::process::id() as f64));
        assert!(!alive(0.0));
    }
}
