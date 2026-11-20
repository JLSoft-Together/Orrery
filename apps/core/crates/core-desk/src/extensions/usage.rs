use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const CACHE_MAX: usize = 64;
const CHUNK: usize = 1 << 20;

#[derive(Default)]
struct Tally {
    offset: u64,
    rest: Vec<u8>,
    ids: HashSet<String>,
    total: u64,
    servers: BTreeMap<String, u64>,
    namespaces: BTreeMap<String, u64>,
}

#[derive(Default)]
pub struct SessionUsage {
    files: Mutex<(HashMap<PathBuf, Tally>, VecDeque<PathBuf>)>,
    found: Mutex<HashMap<String, PathBuf>>,
}

pub fn valid_session(id: &str) -> bool {
    let n = id.len();
    (8..=64).contains(&n) && id.chars().next().is_some_and(|c| c.is_ascii_alphanumeric()) && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn key_ok(k: &str, max: usize) -> bool {
    !k.is_empty() && k.len() <= max && k.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
}

pub fn server_of(tool: &str) -> Option<&str> {
    let rest = tool.strip_prefix("mcp__")?;
    let end = rest.find("__")?;
    let key = &rest[..end];
    key_ok(key, 200).then_some(key)
}

fn namespace_of<'a>(tool: &str, input: &'a Value) -> Option<&'a str> {
    let raw = match tool {
        "Skill" => input.get("skill"),
        "Agent" | "Task" => input.get("subagent_type"),
        _ => None,
    }?
    .as_str()?;
    let at = raw.find(':')?;
    let ns = &raw[..at];
    key_ok(ns, 100).then_some(ns)
}

fn scan_line(t: &mut Tally, line: &[u8]) {
    if !line.windows(10).any(|w| w == b"\"tool_use\"") {
        return;
    }
    let Ok(entry) = serde_json::from_slice::<Value>(line) else { return };
    let Some(Value::Array(content)) = entry.get("message").and_then(|m| m.get("content")) else { return };
    for b in content {
        if b.get("type").and_then(Value::as_str) != Some("tool_use") {
            continue;
        }
        let (Some(id), Some(name)) = (b.get("id").and_then(Value::as_str), b.get("name").and_then(Value::as_str)) else { continue };
        if !t.ids.insert(id.to_string()) {
            continue;
        }
        t.total += 1;
        if let Some(s) = server_of(name) {
            *t.servers.entry(s.to_string()).or_default() += 1;
        }
        if let Some(ns) = b.get("input").and_then(|i| namespace_of(name, i)) {
            *t.namespaces.entry(ns.to_string()).or_default() += 1;
        }
    }
}

fn feed(t: &mut Tally, file: &Path, size: u64) {
    let Ok(mut f) = std::fs::File::open(file) else { return };
    if f.seek(SeekFrom::Start(t.offset)).is_err() {
        return;
    }
    let mut buf = vec![0u8; CHUNK];
    while t.offset < size {
        let want = CHUNK.min((size - t.offset) as usize);
        let n = match f.read(&mut buf[..want]) {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        t.offset += n as u64;
        let mut data = std::mem::take(&mut t.rest);
        data.extend_from_slice(&buf[..n]);
        match data.iter().rposition(|&c| c == b'\n') {
            None => t.rest = data,
            Some(last) => {
                for line in data[..last].split(|&c| c == b'\n') {
                    scan_line(t, line);
                }
                t.rest = data[last + 1..].to_vec();
            }
        }
    }
}

impl SessionUsage {
    fn locate(&self, root: &Path, sid: &str) -> Option<PathBuf> {
        let mut found = self.found.lock().unwrap();
        if let Some(p) = found.get(sid) {
            if p.is_file() {
                return Some(p.clone());
            }
            found.remove(sid);
        }
        let name = format!("{sid}.jsonl");
        let mut dirs: Vec<PathBuf> = std::fs::read_dir(root.join("projects")).ok()?.filter_map(|e| e.ok().map(|e| e.path())).collect();
        dirs.sort();
        let hit = dirs.into_iter().map(|d| d.join(&name)).find(|f| f.is_file())?;
        found.insert(sid.to_string(), hit.clone());
        Some(hit)
    }

    fn update(&self, file: &Path, out: &mut Tally) {
        let Ok(meta) = std::fs::metadata(file) else { return };
        let size = meta.len();
        let mut guard = self.files.lock().unwrap();
        let (map, order) = &mut *guard;
        let mut t = map.remove(file).unwrap_or_default();
        order.retain(|p| p != file);
        if size < t.offset {
            t = Tally::default();
        }
        if size > t.offset {
            feed(&mut t, file, size);
        }
        out.total += t.total;
        for (k, v) in &t.servers {
            *out.servers.entry(k.clone()).or_default() += v;
        }
        for (k, v) in &t.namespaces {
            *out.namespaces.entry(k.clone()).or_default() += v;
        }
        map.insert(file.to_path_buf(), t);
        order.push_back(file.to_path_buf());
        while order.len() > CACHE_MAX {
            if let Some(old) = order.pop_front() {
                map.remove(&old);
            }
        }
    }

    pub fn read(&self, root: &Path, sid: &Value) -> Value {
        let Some(sid) = sid.as_str().filter(|s| valid_session(s)) else { return json!({"ok": false, "code": "badArg"}) };
        let Some(main) = self.locate(root, sid) else { return json!({"ok": false, "code": "missing"}) };
        let mut files = vec![main.clone()];
        if let Some(parent) = main.parent() {
            if let Ok(rd) = std::fs::read_dir(parent.join(sid).join("subagents")) {
                let mut subs: Vec<PathBuf> = rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "jsonl")).collect();
                subs.sort();
                files.extend(subs);
            }
        }
        let mut sum = Tally::default();
        for f in &files {
            self.update(f, &mut sum);
        }
        let map = |m: BTreeMap<String, u64>| Value::Object(m.into_iter().map(|(k, v)| (k, json!(v))).collect::<Map<String, Value>>());
        json!({"ok": true, "total": sum.total, "servers": map(sum.servers), "namespaces": map(sum.namespaces)})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(id: &str, name: &str, input: Value) -> String {
        json!({"type": "assistant", "message": {"content": [{"type": "text", "text": "x"}, {"type": "tool_use", "id": id, "name": name, "input": input}]}}).to_string()
    }

    #[test]
    fn counts_servers_and_namespaces_incrementally() {
        let root = std::env::temp_dir().join(format!("ccm-usage-{}", crate::util::uuid()));
        let sid = "0f0ce96d-9ef8-46ae-8a9d-b13f5a00464b";
        let proj = root.join("projects").join("F--p");
        std::fs::create_dir_all(proj.join(sid).join("subagents")).unwrap();
        let main = proj.join(format!("{sid}.jsonl"));
        let first = [
            line("t1", "mcp__figma__get", json!({})),
            line("t1", "mcp__figma__get", json!({})),
            line("t2", "mcp__claude_ai_Gmail__send", json!({})),
            line("t3", "Skill", json!({"skill": "frontend-design:frontend-design"})),
            line("t4", "Agent", json!({"subagent_type": "caveman:cavecrew-builder"})),
            line("t5", "Skill", json!({"skill": "jin"})),
            line("t6", "Read", json!({"file_path": "C:/secret"})),
            r#"{"type":"attachment","tools":[{"name":"mcp__nope__x"}],"note":"tool_use"}"#.to_string(),
            "not json \"tool_use\"".to_string(),
        ];
        std::fs::write(&main, first.join("\n") + "\n" + &line("t7", "mcp__figma__put", json!({}))[..20]).unwrap();
        std::fs::write(proj.join(sid).join("subagents").join("agent-a.jsonl"), line("s1", "mcp__plugin_github_github__pr", json!({})) + "\n").unwrap();
        let u = SessionUsage::default();
        let out = u.read(&root, &json!(sid));
        assert_eq!(out, json!({"ok": true, "total": 7, "servers": {"figma": 1, "claude_ai_Gmail": 1, "plugin_github_github": 1}, "namespaces": {"frontend-design": 1, "caveman": 1}}));
        assert!(!out.to_string().contains("secret"));
        let full = first.join("\n") + "\n" + &line("t7", "mcp__figma__put", json!({})) + "\n";
        std::fs::write(&main, &full).unwrap();
        assert_eq!(u.read(&root, &json!(sid))["servers"]["figma"], 2);
        std::fs::write(&main, line("n1", "mcp__x__y", json!({})) + "\n").unwrap();
        assert_eq!(u.read(&root, &json!(sid))["servers"], json!({"x": 1, "plugin_github_github": 1}));
        assert_eq!(u.read(&root, &json!("../etc")), json!({"ok": false, "code": "badArg"}));
        assert_eq!(u.read(&root, &json!("0f0ce96d-0000-0000-0000-000000000000")), json!({"ok": false, "code": "missing"}));
        assert_eq!(server_of("mcp__bad name__x"), None);
        assert_eq!(server_of("mcp__solo"), None);
        let _ = std::fs::remove_dir_all(&root);
    }
}
