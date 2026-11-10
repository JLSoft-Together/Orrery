use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use core_monitor::tracker::SensitiveCheck;
use core_sources::sensitive::watch::{SensitiveWatch, WatchDeps, WatchHit};
use core_transcript::rules::{CmdHit, Touch};
use serde_json::{Map, Value};

pub type Roots = Arc<Mutex<HashMap<String, String>>>;

const RULES: [&str; 10] = ["rm-rf", "rmdir", "rd", "del-s", "remove-item-recurse", "git-force-push", "git-reset-hard", "git-clean", "drop-table", "adb-uninstall"];

pub fn touch_from_json(v: &Value) -> Option<Touch> {
    if let Some(f) = v.get("file").and_then(Value::as_str) {
        return Some(Touch::File(f.to_owned()));
    }
    let cmds = v.get("cmds")?.as_array()?;
    let hits = cmds
        .iter()
        .filter_map(|c| {
            let rule = RULES.iter().find(|r| c.get("rule").and_then(Value::as_str) == Some(**r))?;
            let targets = c.get("targets").and_then(Value::as_array).map(|t| t.iter().filter_map(|s| s.as_str().map(str::to_owned)).collect());
            Some(CmdHit { rule, targets })
        })
        .collect();
    Some(Touch::Cmds(hits))
}

pub fn hit_map(h: WatchHit) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("rule".into(), Value::from(h.hit.rule));
    for (k, v) in [("path", h.hit.path), ("file", h.hit.file), ("project", h.project)] {
        if let Some(v) = v {
            m.insert(k.into(), Value::from(v));
        }
    }
    m
}

pub struct SensitiveAdapter(pub Arc<Mutex<SensitiveWatch>>);

impl SensitiveCheck for SensitiveAdapter {
    fn check(&mut self, terminal_id: &str, touch: &Value, cwd: Option<&str>) -> Vec<Map<String, Value>> {
        let Some(t) = touch_from_json(touch) else { return Vec::new() };
        let Ok(mut w) = self.0.lock() else { return Vec::new() };
        w.check(terminal_id, &t, cwd, None).into_iter().map(hit_map).collect()
    }
}

pub fn open_watch(claude_root: &std::path::Path, file: std::path::PathBuf, roots: Roots) -> Arc<Mutex<SensitiveWatch>> {
    let mut deps = WatchDeps::host(claude_root);
    deps.root_of = Some(Box::new(move |cwd: &str| roots.lock().ok().and_then(|r| r.get(cwd).cloned())));
    let mut watch = SensitiveWatch::new(deps, Some(file));
    watch.load();
    Arc::new(Mutex::new(watch))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn touches_round_trip_through_json() {
        let file = Touch::File("C:\\a\\.env".into());
        assert_eq!(touch_from_json(&file.to_json()), Some(file));
        let cmds = Touch::Cmds(vec![CmdHit { rule: "rm-rf", targets: Some(vec!["/".into()]) }, CmdHit { rule: "git-force-push", targets: None }]);
        assert_eq!(touch_from_json(&cmds.to_json()), Some(cmds));
        assert_eq!(touch_from_json(&json!({"cmds": [{"rule": "unknown"}]})), Some(Touch::Cmds(Vec::new())));
        assert_eq!(touch_from_json(&json!({"x": 1})), None);
    }

    #[test]
    fn every_command_rule_the_reader_emits_is_known() {
        let lines = ["rm -rf /tmp/x", "git push --force", "git reset --hard", "git clean -fd", "adb uninstall com.x", "Remove-Item -Recurse x", "rmdir /s x", "del /s x"];
        for line in lines {
            for h in core_transcript::rules::command_hits(line, 0) {
                assert!(RULES.contains(&h.rule), "{line} -> {}", h.rule);
            }
        }
    }

    #[test]
    fn the_adapter_reports_a_secret_file_with_its_project_root() {
        let tmp = std::env::temp_dir().join(format!("ccm-serve-sens-{}", std::process::id()));
        let roots: Roots = Arc::new(Mutex::new(HashMap::new()));
        let cwd = if cfg!(windows) { "C:\\work\\app\\sub" } else { "/work/app/sub" };
        let root = if cfg!(windows) { "C:\\work\\app" } else { "/work/app" };
        roots.lock().unwrap().insert(cwd.into(), root.into());
        let mut a = SensitiveAdapter(open_watch(&tmp, tmp.join("sensitive.json"), roots));
        let file = if cfg!(windows) { "C:\\work\\app\\.env" } else { "/work/app/.env" };
        let hits = a.check("t1", &Touch::File(file.into()).to_json(), Some(cwd));
        assert_eq!(hits.len(), 1, "{hits:?}");
        assert_eq!(hits[0]["project"], json!(cwd));
        assert!(a.check("t1", &Touch::File(file.into()).to_json(), Some(cwd)).is_empty());
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
