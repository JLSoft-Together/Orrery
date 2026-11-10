use super::settings::{read_settings, Read};
use crate::claude::{self, diff, Obj};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub const BRIDGE_EXE: &str = if cfg!(windows) { "orrery-bridge.exe" } else { "orrery-bridge" };
pub const NAMES: [&str; 2] = ["statusline-bridge", "hook-bridge"];

fn slashes(p: &str) -> String {
    p.replace('\\', "/")
}

pub fn bridge_bin_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("bin")
}

pub fn install(source: &Path, data_dir: &Path) -> Result<PathBuf, String> {
    let dir = bridge_bin_dir(data_dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let target = dir.join(BRIDGE_EXE);
    let want = std::fs::read(source).map_err(|e| format!("{}: {e}", source.display()))?;
    if std::fs::read(&target).is_ok_and(|have| have == want) {
        return Ok(target);
    }
    let mut tmp = target.as_os_str().to_owned();
    tmp.push(format!(".{}.tmp", std::process::id()));
    let tmp = PathBuf::from(tmp);
    let res = std::fs::write(&tmp, &want).and_then(|_| {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
        }
        std::fs::rename(&tmp, &target)
    });
    if let Err(e) = res {
        let _ = std::fs::remove_file(&tmp);
        if target.is_file() {
            return Ok(target);
        }
        return Err(e.to_string());
    }
    Ok(target)
}

pub fn command_for(exe: &Path, name: &str) -> String {
    format!("\"{}\" {name}", slashes(&crate::util::path_str(exe)))
}

pub fn install_json(source: &Path, data_dir: &Path) -> Value {
    match install(source, data_dir) {
        Ok(exe) => {
            let script = crate::util::path_str(&exe);
            json!({"ok": true, "statusline": {"script": script, "command": command_for(&exe, NAMES[0])}, "hook": {"script": script, "command": command_for(&exe, NAMES[1])}})
        }
        Err(e) => json!({"ok": false, "error": e}),
    }
}

fn fold(s: &str) -> String {
    let s = slashes(s);
    if cfg!(windows) {
        s.to_lowercase()
    } else {
        s
    }
}

pub fn old_prefixes(data_dir: &Path, name: &str) -> Vec<String> {
    let bin = slashes(&crate::util::path_str(&bridge_bin_dir(data_dir)));
    vec![format!("node \"{bin}/{name}.mjs\""), format!("\"{bin}/{name}.cmd\""), format!("\"{bin}/{name}.sh\"")]
}

pub fn migrated(command: &str, data_dir: &Path, name: &str, next: &str) -> Option<String> {
    for p in old_prefixes(data_dir, name) {
        let n = p.len();
        if command.len() >= n && command.is_char_boundary(n) && fold(&command[..n]) == fold(&p) {
            let rest = &command[n..];
            if rest.is_empty() || rest.starts_with(' ') {
                return Some(format!("{next}{rest}"));
            }
        }
    }
    None
}

fn migrate_hooks(settings: &mut Obj, data_dir: &Path, next: &str) -> usize {
    let Some(Value::Object(hooks)) = settings.get_mut("hooks") else { return 0 };
    let mut n = 0;
    for (_, groups) in hooks.iter_mut() {
        let Some(groups) = groups.as_array_mut() else { continue };
        for g in groups.iter_mut() {
            let Some(list) = g.get_mut("hooks").and_then(Value::as_array_mut) else { continue };
            for h in list.iter_mut() {
                let Some(h) = h.as_object_mut() else { continue };
                let Some(c) = h.get("command").and_then(Value::as_str) else { continue };
                if let Some(nc) = migrated(c, data_dir, NAMES[1], next) {
                    h.insert("command".into(), Value::String(nc));
                    n += 1;
                }
            }
        }
    }
    n
}

pub fn migrate(file: &Path, data_dir: &Path, statusline: Option<&str>, hook: Option<&str>) -> Value {
    let text = match std::fs::read(file) {
        Ok(b) => String::from_utf8_lossy(&b).into_owned(),
        Err(_) => return json!({"result": "none", "statusLine": false, "hooks": 0}),
    };
    let mut settings = match read_settings(file) {
        Read::Ok(s) => s,
        Read::Missing(_) => return json!({"result": "none", "statusLine": false, "hooks": 0}),
        Read::Unreadable => return json!({"result": "unreadable", "statusLine": false, "hooks": 0}),
    };
    let mut line = false;
    if let Some(next) = statusline {
        if let Some(Value::Object(sl)) = settings.get_mut("statusLine") {
            if let Some(nc) = sl.get("command").and_then(Value::as_str).and_then(|c| migrated(c, data_dir, NAMES[0], next)) {
                sl.insert("command".into(), Value::String(nc));
                line = true;
            }
        }
    }
    let hooks = hook.map(|next| migrate_hooks(&mut settings, data_dir, next)).unwrap_or(0);
    if !line && hooks == 0 {
        return json!({"result": "none", "statusLine": false, "hooks": 0});
    }
    let out = diff::format_settings(&Value::Object(settings), &text);
    let result = if claude::write_atomic(file, &out, true) { "ok" } else { "failed" };
    json!({"result": result, "statusLine": line, "hooks": hooks})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_old_bridge_commands() {
        let data = if cfg!(windows) { Path::new("C:\\Users\\Me\\AppData\\Local\\ccm") } else { Path::new("/Users/me/Library/Application Support/ccm") };
        let bin = slashes(&crate::util::path_str(&data.join("bin")));
        let old = format!("node \"{bin}/statusline-bridge.mjs\" --chain-b64 eA==");
        assert_eq!(migrated(&old, data, "statusline-bridge", "\"N\" statusline-bridge").unwrap(), "\"N\" statusline-bridge --chain-b64 eA==");
        assert_eq!(migrated(&format!("\"{bin}/hook-bridge.cmd\""), data, "hook-bridge", "\"N\" hook-bridge").unwrap(), "\"N\" hook-bridge");
        assert!(migrated(&format!("\"{bin}/hook-bridge.cmdx\""), data, "hook-bridge", "N").is_none());
        assert!(migrated("node \"/other/hook-bridge.mjs\"", data, "hook-bridge", "N").is_none());
    }
}
