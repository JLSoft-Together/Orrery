use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};

const KEEP_RUNS: usize = 200;

#[derive(Clone, Debug, PartialEq)]
pub struct RunHeader {
    pub id: String,
    pub at: f64,
    pub source: String,
    pub mode: String,
    pub auto: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunMove {
    pub from: String,
    pub to: String,
    pub size: Option<f64>,
    pub mtime: Option<f64>,
    pub done: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Undo {
    pub at: f64,
    pub restored: f64,
    pub conflicts: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunRecord {
    pub header: RunHeader,
    pub dirs: Vec<String>,
    pub moves: Vec<RunMove>,
    pub backs: Vec<String>,
    pub undo: Option<Undo>,
}

pub fn is_run_id(v: &str) -> bool {
    let Some((a, b)) = v.split_once('-') else { return false };
    (4..=16).contains(&a.len()) && a.bytes().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit()) && b.len() == 8 && b.bytes().all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

fn base36(mut n: u64) -> String {
    if n == 0 {
        return "0".into();
    }
    let mut out = Vec::new();
    while n > 0 {
        let d = (n % 36) as u8;
        out.push(if d < 10 { b'0' + d } else { b'a' + d - 10 });
        n /= 36;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

pub fn hex(n: usize) -> String {
    let mut b = vec![0u8; n];
    if getrandom::fill(&mut b).is_err() {
        let t = crate::disk_map::now_ms();
        for (i, x) in b.iter_mut().enumerate() {
            *x = (t >> ((i % 8) * 8)) as u8 ^ (i as u8).wrapping_mul(31);
        }
    }
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn new_run_id(now: f64) -> String {
    format!("{}-{}", base36(now.max(0.0) as u64), hex(4))
}

fn num(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64).filter(|n| n.is_finite() && *n >= 0.0)
}

fn text(v: Option<&Value>) -> Option<String> {
    v.and_then(Value::as_str).filter(|s| !s.is_empty() && !s.contains('\0')).map(str::to_owned)
}

pub fn parse_run(body: &str) -> Option<RunRecord> {
    let mut header: Option<RunHeader> = None;
    let mut dirs: Vec<String> = Vec::new();
    let mut moves: Vec<RunMove> = Vec::new();
    let mut backs: Vec<String> = Vec::new();
    let mut undo = None;
    for line in body.split('\n') {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(Value::Object(o)) = serde_json::from_str::<Value>(line) else { continue };
        let t = o.get("t").and_then(Value::as_str).unwrap_or("");
        if t == "run" && header.is_none() {
            let id = o.get("id").and_then(Value::as_str).filter(|i| is_run_id(i))?;
            let source = text(o.get("source"))?;
            let at = num(o.get("at"))?;
            header = Some(RunHeader { id: id.to_owned(), at, source, mode: if o.get("mode").and_then(Value::as_str) == Some("month") { "month".into() } else { "type".into() }, auto: o.get("auto") == Some(&Value::Bool(true)) });
            continue;
        }
        header.as_ref()?;
        let from = text(o.get("from"));
        let to = text(o.get("to"));
        match (t, from, to) {
            ("dir", _, _) => {
                if let Some(p) = text(o.get("path")) {
                    if !dirs.contains(&p) {
                        dirs.push(p);
                    }
                }
            }
            ("intent", Some(from), Some(to)) => moves.push(RunMove { from, to, size: None, mtime: None, done: false }),
            ("move", Some(from), Some(to)) => {
                let i = match moves.iter().rposition(|m| m.to == to) {
                    Some(i) => i,
                    None => {
                        moves.push(RunMove { from, to, size: None, mtime: None, done: false });
                        moves.len() - 1
                    }
                };
                let m = &mut moves[i];
                m.done = true;
                m.size = num(o.get("size"));
                m.mtime = num(o.get("mtime"));
            }
            ("fail", _, Some(to)) => {
                if let Some(i) = moves.iter().rposition(|m| m.to == to) {
                    if !moves[i].done {
                        moves.remove(i);
                    }
                }
            }
            ("back", _, Some(to)) => backs.push(to),
            ("undo", _, _) => undo = Some(Undo { at: num(o.get("at")).unwrap_or(0.0), restored: num(o.get("restored")).unwrap_or(0.0), conflicts: num(o.get("conflicts")).unwrap_or(0.0) }),
            _ => {}
        }
    }
    header.map(|header| RunRecord { header, dirs, moves, backs, undo })
}

pub fn summarize(r: &RunRecord) -> Value {
    let done: Vec<&RunMove> = r.moves.iter().filter(|m| m.done).collect();
    let mut o = json!({
        "id": r.header.id,
        "at": r.header.at,
        "source": r.header.source,
        "mode": r.header.mode,
        "auto": r.header.auto,
        "moved": done.len(),
        "bytes": done.iter().map(|m| m.size.unwrap_or(0.0)).sum::<f64>(),
        "undone": r.undo.is_some(),
    });
    if let Some(u) = &r.undo {
        o["undoneAt"] = json!(u.at);
        o["restored"] = json!(u.restored);
        o["conflicts"] = json!(u.conflicts);
    }
    o
}

pub struct RunWriter {
    file: PathBuf,
    header: Option<RunHeader>,
    handle: Option<File>,
}

impl RunWriter {
    fn open(&mut self) -> std::io::Result<&mut File> {
        if self.handle.is_none() {
            if let Some(dir) = self.file.parent() {
                std::fs::create_dir_all(dir)?;
            }
            let mut f = OpenOptions::new().create(true).append(true).open(&self.file)?;
            if let Some(h) = &self.header {
                writeln!(f, "{}", json!({ "t": "run", "id": h.id, "at": h.at, "source": h.source, "mode": h.mode, "auto": h.auto }))?;
            }
            self.handle = Some(f);
        }
        Ok(self.handle.as_mut().expect("handle"))
    }

    pub fn write(&mut self, entry: &Value, sync: bool) -> std::io::Result<()> {
        let f = self.open()?;
        writeln!(f, "{entry}")?;
        if sync {
            let _ = f.sync_data();
        }
        Ok(())
    }
}

pub struct RunStore {
    pub dir: PathBuf,
}

impl RunStore {
    pub fn new(dir: &Path) -> Self {
        RunStore { dir: dir.to_path_buf() }
    }

    pub fn file_of(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.jsonl"))
    }

    pub fn writer(&self, header: RunHeader) -> RunWriter {
        RunWriter { file: self.file_of(&header.id), header: Some(header), handle: None }
    }

    pub fn appender(&self, id: &str) -> RunWriter {
        RunWriter { file: self.file_of(id), header: None, handle: None }
    }

    pub fn read(&self, id: &str) -> Option<RunRecord> {
        if !is_run_id(id) {
            return None;
        }
        parse_run(&std::fs::read_to_string(self.file_of(id)).ok()?)
    }

    pub fn list(&self, limit: usize) -> Vec<Value> {
        let Ok(rd) = std::fs::read_dir(&self.dir) else { return Vec::new() };
        let mut out: Vec<Value> = Vec::new();
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            let Some(id) = name.strip_suffix(".jsonl").filter(|i| is_run_id(i)) else { continue };
            if let Some(r) = self.read(id).filter(|r| !r.moves.is_empty()) {
                out.push(summarize(&r));
            }
        }
        out.sort_by(|a, b| b["at"].as_f64().unwrap_or(0.0).total_cmp(&a["at"].as_f64().unwrap_or(0.0)));
        for old in out.iter().skip(KEEP_RUNS) {
            if let Some(id) = old["id"].as_str() {
                let _ = std::fs::remove_file(self.file_of(id));
            }
        }
        out.truncate(limit);
        out
    }
}
