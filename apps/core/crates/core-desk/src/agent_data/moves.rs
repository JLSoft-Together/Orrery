use super::rules::Agent;
use super::tree::link_target;
use crate::js;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

pub const PHASES: [&str; 6] = ["copying", "copied", "switching", "linked", "undoing", "interrupted"];

#[derive(Debug, Clone, PartialEq)]
pub struct MoveRecord {
    pub agent: Agent,
    pub source: String,
    pub target: String,
    pub backup: String,
    pub at: f64,
    pub phase: &'static str,
    pub original_deleted: bool,
}

impl MoveRecord {
    pub fn to_value(&self) -> Value {
        json!({
            "agent": self.agent.as_str(),
            "source": self.source,
            "target": self.target,
            "backup": self.backup,
            "at": self.at,
            "phase": self.phase,
            "originalDeleted": self.original_deleted,
        })
    }

    pub fn with_phase(&self, phase: &'static str) -> MoveRecord {
        MoveRecord { phase, ..self.clone() }
    }
}

fn parse_record(raw: &Value) -> Option<MoveRecord> {
    let r = raw.as_object()?;
    let agent = Agent::parse(r.get("agent")?)?;
    let text = |k: &str| r.get(k).and_then(Value::as_str).map(str::to_string);
    let (source, target, backup) = (text("source")?, text("target")?, text("backup")?);
    let at = r.get("at")?.as_f64()?;
    let phase = r.get("phase").and_then(Value::as_str).and_then(|p| PHASES.iter().find(|x| **x == p).copied())?;
    Some(MoveRecord { agent, source, target, backup, at, phase, original_deleted: r.get("originalDeleted") == Some(&Value::Bool(true)) })
}

pub struct MoveStore {
    file: PathBuf,
    moves: Vec<MoveRecord>,
}

impl MoveStore {
    pub fn new(file: PathBuf) -> Self {
        MoveStore { file, moves: Vec::new() }
    }

    pub fn load(&mut self) {
        self.moves.clear();
        let Some(data) = fs::read_to_string(&self.file).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) else { return };
        let Some(list) = data.get("moves").and_then(Value::as_array) else { return };
        for raw in list {
            if let Some(rec) = parse_record(raw) {
                self.put(rec);
            }
        }
    }

    fn put(&mut self, rec: MoveRecord) {
        match self.moves.iter_mut().find(|m| m.agent == rec.agent) {
            Some(slot) => *slot = rec,
            None => self.moves.push(rec),
        }
    }

    pub fn get(&self, agent: Agent) -> Option<MoveRecord> {
        self.moves.iter().find(|m| m.agent == agent).cloned()
    }

    pub fn all(&self) -> Vec<MoveRecord> {
        self.moves.clone()
    }

    pub fn set(&mut self, rec: MoveRecord) -> std::io::Result<()> {
        self.put(rec);
        self.save()
    }

    pub fn drop_agent(&mut self, agent: Agent) -> std::io::Result<()> {
        self.moves.retain(|m| m.agent != agent);
        self.save()
    }

    fn save(&self) -> std::io::Result<()> {
        if let Some(dir) = self.file.parent().filter(|d| !d.as_os_str().is_empty()) {
            fs::create_dir_all(dir)?;
        }
        let mut tmp = self.file.as_os_str().to_owned();
        tmp.push(".tmp");
        let body = json!({"version": 1, "moves": self.moves.iter().map(MoveRecord::to_value).collect::<Vec<_>>()});
        fs::write(&tmp, js::stringify_indent(&body, "  "))?;
        fs::rename(&tmp, &self.file)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Dir,
    Link,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Recovery {
    Keep,
    Rollback,
    Linked,
    Interrupted,
    Drop,
}

pub fn decide_recovery(phase: &str, source: SourceKind, backup_exists: bool) -> Recovery {
    let lost = source == SourceKind::Missing && backup_exists;
    match phase {
        "linked" => match source {
            SourceKind::Link => Recovery::Keep,
            _ if lost => Recovery::Rollback,
            _ => Recovery::Drop,
        },
        "interrupted" => Recovery::Keep,
        "copying" | "copied" => {
            if lost {
                Recovery::Rollback
            } else {
                Recovery::Interrupted
            }
        }
        "switching" => match source {
            SourceKind::Link => Recovery::Linked,
            _ if lost => Recovery::Rollback,
            _ => Recovery::Interrupted,
        },
        _ => match source {
            SourceKind::Link => Recovery::Linked,
            _ if lost => Recovery::Rollback,
            SourceKind::Dir => Recovery::Drop,
            _ => Recovery::Interrupted,
        },
    }
}

pub fn source_kind(p: &Path) -> SourceKind {
    if link_target(p).is_some() {
        SourceKind::Link
    } else if fs::symlink_metadata(p).is_ok() {
        SourceKind::Dir
    } else {
        SourceKind::Missing
    }
}

pub fn recover_moves(store: &mut MoveStore) -> Vec<Agent> {
    let mut rolled = Vec::new();
    for rec in store.all() {
        let action = decide_recovery(rec.phase, source_kind(Path::new(&rec.source)), Path::new(&rec.backup).exists());
        match action {
            Recovery::Keep => {}
            Recovery::Rollback => {
                if fs::rename(&rec.backup, &rec.source).is_ok() {
                    rolled.push(rec.agent);
                    let _ = store.set(rec.with_phase("interrupted"));
                }
            }
            Recovery::Linked => {
                let _ = store.set(rec.with_phase("linked"));
            }
            Recovery::Interrupted => {
                let _ = store.set(rec.with_phase("interrupted"));
            }
            Recovery::Drop => {
                let _ = store.drop_agent(rec.agent);
            }
        }
    }
    rolled
}

pub fn parse_mount_line(line: &str) -> Option<(String, String)> {
    let skip = line.chars().next()?.len_utf8();
    let at = line[skip..].find(" on ")? + skip;
    let rest = &line[at + 4..];
    for (i, _) in rest.rmatch_indices(" (") {
        if i == 0 {
            continue;
        }
        let ty: String = rest[i + 2..].chars().take_while(|c| *c != ',' && *c != ')').collect();
        if !ty.is_empty() {
            return Some((rest[..i].to_string(), ty));
        }
    }
    None
}

fn network_fs(ty: &str) -> bool {
    matches!(ty.to_lowercase().as_str(), "smbfs" | "nfs" | "nfs4" | "afpfs" | "webdav" | "cifs" | "fuse.sshfs" | "9p" | "ftp")
}

fn run(file: &str, args: &[&str]) -> Option<String> {
    let mut cmd = std::process::Command::new(file);
    cmd.args(args).stdin(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let out = cmd.output().ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).to_string())
}

#[cfg(windows)]
pub fn is_local_dir(dir: &str) -> bool {
    if dir.starts_with(r"\\") {
        return false;
    }
    let b = dir.as_bytes();
    if b.len() < 2 || !b[0].is_ascii_alphabetic() || b[1] != b':' {
        return false;
    }
    let drive = (b[0] as char).to_ascii_uppercase();
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    let ps = format!(r"{root}\System32\WindowsPowerShell\v1.0\powershell.exe");
    let script = format!("(Get-CimInstance Win32_LogicalDisk -Filter \"DeviceID='{drive}:'\").DriveType");
    let ty = run(&ps, &["-NoProfile", "-NonInteractive", "-Command", &script]).and_then(|o| o.trim().parse::<f64>().ok());
    matches!(ty, Some(t) if t == 2.0 || t == 3.0)
}

#[cfg(not(windows))]
pub fn is_local_dir(dir: &str) -> bool {
    let Some(mounts) = run("mount", &[]) else { return true };
    let real = fs::canonicalize(dir).map(|p| crate::util::path_str(&p)).unwrap_or_else(|_| dir.to_string());
    let mut best: Option<(String, String)> = None;
    for line in mounts.lines() {
        let Some((point, ty)) = parse_mount_line(line) else { continue };
        let prefix = if point.ends_with('/') { point.clone() } else { format!("{point}/") };
        if (real == point || real.starts_with(&prefix)) && best.as_ref().is_none_or(|b| point.len() > b.0.len()) {
            best = Some((point, ty));
        }
    }
    best.is_none_or(|b| !network_fs(&b.1))
}

pub fn free_bytes(dir: &str) -> Option<f64> {
    crate::disk::space::statfs(dir).map(|s| s.free)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_table_matches_the_ts_one() {
        use Recovery::*;
        use SourceKind::*;
        assert_eq!(decide_recovery("linked", Link, true), Keep);
        assert_eq!(decide_recovery("linked", Missing, true), Rollback);
        assert_eq!(decide_recovery("linked", Dir, true), Drop);
        assert_eq!(decide_recovery("interrupted", Missing, true), Keep);
        assert_eq!(decide_recovery("copying", Missing, true), Rollback);
        assert_eq!(decide_recovery("copied", Dir, true), Interrupted);
        assert_eq!(decide_recovery("switching", Link, false), Linked);
        assert_eq!(decide_recovery("switching", Dir, true), Interrupted);
        assert_eq!(decide_recovery("undoing", Dir, true), Drop);
        assert_eq!(decide_recovery("undoing", Missing, false), Interrupted);
        assert_eq!(decide_recovery("undoing", Missing, true), Rollback);
    }

    #[test]
    fn mount_lines() {
        assert_eq!(parse_mount_line("//u@nas/share on /Volumes/share (smbfs, nodev, nosuid)"), Some(("/Volumes/share".into(), "smbfs".into())));
        assert_eq!(parse_mount_line("/dev/disk3s1 on /System/Volumes/Data (apfs, local, journaled)"), Some(("/System/Volumes/Data".into(), "apfs".into())));
        assert_eq!(parse_mount_line("/dev/x on /Volumes/My (Disk) (apfs)").map(|m| m.0), Some("/Volumes/My (Disk)".into()));
        assert_eq!(parse_mount_line("garbage"), None);
        assert!(network_fs("SMBFS") && !network_fs("apfs"));
    }

    #[test]
    fn store_round_trips_and_recovers() {
        let dir = std::env::temp_dir().join(format!("core-desk-ad-store-{}", crate::util::uuid()));
        let file = dir.join("data").join("agent-data-move.json");
        let src = dir.join(".claude");
        let backup = dir.join(".claude.orrery-moved-5");
        fs::create_dir_all(&backup).unwrap();
        let mut store = MoveStore::new(file.clone());
        let rec = MoveRecord { agent: Agent::Claude, source: crate::util::path_str(&src), target: crate::util::path_str(&dir.join("t")), backup: crate::util::path_str(&backup), at: 5.0, phase: "switching", original_deleted: false };
        store.set(rec.clone()).unwrap();
        let text = fs::read_to_string(&file).unwrap();
        assert!(text.starts_with("{\n  \"version\": 1,\n  \"moves\": [\n    {\n      \"agent\": \"claude\""));
        let mut again = MoveStore::new(file.clone());
        again.load();
        assert_eq!(again.get(Agent::Claude), Some(rec));
        assert_eq!(recover_moves(&mut again), vec![Agent::Claude]);
        assert!(src.is_dir() && !backup.exists());
        assert_eq!(again.get(Agent::Claude).map(|r| r.phase), Some("interrupted"));
        fs::remove_dir_all(&dir).unwrap();
    }
}
