use super::templates::{BLOCK_BEGIN, BLOCK_END, CLAUDE_RULES_HEADING, DELEGATE_SKILL, HANDOFF_SKILL, TEMPLATES_VERSION, WORKFLOW_BLOCK};
use crate::{claude, js};
use core_transcript::jsstr::{js_space, js_trim, utf16_cmp};
use regex::Regex;
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

pub const OUTBOX_PATH: &str = "~/.codewhale/notifications/outbox.jsonl";

fn trim_end(s: &str) -> &str {
    s.trim_end_matches(js_space)
}

fn split_lines(text: &str) -> Vec<&str> {
    text.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).collect()
}

pub fn merge_managed_block(existing: Option<&str>, block: &str) -> String {
    let wrapped = format!("{BLOCK_BEGIN}\n{}\n{BLOCK_END}", js_trim(block));
    let Some(existing) = existing.filter(|e| !js_trim(e).is_empty()) else { return format!("{wrapped}\n") };
    if let Some(begin) = existing.find(BLOCK_BEGIN) {
        if let Some(rel) = existing[begin..].find(BLOCK_END) {
            let end = begin + rel;
            if end > begin {
                return format!("{}{wrapped}{}", &existing[..begin], &existing[end + BLOCK_END.len()..]);
            }
        }
    }
    format!("{}\n\n{wrapped}\n", trim_end(existing))
}

fn import_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"(?i)^\s*@(\S+\.md)\s*$").expect("regex"))
}

fn fence_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^\s*(```|~~~)").expect("regex"))
}

pub fn inline_imports(text: &str, read: &dyn Fn(&str) -> Option<String>) -> String {
    let mut fence = false;
    let mut out: Vec<String> = Vec::new();
    for line in split_lines(text) {
        if fence_re().is_match(line) {
            fence = !fence;
        }
        let m = if fence { None } else { import_re().captures(line) };
        let Some(m) = m else {
            out.push(line.to_string());
            continue;
        };
        if let Some(body) = read(&m[1]) {
            out.push(trim_end(&body).to_string());
        }
    }
    out.join("\n")
}

pub fn import_path(reference: &str, root: &str, home: &str) -> Option<String> {
    let abs = if let Some(rest) = reference.strip_prefix("~/") {
        Path::new(home).join(rest)
    } else if claude::is_absolute(reference) {
        PathBuf::from(reference)
    } else {
        Path::new(root).join(reference)
    };
    let abs_s = crate::util::path_str(&crate::util::resolve(&abs.to_string_lossy()));
    let root_r = crate::util::path_str(&crate::util::resolve(root));
    let rel = Path::new(&abs_s).strip_prefix(&root_r).ok()?;
    let rel_s = rel.to_string_lossy();
    (!rel_s.is_empty() && !rel_s.starts_with("..")).then_some(abs_s)
}

pub struct Upsert {
    pub text: String,
    pub changed: bool,
}

pub fn upsert_toml_array_value(text: Option<&str>, key: &str, value: &str) -> Upsert {
    let literal = serde_json::to_string(value).unwrap_or_default();
    let fresh = format!("{key} = [{literal}]");
    let Some(text) = text.filter(|t| !js_trim(t).is_empty()) else { return Upsert { text: format!("{fresh}\n"), changed: true } };
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = split_lines(text).into_iter().map(str::to_string).collect();
    let table_re = Regex::new(r"^\s*\[").expect("regex");
    let first_table = lines.iter().position(|l| table_re.is_match(l));
    let top = first_table.unwrap_or(lines.len());
    let key_re = Regex::new(&format!(r"^\s*{}\s*=", regex::escape(key))).expect("regex");
    if let Some(at) = lines[..top].iter().position(|l| key_re.is_match(l)) {
        let line = lines[at].clone();
        let after = &line[line.find('=').map(|i| i + 1).unwrap_or(0)..];
        let comment = Regex::new(r"\s+#.*$").expect("regex");
        let rhs = js_trim(&comment.replace(after, "")).to_string();
        if rhs.contains(&literal) || rhs.contains(&format!("'{value}'")) {
            return Upsert { text: text.to_string(), changed: false };
        }
        let mut span = 1;
        if rhs.starts_with('[') && !rhs.contains(']') {
            while at + span < lines.len() && !lines[at + span - 1].contains(']') {
                span += 1;
            }
        }
        let one_line = span == 1 && rhs.starts_with('[') && rhs.ends_with(']');
        let inner = if one_line {
            let t = js_trim(&rhs[1..rhs.len() - 1]);
            t.strip_suffix(',').unwrap_or(t).to_string()
        } else {
            String::new()
        };
        let next = if one_line && !inner.is_empty() { format!("{key} = [{inner}, {literal}]") } else { fresh };
        lines.splice(at..at + span, [next]);
        return Upsert { text: lines.join(nl), changed: true };
    }
    match first_table {
        None => Upsert { text: format!("{}{nl}{fresh}{nl}", trim_end(text)), changed: true },
        Some(i) => {
            lines.splice(i..i, [fresh, String::new()]);
            Upsert { text: lines.join(nl), changed: true }
        }
    }
}

pub fn ensure_outbox_path(text: Option<&str>) -> Upsert {
    let entry = format!("path = {}", serde_json::to_string(OUTBOX_PATH).unwrap_or_default());
    let Some(text) = text.filter(|t| !js_trim(t).is_empty()) else { return Upsert { text: format!("[lifecycle_outbox]\n{entry}\n"), changed: true } };
    let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = split_lines(text).into_iter().map(str::to_string).collect();
    let head_re = Regex::new(r"^\[\s*([^\]]+?)\s*\]\s*(#.*)?$").expect("regex");
    let key_re = Regex::new(r#"^([A-Za-z0-9_."-]+?)\s*="#).expect("regex");
    let ws = Regex::new(r"\s+").expect("regex");
    let mut table = String::new();
    let mut header: Option<usize> = None;
    for (i, raw) in lines.iter().enumerate() {
        let line = js_trim(raw);
        if let Some(h) = head_re.captures(line) {
            table = ws.replace_all(&h[1], "").to_string();
            if table == "lifecycle_outbox" {
                header = Some(i);
            }
            continue;
        }
        if let Some(k) = key_re.captures(line) {
            let key = k[1].replace('"', "");
            let full = if table.is_empty() { key } else { format!("{table}.{key}") };
            if full == "lifecycle_outbox.path" {
                return Upsert { text: text.to_string(), changed: false };
            }
        }
    }
    if let Some(h) = header {
        lines.insert(h + 1, entry);
        return Upsert { text: lines.join(nl), changed: true };
    }
    Upsert { text: format!("{}{nl}{nl}[lifecycle_outbox]{nl}{entry}{nl}", trim_end(text)), changed: true }
}

fn server_name_ok(name: &str) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,99}$").expect("regex")).is_match(name)
}

fn str_record(v: Option<&Value>) -> Option<Value> {
    let m = v?.as_object()?;
    let mut out = Map::new();
    for (k, x) in js::entries(m) {
        if let Some(s) = x.as_str() {
            out.insert(k.clone(), Value::String(s.to_string()));
        }
    }
    (!out.is_empty()).then_some(Value::Object(out))
}

pub fn to_codewhale_server(raw: &Value) -> Option<Value> {
    let s = raw.as_object()?;
    if let Some(url) = s.get("url").and_then(Value::as_str) {
        let lower = url.to_lowercase();
        if lower.starts_with("http://") || lower.starts_with("https://") {
            let mut out = Map::new();
            out.insert("url".into(), Value::String(url.to_string()));
            if let Some(h) = str_record(s.get("headers")) {
                out.insert("headers".into(), h);
            }
            return Some(Value::Object(out));
        }
    }
    let command = s.get("command").and_then(Value::as_str).filter(|c| !js_trim(c).is_empty())?;
    let mut out = Map::new();
    out.insert("command".into(), Value::String(command.to_string()));
    let args: Vec<Value> = s.get("args").and_then(Value::as_array).map(|a| a.iter().filter(|x| x.is_string()).cloned().collect()).unwrap_or_default();
    if !args.is_empty() {
        out.insert("args".into(), Value::Array(args));
    }
    if let Some(e) = str_record(s.get("env")) {
        out.insert("env".into(), e);
    }
    Some(Value::Object(out))
}

pub enum McpMerge {
    Merged { text: String, added: Vec<String> },
    Unreadable,
}

fn as_map(v: Option<&Value>) -> Option<&Map<String, Value>> {
    v.and_then(Value::as_object)
}

pub fn merge_mcp(existing: Option<&str>, claude_servers: Option<&Value>) -> McpMerge {
    let mut doc = Map::new();
    if let Some(existing) = existing.filter(|e| !js_trim(e).is_empty()) {
        match serde_json::from_str::<Value>(existing) {
            Ok(Value::Object(m)) => doc = m,
            _ => return McpMerge::Unreadable,
        }
    }
    let key = if as_map(doc.get("servers")).is_some() || as_map(doc.get("mcpServers")).is_none() { "servers" } else { "mcpServers" };
    let mut target: Map<String, Value> = Map::new();
    if let Some(m) = as_map(doc.get(key)) {
        for (k, v) in js::entries(m) {
            target.insert(k.clone(), v.clone());
        }
    }
    let mut known: std::collections::HashSet<String> = std::collections::HashSet::new();
    for k in ["servers", "mcpServers"] {
        if let Some(m) = as_map(doc.get(k)) {
            known.extend(m.keys().map(|n| n.to_lowercase()));
        }
    }
    let mut added = Vec::new();
    if let Some(m) = as_map(claude_servers) {
        for (name, raw) in js::entries(m) {
            let lower = name.to_lowercase();
            if !server_name_ok(name) || lower == "orrery" || known.contains(&lower) {
                continue;
            }
            let Some(server) = to_codewhale_server(raw) else { continue };
            target.insert(name.clone(), server);
            known.insert(lower);
            added.push(name.clone());
        }
    }
    if added.is_empty() {
        return McpMerge::Merged { text: existing.unwrap_or("").to_string(), added };
    }
    let mut next = doc;
    next.insert(key.to_string(), Value::Object(target));
    McpMerge::Merged { text: format!("{}\n", js::stringify_indent(&Value::Object(next), "  ")), added }
}

pub fn is_managed_skill(text: &str) -> bool {
    static FM: OnceLock<Regex> = OnceLock::new();
    static LINE: OnceLock<Regex> = OnceLock::new();
    let fm = FM.get_or_init(|| Regex::new(r"^---\r?\n([\s\S]*?)\r?\n---").expect("regex"));
    let line = LINE.get_or_init(|| Regex::new(r#"(?im)^\s*(x-)?orrery-managed:\s*["']?true["']?\s*$"#).expect("regex"));
    fm.captures(text).is_some_and(|c| line.is_match(&c[1]))
}

pub fn agents_block(rules: Option<&str>) -> String {
    match rules.map(js_trim).filter(|r| !r.is_empty()) {
        Some(r) => format!("{WORKFLOW_BLOCK}\n\n{CLAUDE_RULES_HEADING}\n\n{r}"),
        None => WORKFLOW_BLOCK.to_string(),
    }
}

#[derive(Debug, Clone)]
pub struct SyncPaths {
    pub home: String,
    pub codewhale_home: String,
    pub claude_root: String,
    pub claude_json: String,
    pub codex_home: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Action {
    Create,
    Update,
    Unchanged,
    Skip,
}

impl Action {
    pub fn name(self) -> &'static str {
        match self {
            Action::Create => "create",
            Action::Update => "update",
            Action::Unchanged => "unchanged",
            Action::Skip => "skip",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Planned {
    pub id: &'static str,
    pub file: String,
    pub action: Action,
    pub reason: Option<&'static str>,
    pub content: Option<String>,
    pub private: bool,
}

#[derive(Debug, Clone)]
pub struct SyncPlan {
    pub files: Vec<Planned>,
    pub mcp_added: Vec<String>,
    pub outbox_added: bool,
    pub fingerprint: String,
}

pub fn read_text(file: &str) -> std::io::Result<Option<String>> {
    match std::fs::read(file) {
        Ok(b) => Ok(Some(String::from_utf8_lossy(&b).into_owned())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e),
    }
}

fn action_for(old: Option<&str>, next: &str) -> Action {
    match old {
        None => Action::Create,
        Some(o) if o == next => Action::Unchanged,
        Some(_) => Action::Update,
    }
}

fn join(base: &str, parts: &[&str]) -> String {
    let mut p = PathBuf::from(base);
    for x in parts {
        p.push(x);
    }
    crate::util::path_str(&p)
}

pub fn read_claude_rules(p: &SyncPaths) -> Option<String> {
    let claude_md = read_text(&join(&p.claude_root, &["CLAUDE.md"])).ok().flatten()?;
    let mut imports: HashMap<String, Option<String>> = HashMap::new();
    for line in split_lines(&claude_md) {
        let Some(c) = import_re().captures(line) else { continue };
        let r = c[1].to_string();
        if imports.contains_key(&r) {
            continue;
        }
        let body = import_path(&r, &p.claude_root, &p.home).and_then(|f| read_text(&f).ok().flatten());
        imports.insert(r, body);
    }
    Some(inline_imports(&claude_md, &|r| imports.get(r).cloned().flatten()))
}

pub fn read_claude_servers(p: &SyncPaths) -> Option<Map<String, Value>> {
    let text = read_text(&p.claude_json).ok()?;
    let parsed: Value = serde_json::from_str(text.as_deref().unwrap_or("null")).ok()?;
    match parsed.get("mcpServers") {
        Some(Value::Object(m)) => Some(m.clone()),
        _ => None,
    }
}

pub fn sync_fingerprint(rules: Option<&str>, names: &[String]) -> String {
    let mut sorted: Vec<&String> = names.iter().collect();
    sorted.sort_by(|a, b| utf16_cmp(a, b));
    let joined = sorted.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("\n");
    let mut h = Sha256::new();
    h.update(format!("v{TEMPLATES_VERSION}\0{}\0{joined}", rules.unwrap_or("")).as_bytes());
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

pub fn current_fingerprint(p: &SyncPaths) -> String {
    let rules = read_claude_rules(p);
    let servers = read_claude_servers(p);
    let names: Vec<String> = servers.map(|m| js::keys(&m).into_iter().cloned().collect()).unwrap_or_default();
    sync_fingerprint(rules.as_deref(), &names)
}

#[derive(Debug, Clone)]
pub struct SyncRecord {
    pub at: f64,
    pub hash: String,
}

pub fn read_sync_record(file: &Path) -> Option<SyncRecord> {
    let raw: Value = serde_json::from_slice(&std::fs::read(file).ok()?).ok()?;
    Some(SyncRecord { at: raw.get("at")?.as_f64()?, hash: raw.get("hash")?.as_str()?.to_string() })
}

pub fn sync_state(record: Option<&SyncRecord>, hash: &str) -> &'static str {
    match record {
        None => "never",
        Some(r) if r.hash == hash => "current",
        Some(_) => "stale",
    }
}

pub fn plan_sync(p: &SyncPaths) -> std::io::Result<SyncPlan> {
    let mut files: Vec<Planned> = Vec::new();
    let rules = read_claude_rules(p);
    let servers = read_claude_servers(p);
    let agents_file = join(&p.codewhale_home, &["AGENTS.md"]);
    let agents_old = read_text(&agents_file)?;
    let agents_new = merge_managed_block(agents_old.as_deref(), &agents_block(rules.as_deref()));
    files.push(Planned { id: "agents", file: agents_file, action: action_for(agents_old.as_deref(), &agents_new), reason: None, content: Some(agents_new), private: false });

    let config_file = join(&p.codewhale_home, &["config.toml"]);
    let config_old = read_text(&config_file)?;
    let imports = upsert_toml_array_value(config_old.as_deref(), "project_instruction_imports", "claude");
    let toml = ensure_outbox_path(Some(&imports.text));
    let changed = imports.changed || toml.changed;
    let action = if changed {
        if config_old.is_none() {
            Action::Create
        } else {
            Action::Update
        }
    } else {
        Action::Unchanged
    };
    files.push(Planned { id: "config", file: config_file, action, reason: None, content: Some(toml.text.clone()), private: false });

    let mcp_file = join(&p.codewhale_home, &["mcp.json"]);
    let mcp_old = read_text(&mcp_file)?;
    let server_value = servers.as_ref().map(|m| Value::Object(m.clone()));
    let mut mcp_added: Vec<String> = Vec::new();
    match merge_mcp(mcp_old.as_deref(), server_value.as_ref()) {
        McpMerge::Unreadable => files.push(Planned { id: "mcp", file: mcp_file, action: Action::Skip, reason: Some("unreadable"), content: None, private: false }),
        McpMerge::Merged { added, .. } if added.is_empty() => {
            let skip = mcp_old.is_none();
            files.push(Planned {
                id: "mcp",
                file: mcp_file,
                action: if skip { Action::Skip } else { Action::Unchanged },
                reason: skip.then_some("nothingToAdd"),
                content: None,
                private: false,
            });
        }
        McpMerge::Merged { text, added } => {
            mcp_added = added;
            files.push(Planned { id: "mcp", file: mcp_file, action: if mcp_old.is_none() { Action::Create } else { Action::Update }, reason: None, content: Some(text), private: true });
        }
    }

    let skills: [(&'static str, String, &str); 3] = [
        ("skill-handoff", join(&p.codewhale_home, &["skills", "orrery-handoff", "SKILL.md"]), HANDOFF_SKILL),
        ("skill-delegate-claude", join(&p.claude_root, &["skills", "orrery-delegate", "SKILL.md"]), DELEGATE_SKILL),
        ("skill-delegate-codex", join(&p.codex_home, &["skills", "orrery-delegate", "SKILL.md"]), DELEGATE_SKILL),
    ];
    for (id, file, body) in skills {
        let old = read_text(&file)?;
        if old.as_deref().is_some_and(|o| !is_managed_skill(o)) {
            files.push(Planned { id, file, action: Action::Skip, reason: Some("notManaged"), content: None, private: false });
        } else {
            let action = action_for(old.as_deref(), body);
            files.push(Planned { id, file, action, reason: None, content: Some(body.to_string()), private: false });
        }
    }
    let names: Vec<String> = servers.map(|m| js::keys(&m).into_iter().cloned().collect()).unwrap_or_default();
    Ok(SyncPlan { files, mcp_added, outbox_added: toml.changed, fingerprint: sync_fingerprint(rules.as_deref(), &names) })
}

pub fn write_with_backup(file: &str, content: &str, private: bool) -> std::io::Result<()> {
    let path = Path::new(file);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let backup = format!("{file}.orrery-backup");
    if !Path::new(&backup).exists() {
        match std::fs::copy(file, &backup) {
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }
    let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0);
    let tmp = format!("{file}.{}.{stamp}.tmp", std::process::id());
    write_file(&tmp, content, private)?;
    if let Err(e) = std::fs::rename(&tmp, file) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    Ok(())
}

fn write_file(file: &str, content: &str, private: bool) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mode = if private { 0o600 } else { 0o644 };
        let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(mode).open(file)?;
        f.write_all(content.as_bytes())
    }
    #[cfg(not(unix))]
    {
        let _ = private;
        std::fs::write(file, content)
    }
}

pub fn apply_sync(plan: &SyncPlan, record_file: &Path, at: f64) -> std::io::Result<()> {
    for f in &plan.files {
        if matches!(f.action, Action::Create | Action::Update) {
            if let Some(content) = &f.content {
                write_with_backup(&f.file, content, f.private)?;
            }
        }
    }
    let rec = json!({"at": at, "hash": plan.fingerprint});
    crate::util::write_atomic(record_file, &format!("{}\n", js::stringify(&rec)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn managed_block() {
        let a = merge_managed_block(None, "x");
        assert_eq!(a, "<!-- orrery:begin -->\nx\n<!-- orrery:end -->\n");
        let b = merge_managed_block(Some("keep\n"), "y");
        assert_eq!(b, "keep\n\n<!-- orrery:begin -->\ny\n<!-- orrery:end -->\n");
        let c = merge_managed_block(Some(&b), "z");
        assert_eq!(c, "keep\n\n<!-- orrery:begin -->\nz\n<!-- orrery:end -->\n");
    }

    #[test]
    fn imports_and_paths() {
        let read = |r: &str| (r == "a.md").then(|| "A body\n".to_string());
        assert_eq!(inline_imports("x\n@a.md\n```\n@a.md\n```\n@missing.md\ny", &read), "x\nA body\n```\n@a.md\n```\ny");
        assert!(import_path("../x.md", "/root/c", "/h").is_none());
        assert_eq!(import_path("a.md", "/root/c", "/h"), Some(crate::claude::resolve("/root/c/a.md")));
        assert!(import_path("~/x.md", "/root/c", "/h").is_none());
    }

    #[test]
    fn toml_edits() {
        let t = upsert_toml_array_value(None, "k", "claude");
        assert_eq!(t.text, "k = [\"claude\"]\n");
        let t = upsert_toml_array_value(Some("a = 1\nk = [\"x\"]\n[t]\nb = 2\n"), "k", "claude");
        assert_eq!(t.text, "a = 1\nk = [\"x\", \"claude\"]\n[t]\nb = 2\n");
        let t = upsert_toml_array_value(Some("a = 1\n[t]\nb = 2"), "k", "claude");
        assert_eq!(t.text, "a = 1\nk = [\"claude\"]\n\n[t]\nb = 2");
        assert!(!upsert_toml_array_value(Some("k = [\"claude\"]"), "k", "claude").changed);
        let o = ensure_outbox_path(Some("a = 1\n"));
        assert_eq!(o.text, "a = 1\n\n[lifecycle_outbox]\npath = \"~/.codewhale/notifications/outbox.jsonl\"\n");
        assert!(!ensure_outbox_path(Some("[lifecycle_outbox]\npath = \"x\"\n")).changed);
        assert!(!ensure_outbox_path(Some("lifecycle_outbox.path = \"x\"\n")).changed);
        let o = ensure_outbox_path(Some("[lifecycle_outbox]\nother = 1\n"));
        assert!(o.text.starts_with("[lifecycle_outbox]\npath ="));
    }

    #[test]
    fn mcp_merge_rules() {
        let servers = json!({"figma": {"type": "http", "url": "https://f.example/mcp", "headers": {"a": "b", "n": 1}}, "orrery": {"command": "x"}, "local": {"command": "npx", "args": ["-y", 5, "p"], "env": {"K": "V"}}, "bad name": {"command": "x"}, "none": {}});
        let McpMerge::Merged { text, added } = merge_mcp(None, Some(&servers)) else { panic!() };
        assert_eq!(added, vec!["figma", "local"]);
        let v: Value = serde_json::from_str(&text).unwrap();
        assert_eq!(v["servers"]["figma"], json!({"url": "https://f.example/mcp", "headers": {"a": "b"}}));
        assert_eq!(v["servers"]["local"], json!({"command": "npx", "args": ["-y", "p"], "env": {"K": "V"}}));
        let McpMerge::Merged { added, .. } = merge_mcp(Some(&text), Some(&servers)) else { panic!() };
        assert!(added.is_empty());
        assert!(matches!(merge_mcp(Some("[1]"), Some(&servers)), McpMerge::Unreadable));
    }

    #[test]
    fn managed_skill_and_fingerprint() {
        assert!(is_managed_skill(HANDOFF_SKILL));
        assert!(!is_managed_skill("---\nname: x\n---\n"));
        let a = sync_fingerprint(Some("r"), &["b".into(), "a".into()]);
        assert_eq!(a, sync_fingerprint(Some("r"), &["a".into(), "b".into()]));
        assert_ne!(a, sync_fingerprint(None, &["a".into(), "b".into()]));
        assert_eq!(a.len(), 64);
    }
}
