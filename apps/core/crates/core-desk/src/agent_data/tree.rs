use crate::fs_tree::{walk_tree, Cancelled, EntryType, Signal};
use crate::util;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::SystemTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Halt {
    Cancelled,
    Failed,
}

impl From<Cancelled> for Halt {
    fn from(_: Cancelled) -> Self {
        Halt::Cancelled
    }
}

pub fn aborted(signal: Option<&Signal>) -> bool {
    signal.is_some_and(|s| s.load(Ordering::SeqCst))
}

pub fn clean_link_target(target: &str) -> String {
    let mut t = target.strip_prefix(r"\\?\").or_else(|| target.strip_prefix(r"\??\")).unwrap_or(target).to_string();
    if t.chars().count() > 3 {
        t = t.trim_end_matches(['\\', '/']).to_string();
    }
    t
}

pub fn link_target(p: &Path) -> Option<String> {
    let meta = fs::symlink_metadata(p).ok()?;
    if !meta.file_type().is_symlink() {
        return None;
    }
    let raw = fs::read_link(p).ok()?;
    let text = if raw.is_absolute() { util::path_str(&raw) } else { util::path_str(&util::resolve(&util::path_str(&p.parent().unwrap_or(Path::new("")).join(raw)))) };
    Some(clean_link_target(&text))
}

#[cfg(unix)]
pub fn make_dir_link(target: &Path, at: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, at)
}

#[cfg(windows)]
pub fn make_dir_link(target: &Path, at: &Path) -> io::Result<()> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    let cmd = Path::new(&root).join("System32").join("cmd.exe");
    let line = format!("/d /c mklink /J \"{}\" \"{}\"", at.display(), target.display());
    let ok = Command::new(cmd).raw_arg(line).creation_flags(0x0800_0000).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status()?.success();
    if ok && link_target(at).is_some() {
        Ok(())
    } else {
        Err(io::Error::other("mklink failed"))
    }
}

#[cfg(unix)]
fn make_file_link(target: &Path, at: &Path) -> io::Result<()> {
    std::os::unix::fs::symlink(target, at)
}

#[cfg(windows)]
fn make_file_link(target: &Path, at: &Path) -> io::Result<()> {
    std::os::windows::fs::symlink_file(target, at)
}

#[cfg(unix)]
fn open_for_times(p: &Path) -> io::Result<fs::File> {
    fs::File::open(p)
}

#[cfg(windows)]
fn open_for_times(p: &Path) -> io::Result<fs::File> {
    use std::os::windows::fs::OpenOptionsExt;
    fs::OpenOptions::new().access_mode(0x100).custom_flags(0x0200_0000).open(p)
}

pub fn set_times(p: &Path, accessed: Option<SystemTime>, modified: Option<SystemTime>) {
    let Ok(f) = open_for_times(p) else { return };
    let mut times = fs::FileTimes::new();
    if let Some(a) = accessed {
        times = times.set_accessed(a);
    }
    if let Some(m) = modified {
        times = times.set_modified(m);
    }
    let _ = f.set_times(times);
}

pub fn copy_times(from: &fs::Metadata, to: &Path) {
    set_times(to, from.accessed().ok(), from.modified().ok());
}

const BIG_FILE: u64 = 64 * 1024 * 1024;

fn copy_file(src: &Path, dst: &Path, on_bytes: &mut dyn FnMut(f64), signal: Option<&Signal>) -> io::Result<()> {
    let st = fs::metadata(src)?;
    if st.len() < BIG_FILE {
        fs::copy(src, dst)?;
        on_bytes(st.len() as f64);
    } else {
        let mut input = fs::File::open(src)?;
        let mut out = fs::OpenOptions::new().write(true).create_new(true).open(dst)?;
        let mut buf = vec![0u8; 1024 * 1024];
        loop {
            if aborted(signal) {
                return Err(io::Error::other("cancelled"));
            }
            let n = input.read(&mut buf)?;
            if n == 0 {
                break;
            }
            out.write_all(&buf[..n])?;
            on_bytes(n as f64);
        }
        out.flush()?;
    }
    copy_times(&st, dst);
    Ok(())
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct CopyResult {
    pub files: f64,
    pub bytes: f64,
    pub links: f64,
    pub skipped: Vec<PathBuf>,
}

pub fn copy_tree(src: &Path, dst: &Path, signal: Option<&Signal>, mut on_bytes: impl FnMut(f64)) -> Result<CopyResult, Halt> {
    let mut out = CopyResult::default();
    fs::create_dir(dst).map_err(|_| Halt::Failed)?;
    walk_tree(
        src,
        |e| {
            if aborted(signal) {
                return Err(Cancelled);
            }
            let from = src.join(&e.rel);
            let to = dst.join(&e.rel);
            let r: io::Result<()> = match e.kind {
                EntryType::Dir => fs::create_dir(&to),
                EntryType::File => copy_file(&from, &to, &mut on_bytes, signal).map(|_| {
                    out.files += 1.0;
                    out.bytes += e.size;
                }),
                EntryType::Link => match link_target(&from) {
                    None => Err(io::Error::other("unreadable link")),
                    Some(target) => {
                        let is_dir = fs::metadata(&from).map(|m| m.is_dir()).unwrap_or(true);
                        let made = if is_dir { make_dir_link(Path::new(&target), &to) } else { make_file_link(Path::new(&target), &to) };
                        made.map(|_| out.links += 1.0)
                    }
                },
                EntryType::Other => {
                    out.skipped.push(e.rel.clone());
                    Ok(())
                }
            };
            if r.is_err() {
                if aborted(signal) {
                    return Err(Cancelled);
                }
                out.skipped.push(e.rel.clone());
            }
            Ok(())
        },
        signal,
    )?;
    Ok(out)
}

fn hash_file(file: &Path, size: u64, signal: Option<&Signal>) -> Result<Vec<u8>, Halt> {
    let mut h = Sha256::new();
    let eight = 8 * 1024 * 1024;
    let ranges: Vec<(u64, u64)> = if size == 0 {
        Vec::new()
    } else if size > 256 * 1024 * 1024 {
        vec![(0, eight), (size - eight, eight)]
    } else {
        vec![(0, size)]
    };
    let mut f = if ranges.is_empty() { None } else { Some(fs::File::open(file).map_err(|_| Halt::Failed)?) };
    let mut buf = vec![0u8; 1024 * 1024];
    for (start, len) in ranges {
        if aborted(signal) {
            return Err(Halt::Cancelled);
        }
        let f = f.as_mut().ok_or(Halt::Failed)?;
        f.seek(SeekFrom::Start(start)).map_err(|_| Halt::Failed)?;
        let mut left = len;
        while left > 0 {
            let want = left.min(buf.len() as u64) as usize;
            let n = f.read(&mut buf[..want]).map_err(|_| Halt::Failed)?;
            if n == 0 {
                break;
            }
            h.update(&buf[..n]);
            left -= n as u64;
        }
    }
    Ok(h.finalize().to_vec())
}

#[derive(Debug, Clone, PartialEq)]
pub struct VerifyResult {
    pub ok: bool,
    pub mismatch: Option<String>,
}

struct Listed {
    order: Vec<(PathBuf, EntryType, f64)>,
    index: HashMap<PathBuf, (EntryType, f64)>,
}

fn list(root: &Path, signal: Option<&Signal>) -> Result<Listed, Cancelled> {
    let mut out = Listed { order: Vec::new(), index: HashMap::new() };
    walk_tree(
        root,
        |e| {
            if matches!(e.kind, EntryType::File | EntryType::Link) && out.index.insert(e.rel.clone(), (e.kind, e.size)).is_none() {
                out.order.push((e.rel.clone(), e.kind, e.size));
            }
            Ok(())
        },
        signal,
    )?;
    Ok(out)
}

pub fn verify_copy(src: &Path, dst: &Path, skipped: &[PathBuf], signal: Option<&Signal>) -> Result<VerifyResult, Halt> {
    let skip: HashSet<&PathBuf> = skipped.iter().collect();
    let a = list(src, signal)?;
    let b = list(dst, signal)?;
    let bad = |rel: &PathBuf| Ok(VerifyResult { ok: false, mismatch: Some(util::path_str(rel)) });
    for (rel, kind, size) in &a.order {
        if skip.contains(rel) {
            continue;
        }
        match b.index.get(rel) {
            Some((k, s)) if k == kind && s == size => {}
            _ => return bad(rel),
        }
    }
    for (rel, _, _) in &b.order {
        if !a.index.contains_key(rel) {
            return bad(rel);
        }
    }
    let mut all: Vec<&(PathBuf, EntryType, f64)> = a.order.iter().filter(|(rel, k, _)| *k == EntryType::File && !skip.contains(rel)).collect();
    all.sort_by_key(|x| util::path_str(&x.0));
    let want = all.len().min(48);
    let mut picks: Vec<&(PathBuf, EntryType, f64)> = Vec::new();
    for i in 0..want {
        let e = all[i * all.len() / want];
        if !picks.iter().any(|p| p.0 == e.0) {
            picks.push(e);
        }
    }
    let mut largest: Option<&(PathBuf, EntryType, f64)> = None;
    for e in &all {
        if largest.is_none_or(|m| e.2 > m.2) {
            largest = Some(e);
        }
    }
    if let Some(l) = largest {
        if !picks.iter().any(|p| p.0 == l.0) {
            picks.push(l);
        }
    }
    for (rel, _, size) in picks {
        let x = hash_file(&src.join(rel), *size as u64, signal)?;
        let y = hash_file(&dst.join(rel), *size as u64, signal)?;
        if x != y {
            return bad(rel);
        }
    }
    Ok(VerifyResult { ok: true, mismatch: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("core-desk-ad-{tag}-{}", util::uuid()));
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn link_targets_are_cleaned() {
        assert_eq!(clean_link_target(r"\\?\C:\x\"), r"C:\x");
        assert_eq!(clean_link_target(r"\??\D:\y"), r"D:\y");
        assert_eq!(clean_link_target("C:\\"), "C:\\");
        assert_eq!(clean_link_target("/a/b/"), "/a/b");
    }

    #[test]
    fn copies_keep_links_and_verify() {
        let root = tmp("copy");
        let src = root.join("src");
        fs::create_dir_all(src.join("a/b")).unwrap();
        fs::write(src.join("a/b/x.txt"), "hello").unwrap();
        fs::write(src.join("y.bin"), vec![7u8; 3000]).unwrap();
        let outside = root.join("outside");
        fs::create_dir_all(&outside).unwrap();
        fs::write(outside.join("big.txt"), "never copied").unwrap();
        make_dir_link(&outside, &src.join("link")).unwrap();
        let dst = root.join("dst");
        let mut seen = 0.0;
        let r = copy_tree(&src, &dst, None, |n| seen += n).unwrap();
        assert_eq!((r.files, r.bytes, r.links, seen), (2.0, 3005.0, 1.0, 3005.0));
        assert_eq!(link_target(&dst.join("link")).map(PathBuf::from), Some(util::resolve(&util::path_str(&outside))));
        assert!(verify_copy(&src, &dst, &[], None).unwrap().ok);
        fs::write(dst.join("y.bin"), vec![8u8; 3000]).unwrap();
        let bad = verify_copy(&src, &dst, &[], None).unwrap();
        assert_eq!(bad.mismatch.as_deref(), Some("y.bin"));
        fs::write(dst.join("extra"), "x").unwrap();
        assert!(!verify_copy(&src, &dst, &[], None).unwrap().ok);
        assert_eq!(copy_tree(&src, &dst, None, |_| {}), Err(Halt::Failed));
        let stop = std::sync::atomic::AtomicBool::new(true);
        assert_eq!(copy_tree(&src, &root.join("dst2"), Some(&stop), |_| {}), Err(Halt::Cancelled));
        crate::fs_tree::remove_tree(&dst, None, |_| {}).unwrap();
        assert!(outside.join("big.txt").exists());
        crate::fs_tree::remove_tree(&root, None, |_| {}).unwrap();
    }
}
