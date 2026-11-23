pub mod claude;
pub mod codex;
pub mod edits;
pub mod export;
pub mod page;

#[cfg(test)]
mod edits_tests;
#[cfg(test)]
mod tests;

use crate::time::parse_iso;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::sync::Arc;

pub const PAGE: usize = 30;
pub const PAGE_MAX: usize = 50;
pub const REPLY_BYTES: usize = 256 * 1024;
pub const EVENT_BYTES: usize = 64 * 1024;
pub const SCAN_BYTES: u64 = 4 * 1024 * 1024;
pub const BATCH_MS: u64 = 300;
pub const WATCH_MAX: usize = 2;
pub const ME_MAX: usize = 4000;
pub const AI_MAX: usize = 8000;
pub const SYS_MAX: usize = 300;
pub const TOOLS_MAX: usize = 30;
pub const FILES_MAX: usize = 20;
pub const TARGET_MAX: usize = 120;
pub const TOOL_NAME_MAX: usize = 40;
pub const PATH_MAX: usize = 300;
pub const LAST_MAX: usize = 160;
pub const ID_MAX: usize = 64;
pub const PROTO: u64 = 1;
pub const CATS: [&str; 7] = ["edit", "run", "read", "search", "web", "agent", "other"];
const RAW_KEEP: usize = AI_MAX * 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Agent {
    Claude,
    Codex,
}

impl Agent {
    pub fn as_str(self) -> &'static str {
        match self {
            Agent::Claude => "claude",
            Agent::Codex => "codex",
        }
    }

    pub fn parse(s: &str) -> Option<Agent> {
        match s {
            "claude" => Some(Agent::Claude),
            "codex" => Some(Agent::Codex),
            _ => None,
        }
    }
}

pub trait Clean: Send + Sync {
    fn line(&self, text: &str, max: usize) -> String;
    fn block(&self, text: &str, max: usize) -> (String, bool);
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    pub tool: String,
    pub cat: &'static str,
    pub target: Option<String>,
    pub err: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FileRow {
    pub path: String,
    pub add: u64,
    pub del: u64,
    pub out: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Body {
    Me { raw: String },
    Ai { raw: String },
    Tools { sum: [u64; 7], items: Vec<Tool>, more: u64 },
    Files { root: String, files: Vec<FileRow>, more: u64 },
    Sys { kind: &'static str, raw: Option<String> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub id: String,
    pub seq: u64,
    pub rev: u64,
    pub at: Option<i64>,
    pub turn: Option<u64>,
    pub body: Body,
    dirty: bool,
}

pub fn cat_of(tool: &str) -> &'static str {
    match tool {
        "Edit" | "Write" | "MultiEdit" | "NotebookEdit" | "apply_patch" => "edit",
        "Bash" | "PowerShell" | "shell" | "exec_command" | "local_shell" | "local_shell_call" => "run",
        "Read" | "NotebookRead" | "view_image" => "read",
        "Grep" | "Glob" | "LS" | "ToolSearch" => "search",
        "WebFetch" | "WebSearch" | "web_search" => "web",
        "Task" | "Agent" | "spawn_agent" => "agent",
        _ => "other",
    }
}

fn cat_index(cat: &str) -> usize {
    CATS.iter().position(|c| *c == cat).unwrap_or(CATS.len() - 1)
}

pub fn ms_of(at: Option<&str>) -> Option<i64> {
    at.and_then(parse_iso)
}

fn keep_raw(raw: &mut String) {
    if raw.len() > RAW_KEEP * 4 {
        let mut end = RAW_KEEP * 4;
        while !raw.is_char_boundary(end) {
            end -= 1;
        }
        raw.truncate(end);
    }
}

fn clip(s: &str, max: usize) -> String {
    s.chars().take(max).collect()
}

fn norm(p: &str) -> String {
    p.replace('\\', "/")
}

pub fn rel_path(path: &str, root: &str) -> Option<String> {
    let p = norm(path);
    let r = norm(root.trim_end_matches(['/', '\\']));
    if r.is_empty() {
        return None;
    }
    let windows = r.as_bytes().get(1) == Some(&b':') || r.starts_with("//");
    let hit = if windows { p.len() > r.len() && p[..r.len()].eq_ignore_ascii_case(&r) && p.is_char_boundary(r.len()) } else { p.starts_with(&r) };
    if !hit || p.as_bytes().get(r.len()) != Some(&b'/') {
        return if is_relative(&p) { Some(p.trim_start_matches("./").to_owned()) } else { None };
    }
    let rest = p[r.len() + 1..].to_owned();
    (!rest.is_empty()).then_some(rest)
}

fn is_relative(p: &str) -> bool {
    !(p.starts_with('/') || p.starts_with('~') || p.as_bytes().get(1) == Some(&b':'))
}

pub fn line_count(text: &str) -> u64 {
    if text.is_empty() {
        return 0;
    }
    let n = text.split('\n').count() as u64;
    if text.ends_with('\n') {
        n - 1
    } else {
        n
    }
}

pub fn strip_tag_blocks(text: &str, tag: &str) -> String {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find(&open) {
        out.push_str(&rest[..start]);
        match rest[start..].find(&close) {
            Some(end) => rest = &rest[start + end + close.len()..],
            None => {
                rest = "";
                break;
            }
        }
    }
    out.push_str(rest);
    out
}

pub fn last_paragraph(text: &str) -> String {
    let mut kept = Vec::new();
    let mut fenced = false;
    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            fenced = !fenced;
            kept.push("");
            continue;
        }
        kept.push(if fenced { "" } else { line });
    }
    let joined = kept.join("\n");
    let last = joined.split("\n\n").map(str::trim).filter(|p| !p.is_empty()).last().unwrap_or("");
    last.lines().map(plain_line).collect::<Vec<_>>().join("\n")
}

fn plain_line(line: &str) -> String {
    let t = line.trim_start();
    let t = t.trim_start_matches('#').trim_start();
    let t = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")).or_else(|| t.strip_prefix("> ")).unwrap_or(t);
    let mut out = String::with_capacity(t.len());
    let mut rest = t;
    while let Some(i) = rest.find(['[', '*', '_', '`']) {
        out.push_str(&rest[..i]);
        let tail = &rest[i..];
        if tail.starts_with('[') {
            if let Some(close) = tail.find("](") {
                if let Some(end) = tail[close..].find(')') {
                    out.push_str(&tail[1..close]);
                    rest = &tail[close + end + 1..];
                    continue;
                }
            }
            out.push('[');
            rest = &tail[1..];
        } else if tail.starts_with("**") || tail.starts_with("__") {
            rest = &tail[2..];
        } else if tail.starts_with('`') {
            rest = &tail[1..];
        } else {
            let c = &tail[..1];
            let prev = out.chars().last();
            let next = tail[1..].chars().next();
            let word = |c: Option<char>| c.is_some_and(char::is_alphanumeric);
            if !(word(prev) && word(next)) && (word(prev) || word(next)) {
                rest = &tail[1..];
            } else {
                out.push_str(c);
                rest = &tail[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

impl Item {
    fn new(id: String, seq: u64, at: Option<i64>, turn: Option<u64>, body: Body) -> Item {
        Item { id: clip(&id, ID_MAX), seq, rev: 0, at, turn, body, dirty: true }
    }

    fn bump(&mut self) {
        self.rev += 1;
        self.dirty = true;
    }

    pub fn kind(&self) -> &'static str {
        match self.body {
            Body::Me { .. } => "me",
            Body::Ai { .. } => "ai",
            Body::Tools { .. } => "tools",
            Body::Files { .. } => "files",
            Body::Sys { .. } => "sys",
        }
    }

    pub fn to_json(&self, clean: &dyn Clean) -> Value {
        let mut m = Map::new();
        m.insert("id".into(), Value::from(self.id.clone()));
        m.insert("k".into(), Value::from(self.kind()));
        m.insert("seq".into(), Value::from(self.seq));
        m.insert("rev".into(), Value::from(self.rev));
        if let Some(at) = self.at {
            m.insert("at".into(), Value::from(at));
        }
        match &self.body {
            Body::Me { raw } | Body::Ai { raw } => {
                let max = if matches!(self.body, Body::Me { .. }) { ME_MAX } else { AI_MAX };
                let (text, cut) = clean.block(raw, max);
                m.insert("text".into(), Value::from(text));
                if cut {
                    m.insert("cut".into(), Value::Bool(true));
                }
            }
            Body::Tools { sum, items, more } => {
                let mut s = Map::new();
                for (i, n) in sum.iter().enumerate().filter(|(_, n)| **n > 0) {
                    s.insert(CATS[i].into(), Value::from(*n));
                }
                m.insert("sum".into(), Value::Object(s));
                let list: Vec<Value> = items
                    .iter()
                    .map(|t| {
                        let mut o = json!({ "tool": clip(&clean.line(&t.tool, TOOL_NAME_MAX), TOOL_NAME_MAX), "cat": t.cat });
                        if let Some(target) = t.target.as_deref().map(|x| clean.line(x, TARGET_MAX)).filter(|x| !x.is_empty()) {
                            o["target"] = Value::from(target);
                        }
                        if t.err {
                            o["err"] = Value::Bool(true);
                        }
                        o
                    })
                    .collect();
                m.insert("items".into(), Value::Array(list));
                if *more > 0 {
                    m.insert("more".into(), Value::from(*more));
                }
            }
            Body::Files { root, files, more } => {
                m.insert("root".into(), Value::from(root.clone()));
                let list: Vec<Value> = files
                    .iter()
                    .map(|f| {
                        let mut o = json!({ "path": clean.line(&f.path, PATH_MAX) });
                        if f.add > 0 {
                            o["add"] = Value::from(f.add);
                        }
                        if f.del > 0 {
                            o["del"] = Value::from(f.del);
                        }
                        if f.out {
                            o["out"] = Value::Bool(true);
                        }
                        o
                    })
                    .collect();
                m.insert("files".into(), Value::Array(list));
                if *more > 0 {
                    m.insert("more".into(), Value::from(*more));
                }
            }
            Body::Sys { kind, raw } => {
                m.insert("kind".into(), Value::from(*kind));
                if let Some(t) = raw.as_deref().map(|r| clean.line(r, SYS_MAX)).filter(|t| !t.is_empty()) {
                    m.insert("text".into(), Value::from(t));
                }
            }
        }
        Value::Object(m)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Touch {
    pub turn: Option<u64>,
    pub at: Option<i64>,
    pub path: String,
}

pub struct Builder {
    pub agent: Agent,
    pub root: String,
    pub keep_all: bool,
    pub items: Vec<Item>,
    pub turns: Vec<u64>,
    turn: Option<u64>,
    open_tools: Option<usize>,
    open_ai: Option<(usize, String)>,
    tool_ix: HashMap<String, (usize, usize)>,
    files_ix: Option<usize>,
    out: Vec<Item>,
    pub touched: Option<Vec<Touch>>,
}

impl Builder {
    pub fn new(agent: Agent, root: &str, keep_all: bool) -> Builder {
        Builder { agent, root: root.to_owned(), keep_all, items: Vec::new(), turns: Vec::new(), turn: None, open_tools: None, open_ai: None, tool_ix: HashMap::new(), files_ix: None, out: Vec::new(), touched: None }
    }

    pub fn feed(&mut self, line: &[u8], off: u64) {
        if line.iter().all(u8::is_ascii_whitespace) {
            return;
        }
        let rec: Value = match serde_json::from_slice(line) {
            Ok(v) => v,
            Err(_) => match std::str::from_utf8(line) {
                Ok(_) => return,
                Err(_) => match serde_json::from_str(&String::from_utf8_lossy(line)) {
                    Ok(v) => v,
                    Err(_) => return,
                },
            },
        };
        match self.agent {
            Agent::Claude => claude::feed(self, &rec, off),
            Agent::Codex => codex::feed(self, &rec, off),
        }
    }

    pub fn wants(&self, line: &[u8]) -> bool {
        match self.agent {
            Agent::Claude => claude::wants(line),
            Agent::Codex => codex::wants(line),
        }
    }

    fn start_turn(&mut self, off: u64) {
        if !self.keep_all {
            let done = std::mem::take(&mut self.items);
            self.out.extend(done.into_iter().filter(|i| i.dirty));
        }
        self.turn = Some(off);
        self.turns.push(off);
        self.open_tools = None;
        self.open_ai = None;
        self.tool_ix.clear();
        self.files_ix = None;
    }

    fn push(&mut self, item: Item) -> usize {
        self.items.push(item);
        self.items.len() - 1
    }

    fn me(&mut self, off: u64, at: Option<i64>, raw: String) {
        self.start_turn(off);
        let item = Item::new(format!("u:{off}"), off, at, self.turn, Body::Me { raw });
        self.push(item);
    }

    fn sys(&mut self, off: u64, at: Option<i64>, kind: &'static str, raw: Option<String>) {
        self.open_tools = None;
        self.open_ai = None;
        let item = Item::new(format!("s:{off}"), off, at, self.turn, Body::Sys { kind, raw });
        self.push(item);
    }

    fn ai(&mut self, off: u64, at: Option<i64>, key: &str, text: &str) {
        if text.trim().is_empty() {
            return;
        }
        if let Some((ix, k)) = &self.open_ai {
            if k == key {
                let ix = *ix;
                if let Body::Ai { raw } = &mut self.items[ix].body {
                    raw.push_str("\n\n");
                    raw.push_str(text);
                    keep_raw(raw);
                }
                self.items[ix].bump();
                return;
            }
        }
        self.open_tools = None;
        let mut raw = text.to_owned();
        keep_raw(&mut raw);
        let item = Item::new(format!("a:{off}"), off, at, self.turn, Body::Ai { raw });
        let ix = self.push(item);
        self.open_ai = Some((ix, key.to_owned()));
    }

    fn tool(&mut self, off: u64, at: Option<i64>, call_id: Option<&str>, name: &str, target: Option<String>) {
        let cat = cat_of(name);
        let ix = match self.open_tools {
            Some(ix) => ix,
            None => {
                let item = Item::new(format!("t:{off}"), off, at, self.turn, Body::Tools { sum: [0; 7], items: Vec::new(), more: 0 });
                let ix = self.push(item);
                self.open_tools = Some(ix);
                self.open_ai = None;
                ix
            }
        };
        let item = &mut self.items[ix];
        let fresh = item.items_len() == 0 && item.rev == 0 && item.dirty;
        if let Body::Tools { sum, items, more } = &mut item.body {
            sum[cat_index(cat)] += 1;
            if items.len() < TOOLS_MAX {
                if let Some(id) = call_id {
                    self.tool_ix.insert(id.to_owned(), (ix, items.len()));
                }
                items.push(Tool { tool: clip(name, TOOL_NAME_MAX), cat, target, err: false });
            } else {
                *more += 1;
            }
        }
        if !fresh {
            item.bump();
        }
    }

    fn tool_err(&mut self, call_id: &str) {
        let Some((ix, k)) = self.tool_ix.get(call_id).copied() else { return };
        let Some(item) = self.items.get_mut(ix) else { return };
        let changed = match &mut item.body {
            Body::Tools { items, .. } => items.get_mut(k).is_some_and(|t| !std::mem::replace(&mut t.err, true)),
            _ => false,
        };
        if changed {
            item.bump();
        }
    }

    fn file(&mut self, at: Option<i64>, path: &str, add: u64, del: u64) {
        if path.is_empty() {
            return;
        }
        if let Some(t) = self.touched.as_mut() {
            t.push(Touch { turn: self.turn, at, path: norm(path) });
        }
        let (rel, out) = match rel_path(path, &self.root) {
            Some(r) => (r, false),
            None => (norm(path), true),
        };
        let rel = clip(&rel, PATH_MAX);
        let ix = match self.files_ix {
            Some(ix) => ix,
            None => {
                let turn = self.turn.unwrap_or(0);
                let item = Item::new(format!("f:{turn}"), turn + 1, at, self.turn, Body::Files { root: self.root.clone(), files: Vec::new(), more: 0 });
                let ix = self.push(item);
                self.files_ix = Some(ix);
                ix
            }
        };
        let item = &mut self.items[ix];
        let fresh = item.rev == 0 && item.dirty && matches!(&item.body, Body::Files { files, more, .. } if files.is_empty() && *more == 0);
        if let Body::Files { files, more, .. } = &mut item.body {
            if let Some(f) = files.iter_mut().find(|f| f.path == rel) {
                f.add += add;
                f.del += del;
            } else if files.len() < FILES_MAX {
                files.push(FileRow { path: rel, add, del, out });
            } else {
                *more += 1;
            }
        }
        if !fresh {
            item.bump();
        }
    }

    pub fn take_dirty(&mut self) -> Vec<Item> {
        let mut out = std::mem::take(&mut self.out);
        for i in self.items.iter_mut().filter(|i| i.dirty) {
            i.dirty = false;
            out.push(i.clone());
        }
        out
    }

    pub fn mark_clean(&mut self) {
        self.out.clear();
        for i in &mut self.items {
            i.dirty = false;
        }
    }
}

impl Item {
    fn items_len(&self) -> usize {
        match &self.body {
            Body::Tools { items, more, .. } => items.len() + *more as usize,
            _ => 0,
        }
    }
}

pub fn sorted(mut items: Vec<Item>) -> Vec<Item> {
    items.sort_by(|a, b| a.seq.cmp(&b.seq).then_with(|| a.id.cmp(&b.id)));
    items
}

pub type SharedClean = Arc<dyn Clean>;
