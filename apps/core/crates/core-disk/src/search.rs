use crate::fold::{cmp_ci, contains_ci, fold, glob_ci};
use crate::index::{flag, Index, Sort, NONE};
use std::cmp::Ordering as Cmp;
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Debug, Clone, PartialEq)]
enum Term {
    Sub(Vec<u8>),
    Glob(Vec<u8>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Only {
    Files,
    Dirs,
}

#[derive(Debug, Clone)]
pub struct Query {
    terms: Vec<Term>,
    pub min_size: Option<u64>,
    pub max_size: Option<u64>,
    pub after: Option<u32>,
    pub before: Option<u32>,
    pub group: Option<u8>,
    pub only: Option<Only>,
    pub under: Option<u32>,
    pub sort: Sort,
    pub limit: usize,
}

impl Query {
    pub fn new(text: &str) -> Self {
        let terms = text
            .split_whitespace()
            .map(|t| {
                let f = fold(t).into_bytes();
                if f.contains(&b'*') || f.contains(&b'?') {
                    Term::Glob(f)
                } else {
                    Term::Sub(f)
                }
            })
            .collect();
        Query { terms, min_size: None, max_size: None, after: None, before: None, group: None, only: None, under: None, sort: Sort::Size, limit: 100 }
    }
}

pub struct Found {
    pub total: usize,
    pub items: Vec<u32>,
}

const PAR_MIN: usize = 200_000;
const CHUNK: usize = 32_768;

fn matches(ix: &Index, q: &Query, i: u32) -> bool {
    let n = &ix.nodes[i as usize];
    if n.parent == NONE {
        return false;
    }
    let dir = n.flags & flag::DIR != 0;
    match q.only {
        Some(Only::Files) if dir => return false,
        Some(Only::Dirs) if !dir => return false,
        _ => {}
    }
    if let Some(g) = q.group {
        if dir || n.group != g {
            return false;
        }
    }
    if q.min_size.is_some_and(|m| n.size < m) || q.max_size.is_some_and(|m| n.size > m) {
        return false;
    }
    if q.after.is_some_and(|t| n.mtime < t) || q.before.is_some_and(|t| n.mtime > t) {
        return false;
    }
    if !q.terms.is_empty() {
        let name = ix.folded(i);
        for t in &q.terms {
            let ok = match t {
                Term::Sub(s) => contains_ci(name, s),
                Term::Glob(g) => glob_ci(name, g),
            };
            if !ok {
                return false;
            }
        }
    }
    true
}

fn better(ix: &Index, sort: Sort, a: u32, b: u32) -> Cmp {
    let (na, nb) = (&ix.nodes[a as usize], &ix.nodes[b as usize]);
    match sort {
        Sort::Size => nb.size.cmp(&na.size),
        Sort::Mtime => nb.mtime.cmp(&na.mtime),
        Sort::Name => cmp_ci(ix.folded(a), ix.folded(b)),
    }
    .then(a.cmp(&b))
}

struct Top<'a> {
    ix: &'a Index,
    sort: Sort,
    limit: usize,
    items: Vec<u32>,
    count: usize,
}

impl<'a> Top<'a> {
    fn new(ix: &'a Index, sort: Sort, limit: usize) -> Self {
        Top { ix, sort, limit, items: Vec::with_capacity(limit.min(4096) * 2), count: 0 }
    }

    fn add(&mut self, i: u32) {
        self.count += 1;
        if self.limit == 0 {
            return;
        }
        self.items.push(i);
        if self.items.len() >= self.limit * 2 + 64 {
            self.trim();
        }
    }

    fn trim(&mut self) {
        let (ix, sort, limit) = (self.ix, self.sort, self.limit);
        if self.items.len() > limit {
            self.items.select_nth_unstable_by(limit, |a, b| better(ix, sort, *a, *b));
            self.items.truncate(limit);
        }
    }

    fn scan<I: Iterator<Item = u32>>(&mut self, q: &Query, it: I) {
        for i in it {
            if matches(self.ix, q, i) {
                self.add(i);
            }
        }
    }
}

fn subtree(ix: &Index, root: u32) -> Vec<u32> {
    let mut out = Vec::new();
    let mut stack = vec![root];
    while let Some(d) = stack.pop() {
        for c in ix.children(d) {
            out.push(c);
            if ix.nodes[c as usize].count > 0 {
                stack.push(c);
            }
        }
    }
    out
}

fn threads() -> usize {
    (std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4) / 2).max(1)
}

pub fn search(ix: &Index, q: &Query) -> Found {
    let list = q.under.map(|u| subtree(ix, u));
    let total = list.as_ref().map(|l| l.len()).unwrap_or(ix.nodes.len());
    let threads = threads();
    let (count, mut items) = if total < PAR_MIN || threads < 2 {
        let mut top = Top::new(ix, q.sort, q.limit);
        match &list {
            Some(l) => top.scan(q, l.iter().copied()),
            None => top.scan(q, 0..ix.nodes.len() as u32),
        }
        top.trim();
        (top.count, top.items)
    } else {
        let next = AtomicUsize::new(0);
        let chunks = total.div_ceil(CHUNK);
        let parts: Vec<(usize, Vec<u32>)> = std::thread::scope(|s| {
            let handles: Vec<_> = (0..threads)
                .map(|_| {
                    s.spawn(|| {
                        let mut top = Top::new(ix, q.sort, q.limit);
                        loop {
                            let c = next.fetch_add(1, Ordering::Relaxed);
                            if c >= chunks {
                                break;
                            }
                            let (a, b) = (c * CHUNK, ((c + 1) * CHUNK).min(total));
                            match &list {
                                Some(l) => top.scan(q, l[a..b].iter().copied()),
                                None => top.scan(q, a as u32..b as u32),
                            }
                        }
                        top.trim();
                        (top.count, top.items)
                    })
                })
                .collect();
            handles.into_iter().filter_map(|h| h.join().ok()).collect()
        });
        let count = parts.iter().map(|p| p.0).sum();
        (count, parts.into_iter().flat_map(|p| p.1).collect())
    };
    items.sort_unstable_by(|a, b| better(ix, q.sort, *a, *b));
    items.truncate(q.limit);
    Found { total: count, items }
}
