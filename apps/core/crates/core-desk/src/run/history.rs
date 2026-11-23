use super::settings::{read_json, write_json};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

pub const RUNS_DIR: &str = "runs";
pub const INDEX_FILE: &str = "index.json";
pub const MAX_RUNS: usize = 20;
pub const MAX_TOTAL_BYTES: u64 = 50 * 1024 * 1024;
pub const MAX_RUN_BYTES: u64 = 25 * 1024 * 1024;
pub const MAX_PAGE: usize = 2_000;
const MAX_TEXT: usize = 8_192;

struct Active {
    writer: BufWriter<File>,
    lines: u64,
    bytes: u64,
    truncated: bool,
    dir: PathBuf,
}

pub type Mask = Arc<dyn Fn(&str) -> String + Send + Sync>;

pub struct RunHistory {
    base: PathBuf,
    mask: Mask,
    active: Mutex<HashMap<String, Active>>,
}

pub fn project_hash(root: &str) -> String {
    let d = Sha256::digest(root.as_bytes());
    d[..8].iter().map(|b| format!("{b:02x}")).collect()
}

pub fn safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

fn cut(text: &str) -> &str {
    if text.len() <= MAX_TEXT {
        return text;
    }
    let mut end = MAX_TEXT;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

pub fn exit_code_of(session: &Value) -> Option<i64> {
    match session["failure"].as_object() {
        None => (session["state"] == "stopped").then_some(0),
        Some(f) => {
            if f.get("code").and_then(Value::as_str) == Some("exit") {
                return Some(f.get("detail").and_then(Value::as_str).and_then(|d| d.parse::<i64>().ok()).unwrap_or(1));
            }
            Some(1)
        }
    }
}

fn read_index(dir: &Path) -> Vec<Value> {
    read_json(&dir.join(INDEX_FILE)).and_then(|v| v.as_array().cloned()).unwrap_or_default()
}

fn write_index(dir: &Path, list: &[Value]) {
    let _ = write_json(&dir.join(INDEX_FILE), &Value::Array(list.to_vec()));
}

fn log_path(dir: &Path, id: &str) -> PathBuf {
    dir.join(format!("{id}.log"))
}

fn project_dirs(base: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(base).map(|rd| rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect()).unwrap_or_default()
}

fn remove_entry(dir: &Path, id: &str) {
    let _ = std::fs::remove_file(log_path(dir, id));
}

fn evictable(e: &Value) -> bool {
    e["pinned"] != true && !e["end"].is_null()
}

fn trim_project(dir: &Path, list: &mut Vec<Value>) {
    let mut keep = 0usize;
    let mut drop_ids: Vec<String> = Vec::new();
    let mut order: Vec<usize> = (0..list.len()).collect();
    order.sort_by_key(|i| std::cmp::Reverse(list[*i]["start"].as_i64().unwrap_or(0)));
    for i in order {
        if !evictable(&list[i]) {
            continue;
        }
        keep += 1;
        if keep > MAX_RUNS {
            drop_ids.push(list[i]["id"].as_str().unwrap_or("").to_string());
        }
    }
    for id in &drop_ids {
        remove_entry(dir, id);
    }
    list.retain(|e| !drop_ids.iter().any(|d| e["id"] == d.as_str()));
}

impl RunHistory {
    pub fn new(data_dir: &Path, mask: Mask) -> Self {
        RunHistory { base: data_dir.join(RUNS_DIR), mask, active: Mutex::new(HashMap::new()) }
    }

    fn dir_of(&self, root: &str) -> PathBuf {
        self.base.join(project_hash(root))
    }

    pub fn begin(&self, root: &str, id: &str, config_id: &str, target: &str, device: Option<&str>, start: i64) {
        if !safe_id(id) {
            return;
        }
        let dir = self.dir_of(root);
        if std::fs::create_dir_all(&dir).is_err() {
            return;
        }
        let Ok(file) = File::create(log_path(&dir, id)) else { return };
        let mut list = read_index(&dir);
        list.push(json!({
            "id": id, "project": root, "target": target, "config": config_id, "device": device, "start": start, "end": null,
            "exitCode": null, "lines": 0, "bytes": 0, "pinned": false, "truncated": false,
        }));
        write_index(&dir, &list);
        self.active.lock().unwrap().insert(id.to_string(), Active { writer: BufWriter::new(file), lines: 0, bytes: 0, truncated: false, dir });
    }

    pub fn append(&self, id: &str, stream: &str, text: &str, level: Option<&str>, tag: Option<&str>, at: i64) {
        let masked = (self.mask)(cut(text));
        let mut g = self.active.lock().unwrap();
        let Some(a) = g.get_mut(id) else { return };
        if a.truncated {
            return;
        }
        let mut line = json!({"stream": stream, "text": masked, "at": at});
        if let Some(l) = level {
            line["level"] = json!(l);
        }
        if let Some(t) = tag {
            line["tag"] = json!(t);
        }
        let s = line.to_string();
        let add = s.len() as u64 + 1;
        if a.bytes + add > MAX_RUN_BYTES {
            a.truncated = true;
            return;
        }
        if a.writer.write_all(s.as_bytes()).is_err() || a.writer.write_all(b"\n").is_err() {
            a.truncated = true;
            return;
        }
        a.lines += 1;
        a.bytes += add;
    }

    pub fn finish(&self, id: &str, end: i64, exit_code: Option<i64>) {
        let Some(mut a) = self.active.lock().unwrap().remove(id) else { return };
        let _ = a.writer.flush();
        let mut list = read_index(&a.dir);
        for e in list.iter_mut() {
            if e["id"] == id {
                e["end"] = json!(end);
                e["exitCode"] = json!(exit_code);
                e["lines"] = json!(a.lines);
                e["bytes"] = json!(a.bytes);
                e["truncated"] = json!(a.truncated);
            }
        }
        trim_project(&a.dir, &mut list);
        write_index(&a.dir, &list);
        self.trim_total();
    }

    fn trim_total(&self) {
        let mut all: Vec<(PathBuf, String, i64, u64)> = Vec::new();
        let mut total = 0u64;
        for dir in project_dirs(&self.base) {
            for e in read_index(&dir) {
                let bytes = e["bytes"].as_u64().unwrap_or(0);
                total += bytes;
                if evictable(&e) {
                    all.push((dir.clone(), e["id"].as_str().unwrap_or("").to_string(), e["start"].as_i64().unwrap_or(0), bytes));
                }
            }
        }
        if total <= MAX_TOTAL_BYTES {
            return;
        }
        all.sort_by_key(|e| e.2);
        for (dir, id, _, bytes) in all {
            if total <= MAX_TOTAL_BYTES {
                break;
            }
            remove_entry(&dir, &id);
            let mut list = read_index(&dir);
            list.retain(|e| e["id"] != id.as_str());
            write_index(&dir, &list);
            total = total.saturating_sub(bytes);
        }
    }

    fn flush_active(&self, id: &str) -> Option<(u64, bool)> {
        let mut g = self.active.lock().unwrap();
        let a = g.get_mut(id)?;
        let _ = a.writer.flush();
        Some((a.lines, a.truncated))
    }

    pub fn list(&self, root: &str) -> Vec<Value> {
        let dir = self.dir_of(root);
        let mut list = read_index(&dir);
        let mut changed = false;
        {
            let g = self.active.lock().unwrap();
            for e in list.iter_mut() {
                let id = e["id"].as_str().unwrap_or("").to_string();
                match g.get(&id) {
                    Some(a) => {
                        e["lines"] = json!(a.lines);
                        e["bytes"] = json!(a.bytes);
                        e["truncated"] = json!(a.truncated);
                    }
                    None if e["end"].is_null() => {
                        e["end"] = e["start"].clone();
                        e["interrupted"] = json!(true);
                        changed = true;
                    }
                    None => {}
                }
            }
        }
        if changed {
            write_index(&dir, &list);
        }
        list.sort_by_key(|e| std::cmp::Reverse(e["start"].as_i64().unwrap_or(0)));
        list
    }

    pub fn read(&self, root: &str, id: &str, offset: usize, limit: usize) -> Option<Value> {
        if !safe_id(id) {
            return None;
        }
        let dir = self.dir_of(root);
        let entry = read_index(&dir).into_iter().find(|e| e["id"] == id)?;
        let live = self.flush_active(id);
        let total = live.map(|l| l.0).unwrap_or_else(|| entry["lines"].as_u64().unwrap_or(0));
        let limit = limit.clamp(1, MAX_PAGE);
        let mut reader = BufReader::new(File::open(log_path(&dir, id)).ok()?);
        let mut buf = Vec::new();
        let mut skipped = 0usize;
        while skipped < offset {
            buf.clear();
            if reader.read_until(b'\n', &mut buf).ok()? == 0 {
                break;
            }
            skipped += 1;
        }
        let mut lines: Vec<Value> = Vec::with_capacity(limit);
        while lines.len() < limit {
            buf.clear();
            if reader.read_until(b'\n', &mut buf).ok()? == 0 {
                break;
            }
            if buf.last() != Some(&b'\n') {
                break;
            }
            if let Ok(v) = serde_json::from_slice::<Value>(&buf[..buf.len() - 1]) {
                lines.push(v);
            }
        }
        Some(json!({"id": id, "offset": offset, "total": total, "lines": lines}))
    }

    pub fn pin(&self, root: &str, id: &str, pinned: bool) -> bool {
        if !safe_id(id) {
            return false;
        }
        let dir = self.dir_of(root);
        let mut list = read_index(&dir);
        let mut hit = false;
        for e in list.iter_mut() {
            if e["id"] == id {
                e["pinned"] = json!(pinned);
                hit = true;
            }
        }
        if hit {
            if !pinned {
                trim_project(&dir, &mut list);
            }
            write_index(&dir, &list);
        }
        hit
    }

    pub fn delete(&self, root: &str, id: &str) -> bool {
        if !safe_id(id) || self.active.lock().unwrap().contains_key(id) {
            return false;
        }
        let dir = self.dir_of(root);
        let mut list = read_index(&dir);
        let before = list.len();
        list.retain(|e| e["id"] != id);
        if list.len() == before {
            return false;
        }
        remove_entry(&dir, id);
        write_index(&dir, &list);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ccm-hist-{tag}-{}-{}", std::process::id(), crate::util::uuid()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn plain() -> Mask {
        Arc::new(|t: &str| t.to_string())
    }

    fn run(h: &RunHistory, root: &str, id: &str, start: i64, lines: usize, code: i64) {
        h.begin(root, id, "cfg", "app", None, start);
        for i in 0..lines {
            h.append(id, "stdout", &format!("line {i}"), None, None, start + i as i64);
        }
        h.finish(id, start + 10, Some(code));
    }

    #[test]
    fn save_list_and_page() {
        let h = RunHistory::new(&tmp("a"), plain());
        run(&h, "/p", "r1", 100, 5, 0);
        let list = h.list("/p");
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["lines"], 5);
        assert_eq!(list[0]["exitCode"], 0);
        let page = h.read("/p", "r1", 2, 2).unwrap();
        assert_eq!(page["total"], 5);
        let texts: Vec<&str> = page["lines"].as_array().unwrap().iter().map(|l| l["text"].as_str().unwrap()).collect();
        assert_eq!(texts, ["line 2", "line 3"]);
        assert!(h.read("/p", "../x", 0, 10).is_none());
        assert!(h.list("/other").is_empty());
    }

    #[test]
    fn page_big_log_without_loading_all() {
        let h = RunHistory::new(&tmp("big"), plain());
        run(&h, "/p", "big", 1, 100_000, 0);
        let page = h.read("/p", "big", 99_990, MAX_PAGE).unwrap();
        assert_eq!(page["total"], 100_000);
        assert_eq!(page["lines"].as_array().unwrap().len(), 10);
        assert_eq!(page["lines"][9]["text"], "line 99999");
    }

    #[test]
    fn rotation_keeps_twenty_and_pinned() {
        let h = RunHistory::new(&tmp("rot"), plain());
        run(&h, "/p", "pin", 1, 1, 0);
        assert!(h.pin("/p", "pin", true));
        for i in 0..25 {
            run(&h, "/p", &format!("r{i}"), 100 + i, 1, 0);
        }
        let list = h.list("/p");
        let ids: Vec<&str> = list.iter().map(|e| e["id"].as_str().unwrap()).collect();
        assert_eq!(list.len(), 21);
        assert!(ids.contains(&"pin") && ids.contains(&"r24") && !ids.contains(&"r4") && ids.contains(&"r5"));
        assert!(h.read("/p", "r0", 0, 10).is_none());
        assert!(h.read("/p", "pin", 0, 10).is_some());
        assert!(h.pin("/p", "pin", false));
        assert_eq!(h.list("/p").len(), 20);
    }

    #[test]
    fn total_size_capped_oldest_first_pinned_kept() {
        let base = tmp("size");
        let h = RunHistory::new(&base, plain());
        let big = "x".repeat(4000);
        let mk = |id: &str, start: i64, mb_lines: usize| {
            h.begin("/p", id, "c", "t", None, start);
            for _ in 0..mb_lines {
                h.append(id, "stdout", &big, None, None, start);
            }
            h.finish(id, start + 1, Some(0));
        };
        mk("old", 1, 6000);
        assert!(h.pin("/p", "old", true));
        for i in 0..6 {
            mk(&format!("n{i}"), 10 + i, 6000);
        }
        let list = h.list("/p");
        let total: u64 = list.iter().map(|e| e["bytes"].as_u64().unwrap()).sum();
        assert!(total <= MAX_TOTAL_BYTES, "{total}");
        let ids: Vec<&str> = list.iter().map(|e| e["id"].as_str().unwrap()).collect();
        assert!(ids.contains(&"old") && ids.contains(&"n5") && !ids.contains(&"n0"));
    }

    #[test]
    fn single_run_capped_and_marked() {
        let h = RunHistory::new(&tmp("cap"), plain());
        let big = "y".repeat(8000);
        h.begin("/p", "c1", "c", "t", None, 1);
        for _ in 0..4000 {
            h.append("c1", "stdout", &big, None, None, 1);
        }
        h.finish("c1", 2, Some(0));
        let e = &h.list("/p")[0];
        assert_eq!(e["truncated"], true);
        assert!(e["bytes"].as_u64().unwrap() <= MAX_RUN_BYTES);
    }

    #[test]
    fn delete_and_interrupted() {
        let base = tmp("del");
        let h = RunHistory::new(&base, plain());
        run(&h, "/p", "d1", 1, 2, 1);
        h.begin("/p", "live", "c", "t", None, 50);
        h.append("live", "stdout", "hi", None, None, 50);
        assert!(!h.delete("/p", "live"));
        assert_eq!(h.read("/p", "live", 0, 10).unwrap()["lines"].as_array().unwrap().len(), 1);
        assert!(h.delete("/p", "d1") && !h.delete("/p", "d1"));
        let h2 = RunHistory::new(&base, plain());
        let list = h2.list("/p");
        assert_eq!(list[0]["interrupted"], true);
        assert!(!list[0]["end"].is_null());
    }

    #[test]
    fn secrets_masked_on_disk() {
        let base = tmp("sec");
        let h = RunHistory::new(&base, Arc::new(|t: &str| core_sources::mask::mask_secrets(t, "/home/me", "linux")));
        h.begin("/p", "s1", "c", "t", None, 1);
        h.append("s1", "stdout", "node vite --token=abcd1234secret", None, None, 1);
        h.finish("s1", 2, Some(0));
        let raw = std::fs::read_to_string(base.join(RUNS_DIR).join(project_hash("/p")).join("s1.log")).unwrap();
        assert!(!raw.contains("abcd1234secret") && raw.contains("[masked]"));
    }

    #[test]
    fn exit_codes() {
        assert_eq!(exit_code_of(&json!({"state": "stopped", "failure": null})), Some(0));
        assert_eq!(exit_code_of(&json!({"state": "failed", "failure": {"code": "exit", "detail": "3"}})), Some(3));
        assert_eq!(exit_code_of(&json!({"state": "failed", "failure": {"code": "build"}})), Some(1));
    }
}
