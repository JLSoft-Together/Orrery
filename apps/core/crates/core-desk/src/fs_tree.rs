use crate::util::mtime_ms;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Cancelled;

pub type Signal = AtomicBool;

fn check(signal: Option<&Signal>) -> Result<(), Cancelled> {
    match signal {
        Some(s) if s.load(Ordering::SeqCst) => Err(Cancelled),
        _ => Ok(()),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryType {
    File,
    Dir,
    Link,
    Other,
}

pub struct TreeEntry {
    pub rel: PathBuf,
    pub kind: EntryType,
    pub size: f64,
    pub mtime_ms: f64,
}

fn type_of(meta: &fs::Metadata) -> EntryType {
    let t = meta.file_type();
    if t.is_symlink() {
        EntryType::Link
    } else if t.is_dir() {
        EntryType::Dir
    } else if t.is_file() {
        EntryType::File
    } else {
        EntryType::Other
    }
}

pub fn read_names(dir: &Path) -> Vec<String> {
    match fs::read_dir(dir) {
        Ok(rd) => rd.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().to_string()).collect(),
        Err(_) => Vec::new(),
    }
}

pub fn walk_tree<F>(root: &Path, mut on_entry: F, signal: Option<&Signal>) -> Result<(), Cancelled>
where
    F: FnMut(&TreeEntry) -> Result<(), Cancelled>,
{
    let mut stack: Vec<PathBuf> = vec![PathBuf::new()];
    while let Some(rel) = stack.pop() {
        check(signal)?;
        let dir = root.join(&rel);
        let names = match fs::read_dir(&dir) {
            Ok(rd) => rd.filter_map(|e| e.ok()).map(|e| e.file_name()).collect::<Vec<_>>(),
            Err(_) => continue,
        };
        for name in names {
            let child = if rel.as_os_str().is_empty() { PathBuf::from(&name) } else { rel.join(&name) };
            let meta = fs::symlink_metadata(root.join(&child)).ok();
            let kind = meta.as_ref().map(type_of).unwrap_or(EntryType::Other);
            let entry = TreeEntry {
                size: if kind == EntryType::File { meta.as_ref().map(|m| m.len() as f64).unwrap_or(0.0) } else { 0.0 },
                mtime_ms: meta.as_ref().map(mtime_ms).unwrap_or(0.0),
                kind,
                rel: child,
            };
            on_entry(&entry)?;
            if kind == EntryType::Dir {
                stack.push(entry.rel);
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreeSize {
    pub files: f64,
    pub bytes: f64,
    pub newest_ms: Option<f64>,
}

pub fn tree_size(root: &Path, signal: Option<&Signal>, mut on_file: impl FnMut(f64)) -> Result<TreeSize, Cancelled> {
    let mut files = 0.0;
    let mut bytes = 0.0;
    let mut newest: f64 = 0.0;
    walk_tree(
        root,
        |e| {
            if e.kind == EntryType::File {
                files += 1.0;
                bytes += e.size;
                newest = newest.max(e.mtime_ms);
                on_file(e.size);
            }
            Ok(())
        },
        signal,
    )?;
    Ok(TreeSize { files, bytes, newest_ms: if newest == 0.0 { None } else { Some(newest) } })
}

pub fn remove_link(at: &Path) -> std::io::Result<()> {
    let meta = fs::symlink_metadata(at)?;
    if !meta.file_type().is_symlink() {
        return Err(std::io::Error::other("not a link"));
    }
    fs::remove_file(at).or_else(|_| fs::remove_dir(at))
}

fn unlink(p: &Path) -> std::io::Result<()> {
    match fs::remove_file(p) {
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {
            let meta = fs::metadata(p)?;
            let mut perm = meta.permissions();
            #[allow(clippy::permissions_set_readonly_false)]
            perm.set_readonly(false);
            fs::set_permissions(p, perm)?;
            fs::remove_file(p)
        }
        other => other,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Removed {
    pub files: f64,
    pub bytes: f64,
    pub failed: f64,
}

pub fn remove_tree(root: &Path, signal: Option<&Signal>, mut on_file: impl FnMut(f64)) -> Result<Removed, Cancelled> {
    let mut out = Removed::default();
    let top = match fs::symlink_metadata(root) {
        Ok(m) => m,
        Err(_) => return Ok(out),
    };
    if top.file_type().is_symlink() {
        let _ = remove_link(root);
        return Ok(out);
    }
    let mut dirs: Vec<PathBuf> = Vec::new();
    walk_tree(
        root,
        |e| {
            let p = root.join(&e.rel);
            if e.kind == EntryType::Dir {
                dirs.push(p);
                return Ok(());
            }
            let r = if e.kind == EntryType::Link { remove_link(&p) } else { unlink(&p) };
            match r {
                Ok(()) if e.kind != EntryType::Link => {
                    out.files += 1.0;
                    out.bytes += e.size;
                    on_file(e.size);
                }
                Ok(()) => {}
                Err(_) => out.failed += 1.0,
            }
            Ok(())
        },
        signal,
    )?;
    dirs.sort_by_key(|d| std::cmp::Reverse(d.as_os_str().len()));
    for d in dirs {
        let _ = fs::remove_dir(d);
    }
    if fs::remove_dir(root).is_err() {
        out.failed += 1.0;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_then_remove() {
        let dir = std::env::temp_dir().join(format!("core-desk-tree-{}", crate::util::uuid()));
        fs::create_dir_all(dir.join("a/b")).unwrap();
        fs::write(dir.join("a/b/x.txt"), "hello").unwrap();
        fs::write(dir.join("y.txt"), "abc").unwrap();
        let s = tree_size(&dir, None, |_| {}).unwrap();
        assert_eq!((s.files, s.bytes), (2.0, 8.0));
        assert!(s.newest_ms.is_some());
        let stop = AtomicBool::new(true);
        assert_eq!(tree_size(&dir, Some(&stop), |_| {}), Err(Cancelled));
        let r = remove_tree(&dir, None, |_| {}).unwrap();
        assert_eq!((r.files, r.bytes, r.failed), (2.0, 8.0, 0.0));
        assert!(!dir.exists());
    }
}
