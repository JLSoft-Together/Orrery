use super::analyze::Snapshot;
use crate::js;
use crate::preview::paths::root_key;
use crate::util::write_atomic;
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use std::path::{Path, PathBuf};

pub const HISTORY_DIR: &str = "apk-history";
const HISTORY_MAX_BYTES: u64 = 64 * 1024 * 1024;

struct Slots {
    last: Snapshot,
    prev: Option<Snapshot>,
}

fn snapshot_of(v: &Value) -> Option<Snapshot> {
    let report = v.get("report").filter(|r| r.is_object())?;
    report.get("path").and_then(Value::as_str)?;
    report.get("groups").and_then(Value::as_array)?;
    let files = v.get("files").and_then(Value::as_array)?;
    let analyzed_at = v.get("analyzedAt").and_then(Value::as_f64)?;
    let files = files
        .iter()
        .map(|f| {
            let a = f.as_array()?;
            Some((a.first()?.as_str()?.to_string(), a.get(1)?.as_u64()?, a.get(2)?.as_u64()?))
        })
        .collect::<Option<Vec<_>>>()?;
    Some(Snapshot { report: report.clone(), analyzed_at, files })
}

fn snapshot_json(s: &Snapshot) -> Value {
    json!({"report": s.report, "analyzedAt": s.analyzed_at, "files": s.files.iter().map(|f| json!([f.0, f.1, f.2])).collect::<Vec<_>>()})
}

fn same_build(a: &Snapshot, b: &Snapshot) -> bool {
    let path = |s: &Snapshot| root_key(s.report.get("path").and_then(Value::as_str).unwrap_or(""));
    path(a) == path(b) && a.report.get("fileSize") == b.report.get("fileSize") && a.report.get("mtime").and_then(Value::as_f64) == b.report.get("mtime").and_then(Value::as_f64)
}

pub struct History {
    dir: PathBuf,
}

impl History {
    pub fn new(data_dir: &Path) -> Self {
        History { dir: data_dir.join(HISTORY_DIR) }
    }

    pub fn file_of(&self, project: &str) -> PathBuf {
        let hex: String = Sha1::digest(root_key(project).as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
        self.dir.join(format!("{hex}.json"))
    }

    fn read(&self, project: &str) -> Option<Slots> {
        let file = self.file_of(project);
        if std::fs::metadata(&file).ok()?.len() > HISTORY_MAX_BYTES {
            return None;
        }
        let v = js::parse(&std::fs::read_to_string(&file).ok()?)?;
        let last = snapshot_of(v.get("last")?)?;
        Some(Slots { last, prev: v.get("prev").and_then(snapshot_of) })
    }

    pub fn record(&self, project: &str, snap: &Snapshot) -> Option<Snapshot> {
        let next = match self.read(project) {
            None => Slots { last: snap.clone(), prev: None },
            Some(cur) if same_build(&cur.last, snap) => Slots { last: Snapshot { analyzed_at: cur.last.analyzed_at, ..snap.clone() }, prev: cur.prev },
            Some(cur) => Slots { last: snap.clone(), prev: Some(cur.last) },
        };
        let file = self.file_of(project);
        let body = json!({"last": snapshot_json(&next.last), "prev": next.prev.as_ref().map(snapshot_json)});
        if std::fs::create_dir_all(&self.dir).is_ok() {
            let _ = write_atomic(&file, &js::stringify(&body));
        }
        next.prev
    }
}
