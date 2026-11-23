use super::page::page;
use super::{Agent, Clean, PAGE_MAX};
use crate::mask::mask_secrets;
use crate::time::civil_from_days;
use serde_json::Value;
use std::collections::HashMap;
use std::io;
use std::path::Path;

pub const EXPORT_MAX: usize = 2 * 1024 * 1024;
pub const PROMPT_SHORT_MAX: usize = 300;
const PAGES_MAX: usize = 400;
const FILE_ROWS_MAX: usize = 200;
const CUT_NOTE: &str = "> Đã cắt phần đầu của session, chỉ giữ phần mới nhất để file không vượt 2 MB.";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    Full,
    Short,
}

impl Form {
    pub fn parse(s: &str) -> Option<Form> {
        match s {
            "full" => Some(Form::Full),
            "short" => Some(Form::Short),
            _ => None,
        }
    }
}

pub struct Meta<'a> {
    pub agent: Agent,
    pub project: &'a str,
    pub branch: &'a str,
    pub home: &'a str,
    pub platform: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Export {
    pub text: String,
    pub cut: bool,
}

pub fn label(agent: Agent) -> &'static str {
    match agent {
        Agent::Claude => "Claude",
        Agent::Codex => "Codex",
    }
}

fn weight(item: &Value, form: Form) -> usize {
    let me = item["k"] == "me";
    let text = item["text"].as_str().map_or(0, str::len);
    if form == Form::Short && !me {
        return 64;
    }
    text + 64
}

fn collect(path: &Path, agent: Agent, root: &str, form: Form, clean: &dyn Clean) -> io::Result<(Vec<Value>, bool)> {
    let mut chunks: Vec<Vec<Value>> = Vec::new();
    let mut before: Option<u64> = None;
    let mut bytes = 0usize;
    let mut more = true;
    for _ in 0..PAGES_MAX {
        let pg = page(path, agent, root, before, PAGE_MAX, clean, false)?;
        bytes += pg.items.iter().map(|i| weight(i, form)).sum::<usize>();
        chunks.push(pg.items);
        match pg.older {
            Some(o) if before.is_none_or(|b| o < b) => before = Some(o),
            _ => {
                more = false;
                break;
            }
        }
        if bytes > EXPORT_MAX {
            break;
        }
    }
    Ok((chunks.into_iter().rev().flatten().collect(), more))
}

fn stamp(ms: i64) -> String {
    let (y, m, d) = civil_from_days(ms.div_euclid(86_400_000));
    let minutes = ms.rem_euclid(86_400_000) / 60_000;
    format!("{y:04}-{m:02}-{d:02} {:02}:{:02}", minutes / 60, minutes % 60)
}

fn flat(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn clip(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_owned();
    }
    let mut out: String = text.chars().take(max.saturating_sub(3)).collect();
    out.push_str("...");
    out
}

fn tool_line(item: &Value) -> Option<String> {
    let list = item["items"].as_array()?;
    let more = item["more"].as_u64().unwrap_or(0);
    let mut order: Vec<(String, bool, u64)> = Vec::new();
    let mut errs = 0u64;
    for t in list {
        let name = t["tool"].as_str().filter(|n| !n.is_empty()).unwrap_or("tool");
        let files = matches!(t["cat"].as_str(), Some("edit" | "read"));
        if t["err"] == true {
            errs += 1;
        }
        match order.iter_mut().find(|(n, _, _)| n == name) {
            Some(row) => row.2 += 1,
            None => order.push((name.to_owned(), files, 1)),
        }
    }
    let mut parts: Vec<String> = order.iter().map(|(n, files, c)| format!("{n} {c} {}", if *files { "file" } else { "lần" })).collect();
    if more > 0 {
        parts.push(if parts.is_empty() { format!("{more} tool") } else { format!("+{more} khác") });
    }
    if parts.is_empty() {
        return None;
    }
    let mut line = format!("> Tool: {}", parts.join(", "));
    if errs > 0 {
        line.push_str(&format!(" ({errs} lỗi)"));
    }
    Some(line)
}

#[derive(Default)]
struct Files {
    order: Vec<String>,
    rows: HashMap<String, (u64, u64)>,
    more: u64,
}

impl Files {
    fn add(&mut self, item: &Value) {
        self.more += item["more"].as_u64().unwrap_or(0);
        for f in item["files"].as_array().into_iter().flatten() {
            let Some(path) = f["path"].as_str().filter(|p| !p.is_empty()) else { continue };
            let (a, d) = (f["add"].as_u64().unwrap_or(0), f["del"].as_u64().unwrap_or(0));
            match self.rows.get_mut(path) {
                Some(row) => {
                    row.0 += a;
                    row.1 += d;
                }
                None => {
                    self.order.push(path.to_owned());
                    self.rows.insert(path.to_owned(), (a, d));
                }
            }
        }
    }

    fn render(&self) -> String {
        if self.order.is_empty() && self.more == 0 {
            return String::new();
        }
        let mut out = String::from("## File đã sửa\n\n");
        for p in self.order.iter().take(FILE_ROWS_MAX) {
            let (a, d) = self.rows[p];
            out.push_str(&format!("- `{p}`"));
            if a > 0 {
                out.push_str(&format!(" +{a}"));
            }
            if d > 0 {
                out.push_str(&format!(" -{d}"));
            }
            out.push('\n');
        }
        let rest = self.order.len().saturating_sub(FILE_ROWS_MAX) as u64 + self.more;
        if rest > 0 {
            out.push_str(&format!("- +{rest} file nữa\n"));
        }
        out
    }
}

fn header(meta: &Meta, items: &[Value]) -> String {
    let name = label(meta.agent);
    let project = if meta.project.is_empty() { "-" } else { meta.project };
    let mut out = format!("# Session {name}: {project}\n\n- Agent: {name}\n- Dự án: {project}\n");
    if !meta.branch.is_empty() {
        out.push_str(&format!("- Nhánh: {}\n", meta.branch));
    }
    let times: Vec<i64> = items.iter().filter_map(|i| i["at"].as_i64()).collect();
    if let (Some(a), Some(b)) = (times.iter().min(), times.iter().max()) {
        out.push_str(&format!("- Thời gian: {} đến {} (UTC)\n", stamp(*a), stamp(*b)));
    }
    out.push('\n');
    out
}

fn full_body(meta: &Meta, items: &[Value], files: &mut Files) -> String {
    let who = label(meta.agent);
    let mut out = String::new();
    for item in items {
        let text = item["text"].as_str().unwrap_or("").trim();
        match item["k"].as_str().unwrap_or("") {
            "me" if !text.is_empty() => {
                out.push_str(&format!("### Bạn\n\n{text}\n\n"));
                if item["cut"] == true {
                    out.push_str("_(nội dung dài, đã rút gọn)_\n\n");
                }
            }
            "ai" if !text.is_empty() => {
                out.push_str(&format!("### {who}\n\n{text}\n\n"));
                if item["cut"] == true {
                    out.push_str("_(nội dung dài, đã rút gọn)_\n\n");
                }
            }
            "tools" => {
                if let Some(line) = tool_line(item) {
                    out.push_str(&line);
                    out.push_str("\n\n");
                }
            }
            "files" => files.add(item),
            _ => {}
        }
    }
    out
}

fn short_body(items: &[Value], files: &mut Files) -> String {
    let mut out = String::new();
    let mut n = 0;
    for item in items {
        match item["k"].as_str().unwrap_or("") {
            "me" => {
                let text = flat(item["text"].as_str().unwrap_or(""));
                if text.is_empty() {
                    continue;
                }
                if n == 0 {
                    out.push_str("## Yêu cầu\n\n");
                }
                n += 1;
                out.push_str(&format!("{n}. {}\n", clip(&text, PROMPT_SHORT_MAX)));
            }
            "files" => files.add(item),
            _ => {}
        }
    }
    if n > 0 {
        out.push('\n');
    }
    out
}

fn cut_head(body: &str, keep: usize) -> &str {
    if body.len() <= keep {
        return body;
    }
    let mut at = body.len() - keep;
    while !body.is_char_boundary(at) {
        at += 1;
    }
    match body[at..].find("\n### ") {
        Some(i) => &body[at + i + 1..],
        None => &body[at..],
    }
}

pub fn export(path: &Path, root: &str, meta: &Meta, form: Form, clean: &dyn Clean) -> io::Result<Export> {
    let (items, older) = collect(path, meta.agent, root, form, clean)?;
    let mut files = Files::default();
    let (head, body) = match form {
        Form::Full => (header(meta, &items), full_body(meta, &items, &mut files)),
        Form::Short => (String::new(), short_body(&items, &mut files)),
    };
    let tail = files.render();
    let mut cut = older;
    let fixed = head.len() + tail.len() + CUT_NOTE.len() + 4;
    let body = if head.len() + body.len() + tail.len() > EXPORT_MAX {
        cut = true;
        cut_head(&body, EXPORT_MAX.saturating_sub(fixed)).to_owned()
    } else {
        body
    };
    let note = if cut { format!("{CUT_NOTE}\n\n") } else { String::new() };
    let text = format!("{head}{note}{body}{tail}");
    let text = mask_secrets(text.trim_end(), meta.home, meta.platform);
    Ok(Export { text: format!("{text}\n"), cut })
}

pub fn file_name(meta: &Meta, now_ms: i64) -> String {
    let stamp = stamp(now_ms).replace([' ', ':'], "-");
    let project: String = meta.project.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' }).collect();
    let project = project.trim_matches('-');
    let project = if project.is_empty() { "session" } else { project };
    format!("{}-{project}-{stamp}.md", label(meta.agent).to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    struct Plain;

    impl Clean for Plain {
        fn line(&self, text: &str, max: usize) -> String {
            flat(text).chars().take(max).collect()
        }
        fn block(&self, text: &str, max: usize) -> (String, bool) {
            (text.chars().take(max).collect(), text.chars().count() > max)
        }
    }

    const ROOT: &str = "C:\\work\\api";
    const HOME: &str = "C:\\Users\\dauph";
    const TS: &str = "2026-11-20T01:00:00.000Z";
    const KEY: &str = "sk-ant-api03-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    const GH: &str = "ghp_abcdefghijklmnopqrstuvwxyz0123456789";

    fn meta(agent: Agent) -> Meta<'static> {
        Meta { agent, project: "api", branch: "feat/login", home: HOME, platform: "win32" }
    }

    fn file(name: &str, body: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("ccm-export-{}-{name}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("s.jsonl");
        std::fs::write(&p, body).unwrap();
        p
    }

    fn line(v: Value) -> String {
        format!("{v}\n")
    }

    fn prompt(uuid: &str, text: &str) -> String {
        line(json!({ "type": "user", "uuid": uuid, "timestamp": TS, "isSidechain": false, "message": { "role": "user", "content": text } }))
    }

    fn say(id: &str, text: &str) -> String {
        line(json!({ "type": "assistant", "timestamp": TS, "message": { "id": id, "model": "m", "content": [{ "type": "text", "text": text }], "stop_reason": "end_turn" } }))
    }

    fn tool(id: &str, tid: &str, name: &str, input: Value) -> String {
        line(json!({ "type": "assistant", "timestamp": TS, "message": { "id": id, "model": "m", "content": [{ "type": "tool_use", "id": tid, "name": name, "input": input }], "stop_reason": "tool_use" } }))
    }

    fn claude_sample() -> String {
        let mut s = prompt("u1", &format!("Fix login, key {KEY} and {GH}, see C:\\Users\\dauph\\notes.txt"));
        s += &tool("m1", "t1", "Read", json!({ "file_path": "C:\\work\\api\\auth\\google.ts" }));
        s += &tool("m2", "t2", "Bash", json!({ "command": "npm test" }));
        s += &tool("m3", "t3", "Bash", json!({ "command": "npm run lint" }));
        s += &tool("m4", "t4", "Edit", json!({ "file_path": "C:\\work\\api\\auth\\google.ts", "old_string": "a", "new_string": "b\nc" }));
        s += &tool("m5", "t5", "Edit", json!({ "file_path": "C:\\work\\api\\auth\\token.ts", "old_string": "a", "new_string": "b" }));
        s += &say("m6", "Fixed the redirect.");
        s += &prompt("u2", "Now add a test");
        s += &say("m7", "Test added.");
        s
    }

    #[test]
    fn claude_full_export_orders_turns_groups_tools_and_masks() {
        let p = file("claude", &claude_sample());
        let out = export(&p, ROOT, &meta(Agent::Claude), Form::Full, &Plain).unwrap();
        let t = &out.text;
        assert!(!out.cut);
        assert!(t.starts_with("# Session Claude: api\n"), "{t}");
        assert!(t.contains("- Nhánh: feat/login"));
        assert!(t.contains("- Thời gian: 2026-11-20 01:00 đến 2026-11-20 01:00 (UTC)"));
        let order: Vec<usize> = ["### Bạn\n\nFix login", "> Tool: ", "### Claude\n\nFixed the redirect.", "### Bạn\n\nNow add a test", "### Claude\n\nTest added.", "## File đã sửa"].iter().map(|k| t.find(k).unwrap_or_else(|| panic!("{k} missing in {t}"))).collect();
        assert!(order.windows(2).all(|w| w[0] < w[1]), "{order:?} {t}");
        assert!(t.contains("Read 1 file"), "{t}");
        assert!(t.contains("Bash 2 lần"), "{t}");
        assert!(t.contains("Edit 2 file"), "{t}");
        assert!(t.contains("- `auth/google.ts` +2"), "{t}");
        assert!(t.contains("- `auth/token.ts` +1"), "{t}");
        for bad in ["sk-ant-", "ghp_", "dauph", "AAAAAAAAAAAA"] {
            assert!(!t.contains(bad), "{bad} leaked: {t}");
        }
    }

    #[test]
    fn codex_export_uses_codex_label_and_files() {
        let ev = |ty: &str, payload: Value| line(json!({ "timestamp": TS, "type": ty, "payload": payload }));
        let mut s = ev("session_meta", json!({ "id": "x", "cwd": ROOT }));
        s += &ev("response_item", json!({ "type": "message", "role": "user", "content": [{ "type": "input_text", "text": format!("Fix login {GH}") }] }));
        s += &ev("event_msg", json!({ "type": "item_completed", "item": { "type": "CommandExecution", "id": "e1", "command": ["powershell.exe", "-Command", "npm test"], "status": "completed", "exit_code": 0 } }));
        s += &ev("event_msg", json!({ "type": "item_completed", "item": { "type": "FileChange", "id": "e3", "changes": { "C:\\work\\api\\a.ts": { "type": "update", "unified_diff": "@@ -1 +1,2 @@\n-x\n+y\n+z\n" } } } }));
        s += &ev("response_item", json!({ "type": "message", "role": "assistant", "phase": "final_answer", "id": "m2", "content": [{ "type": "output_text", "text": "Fixed it." }] }));
        let p = file("codex", &s);
        let out = export(&p, ROOT, &meta(Agent::Codex), Form::Full, &Plain).unwrap();
        let t = &out.text;
        assert!(t.starts_with("# Session Codex: api\n"), "{t}");
        assert!(t.find("### Bạn").unwrap() < t.find("### Codex\n\nFixed it.").unwrap(), "{t}");
        assert!(t.contains("- `a.ts` +2 -1"), "{t}");
        assert!(!t.contains("ghp_"), "{t}");
    }

    #[test]
    fn short_form_keeps_clipped_prompts_and_files_only() {
        let long = "x".repeat(600);
        let mut s = prompt("u1", &format!("Add login\nwith retry {long}"));
        s += &tool("m1", "t1", "Edit", json!({ "file_path": "C:\\work\\api\\a.ts", "old_string": "a", "new_string": "b" }));
        s += &say("m2", "Fixed the redirect.");
        s += &prompt("u2", "Second ask");
        let p = file("short", &s);
        let out = export(&p, ROOT, &meta(Agent::Claude), Form::Short, &Plain).unwrap();
        let t = &out.text;
        assert!(t.starts_with("## Yêu cầu\n\n1. Add login with retry"), "{t}");
        assert!(t.contains("2. Second ask"));
        assert!(!t.contains("Fixed the redirect"));
        assert!(!t.contains("# Session"));
        assert!(t.contains("## File đã sửa"));
        let first = t.lines().nth(2).unwrap();
        assert!(first.chars().count() <= PROMPT_SHORT_MAX + 3, "{}", first.chars().count());
        assert!(first.ends_with("..."));
    }

    #[test]
    fn a_huge_session_is_cut_at_the_head_under_two_megabytes() {
        let mut s = String::new();
        let body = "lorem ipsum dolor sit amet ".repeat(150);
        for n in 0..900 {
            s += &prompt(&format!("u{n}"), &format!("question {n:04} {body}"));
            s += &say(&format!("m{n}"), &format!("answer {n:04} {body}"));
        }
        let p = file("huge", &s);
        let out = export(&p, ROOT, &meta(Agent::Claude), Form::Full, &Plain).unwrap();
        assert!(out.cut);
        assert!(out.text.len() <= EXPORT_MAX, "{}", out.text.len());
        assert!(out.text.contains("Đã cắt phần đầu"));
        assert!(out.text.contains("answer 0899"));
        assert!(!out.text.contains("question 0000"));
        assert!(out.text.starts_with("# Session Claude"));
    }

    #[test]
    fn empty_session_still_gives_a_header() {
        let p = file("empty", "");
        let out = export(&p, ROOT, &meta(Agent::Claude), Form::Full, &Plain).unwrap();
        assert!(out.text.starts_with("# Session Claude: api"));
        assert!(!out.text.contains("###"));
    }

    #[test]
    fn file_name_is_safe() {
        let m = Meta { agent: Agent::Codex, project: "my app/x", branch: "", home: "", platform: "win32" };
        assert_eq!(file_name(&m, 1_795_000_000_000), "codex-my-app-x-2026-11-18-11-06.md");
    }
}
