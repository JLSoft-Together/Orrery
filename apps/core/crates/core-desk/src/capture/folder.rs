use super::files::{is_inside, is_orrery_file, join, parse_config, pick_expired, shot_name, CapturedFile, REC_SUB, SHOT_SUB};
use crate::{js, util};
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[cfg(windows)]
pub fn stamp() -> String {
    let mut st: windows_sys::Win32::Foundation::SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut st) };
    format!("{:04}{:02}{:02}-{:02}{:02}{:02}", st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond)
}

#[cfg(target_os = "macos")]
pub fn stamp() -> String {
    let now = unsafe { libc::time(std::ptr::null_mut()) };
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    unsafe { libc::localtime_r(&now, &mut tm) };
    format!("{:04}{:02}{:02}-{:02}{:02}{:02}", tm.tm_year + 1900, tm.tm_mon + 1, tm.tm_mday, tm.tm_hour, tm.tm_min, tm.tm_sec)
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn stamp() -> String {
    let iso = util::iso_string(util::now_ms());
    format!("{}{}{}-{}{}{}", &iso[0..4], &iso[5..7], &iso[8..10], &iso[11..13], &iso[14..16], &iso[17..19])
}

#[cfg(unix)]
fn writable(dir: &Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let Ok(c) = std::ffi::CString::new(dir.as_os_str().as_bytes()) else { return false };
    unsafe { libc::access(c.as_ptr(), libc::W_OK) == 0 }
}

#[cfg(not(unix))]
fn writable(dir: &Path) -> bool {
    std::fs::metadata(dir).map(|m| !m.permissions().readonly()).unwrap_or(false)
}

fn root_of(dir: &Path) -> PathBuf {
    dir.ancestors().last().map(Path::to_path_buf).unwrap_or_default()
}

pub struct Folder {
    file: PathBuf,
    cfg: Mutex<Value>,
    warned: Mutex<HashSet<String>>,
}

pub struct Status {
    pub value: Value,
    pub active: Option<String>,
    pub missing: Option<String>,
}

impl Folder {
    pub fn new(file: PathBuf) -> Self {
        let raw = std::fs::read_to_string(&file).ok().and_then(|t| js::parse(&t)).unwrap_or(Value::Null);
        Folder { file, cfg: Mutex::new(parse_config(&raw)), warned: Mutex::new(HashSet::new()) }
    }

    pub fn config(&self) -> Value {
        self.cfg.lock().unwrap().clone()
    }

    pub fn dir(&self) -> Option<String> {
        self.config().get("dir").and_then(Value::as_str).map(str::to_string)
    }

    fn problem(dir: &str) -> Option<&'static str> {
        let p = Path::new(dir);
        if !root_of(p).exists() || !p.exists() || !p.is_dir() {
            return Some("missing");
        }
        if writable(p) {
            None
        } else {
            Some("not-writable")
        }
    }

    pub fn status(&self) -> Status {
        let config = self.config();
        let Some(dir) = config.get("dir").and_then(Value::as_str).map(str::to_string) else {
            return Status { value: json!({"config": config, "active": null, "problem": null}), active: None, missing: None };
        };
        let problem = Self::problem(&dir);
        let mut warned = self.warned.lock().unwrap();
        let mut missing = None;
        if problem == Some("missing") && warned.insert(dir.clone()) {
            missing = Some(dir.clone());
        }
        if problem.is_none() {
            warned.remove(&dir);
        }
        let active = if problem.is_some() { None } else { Some(dir) };
        Status { value: json!({"config": config, "active": active, "problem": problem}), active, missing }
    }

    pub fn set(&self, patch: &Map<String, Value>) {
        let mut cfg = self.cfg.lock().unwrap();
        if let Value::Object(m) = &mut *cfg {
            for (k, v) in patch {
                m.insert(k.clone(), v.clone());
            }
        }
        let _ = std::fs::write(&self.file, js::stringify(&cfg));
    }

    pub fn write_shot(&self, dir: &str, png: &[u8], device: &str, frame: Option<i64>) -> Option<String> {
        let shots = Path::new(dir).join(SHOT_SUB);
        std::fs::create_dir_all(&shots).ok()?;
        let at = stamp();
        for dup in 1..=50 {
            let file = shots.join(shot_name(device, &at, frame, dup));
            match std::fs::OpenOptions::new().write(true).create_new(true).open(&file) {
                Ok(mut f) => {
                    use std::io::Write;
                    return f.write_all(png).ok().map(|_| util::path_str(&file));
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(_) => return None,
            }
        }
        None
    }

    fn list(dir: &str, sub: &str) -> Vec<CapturedFile> {
        let folder = Path::new(dir).join(sub);
        let mut out = Vec::new();
        for name in crate::fs_tree::read_names(&folder) {
            if !is_orrery_file(sub, &name) {
                continue;
            }
            let file = folder.join(&name);
            if let Ok(m) = std::fs::metadata(&file) {
                if m.is_file() {
                    out.push(CapturedFile { path: util::path_str(&file), size: m.len() as f64, mtime_ms: util::mtime_ms(&m) });
                }
            }
        }
        out
    }

    pub fn sweep(&self, active: &str, held: &[String]) -> usize {
        let cfg = self.config();
        let mut files = Self::list(active, REC_SUB);
        files.extend(Self::list(active, SHOT_SUB));
        let held: HashSet<String> = held.iter().map(|h| util::path_str(&util::resolve(h))).collect();
        let keep = cfg.get("keepDays").and_then(Value::as_f64).unwrap_or(7.0);
        let max = cfg.get("maxGb").and_then(Value::as_f64).unwrap_or(2.0) * 1024f64.powi(3);
        pick_expired(&files, util::now_ms(), keep, max)
            .into_iter()
            .filter(|f| !held.contains(&util::path_str(&util::resolve(&f.path))))
            .filter(|f| std::fs::remove_file(&f.path).is_ok() || !Path::new(&f.path).exists())
            .count()
    }

    pub fn shot_ok(dir: &str, file: &str) -> bool {
        is_inside(&join(dir, SHOT_SUB), file)
    }
}
