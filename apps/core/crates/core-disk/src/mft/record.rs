pub const STRIDE: usize = 512;
pub const ATTR_STANDARD: u32 = 0x10;
pub const ATTR_LIST: u32 = 0x20;
pub const ATTR_FILE_NAME: u32 = 0x30;
pub const ATTR_DATA: u32 = 0x80;
pub const ATTR_REPARSE: u32 = 0xC0;
pub const ATTR_END: u32 = 0xFFFF_FFFF;
pub const NS_POSIX: u8 = 0;
pub const NS_WIN32: u8 = 1;
pub const NS_DOS: u8 = 2;
pub const NS_BOTH: u8 = 3;
pub const REF_MASK: u64 = 0x0000_FFFF_FFFF_FFFF;
const FLAG_IN_USE: u16 = 1;
const FLAG_DIR: u16 = 2;
const DATA_COMPRESSED: u16 = 0x0001;
const DATA_SPARSE: u16 = 0x8000;
const EPOCH_DIFF: u64 = 116_444_736_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordError {
    Magic,
    Fixup,
    Bounds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FileName {
    pub parent: u64,
    pub parent_seq: u16,
    pub ns: u8,
    pub start: u32,
    pub len: u32,
}

#[derive(Debug, Clone, Default)]
pub struct Parsed {
    pub in_use: bool,
    pub dir: bool,
    pub seq: u16,
    pub base: u64,
    pub attrs: Option<u32>,
    pub mtime: u32,
    pub reparse: Option<u32>,
    pub data: Option<(u64, u64)>,
    pub list: bool,
    pub runs: Option<(usize, usize)>,
    pub names: Vec<FileName>,
    pub text: String,
}

impl Parsed {
    pub fn name(&self, n: &FileName) -> &str {
        &self.text[n.start as usize..(n.start + n.len) as usize]
    }

    fn clear(&mut self) {
        self.in_use = false;
        self.dir = false;
        self.seq = 0;
        self.base = 0;
        self.attrs = None;
        self.mtime = 0;
        self.reparse = None;
        self.data = None;
        self.list = false;
        self.runs = None;
        self.names.clear();
        self.text.clear();
    }
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    b.get(at..at.checked_add(2)?).map(|s| u16::from_le_bytes([s[0], s[1]]))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    b.get(at..at.checked_add(4)?).map(|s| u32::from_le_bytes(s.try_into().unwrap()))
}

fn u64_at(b: &[u8], at: usize) -> Option<u64> {
    b.get(at..at.checked_add(8)?).map(|s| u64::from_le_bytes(s.try_into().unwrap()))
}

pub fn filetime_secs(t: u64) -> u32 {
    (t.saturating_sub(EPOCH_DIFF) / 10_000_000).min(u32::MAX as u64) as u32
}

pub fn apply_fixup(rec: &mut [u8]) -> Result<(), RecordError> {
    if rec.len() < 48 || &rec[0..4] != b"FILE" {
        return Err(RecordError::Magic);
    }
    let usa_off = u16_at(rec, 4).ok_or(RecordError::Bounds)? as usize;
    let usa_count = u16_at(rec, 6).ok_or(RecordError::Bounds)? as usize;
    if usa_count < 2 || (usa_count - 1) * STRIDE != rec.len() || usa_off + usa_count * 2 > rec.len() || usa_off < 0x28 {
        return Err(RecordError::Fixup);
    }
    let usn = [rec[usa_off], rec[usa_off + 1]];
    for i in 1..usa_count {
        let end = i * STRIDE - 2;
        if rec[end..end + 2] != usn {
            return Err(RecordError::Fixup);
        }
        let fix = usa_off + i * 2;
        let (a, b) = (rec[fix], rec[fix + 1]);
        rec[end] = a;
        rec[end + 1] = b;
    }
    Ok(())
}

pub fn parse(rec: &mut [u8], out: &mut Parsed) -> Result<(), RecordError> {
    out.clear();
    apply_fixup(rec)?;
    parse_fixed(rec, out)
}

pub fn parse_fixed(rec: &[u8], out: &mut Parsed) -> Result<(), RecordError> {
    out.clear();
    let flags = u16_at(rec, 0x16).ok_or(RecordError::Bounds)?;
    out.in_use = flags & FLAG_IN_USE != 0;
    out.dir = flags & FLAG_DIR != 0;
    out.seq = u16_at(rec, 0x10).ok_or(RecordError::Bounds)?;
    out.base = u64_at(rec, 0x20).ok_or(RecordError::Bounds)? & REF_MASK;
    if !out.in_use {
        return Ok(());
    }
    let used = (u32_at(rec, 0x18).ok_or(RecordError::Bounds)? as usize).min(rec.len());
    let mut at = u16_at(rec, 0x14).ok_or(RecordError::Bounds)? as usize;
    while at + 8 <= used {
        let kind = u32_at(rec, at).ok_or(RecordError::Bounds)?;
        if kind == ATTR_END {
            break;
        }
        let len = u32_at(rec, at + 4).ok_or(RecordError::Bounds)? as usize;
        if len < 0x18 || at + len > used || !len.is_multiple_of(8) {
            return Err(RecordError::Bounds);
        }
        attribute(&rec[at..at + len], at, kind, out)?;
        at += len;
    }
    Ok(())
}

fn resident(a: &[u8]) -> Option<&[u8]> {
    if a[8] != 0 {
        return None;
    }
    let vlen = u32_at(a, 0x10)? as usize;
    let voff = u16_at(a, 0x14)? as usize;
    a.get(voff..voff.checked_add(vlen)?)
}

fn attribute(a: &[u8], base: usize, kind: u32, out: &mut Parsed) -> Result<(), RecordError> {
    let named = a[9] != 0;
    match kind {
        ATTR_STANDARD => {
            let v = resident(a).ok_or(RecordError::Bounds)?;
            out.mtime = filetime_secs(u64_at(v, 0x08).ok_or(RecordError::Bounds)?);
            out.attrs = Some(u32_at(v, 0x20).ok_or(RecordError::Bounds)?);
        }
        ATTR_LIST => out.list = true,
        ATTR_FILE_NAME => {
            let v = resident(a).ok_or(RecordError::Bounds)?;
            let parent = u64_at(v, 0).ok_or(RecordError::Bounds)?;
            let chars = *v.get(0x40).ok_or(RecordError::Bounds)? as usize;
            let ns = *v.get(0x41).ok_or(RecordError::Bounds)?;
            let raw = v.get(0x42..0x42 + chars * 2).ok_or(RecordError::Bounds)?;
            let units = raw.as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c));
            let start = out.text.len() as u32;
            out.text.extend(char::decode_utf16(units).map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER)));
            out.names.push(FileName { parent: parent & REF_MASK, parent_seq: (parent >> 48) as u16, ns, start, len: out.text.len() as u32 - start });
        }
        ATTR_DATA if !named => {
            if a[8] == 0 {
                let v = resident(a).ok_or(RecordError::Bounds)?;
                out.data = Some((0, v.len() as u64));
            } else {
                let vcn = u64_at(a, 0x10).ok_or(RecordError::Bounds)?;
                if vcn == 0 {
                    let runs = u16_at(a, 0x20).ok_or(RecordError::Bounds)? as usize;
                    let flags = u16_at(a, 0x0C).ok_or(RecordError::Bounds)?;
                    let mut alloc = u64_at(a, 0x28).ok_or(RecordError::Bounds)?;
                    let len = u64_at(a, 0x30).ok_or(RecordError::Bounds)?;
                    if flags & (DATA_COMPRESSED | DATA_SPARSE) != 0 && runs >= 0x48 {
                        alloc = u64_at(a, 0x40).ok_or(RecordError::Bounds)?;
                    }
                    out.data = Some((alloc, len));
                    if runs < a.len() {
                        out.runs = Some((base + runs, base + a.len()));
                    }
                }
            }
        }
        ATTR_REPARSE => out.reparse = Some(resident(a).and_then(|v| u32_at(v, 0)).unwrap_or(0)),
        _ => {}
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Run {
    pub lcn: i64,
    pub clusters: u64,
}

pub fn data_runs(b: &[u8]) -> Option<Vec<Run>> {
    let mut out = Vec::new();
    let mut at = 0usize;
    let mut lcn: i64 = 0;
    loop {
        let h = *b.get(at)?;
        if h == 0 {
            return Some(out);
        }
        let (ln, on) = ((h & 0x0F) as usize, (h >> 4) as usize);
        if ln == 0 || ln > 8 || on > 8 {
            return None;
        }
        let lb = b.get(at + 1..at + 1 + ln)?;
        let mut clusters = 0u64;
        for (i, x) in lb.iter().enumerate() {
            clusters |= (*x as u64) << (8 * i);
        }
        if on == 0 {
            return None;
        }
        let ob = b.get(at + 1 + ln..at + 1 + ln + on)?;
        let mut delta = 0i64;
        for (i, x) in ob.iter().enumerate() {
            delta |= (*x as i64) << (8 * i);
        }
        if on < 8 && ob[on - 1] & 0x80 != 0 {
            delta |= -1i64 << (8 * on);
        }
        lcn = lcn.checked_add(delta)?;
        if lcn < 0 {
            return None;
        }
        out.push(Run { lcn, clusters });
        at += 1 + ln + on;
    }
}
