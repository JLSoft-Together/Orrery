use crate::js;
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agent {
    Claude,
    Codex,
}

impl Agent {
    pub const ALL: [Agent; 2] = [Agent::Claude, Agent::Codex];

    pub fn parse(v: &Value) -> Option<Agent> {
        match v.as_str() {
            Some("claude") => Some(Agent::Claude),
            Some("codex") => Some(Agent::Codex),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Agent::Claude => "claude",
            Agent::Codex => "codex",
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Rule {
    pub id: &'static str,
    pub kind: &'static str,
    pub names: &'static [&'static str],
    pub cleanable: bool,
    pub idle_only: bool,
}

const fn g(id: &'static str, kind: &'static str, names: &'static [&'static str], cleanable: bool, idle_only: bool) -> Rule {
    Rule { id, kind, names, cleanable, idle_only }
}

const CLAUDE: [Rule; 12] = [
    g("transcripts", "history", &["projects"], false, false),
    g("liveSessions", "history", &["sessions"], false, false),
    g("promptHistory", "history", &["history.jsonl", "paste-cache"], false, false),
    g("checkpoints", "history", &["file-history"], false, false),
    g("notes", "history", &["plans", "todos", "tasks", "teams", "agent-memory", "agent-memory-local", "memory", "usage-data"], false, false),
    g("debugLogs", "cache", &["debug", "logs", "telemetry", "statsig", "traces", "startup-perf", "api-dumps", "dump-prompts"], true, false),
    g("shellSnapshots", "cache", &["shell-snapshots", "session-env"], true, true),
    g("appCache", "cache", &["cache", "image-cache", "mcp-discovery-cache", "mcp-health-cache.json", "mcp-needs-auth-cache.json", "stats-cache.json"], true, false),
    g("plugins", "program", &["plugins"], false, false),
    g("extensions", "config", &["skills", "agents", "commands", "hooks", "output-styles", "workflows", "rules", "routines", "themes"], false, false),
    g(
        "settings",
        "config",
        &[
            "settings.json*",
            "settings.local.json*",
            "CLAUDE.md",
            "keybindings.json",
            "policy-limits.json*",
            "remote-settings.json*",
            "scheduled_tasks.json",
            "launch.json",
            "backups",
            ".last-cleanup",
            ".last-update-result.json",
            "daemon*",
            "state",
            "jobs",
            "ide",
            "chrome",
        ],
        false,
        false,
    ),
    g("credentials", "config", &[".credentials.json", "channels"], false, false),
];

const CODEX: [Rule; 11] = [
    g("rollouts", "history", &["sessions", "archived_sessions"], false, false),
    g("threads", "history", &["thread_history_*", "state_*", "session_index.jsonl", "history.jsonl", "goals_*", "queue_*", "memories*", "attachments"], false, false),
    g("generatedImages", "history", &["generated_images"], false, false),
    g("codexLogs", "cache", &["log", "sandbox*.log"], true, false),
    g("logDb", "cache", &["logs_*"], false, false),
    g("codexCache", "cache", &["cache", "models_cache.json"], true, false),
    g("codexTemp", "cache", &[".tmp", "tmp"], true, true),
    g("codexPrograms", "program", &["packages", "plugins", ".sandbox-bin", "vendor_imports"], false, false),
    g("codexExtensions", "config", &["skills", "rules", "AGENTS.md", "hooks.json"], false, false),
    g(
        "codexSettings",
        "config",
        &[
            "config.toml*",
            "version.json",
            ".codex-global-state.json*",
            "..codex-global-state.json*",
            "computer-use",
            "browser",
            "chrome-native-hosts*",
            "app-server-*",
            "process_manager",
            "*-locks",
            ".personality_migration",
            ".sandbox_migration",
            ".app-server-state-reconciled-*",
            ".codex-provisioning-*",
            ".sqlite-maintenance.lock",
        ],
        false,
        false,
    ),
    g("codexCredentials", "config", &["auth.json", ".sandbox-secrets", ".sandbox", "cap_sid", "installation_id"], false, false),
];

pub static UNKNOWN: Rule = g("unknown", "unknown", &[], false, false);

pub fn groups(agent: Agent) -> &'static [Rule] {
    match agent {
        Agent::Claude => &CLAUDE,
        Agent::Codex => &CODEX,
    }
}

fn matches(pattern: &str, name: &str) -> bool {
    if let Some(rest) = pattern.strip_prefix('*') {
        return name.ends_with(rest);
    }
    match pattern.find('*') {
        None => pattern == name,
        Some(star) => name.starts_with(&pattern[..star]) && name.ends_with(&pattern[star + 1..]) && js::len(name) + 1 >= js::len(pattern),
    }
}

pub fn classify_entry(agent: Agent, name: &str, case_insensitive: bool) -> &'static Rule {
    let n = if case_insensitive { name.to_lowercase() } else { name.to_string() };
    for rule in groups(agent) {
        let hit = rule.names.iter().any(|p| if case_insensitive { matches(&p.to_lowercase(), &n) } else { matches(p, &n) });
        if hit {
            return rule;
        }
    }
    &UNKNOWN
}

pub fn group_rule(agent: Agent, id: &str) -> Option<&'static Rule> {
    groups(agent).iter().find(|r| r.id == id)
}

pub const CLEAN_AGE_DAYS: [f64; 4] = [0.0, 7.0, 30.0, 90.0];
pub const CLEAN_MIN_AGE_MS: f64 = 24.0 * 60.0 * 60.0 * 1000.0;
pub const STRIP_AGE_DAYS: [f64; 3] = [30.0, 60.0, 90.0];
pub const DAY: f64 = 24.0 * 60.0 * 60.0 * 1000.0;

#[derive(Debug, Clone, PartialEq)]
pub struct CleanRequest {
    pub agent: Agent,
    pub groups: Vec<&'static str>,
    pub older_days: f64,
}

pub fn parse_clean_request(raw: &Value) -> Option<CleanRequest> {
    let r = raw.as_object()?;
    let agent = Agent::parse(r.get("agent")?)?;
    let list = r.get("groups")?.as_array()?;
    if list.is_empty() || list.len() > 20 {
        return None;
    }
    let older_days = r.get("olderDays")?.as_f64().filter(|d| CLEAN_AGE_DAYS.contains(d))?;
    let mut groups: Vec<&'static str> = Vec::new();
    for v in list {
        let rule = v.as_str().and_then(|id| group_rule(agent, id)).filter(|r| r.cleanable)?;
        if !groups.contains(&rule.id) {
            groups.push(rule.id);
        }
    }
    Some(CleanRequest { agent, groups, older_days })
}

pub fn parse_strip_days(raw: &Value) -> Option<f64> {
    raw.as_object()?.get("minDays")?.as_f64().filter(|d| STRIP_AGE_DAYS.contains(d))
}

pub fn agent_of(raw: &Value) -> Option<Agent> {
    Agent::parse(raw.as_object()?.get("agent")?)
}

pub fn moved_name(base: &str, at: f64) -> String {
    format!("{base}.orrery-moved-{}", js::number(at))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn classifies_like_the_shared_table() {
        assert_eq!(classify_entry(Agent::Claude, "projects", false).kind, "history");
        let s = classify_entry(Agent::Claude, "shell-snapshots", false);
        assert!(s.cleanable && s.idle_only);
        assert_eq!(classify_entry(Agent::Claude, "settings.json.bak-statusline", false).kind, "config");
        assert_eq!(classify_entry(Agent::Claude, ".caveman-active.123", false).kind, "unknown");
        assert_eq!(classify_entry(Agent::Codex, "thread_history_1.sqlite-wal", false).kind, "history");
        assert!(classify_entry(Agent::Codex, "sandbox.2026-05-27.log", false).cleanable);
        assert!(!classify_entry(Agent::Codex, "logs_2.sqlite", false).cleanable);
        assert_eq!(classify_entry(Agent::Codex, "x-locks", false).id, "codexSettings");
        assert_eq!(classify_entry(Agent::Claude, "Projects", true).id, "transcripts");
        assert_eq!(classify_entry(Agent::Claude, "Projects", false).id, "unknown");
        assert_eq!(classify_entry(Agent::Codex, "sandbo.log", false).id, "unknown");
    }

    #[test]
    fn parses_requests_like_the_shared_parser() {
        let ok = parse_clean_request(&json!({"agent": "claude", "groups": ["debugLogs", "debugLogs", "appCache"], "olderDays": 7})).unwrap();
        assert_eq!(ok.groups, vec!["debugLogs", "appCache"]);
        assert!(parse_clean_request(&json!({"agent": "claude", "groups": ["transcripts"], "olderDays": 7})).is_none());
        assert!(parse_clean_request(&json!({"agent": "claude", "groups": ["debugLogs"], "olderDays": 8})).is_none());
        assert!(parse_clean_request(&json!({"agent": "claude", "groups": [], "olderDays": 0})).is_none());
        assert!(parse_clean_request(&json!({"agent": "x", "groups": ["debugLogs"], "olderDays": 0})).is_none());
        assert!(parse_clean_request(&json!({"agent": "codex", "groups": ["debugLogs"], "olderDays": 0})).is_none());
        assert_eq!(parse_strip_days(&json!({"minDays": 60})), Some(60.0));
        assert_eq!(parse_strip_days(&json!({"minDays": "60"})), None);
        assert_eq!(agent_of(&json!({"agent": "codex"})), Some(Agent::Codex));
        assert_eq!(agent_of(&json!(["codex"])), None);
        assert_eq!(moved_name(".claude", 1_700_000_000_000.0), ".claude.orrery-moved-1700000000000");
    }
}
