use crate::claude::Obj;
use serde_json::{json, Value};
use std::collections::BTreeMap;

fn tool_from(matcher: Option<&str>) -> String {
    let first = core_transcript::jsstr::js_trim(matcher.unwrap_or("").split('|').next().unwrap_or(""));
    let mut cs = first.chars();
    let simple = matches!(cs.next(), Some(c) if c.is_ascii_alphabetic()) && cs.all(|c| c.is_ascii_alphanumeric() || c == '_');
    if simple {
        first.to_string()
    } else {
        "Bash".into()
    }
}

fn sample_input(tool: &str, cwd: &str) -> Value {
    let base = cwd.trim_end_matches(['\\', '/']);
    let sep = if cwd.contains('\\') { '\\' } else { '/' };
    let file = format!("{base}{sep}orrery-hook-test.txt");
    match tool {
        "Bash" => json!({"command": "echo orrery-hook-test", "description": "Orrery hook test"}),
        "Read" => json!({"file_path": file}),
        "Write" => json!({"file_path": file, "content": "orrery hook test\n"}),
        "Edit" | "MultiEdit" => json!({"file_path": file, "old_string": "a", "new_string": "b"}),
        "Glob" | "Grep" => json!({"pattern": "orrery-hook-test"}),
        _ => json!({}),
    }
}

fn merge(base: &Value, extra: Value) -> Value {
    let mut out = base.as_object().cloned().unwrap_or_default();
    if let Value::Object(m) = extra {
        for (k, v) in m {
            out.insert(k, v);
        }
    }
    Value::Object(out)
}

pub fn fake_payload(event: &str, matcher: Option<&str>, cwd: &str, session_id: &str) -> Value {
    let base = json!({"session_id": session_id, "transcript_path": "", "cwd": cwd, "hook_event_name": event, "permission_mode": "default"});
    let pick = |options: &[&str], fallback: &str| -> String {
        let parts: Vec<&str> = matcher.unwrap_or("").split('|').collect();
        options.iter().find(|o| parts.contains(o)).map(|s| s.to_string()).unwrap_or_else(|| fallback.to_string())
    };
    let tool = tool_from(matcher);
    let input = sample_input(&tool, cwd);
    let call = json!({"tool_name": tool, "tool_input": input, "tool_use_id": "toolu_orrery_hook_test"});
    let with_call = |extra: Value| merge(&merge(&base, call.clone()), extra);
    match event {
        "PreToolUse" => with_call(json!({})),
        "PostToolUse" => with_call(json!({"tool_response": {}})),
        "PostToolUseFailure" => with_call(json!({"error": "Orrery hook test failure"})),
        "PermissionRequest" => merge(&base, json!({"tool_name": tool, "tool_input": input, "permission_suggestions": []})),
        "PermissionDenied" => with_call(json!({"reason": "Orrery hook test"})),
        "UserPromptSubmit" => merge(&base, json!({"prompt": "Orrery hook test (never sent to Claude)"})),
        "UserPromptExpansion" => merge(&base, json!({"expansion_type": "slash_command", "command_name": "orrery-hook-test", "command_args": "", "prompt": "Orrery hook test"})),
        "SessionStart" => merge(&base, json!({"source": pick(&["startup", "resume", "clear", "compact"], "startup")})),
        "SessionEnd" => merge(&base, json!({"reason": "other"})),
        "Stop" => merge(&base, json!({"stop_hook_active": false, "last_assistant_message": ""})),
        "StopFailure" => merge(&base, json!({"error": "Orrery hook test", "last_assistant_message": ""})),
        "SubagentStart" => merge(&base, json!({"agent_id": "orrery-hook-test", "agent_type": "general-purpose"})),
        "SubagentStop" => merge(
            &base,
            json!({"stop_hook_active": false, "agent_id": "orrery-hook-test", "agent_transcript_path": "", "agent_type": "general-purpose", "last_assistant_message": ""}),
        ),
        "Notification" => merge(&base, json!({"message": "Orrery hook test", "title": "Orrery", "notification_type": "idle_prompt"})),
        "PreCompact" => merge(&base, json!({"trigger": pick(&["manual", "auto"], "manual"), "custom_instructions": null})),
        "PostCompact" => merge(&base, json!({"trigger": pick(&["manual", "auto"], "manual"), "compact_summary": ""})),
        _ => base,
    }
}

const ENV_KEEP: [&str; 15] = ["PATH", "PATHEXT", "HOME", "USERPROFILE", "SYSTEMROOT", "SYSTEMDRIVE", "WINDIR", "COMSPEC", "TEMP", "TMP", "TMPDIR", "APPDATA", "LOCALAPPDATA", "LANG", "CLAUDE_CONFIG_DIR"];

pub fn test_env(env: &[(String, String)], project_dir: &str, plugin_root: Option<&str>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for (k, v) in env {
        if ENV_KEEP.contains(&k.to_uppercase().as_str()) {
            out.insert(k.clone(), v.clone());
        }
    }
    out.insert("CLAUDE_PROJECT_DIR".into(), project_dir.into());
    if let Some(r) = plugin_root.filter(|r| !r.is_empty()) {
        out.insert("CLAUDE_PLUGIN_ROOT".into(), r.into());
    }
    out
}

pub struct Spec {
    pub file: String,
    pub args: Vec<String>,
}

pub fn shell_for(hook: &Obj, command: &str, env: &dyn Fn(&str) -> Option<String>, win: bool, exists: &dyn Fn(&str) -> bool) -> Spec {
    if let Some(args) = hook.get("args").and_then(Value::as_array) {
        return Spec { file: command.into(), args: args.iter().filter_map(Value::as_str).map(str::to_string).collect() };
    }
    let ps = || Spec {
        file: if win { "powershell.exe" } else { "pwsh" }.into(),
        args: vec!["-NoProfile".into(), "-NonInteractive".into(), "-Command".into(), command.into()],
    };
    if hook.get("shell").and_then(Value::as_str) == Some("powershell") {
        return ps();
    }
    if win {
        let join = |base: Option<String>, tail: &[&str]| base.filter(|b| !b.is_empty()).map(|b| win_join(&b, tail));
        let candidates = [
            env("CLAUDE_CODE_GIT_BASH_PATH").filter(|s| !s.is_empty()),
            join(env("ProgramFiles"), &["Git", "bin", "bash.exe"]),
            join(env("ProgramFiles(x86)"), &["Git", "bin", "bash.exe"]),
            join(env("LOCALAPPDATA"), &["Programs", "Git", "bin", "bash.exe"]),
        ];
        return match candidates.into_iter().flatten().find(|c| exists(c)) {
            Some(bash) => Spec { file: bash, args: vec!["-c".into(), command.into()] },
            None => ps(),
        };
    }
    let sh = env("SHELL").filter(|s| s.ends_with("/bash") || s.ends_with("/zsh") || s.ends_with("/sh")).unwrap_or_else(|| "/bin/sh".into());
    Spec { file: sh, args: vec!["-c".into(), command.into()] }
}

fn win_join(base: &str, tail: &[&str]) -> String {
    let mut s = base.replace('/', "\\");
    for t in tail {
        if !s.ends_with('\\') {
            s.push('\\');
        }
        s.push_str(t);
    }
    s
}

pub fn expand_plugin_root(s: &str, root: Option<&str>) -> String {
    match root {
        Some(r) => s.replace("${CLAUDE_PLUGIN_ROOT}", r),
        None => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_shapes() {
        let p = fake_payload("PreToolUse", Some("Write|Edit"), "C:\\p\\", "sid");
        assert_eq!(crate::js::stringify(&p), r#"{"session_id":"sid","transcript_path":"","cwd":"C:\\p\\","hook_event_name":"PreToolUse","permission_mode":"default","tool_name":"Write","tool_input":{"file_path":"C:\\p\\orrery-hook-test.txt","content":"orrery hook test\n"},"tool_use_id":"toolu_orrery_hook_test"}"#);
        let c = fake_payload("PreCompact", Some("auto"), "/p", "s");
        assert_eq!(c["trigger"], "auto");
        assert_eq!(fake_payload("X", None, "/p", "s").as_object().unwrap().len(), 5);
    }

    #[test]
    fn shells() {
        let none = |_: &str| None;
        let no = |_: &str| false;
        let h = serde_json::Map::new();
        assert_eq!(shell_for(&h, "x", &none, true, &no).file, "powershell.exe");
        assert_eq!(shell_for(&h, "x", &|k| (k == "SHELL").then(|| "/bin/zsh".to_string()), false, &no).file, "/bin/zsh");
        let pf = |k: &str| (k == "ProgramFiles").then(|| "C:\\PF".to_string());
        assert_eq!(shell_for(&h, "x", &pf, true, &|p| p == "C:\\PF\\Git\\bin\\bash.exe").args, vec!["-c", "x"]);
    }
}
