use super::paths::{is_absolute, is_within, real_or_null};
use super::perm::{plan_rules, read_settings, FileState};
use crate::claude::diff::{format_settings, json_diff};
use crate::js;
use core_transcript::jsstr::units;
use regex::Regex;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub const DENY_BACKUP_SUFFIX: &str = ".orrery-bak";
const REGISTRY_MAX_FILES: usize = 300;
const REGISTRY_FILE_MAX: u64 = 64 * 1024;
const SECRET_FILE_GLOBS: [&str; 10] = [".env*", "*.jks", "*.keystore", "*.p12", "*.pem", "key.properties", "local.properties", "google-services.json", "GoogleService-Info.plist", "*.mobileprovision"];
const COMMAND_RULES: [&str; 8] = ["git-force-push", "git-reset-hard", "git-clean", "rm-rf", "del-s", "remove-item-recurse", "adb-uninstall", "drop-table"];
const CUSTOM_PREFIX: &str = "custom:";
const FILE_TOOLS: [&str; 4] = ["Edit", "Write", "MultiEdit", "NotebookEdit"];

fn glob_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^[A-Za-z0-9_.*?@+~-]{1,100}$").expect("regex"))
}

fn rule_group(rule: &str) -> Option<&'static str> {
    if rule == "outside-project" {
        Some("outside")
    } else if rule.starts_with(CUSTOM_PREFIX) {
        Some("custom")
    } else if SECRET_FILE_GLOBS.contains(&rule) {
        Some("secrets")
    } else if COMMAND_RULES.contains(&rule) {
        Some("command")
    } else {
        None
    }
}

fn command_deny(rule: &str) -> Option<(&'static [&'static str], &'static [&'static str])> {
    const BOTH: &[&str] = &["Bash", "PowerShell"];
    Some(match rule {
        "git-force-push" => (BOTH, &["git push --force *", "git push -f *", "git push * --force *", "git push * -f *"]),
        "git-reset-hard" => (BOTH, &["git reset --hard *", "git reset * --hard *"]),
        "git-clean" => (BOTH, &["git clean *"]),
        "rm-rf" => (&["Bash"], &["rm -rf *", "rm -fr *", "rm -Rf *"]),
        "adb-uninstall" => (BOTH, &["adb uninstall *", "adb * uninstall *"]),
        _ => return None,
    })
}

pub fn deny_rules_for(rule: &Value, tool: &Value) -> Vec<String> {
    let (Some(rule), Some(tool)) = (rule.as_str(), tool.as_str()) else { return Vec::new() };
    if units(rule) > 220 {
        return Vec::new();
    }
    let Some(group) = rule_group(rule) else { return Vec::new() };
    if group == "secrets" || group == "custom" {
        let glob = if group == "custom" { rule[CUSTOM_PREFIX.len()..].to_string() } else { rule.to_string() };
        let only_wild = glob.chars().all(|c| c == '*' || c == '?');
        return if FILE_TOOLS.contains(&tool) && glob_re().is_match(&glob) && !only_wild { vec![format!("Edit(**/{glob})")] } else { Vec::new() };
    }
    let Some((shells, patterns)) = command_deny(rule) else { return Vec::new() };
    if !(tool == "Bash" || tool == "PowerShell") || !shells.contains(&tool) {
        return Vec::new();
    }
    patterns.iter().map(|p| format!("{tool}({p})")).collect()
}

pub fn deny_file(scope: &str, project: &str, config_dir: &str) -> PathBuf {
    if scope == "user" {
        return Path::new(config_dir).join("settings.json");
    }
    Path::new(project).join(".claude").join(if scope == "project" { "settings.json" } else { "settings.local.json" })
}

pub fn session_cwds(config_dir: &str) -> Vec<String> {
    let dir = Path::new(config_dir).join("sessions");
    let Ok(rd) = std::fs::read_dir(&dir) else { return Vec::new() };
    let mut names: Vec<String> = rd.filter_map(Result::ok).filter_map(|e| e.file_name().into_string().ok()).filter(|n| n.ends_with(".json")).collect();
    names.truncate(REGISTRY_MAX_FILES);
    let mut out: Vec<String> = Vec::new();
    for name in names {
        let file = dir.join(name);
        if !std::fs::metadata(&file).map(|m| m.is_file() && m.len() <= REGISTRY_FILE_MAX).unwrap_or(false) {
            continue;
        }
        let Ok(bytes) = std::fs::read(&file) else { continue };
        let Some(Value::Object(o)) = js::parse(&String::from_utf8_lossy(&bytes)) else { continue };
        if let Some(cwd) = o.get("cwd").and_then(Value::as_str).filter(|c| is_absolute(c)) {
            if !out.iter().any(|x| x == cwd) {
                out.push(cwd.to_string());
            }
        }
    }
    out
}

pub fn is_session_project(project: &str, cwds: &[String]) -> bool {
    let real: Vec<String> = cwds.iter().filter_map(|c| real_or_null(c)).collect();
    if real.iter().any(|c| is_within(project, c) && is_within(c, project)) {
        return true;
    }
    if !real.iter().any(|c| is_within(project, c)) {
        return false;
    }
    Path::new(project).join(".git").exists()
}

pub struct DenyReq<'a> {
    pub project: &'a str,
    pub rule: &'a Value,
    pub tool: &'a Value,
    pub scope: &'a Value,
    pub config_dir: &'a str,
}

pub fn write_deny(req: &DenyReq, confirm: &dyn Fn(Value) -> bool) -> Value {
    let fail = |e: &str| json!({"ok": false, "error": e});
    let scope = match req.scope.as_str() {
        Some(s @ ("project" | "local" | "user")) => s,
        _ => return fail("invalid"),
    };
    let rules = deny_rules_for(req.rule, req.tool);
    if rules.is_empty() {
        return fail("invalid");
    }
    let file = deny_file(scope, req.project, req.config_dir);
    let dir = file.parent().map(Path::to_path_buf).unwrap_or_default();
    if scope != "user" {
        if let Some(rd) = real_or_null(&dir.to_string_lossy()) {
            if !is_within(req.project, &rd) {
                return fail("denied");
            }
        }
        if let Some(rf) = real_or_null(&file.to_string_lossy()) {
            if !is_within(req.project, &rf) {
                return fail("denied");
            }
        }
    }
    let read = read_settings(&file);
    let Some(settings) = read.settings.as_ref().filter(|_| read.state != FileState::Unreadable) else { return fail("unreadable") };
    let (next, added) = plan_rules(settings, "deny", &rules, true);
    if added.is_empty() {
        return json!({"ok": true, "scope": scope, "added": []});
    }
    let text = format_settings(&next, &read.text);
    let file_s = file.to_string_lossy().to_string();
    if !confirm(json!({"file": file_s, "scope": scope, "added": added, "diff": json_diff(&read.text, &text)})) {
        return fail("cancelled");
    }
    let again = read_settings(&file);
    if again.text != read.text || again.state != read.state {
        return fail("changed");
    }
    let mut tmp = file.as_os_str().to_owned();
    tmp.push(".orrery-tmp");
    let tmp = PathBuf::from(tmp);
    let res = (|| -> std::io::Result<()> {
        std::fs::create_dir_all(&dir)?;
        if read.state == FileState::Ok {
            let mut b = file.as_os_str().to_owned();
            b.push(DENY_BACKUP_SUFFIX);
            std::fs::copy(&file, PathBuf::from(b))?;
        }
        std::fs::write(&tmp, &text)?;
        std::fs::rename(&tmp, &file)
    })();
    match res {
        Ok(()) => json!({"ok": true, "scope": scope, "added": added}),
        Err(_) => {
            let _ = std::fs::remove_file(&tmp);
            fail("failed")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deny_rules() {
        assert_eq!(deny_rules_for(&json!(".env*"), &json!("Edit")), vec!["Edit(**/.env*)"]);
        assert_eq!(deny_rules_for(&json!("custom:*.secret"), &json!("Write")), vec!["Edit(**/*.secret)"]);
        assert!(deny_rules_for(&json!("custom:*"), &json!("Write")).is_empty());
        assert!(deny_rules_for(&json!("custom:a/b"), &json!("Write")).is_empty());
        assert_eq!(deny_rules_for(&json!("rm-rf"), &json!("Bash")).len(), 3);
        assert!(deny_rules_for(&json!("rm-rf"), &json!("PowerShell")).is_empty());
        assert!(deny_rules_for(&json!("drop-table"), &json!("Bash")).is_empty());
        assert!(deny_rules_for(&json!("nope"), &json!("Bash")).is_empty());
    }
}
