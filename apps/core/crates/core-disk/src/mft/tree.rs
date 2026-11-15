use super::record::{parse, Parsed, NS_DOS};
use crate::index::{flag, Builder, Index, Stats, NONE};
use std::collections::VecDeque;

pub const ROOT: u32 = 5;
pub const FIRST_USER: u32 = 16;
const ATTR_HIDDEN: u32 = 0x2;
const ATTR_SYSTEM: u32 = 0x4;
const CLOUD_ATTRS: u32 = 0x1000 | 0x40000 | 0x400000;
const LINK_TAGS: [u32; 5] = [0xA000_0003, 0xA000_000C, 0x8000_001B, 0xA000_001D, 0x8000_0023];
const R_USED: u16 = 1;
const R_DIR: u16 = 2;
const R_HIDDEN: u16 = 4;
const R_SYSTEM: u16 = 8;
const R_CLOUD: u16 = 16;
const R_LINK: u16 = 32;

pub fn is_cloud_tag(tag: u32) -> bool {
    tag & 0xFFFF_0FFF == 0x9000_001A
}

#[derive(Debug, Clone, Copy, Default)]
struct Rec {
    size: u64,
    len: u64,
    mtime: u32,
    first: u32,
    flags: u16,
    seq: u16,
}

#[derive(Debug, Clone, Copy)]
struct Name {
    parent: u32,
    parent_seq: u16,
    ns: u8,
    off: u32,
    len: u32,
    next: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeError {
    TooLarge,
    NoRoot,
}

impl std::fmt::Display for TreeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            TreeError::TooLarge => "index too large",
            TreeError::NoRoot => "root record missing",
        })
    }
}

#[derive(Default)]
pub struct Table {
    recs: Vec<Rec>,
    names: Vec<Name>,
    pool: String,
    parsed: Parsed,
    pub records: u64,
    pub bad: u64,
    pub skipped: u64,
    limit: usize,
}

impl Table {
    pub fn new(expected: usize) -> Self {
        Table { recs: Vec::with_capacity(expected.min(1 << 26)), limit: expected.min(NONE as usize - 1), ..Default::default() }
    }

    pub fn with_limit(limit: usize) -> Self {
        Table { limit, ..Default::default() }
    }

    fn rec(&mut self, n: u64) -> Option<&mut Rec> {
        if n >= self.limit as u64 {
            return None;
        }
        let i = n as usize;
        if i >= self.recs.len() {
            self.recs.resize(i + 1, Rec { first: NONE, ..Default::default() });
        }
        Some(&mut self.recs[i])
    }

    pub fn add_chunk(&mut self, chunk: &mut [u8], first_record: u64, record_size: usize) {
        if record_size == 0 {
            return;
        }
        for (k, rec) in chunk.chunks_exact_mut(record_size).enumerate() {
            self.add(first_record + k as u64, rec);
        }
    }

    pub fn add(&mut self, number: u64, rec: &mut [u8]) {
        self.records += 1;
        let mut p = std::mem::take(&mut self.parsed);
        match parse(rec, &mut p) {
            Ok(()) if p.in_use => self.take(number, &p),
            Ok(()) => {}
            Err(_) => {
                if rec.iter().take(4).any(|b| *b != 0) {
                    self.bad += 1;
                }
            }
        }
        self.parsed = p;
    }

    fn take(&mut self, number: u64, p: &Parsed) {
        let owner = if p.base != 0 { p.base } else { number };
        if owner >= self.limit as u64 {
            self.skipped += 1;
            return;
        }
        let mut head = {
            let Some(r) = self.rec(owner) else { return };
            if p.base == 0 {
                let mut f = R_USED;
                if p.dir {
                    f |= R_DIR;
                }
                if let Some(a) = p.attrs {
                    if a & ATTR_HIDDEN != 0 {
                        f |= R_HIDDEN;
                    }
                    if a & ATTR_SYSTEM != 0 {
                        f |= R_SYSTEM;
                    }
                    if a & CLOUD_ATTRS != 0 {
                        f |= R_CLOUD;
                    }
                }
                if let Some(tag) = p.reparse {
                    if LINK_TAGS.contains(&tag) {
                        f |= R_LINK;
                    } else if is_cloud_tag(tag) {
                        f |= R_CLOUD;
                    }
                }
                r.flags = f;
                r.seq = p.seq;
                r.mtime = p.mtime;
            }
            if let Some((alloc, len)) = p.data {
                r.size = alloc;
                r.len = len;
            }
            r.first
        };
        for n in &p.names {
            if n.parent >= self.limit as u64 {
                continue;
            }
            let name = p.name(n);
            let off = self.pool.len() as u32;
            self.pool.push_str(name);
            self.names.push(Name { parent: n.parent as u32, parent_seq: n.parent_seq, ns: n.ns, off, len: name.len() as u32, next: head });
            head = (self.names.len() - 1) as u32;
        }
        if let Some(r) = self.rec(owner) {
            r.first = head;
        }
    }

    fn links(&self, r: u32) -> impl Iterator<Item = &Name> {
        let mut at = self.recs.get(r as usize).map(|x| x.first).unwrap_or(NONE);
        std::iter::from_fn(move || {
            if at == NONE {
                return None;
            }
            let n = &self.names[at as usize];
            at = n.next;
            Some(n)
        })
    }

    fn visible(&self, r: u32) -> Vec<u32> {
        let mut all = Vec::new();
        let mut dos = Vec::new();
        let mut at = self.recs[r as usize].first;
        while at != NONE {
            let n = &self.names[at as usize];
            if n.ns == NS_DOS {
                dos.push(at);
            } else {
                all.push(at);
            }
            at = n.next;
        }
        if all.is_empty() {
            all = dos;
        }
        all.retain(|&i| unique(&self.names[i as usize], &self.names, &self.pool));
        all.reverse();
        all
    }

    pub fn build(self, root_name: &str) -> Result<Index, TreeError> {
        self.build_with(root_name, Builder::new())
    }

    pub fn build_with(self, root_name: &str, mut b: Builder) -> Result<Index, TreeError> {
        let total = self.recs.len();
        if total <= ROOT as usize || self.recs[ROOT as usize].flags & R_USED == 0 || self.recs[ROOT as usize].flags & R_DIR == 0 {
            return Err(TreeError::NoRoot);
        }
        let mut count = vec![0u32; total + 1];
        let mut kids_of: Vec<(u32, u32)> = Vec::new();
        for r in FIRST_USER..total as u32 {
            let rec = &self.recs[r as usize];
            if rec.flags & R_USED == 0 {
                continue;
            }
            for name in self.visible(r) {
                let n = &self.names[name as usize];
                let p = n.parent as usize;
                if p >= total || p == r as usize {
                    continue;
                }
                let pr = &self.recs[p];
                if pr.flags & (R_USED | R_DIR) != (R_USED | R_DIR) || pr.seq != n.parent_seq {
                    continue;
                }
                count[p] += 1;
                kids_of.push((n.parent, name));
            }
        }
        let mut start = vec![0u32; total + 1];
        let mut acc = 0u32;
        for i in 0..total {
            start[i] = acc;
            acc += count[i];
        }
        start[total] = acc;
        let mut fill = start.clone();
        let mut kids = vec![(0u32, 0u32); kids_of.len()];
        let owner_of = self.owners();
        for (parent, name) in kids_of {
            let slot = &mut fill[parent as usize];
            kids[*slot as usize] = (owner_of[name as usize], name);
            *slot += 1;
        }
        drop(fill);
        drop(count);
        let root_rec = self.recs[ROOT as usize];
        let top = b.push(NONE, root_name, flag::DIR | self.node_flags(ROOT) & !flag::LINK, 0, 0, root_rec.mtime);
        let mut entered = vec![false; total];
        let mut counted = vec![false; total];
        entered[ROOT as usize] = true;
        let mut queue = VecDeque::from([(top, ROOT)]);
        while let Some((node, r)) = queue.pop_front() {
            if b.overflowed() {
                return Err(TreeError::TooLarge);
            }
            let range = start[r as usize] as usize..start[r as usize + 1] as usize;
            let first = b.nodes.len() as u32;
            let mut n = 0u32;
            for &(child, name) in &kids[range] {
                let rec = self.recs[child as usize];
                let nm = &self.names[name as usize];
                let text = &self.pool[nm.off as usize..(nm.off + nm.len) as usize];
                let mut f = self.node_flags(child);
                let dir = rec.flags & R_DIR != 0;
                let (mut size, len) = if dir { (0, 0) } else { (rec.size, rec.len) };
                if f & (flag::CLOUD | flag::LINK) != 0 {
                    size = 0;
                }
                if !dir {
                    if counted[child as usize] {
                        f |= flag::HARDLINK;
                        size = 0;
                    }
                    counted[child as usize] = true;
                }
                let c = b.push(node, text, f, size, if f & flag::HARDLINK != 0 { 0 } else { len }, rec.mtime);
                n += 1;
                if dir && f & flag::LINK == 0 && !entered[child as usize] {
                    entered[child as usize] = true;
                    queue.push_back((c, child));
                }
            }
            if b.overflowed() {
                return Err(TreeError::TooLarge);
            }
            let nd = &mut b.nodes[node as usize];
            nd.first = if n > 0 { first } else { 0 };
            nd.count = n;
        }
        let mut ix = b.finish(vec![top], Stats::default());
        ix.sum_sizes();
        ix.recount();
        Ok(ix)
    }

    fn owners(&self) -> Vec<u32> {
        let mut out = vec![NONE; self.names.len()];
        for (r, rec) in self.recs.iter().enumerate() {
            let mut at = rec.first;
            while at != NONE {
                out[at as usize] = r as u32;
                at = self.names[at as usize].next;
            }
        }
        out
    }

    fn node_flags(&self, r: u32) -> u16 {
        let rec = &self.recs[r as usize];
        let mut f = 0u16;
        if rec.flags & R_DIR != 0 {
            f |= flag::DIR;
        }
        if rec.flags & R_HIDDEN != 0 {
            f |= flag::HIDDEN;
        }
        if rec.flags & R_SYSTEM != 0 {
            f |= flag::SYSTEM;
        }
        if rec.flags & R_CLOUD != 0 {
            f |= flag::CLOUD;
        }
        if rec.flags & R_LINK != 0 {
            f |= flag::LINK;
        }
        f
    }

    pub fn names_of(&self, r: u32) -> Vec<String> {
        self.links(r).map(|n| self.pool[n.off as usize..(n.off + n.len) as usize].to_string()).collect()
    }
}

fn unique(n: &Name, names: &[Name], pool: &str) -> bool {
    let text = &pool[n.off as usize..(n.off + n.len) as usize];
    let mut at = n.next;
    while at != NONE {
        let o = &names[at as usize];
        if o.parent == n.parent && o.ns != NS_DOS && &pool[o.off as usize..(o.off + o.len) as usize] == text {
            return false;
        }
        at = o.next;
    }
    true
}
