use crate::fold::{cmp_ci, fold_into, group_of};
use std::collections::VecDeque;

pub const NONE: u32 = u32::MAX;

pub mod flag {
    pub const DIR: u16 = 1;
    pub const LINK: u16 = 2;
    pub const CLOUD: u16 = 4;
    pub const DENIED: u16 = 8;
    pub const HARDLINK: u16 = 16;
    pub const HIDDEN: u16 = 32;
    pub const SYSTEM: u16 = 64;
    pub const MOUNT: u16 = 128;
    pub const ERROR: u16 = 256;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Node {
    pub parent: u32,
    pub name_off: u32,
    pub name_len: u16,
    pub fold_len: u16,
    pub flags: u16,
    pub group: u8,
    pub first: u32,
    pub count: u32,
    pub size: u64,
    pub len: u64,
    pub mtime: u32,
}

impl Node {
    pub fn is_dir(&self) -> bool {
        self.flags & flag::DIR != 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Stats {
    pub files: u64,
    pub dirs: u64,
    pub bytes: u64,
    pub denied: u64,
    pub errors: u64,
    pub elapsed_ms: u64,
    pub scanned_at: u64,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Index {
    pub nodes: Vec<Node>,
    pub pool: Vec<u8>,
    pub roots: Vec<u32>,
    pub stats: Stats,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sort {
    Size,
    Name,
    Mtime,
}

impl Sort {
    pub fn parse(s: &str) -> Sort {
        match s {
            "name" => Sort::Name,
            "mtime" | "date" => Sort::Mtime,
            _ => Sort::Size,
        }
    }
}

pub const SEP: char = if cfg!(windows) { '\\' } else { '/' };

fn is_sep(c: char) -> bool {
    c == '/' || (cfg!(windows) && c == '\\')
}

pub fn eq_name(a: &str, b: &str) -> bool {
    if cfg!(any(windows, target_os = "macos")) {
        a.eq_ignore_ascii_case(b) || (!a.is_ascii() && a.to_lowercase() == b.to_lowercase())
    } else {
        a == b
    }
}

pub fn clean_path(p: &str) -> String {
    let mut s = if cfg!(windows) { p.trim().replace('/', "\\") } else { p.trim().to_string() };
    if cfg!(windows) {
        if let Some(rest) = s.strip_prefix("\\\\?\\UNC\\") {
            s = format!("\\\\{rest}");
        } else if let Some(rest) = s.strip_prefix("\\\\?\\") {
            s = rest.to_string();
        }
        if s.len() == 2 && s.ends_with(':') {
            s.push('\\');
        }
    }
    while s.len() > 1 && s.ends_with(SEP) && !(cfg!(windows) && s.len() == 3 && s.as_bytes()[1] == b':') {
        s.pop();
    }
    s
}

pub fn under(path: &str, base: &str) -> bool {
    if path.len() < base.len() || !path.is_char_boundary(base.len()) || !eq_name(&path[..base.len()], base) {
        return false;
    }
    path.len() == base.len() || base.ends_with(SEP) || path[base.len()..].starts_with(SEP)
}

pub struct Builder {
    pub nodes: Vec<Node>,
    pub pool: Vec<u8>,
    scratch: String,
}

impl Default for Builder {
    fn default() -> Self {
        Self::new()
    }
}

impl Builder {
    pub fn new() -> Self {
        Builder { nodes: Vec::new(), pool: Vec::new(), scratch: String::new() }
    }

    pub fn with_capacity(nodes: usize, pool: usize) -> Self {
        Builder { nodes: Vec::with_capacity(nodes), pool: Vec::with_capacity(pool), scratch: String::new() }
    }

    pub fn push(&mut self, parent: u32, name: &str, flags: u16, size: u64, len: u64, mtime: u32) -> u32 {
        let name = if name.len() > u16::MAX as usize { &name[..name.floor_char_boundary(u16::MAX as usize)] } else { name };
        let name_off = self.pool.len() as u32;
        self.pool.extend_from_slice(name.as_bytes());
        let mut fold_len = 0u16;
        if !name.is_ascii() {
            fold_into(name, &mut self.scratch);
            let f = &self.scratch[..self.scratch.floor_char_boundary(u16::MAX as usize)];
            self.pool.extend_from_slice(f.as_bytes());
            fold_len = f.len() as u16;
        }
        let group = if flags & flag::DIR == 0 { group_of(name) } else { 0 };
        self.nodes.push(Node { parent, name_off, name_len: name.len() as u16, fold_len, flags, group, first: 0, count: 0, size, len, mtime });
        (self.nodes.len() - 1) as u32
    }

    fn copy_from(&mut self, src: &Index, i: u32, parent: u32) -> u32 {
        let n = src.nodes[i as usize];
        let start = n.name_off as usize;
        let end = start + n.name_len as usize + n.fold_len as usize;
        let name_off = self.pool.len() as u32;
        self.pool.extend_from_slice(&src.pool[start..end]);
        self.nodes.push(Node { parent, name_off, first: 0, count: 0, ..n });
        (self.nodes.len() - 1) as u32
    }

    pub fn append_tree(&mut self, src: &Index, root: u32) -> u32 {
        let top = self.copy_from(src, root, NONE);
        let mut queue = VecDeque::from([(root, top)]);
        while let Some((s, d)) = queue.pop_front() {
            let sn = src.nodes[s as usize];
            if sn.count == 0 {
                continue;
            }
            let first = self.nodes.len() as u32;
            for c in sn.first..sn.first + sn.count {
                let nc = self.copy_from(src, c, d);
                if src.nodes[c as usize].count > 0 {
                    queue.push_back((c, nc));
                }
            }
            let dn = &mut self.nodes[d as usize];
            dn.first = first;
            dn.count = sn.count;
        }
        top
    }

    pub fn finish(self, roots: Vec<u32>, stats: Stats) -> Index {
        let mut ix = Index { nodes: self.nodes, pool: self.pool, roots, stats };
        ix.recount();
        ix
    }
}

impl Index {
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn name(&self, i: u32) -> &str {
        let n = &self.nodes[i as usize];
        let s = n.name_off as usize;
        std::str::from_utf8(&self.pool[s..s + n.name_len as usize]).unwrap_or("")
    }

    pub fn folded(&self, i: u32) -> &[u8] {
        let n = &self.nodes[i as usize];
        let s = n.name_off as usize;
        if n.fold_len > 0 {
            let f = s + n.name_len as usize;
            &self.pool[f..f + n.fold_len as usize]
        } else {
            &self.pool[s..s + n.name_len as usize]
        }
    }

    pub fn children(&self, i: u32) -> std::ops::Range<u32> {
        let n = &self.nodes[i as usize];
        n.first..n.first + n.count
    }

    pub fn path(&self, i: u32) -> String {
        let mut chain = Vec::new();
        let mut at = i;
        while at != NONE {
            chain.push(at);
            at = self.nodes[at as usize].parent;
        }
        let mut out = String::new();
        for (k, n) in chain.iter().rev().enumerate() {
            if k > 0 && !out.ends_with(SEP) {
                out.push(SEP);
            }
            out.push_str(self.name(*n));
        }
        out
    }

    pub fn find(&self, path: &str) -> Option<u32> {
        let path = clean_path(path);
        for &r in &self.roots {
            let base = self.name(r);
            if !under(&path, base) {
                continue;
            }
            let mut at = r;
            for part in path[base.len()..].split(is_sep).filter(|p| !p.is_empty()) {
                at = self.children(at).find(|c| eq_name(self.name(*c), part))?;
            }
            return Some(at);
        }
        None
    }

    pub fn root_paths(&self) -> Vec<String> {
        self.roots.iter().map(|r| self.name(*r).to_string()).collect()
    }

    pub fn heap_bytes(&self) -> usize {
        self.nodes.capacity() * std::mem::size_of::<Node>() + self.pool.capacity() + self.roots.capacity() * 4
    }

    pub fn recount(&mut self) {
        let (mut files, mut dirs, mut denied, mut errors) = (0, 0, 0, 0);
        for n in &self.nodes {
            if n.is_dir() {
                dirs += 1;
            } else {
                files += 1;
            }
            if n.flags & flag::DENIED != 0 {
                denied += 1;
            }
            if n.flags & flag::ERROR != 0 {
                errors += 1;
            }
        }
        let bytes = self.roots.iter().map(|r| self.nodes[*r as usize].size).sum();
        self.stats = Stats { files, dirs, bytes, denied, errors, ..self.stats };
    }

    pub fn sum_sizes(&mut self) {
        for i in (0..self.nodes.len()).rev() {
            let n = self.nodes[i];
            if n.parent != NONE {
                let p = &mut self.nodes[n.parent as usize];
                p.size += n.size;
                p.len += n.len;
            }
        }
    }

    pub fn sorted(&self, items: &mut [u32], sort: Sort) {
        match sort {
            Sort::Size => items.sort_unstable_by(|a, b| self.nodes[*b as usize].size.cmp(&self.nodes[*a as usize].size).then(a.cmp(b))),
            Sort::Mtime => items.sort_unstable_by(|a, b| self.nodes[*b as usize].mtime.cmp(&self.nodes[*a as usize].mtime).then(a.cmp(b))),
            Sort::Name => items.sort_unstable_by(|a, b| cmp_ci(self.folded(*a), self.folded(*b)).then(a.cmp(b))),
        }
    }

    pub fn merge(old: &Index, new: Index) -> Index {
        let new_roots = new.root_paths();
        let kept: Vec<u32> = old.roots.iter().copied().filter(|r| {
            let p = old.name(*r);
            !new_roots.iter().any(|n| under(p, n) || under(n, p))
        }).collect();
        if kept.is_empty() {
            return new;
        }
        let mut b = Builder::with_capacity(old.nodes.len() + new.nodes.len(), old.pool.len() + new.pool.len());
        let mut roots = Vec::new();
        for r in kept {
            roots.push(b.append_tree(old, r));
        }
        for r in &new.roots {
            roots.push(b.append_tree(&new, *r));
        }
        b.finish(roots, new.stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(root: &str) -> Index {
        let mut b = Builder::new();
        let r = b.push(NONE, root, flag::DIR, 0, 0, 0);
        let first = b.nodes.len() as u32;
        let d = b.push(r, "Ảnh", flag::DIR, 0, 0, 0);
        b.push(r, "a.txt", 0, 10, 10, 1);
        b.nodes[r as usize].first = first;
        b.nodes[r as usize].count = 2;
        let f2 = b.nodes.len() as u32;
        b.push(d, "b.jpg", 0, 30, 25, 2);
        b.nodes[d as usize].first = f2;
        b.nodes[d as usize].count = 1;
        let mut ix = b.finish(vec![r], Stats::default());
        ix.sum_sizes();
        ix.recount();
        ix
    }

    #[test]
    fn paths_and_find() {
        let root = if cfg!(windows) { "C:\\data" } else { "/data" };
        let ix = sample(root);
        assert_eq!(ix.nodes[0].size, 40);
        let p = ix.path(3);
        assert_eq!(p, format!("{root}{SEP}Ảnh{SEP}b.jpg"));
        assert_eq!(ix.find(&p), Some(3));
        assert_eq!(ix.find(root), Some(0));
        assert_eq!(ix.folded(1), b"anh");
        assert_eq!(ix.stats.files, 2);
    }

    #[test]
    fn merge_keeps_disjoint_roots() {
        let (a, b) = if cfg!(windows) { ("C:\\a", "D:\\b") } else { ("/a", "/b") };
        let m = Index::merge(&sample(a), sample(b));
        assert_eq!(m.root_paths(), vec![a.to_string(), b.to_string()]);
        assert_eq!(m.len(), 8);
        assert_eq!(m.find(&format!("{b}{SEP}Ảnh{SEP}b.jpg")).map(|i| m.nodes[i as usize].size), Some(30));
        let again = Index::merge(&m, sample(a));
        assert_eq!(again.root_paths(), vec![b.to_string(), a.to_string()]);
    }
}
