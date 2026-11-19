use crate::index::{Index, Node, Stats, NONE};
use std::io::{Error, ErrorKind, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

pub const FILE: &str = "index.bin";
pub const MAGIC: &[u8; 8] = b"ORRDISK\0";
pub const VERSION: u32 = 1;
const NODE_BYTES: usize = 43;
pub const MAX_DECODED: u64 = 4 << 30;

static SAVE_LOCK: Mutex<()> = Mutex::new(());
static SAVE_SEQ: AtomicU64 = AtomicU64::new(0);

pub fn file_in(dir: &Path) -> PathBuf {
    dir.join(FILE)
}

fn bad(msg: &str) -> Error {
    Error::new(ErrorKind::InvalidData, msg.to_string())
}

pub fn encode(ix: &Index) -> Vec<u8> {
    let mut b = Vec::with_capacity(64 + ix.nodes.len() * NODE_BYTES + ix.pool.len() + ix.roots.len() * 4);
    let s = &ix.stats;
    for v in [s.files, s.dirs, s.bytes, s.denied, s.errors, s.elapsed_ms, s.scanned_at] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&(ix.roots.len() as u32).to_le_bytes());
    for r in &ix.roots {
        b.extend_from_slice(&r.to_le_bytes());
    }
    b.extend_from_slice(&(ix.nodes.len() as u64).to_le_bytes());
    b.extend_from_slice(&(ix.pool.len() as u64).to_le_bytes());
    for n in &ix.nodes {
        b.extend_from_slice(&n.parent.to_le_bytes());
        b.extend_from_slice(&n.name_off.to_le_bytes());
        b.extend_from_slice(&n.name_len.to_le_bytes());
        b.extend_from_slice(&n.fold_len.to_le_bytes());
        b.extend_from_slice(&n.flags.to_le_bytes());
        b.push(n.group);
        b.extend_from_slice(&n.first.to_le_bytes());
        b.extend_from_slice(&n.count.to_le_bytes());
        b.extend_from_slice(&n.size.to_le_bytes());
        b.extend_from_slice(&n.len.to_le_bytes());
        b.extend_from_slice(&n.mtime.to_le_bytes());
    }
    b.extend_from_slice(&ix.pool);
    b
}

struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], Error> {
        let end = self.at.checked_add(n).filter(|e| *e <= self.b.len()).ok_or_else(|| bad("truncated"))?;
        let s = &self.b[self.at..end];
        self.at = end;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8, Error> {
        Ok(self.take(1)?[0])
    }
    fn u16(&mut self) -> Result<u16, Error> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().unwrap()))
    }
    fn u32(&mut self) -> Result<u32, Error> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().unwrap()))
    }
    fn u64(&mut self) -> Result<u64, Error> {
        Ok(u64::from_le_bytes(self.take(8)?.try_into().unwrap()))
    }
}

pub fn decode(b: &[u8]) -> Result<Index, Error> {
    let mut r = Reader { b, at: 0 };
    let mut v = [0u64; 7];
    for x in v.iter_mut() {
        *x = r.u64()?;
    }
    let stats = Stats { files: v[0], dirs: v[1], bytes: v[2], denied: v[3], errors: v[4], elapsed_ms: v[5], scanned_at: v[6] };
    let nroots = r.u32()? as usize;
    if nroots > b.len() / 4 {
        return Err(bad("roots"));
    }
    let mut roots = Vec::with_capacity(nroots);
    for _ in 0..nroots {
        roots.push(r.u32()?);
    }
    let count = r.u64()? as usize;
    let pool_len = r.u64()? as usize;
    if count.checked_mul(NODE_BYTES).and_then(|n| n.checked_add(pool_len)).is_none_or(|n| n > b.len() - r.at) || count >= NONE as usize {
        return Err(bad("size"));
    }
    let mut nodes = Vec::with_capacity(count);
    for _ in 0..count {
        nodes.push(Node {
            parent: r.u32()?,
            name_off: r.u32()?,
            name_len: r.u16()?,
            fold_len: r.u16()?,
            flags: r.u16()?,
            group: r.u8()?,
            first: r.u32()?,
            count: r.u32()?,
            size: r.u64()?,
            len: r.u64()?,
            mtime: r.u32()?,
        });
    }
    let pool_bytes = r.take(pool_len)?;
    let text = std::str::from_utf8(pool_bytes).map_err(|_| bad("names"))?;
    for (i, n) in nodes.iter().enumerate() {
        let name_end = n.name_off as usize + n.name_len as usize + n.fold_len as usize;
        let kids_end = n.first as u64 + n.count as u64;
        let parent_ok = if n.parent == NONE { roots.contains(&(i as u32)) } else { (n.parent as usize) < i };
        if name_end > pool_len || kids_end > count as u64 || !parent_ok || (n.count > 0 && (n.first as usize) <= i) {
            return Err(bad("node"));
        }
        if !text.is_char_boundary(n.name_off as usize) || !text.is_char_boundary(n.name_off as usize + n.name_len as usize) || !text.is_char_boundary(name_end) {
            return Err(bad("node"));
        }
    }
    let pool = pool_bytes.to_vec();
    if roots.iter().any(|r| *r as usize >= count || nodes[*r as usize].parent != NONE) {
        return Err(bad("roots"));
    }
    Ok(Index { nodes, pool, roots, stats })
}

pub fn save(ix: &Index, dir: &Path) -> Result<(PathBuf, u64), Error> {
    std::fs::create_dir_all(dir)?;
    let body = zstd::stream::encode_all(&encode(ix)[..], 3)?;
    let mut out = Vec::with_capacity(body.len() + 12);
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    out.extend_from_slice(&body);
    let file = file_in(dir);
    let _guard = SAVE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let tmp = dir.join(format!("{FILE}.{}.{}.tmp", std::process::id(), SAVE_SEQ.fetch_add(1, Ordering::Relaxed)));
    let written = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(&out)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, &file)
    })();
    if let Err(e) = written {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    Ok((file, out.len() as u64))
}

pub fn load(dir: &Path) -> Result<Option<Index>, Error> {
    let file = file_in(dir);
    let raw = match std::fs::read(&file) {
        Ok(r) => r,
        Err(e) if e.kind() == ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    if raw.len() < 12 || &raw[..8] != MAGIC {
        return Err(bad("magic"));
    }
    let version = u32::from_le_bytes(raw[8..12].try_into().unwrap());
    if version != VERSION {
        return Err(bad("version"));
    }
    let body = inflate(&raw[12..], MAX_DECODED)?;
    decode(&body).map(Some)
}

pub fn inflate(body: &[u8], limit: u64) -> Result<Vec<u8>, Error> {
    let mut out = Vec::new();
    zstd::stream::read::Decoder::new(body)?.take(limit.saturating_add(1)).read_to_end(&mut out)?;
    if out.len() as u64 > limit {
        return Err(bad("too large"));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inflate_respects_limit() {
        let body = zstd::stream::encode_all(&vec![0u8; 100_000][..], 3).unwrap();
        assert_eq!(inflate(&body, 100_000).unwrap().len(), 100_000);
        assert!(inflate(&body, 99_999).is_err());
        assert!(inflate(&body, 10).is_err());
    }

    #[test]
    fn concurrent_saves_stay_readable() {
        let dir = std::env::temp_dir().join(format!("core-disk-store-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let ix = Index::default();
        let hs: Vec<_> = (0..8).map(|_| {
            let (d, i) = (dir.clone(), ix.clone());
            std::thread::spawn(move || save(&i, &d).unwrap())
        }).collect();
        for h in hs {
            h.join().unwrap();
        }
        assert!(load(&dir).unwrap().is_some());
        let leftovers = std::fs::read_dir(&dir).unwrap().filter(|e| e.as_ref().unwrap().file_name().to_string_lossy().ends_with(".tmp")).count();
        assert_eq!(leftovers, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
