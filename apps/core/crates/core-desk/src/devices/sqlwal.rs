const WAL_HEADER: usize = 32;
const FRAME_HEADER: usize = 24;

pub struct WalApplied {
    pub db: Vec<u8>,
    pub frames: u64,
    pub partial: bool,
}

fn be(b: &[u8], at: usize) -> u32 {
    u32::from_be_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn word(b: &[u8], at: usize, big: bool) -> u32 {
    let a = [b[at], b[at + 1], b[at + 2], b[at + 3]];
    if big {
        u32::from_be_bytes(a)
    } else {
        u32::from_le_bytes(a)
    }
}

fn checksum(buf: &[u8], start: usize, end: usize, big: bool, s: (u32, u32)) -> (u32, u32) {
    let (mut s0, mut s1) = s;
    let mut i = start;
    while i < end {
        s0 = s0.wrapping_add(word(buf, i, big)).wrapping_add(s1);
        s1 = s1.wrapping_add(word(buf, i + 4, big)).wrapping_add(s0);
        i += 8;
    }
    (s0, s1)
}

pub fn is_sqlite(b: &[u8]) -> bool {
    b.len() >= 100 && &b[..16] == b"SQLite format 3\0"
}

pub fn apply_wal(db: &[u8], wal: Option<&[u8]>) -> WalApplied {
    let mut out = db.to_vec();
    let (mut frames, mut partial) = (0u64, false);
    if let Some(wal) = wal.filter(|w| w.len() >= WAL_HEADER) {
        let magic = be(wal, 0);
        let page = be(wal, 8) as usize;
        let valid = (512..=65536).contains(&page) && page.is_power_of_two();
        if (magic == 0x377f0682 || magic == 0x377f0683) && valid {
            let big = magic == 0x377f0683;
            let mut sum = checksum(wal, 0, 24, big, (0, 0));
            if sum.0 == be(wal, 24) && sum.1 == be(wal, 28) {
                let (salt1, salt2) = (be(wal, 16), be(wal, 20));
                let mut pages: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
                let mut pending: Vec<(u32, usize)> = Vec::new();
                let mut db_pages = 0u32;
                let mut at = WAL_HEADER;
                while at + FRAME_HEADER + page <= wal.len() {
                    if be(wal, at + 8) != salt1 || be(wal, at + 12) != salt2 {
                        break;
                    }
                    sum = checksum(wal, at, at + 8, big, sum);
                    sum = checksum(wal, at + FRAME_HEADER, at + FRAME_HEADER + page, big, sum);
                    if sum.0 != be(wal, at + 16) || sum.1 != be(wal, at + 20) {
                        partial = true;
                        break;
                    }
                    pending.push((be(wal, at), at + FRAME_HEADER));
                    let commit = be(wal, at + 4);
                    if commit != 0 {
                        for (p, off) in pending.drain(..) {
                            pages.insert(p, off);
                            frames += 1;
                        }
                        db_pages = commit;
                    }
                    at += FRAME_HEADER + page;
                }
                if db_pages != 0 {
                    let mut next = vec![0u8; db_pages as usize * page];
                    let keep = out.len().min(next.len());
                    next[..keep].copy_from_slice(&out[..keep]);
                    for (p, off) in pages {
                        if p >= 1 && p <= db_pages {
                            let dst = (p as usize - 1) * page;
                            next[dst..dst + page].copy_from_slice(&wal[off..off + page]);
                        }
                    }
                    out = next;
                }
            }
        }
    }
    if is_sqlite(&out) {
        out[18] = 1;
        out[19] = 1;
    }
    WalApplied { db: out, frames, partial }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn without_wal_only_switches_journal_mode() {
        let mut db = vec![0u8; 100];
        db[..16].copy_from_slice(b"SQLite format 3\0");
        db[18] = 2;
        db[19] = 2;
        let r = apply_wal(&db, None);
        assert_eq!((r.db[18], r.db[19], r.frames, r.partial), (1, 1, 0, false));
        assert!(!is_sqlite(&[0u8; 10]));
    }
}
