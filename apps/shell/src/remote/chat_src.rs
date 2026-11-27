use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use core_transcript::chat::page::{self, Live};
use core_transcript::chat::{last_paragraph, Agent, Clean, EVENT_BYTES, ID_MAX, LAST_MAX};
use core_transcript::tracker::find_transcript;
use serde_json::{json, Value};

use super::pair::is_hidden;
use super::push_content::{clean, cut, mask_text};
use crate::state::lock;

const ROLLOUT_CACHE: usize = 64;
const EVENT_MARGIN: usize = 1024;
const PAGE_CACHE: usize = 16;
const PAGE_CACHE_BYTES: usize = 4 * 1024 * 1024;

type PageKey = (PathBuf, u64, Option<u64>, usize);

pub struct ShellClean;

impl Clean for ShellClean {
    fn line(&self, text: &str, max: usize) -> String {
        clean(text, &mask_text, max)
    }

    fn block(&self, text: &str, max: usize) -> (String, bool) {
        block_text(text, &mask_text, max)
    }
}

pub fn block_text(text: &str, mask: &dyn Fn(&str) -> String, max: usize) -> (String, bool) {
    let kept: String = text.chars().filter(|c| matches!(c, '\n' | '\t') || !(c.is_control() || is_hidden(*c))).collect();
    let masked = mask(&kept);
    if masked.chars().count() > max {
        (cut(&masked, max), true)
    } else {
        (masked, false)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    pub agent: Agent,
    pub terminal: Option<String>,
    pub cwd: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Source {
    pub path: PathBuf,
    pub id: String,
    pub root: String,
}

pub struct Roots {
    pub projects: PathBuf,
    pub sessions: PathBuf,
}

impl Roots {
    pub fn from_env() -> Roots {
        let home = PathBuf::from(std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).unwrap_or_default());
        let claude = std::env::var_os("CLAUDE_CONFIG_DIR").map_or_else(|| home.join(".claude"), PathBuf::from);
        let codex = std::env::var_os("CODEX_HOME").map_or_else(|| home.join(".codex"), PathBuf::from);
        Roots { projects: claude.join("projects"), sessions: codex.join("sessions") }
    }
}

fn pane_of<'a>(summary: &'a Value, key: &str) -> Option<&'a Value> {
    summary["tabs"].as_array()?.iter().flat_map(|t| t["panes"].as_array().into_iter().flatten()).find(|p| p["key"].as_str() == Some(key))
}

pub fn target_of(summary: &Value, key: &str) -> Option<Target> {
    let p = pane_of(summary, key)?;
    if p["kind"].as_str().is_some_and(|k| k != "term") {
        return None;
    }
    let agent = Agent::parse(p["agent"].as_str()?)?;
    Some(Target { agent, terminal: p["terminalId"].as_str().filter(|t| !t.is_empty()).map(str::to_owned), cwd: p["cwd"].as_str().unwrap_or("").to_owned() })
}

pub fn agent_panes(summary: &Value) -> Vec<(String, Target)> {
    let panes = summary["tabs"].as_array().into_iter().flatten().flat_map(|t| t["panes"].as_array().into_iter().flatten());
    panes.filter_map(|p| p["key"].as_str().and_then(|k| target_of(summary, k).map(|t| (k.to_owned(), t)))).collect()
}

fn id_ok(id: &str) -> bool {
    (1..=100).contains(&id.len()) && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

fn sorted_desc(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    names.sort_unstable_by(|a, b| b.cmp(a));
    names
}

pub fn find_rollout(sessions: &Path, id: &str) -> Option<PathBuf> {
    let suffix = format!("-{id}.jsonl");
    for year in sorted_desc(sessions) {
        let ydir = sessions.join(&year);
        for month in sorted_desc(&ydir) {
            let mdir = ydir.join(&month);
            for day in sorted_desc(&mdir) {
                let dir = mdir.join(&day);
                if let Some(hit) = sorted_desc(&dir).into_iter().find(|n| n.ends_with(&suffix) && n.starts_with("rollout-")) {
                    return Some(dir.join(hit));
                }
            }
        }
    }
    None
}

#[derive(Default)]
pub struct Hub {
    gens: Mutex<HashMap<PathBuf, (u64, u64)>>,
    rollouts: Mutex<HashMap<String, PathBuf>>,
    lasts: Mutex<HashMap<PathBuf, (u64, Option<(String, Option<i64>)>)>>,
    pages: Mutex<VecDeque<(PageKey, usize, Value)>>,
}

impl Hub {
    pub fn source_of(&self, t: &Target, terminal: Option<&Value>, roots: &Roots) -> Option<Source> {
        let term = terminal.unwrap_or(&Value::Null);
        let root = term["cwd"].as_str().filter(|c| !c.is_empty()).unwrap_or(&t.cwd).to_owned();
        match t.agent {
            Agent::Claude => {
                let id = term["claudeSessionId"].as_str().filter(|i| id_ok(i))?;
                let path = find_transcript(&roots.projects, id, Some(&root))?;
                Some(Source { path, id: id.to_owned(), root })
            }
            Agent::Codex => {
                let id = term["codexSessionId"].as_str().or_else(|| t.terminal.as_deref().and_then(|x| x.strip_prefix("codex:"))).filter(|i| id_ok(i))?;
                let path = self.rollout(&roots.sessions, id)?;
                Some(Source { path, id: id.to_owned(), root })
            }
        }
    }

    fn rollout(&self, sessions: &Path, id: &str) -> Option<PathBuf> {
        if let Some(p) = lock(&self.rollouts).get(id).filter(|p| p.exists()) {
            return Some(p.clone());
        }
        let found = find_rollout(sessions, id)?;
        let mut map = lock(&self.rollouts);
        if map.len() >= ROLLOUT_CACHE {
            map.clear();
        }
        map.insert(id.to_owned(), found.clone());
        Some(found)
    }

    pub fn gen(&self, src: &Source) -> String {
        let size = std::fs::metadata(&src.path).map_or(0, |m| m.len());
        let mut map = lock(&self.gens);
        let entry = map.entry(src.path.clone()).or_insert((0, size));
        if size < entry.1 {
            entry.0 += 1;
        }
        entry.1 = size;
        gen_name(&src.id, entry.0)
    }

    pub fn bump(&self, src: &Source) -> String {
        let size = std::fs::metadata(&src.path).map_or(0, |m| m.len());
        let mut map = lock(&self.gens);
        let entry = map.entry(src.path.clone()).or_insert((0, size));
        entry.0 += 1;
        entry.1 = size;
        gen_name(&src.id, entry.0)
    }

    pub fn cached(&self, key: &PageKey) -> Option<Value> {
        lock(&self.pages).iter().find(|(k, _, _)| k == key).map(|(_, _, v)| v.clone())
    }

    pub fn keep(&self, key: PageKey, v: &Value) {
        let n = v.to_string().len();
        if n > PAGE_CACHE_BYTES / 4 {
            return;
        }
        let mut q = lock(&self.pages);
        q.retain(|(k, _, _)| k.0 != key.0 || k.1 == key.1);
        q.push_back((key, n, v.clone()));
        while q.len() > PAGE_CACHE || q.iter().map(|e| e.1).sum::<usize>() > PAGE_CACHE_BYTES {
            q.pop_front();
        }
    }

    pub fn last(&self, src: &Source, agent: Agent) -> Option<(String, Option<i64>)> {
        let size = std::fs::metadata(&src.path).ok()?.len();
        if let Some((s, v)) = lock(&self.lasts).get(&src.path) {
            if *s == size {
                return v.clone();
            }
        }
        let got = page::last_reply(&src.path, agent).ok().flatten().and_then(|(text, at)| {
            let line = ShellClean.line(&last_paragraph(&text), LAST_MAX);
            (!line.is_empty()).then_some((line, at))
        });
        let mut map = lock(&self.lasts);
        if map.len() >= ROLLOUT_CACHE {
            map.clear();
        }
        map.insert(src.path.clone(), (size, got.clone()));
        got
    }
}

pub fn gen_name(id: &str, n: u64) -> String {
    let tail = format!(":{n}");
    let keep = ID_MAX.saturating_sub(tail.len());
    format!("{}{tail}", &id[..id.len().min(keep)])
}

pub struct Paged {
    pub reply: Value,
    pub gen: String,
    pub live: Option<Live>,
    pub source: Option<Source>,
}

pub struct Ask<'a> {
    pub pane: &'a str,
    pub target: &'a Target,
    pub gen: Option<&'a str>,
    pub before: Option<u64>,
    pub limit: usize,
    pub live: bool,
}

pub fn page_of(hub: &Hub, a: &Ask, source: Option<Source>) -> Result<Paged, &'static str> {
    let agent = a.target.agent.as_str();
    let Some(src) = source else {
        return Ok(Paged { reply: json!({ "ok": true, "paneKey": a.pane, "agent": agent, "gen": "", "items": [], "older": null }), gen: String::new(), live: None, source: None });
    };
    let gen = hub.gen(&src);
    let before = if a.gen.is_some_and(|g| g != gen) { None } else { a.before };
    let size = std::fs::metadata(&src.path).map_or(0, |m| m.len());
    let key = (src.path.clone(), size, before, a.limit);
    let live = a.live && before.is_none();
    if !live {
        if let Some(mut reply) = hub.cached(&key).filter(|r| r["gen"] == gen.as_str()) {
            reply["paneKey"] = Value::from(a.pane);
            return Ok(Paged { reply, gen, live: None, source: Some(src) });
        }
    }
    let pg = page::page(&src.path, a.target.agent, &src.root, before, a.limit, &ShellClean, live).map_err(|_| "internal")?;
    let reply = json!({ "ok": true, "paneKey": a.pane, "agent": agent, "gen": gen, "items": pg.items, "older": pg.older });
    hub.keep(key, &reply);
    Ok(Paged { reply, gen, live: pg.live, source: Some(src) })
}

fn shrink(item: &mut Value) {
    let Some(text) = item["text"].as_str() else {
        if let Some(list) = item["items"].as_array_mut() {
            let gone = list.len() / 2;
            list.truncate(list.len() - gone);
            let more = item["more"].as_u64().unwrap_or(0) + gone as u64;
            item["more"] = Value::from(more);
        }
        return;
    };
    let n = text.chars().count();
    item["text"] = Value::from(cut(text, (n / 2).max(1)));
    item["cut"] = Value::Bool(true);
}

fn size_of(v: &Value) -> usize {
    serde_json::to_vec(v).map_or(usize::MAX, |b| b.len() + 1)
}

pub fn items_events(pane: &str, gen: &str, items: Vec<Value>) -> Vec<Value> {
    let budget = EVENT_BYTES - EVENT_MARGIN;
    let mut out = Vec::new();
    let mut batch: Vec<Value> = Vec::new();
    let mut bytes = 0usize;
    for mut item in items {
        let mut guard = 0;
        while size_of(&item) > budget && guard < 16 {
            shrink(&mut item);
            guard += 1;
        }
        let n = size_of(&item);
        if bytes + n > budget && !batch.is_empty() {
            out.push(json!({ "t": "chat.items", "paneKey": pane, "gen": gen, "items": std::mem::take(&mut batch) }));
            bytes = 0;
        }
        bytes += n;
        batch.push(item);
    }
    if !batch.is_empty() {
        out.push(json!({ "t": "chat.items", "paneKey": pane, "gen": gen, "items": batch }));
    }
    out
}

pub type LastRow = (String, Option<String>, Option<i64>);

pub fn last_event(full: bool, rows: &[LastRow], sent: &HashMap<String, String>) -> Option<Value> {
    let item = |(k, text, at): &LastRow| {
        let mut o = json!({ "paneKey": k, "text": text });
        if let (Some(at), Some(_)) = (at, text) {
            o["at"] = Value::from(*at);
        }
        o
    };
    if full {
        let items: Vec<Value> = rows.iter().filter(|r| r.1.is_some()).map(item).collect();
        return Some(json!({ "t": "chat.last", "full": true, "items": items }));
    }
    let mut items: Vec<Value> = rows.iter().filter(|r| r.1.as_ref() != sent.get(&r.0)).filter(|r| r.1.is_some() || sent.contains_key(&r.0)).map(item).collect();
    for k in sent.keys().filter(|k| !rows.iter().any(|r| &r.0 == *k)) {
        items.push(json!({ "paneKey": k, "text": null }));
    }
    (!items.is_empty()).then(|| json!({ "t": "chat.last", "items": items }))
}

pub fn remember(rows: &[LastRow]) -> HashMap<String, String> {
    rows.iter().filter_map(|(k, t, _)| t.clone().map(|t| (k.clone(), t))).collect()
}

pub fn mask_menus(summary: &mut Value) {
    let Some(tabs) = summary.get_mut("tabs").and_then(Value::as_array_mut) else { return };
    for p in tabs.iter_mut().filter_map(|t| t.get_mut("panes").and_then(Value::as_array_mut)).flatten() {
        let Some(obj) = p.as_object_mut() else { continue };
        if let Some(menu) = obj.get_mut("menu") {
            if !fix_menu(menu) {
                obj.remove("menu");
            }
        }
        fix_pane_extras(obj);
    }
}

const MENU_ITEMS_MAX: usize = 12;
const MENU_LABEL_MAX: usize = 120;
const MENU_QUESTION_MAX: usize = 300;
const MENU_SIG_MAX: usize = 16;

fn fix_pane_extras(obj: &mut serde_json::Map<String, Value>) {
    if let Some(q) = obj.remove("question") {
        let text = clean(q.as_str().unwrap_or(""), &mask_text, MENU_QUESTION_MAX);
        if !text.is_empty() && !obj.contains_key("menu") {
            obj.insert("question".into(), Value::from(text));
        }
    }
    if obj.get("snoozed").is_some_and(|s| s != &Value::Bool(true)) {
        obj.remove("snoozed");
    }
}

fn fix_menu(menu: &mut Value) -> bool {
    let sig_ok = menu["sig"].as_str().is_some_and(|s| (1..=MENU_SIG_MAX).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_alphanumeric()));
    let Some(items) = menu.get_mut("items").and_then(Value::as_array_mut) else { return false };
    if !sig_ok || items.is_empty() {
        return false;
    }
    items.truncate(MENU_ITEMS_MAX);
    for it in items.iter_mut() {
        let label = it["label"].as_str().unwrap_or("").to_owned();
        it["label"] = Value::from(clean(&label, &mask_text, MENU_LABEL_MAX));
    }
    let q = menu["question"].as_str().unwrap_or("").to_owned();
    menu["question"] = Value::from(clean(&q, &mask_text, MENU_QUESTION_MAX));
    true
}
