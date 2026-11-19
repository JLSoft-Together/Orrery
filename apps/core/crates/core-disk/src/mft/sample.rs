use super::record::{ATTR_DATA, ATTR_END, ATTR_FILE_NAME, ATTR_LIST, ATTR_REPARSE, ATTR_STANDARD, NS_BOTH, NS_DOS, NS_POSIX, NS_WIN32, STRIDE};

pub const RECORD: usize = 1024;
pub const MTIME_FT: u64 = 133_000_000_000_000_000;

#[derive(Debug, Clone)]
pub enum Data {
    Resident(usize),
    Runs { alloc: u64, len: u64, compressed: Option<u64>, vcn: u64, runs: Vec<u8> },
}

#[derive(Debug, Clone)]
pub struct Spec {
    pub seq: u16,
    pub in_use: bool,
    pub dir: bool,
    pub base: u64,
    pub attrs: Option<u32>,
    pub names: Vec<(u64, u16, u8, String)>,
    pub data: Option<Data>,
    pub reparse: Option<u32>,
    pub list: bool,
}

impl Spec {
    pub fn file(parent: u64, name: &str) -> Spec {
        Spec { seq: 1, in_use: true, dir: false, base: 0, attrs: Some(0x20), names: vec![(parent, 1, NS_BOTH, name.to_string())], data: None, reparse: None, list: false }
    }

    pub fn dir(parent: u64, name: &str) -> Spec {
        Spec { dir: true, attrs: Some(0x10), ..Spec::file(parent, name) }
    }

    pub fn alloc(mut self, alloc: u64, len: u64) -> Spec {
        self.data = Some(Data::Runs { alloc, len, compressed: None, vcn: 0, runs: vec![0x11, 0x01, 0x20, 0x00] });
        self
    }
}

fn pad8(v: &mut Vec<u8>) {
    while !v.len().is_multiple_of(8) {
        v.push(0);
    }
}

fn resident(kind: u32, id: u16, value: &[u8]) -> Vec<u8> {
    let mut a = vec![0u8; 0x18];
    a[0..4].copy_from_slice(&kind.to_le_bytes());
    a[8] = 0;
    a[0x0A..0x0C].copy_from_slice(&0x18u16.to_le_bytes());
    a[0x0E..0x10].copy_from_slice(&id.to_le_bytes());
    a[0x10..0x14].copy_from_slice(&(value.len() as u32).to_le_bytes());
    a[0x14..0x16].copy_from_slice(&0x18u16.to_le_bytes());
    a.extend_from_slice(value);
    pad8(&mut a);
    let len = a.len() as u32;
    a[4..8].copy_from_slice(&len.to_le_bytes());
    a
}

fn non_resident(id: u16, alloc: u64, len: u64, compressed: Option<u64>, vcn: u64, runs: &[u8]) -> Vec<u8> {
    let head = if compressed.is_some() { 0x48 } else { 0x40 };
    let mut a = vec![0u8; head];
    a[0..4].copy_from_slice(&ATTR_DATA.to_le_bytes());
    a[8] = 1;
    a[0x0A..0x0C].copy_from_slice(&(head as u16).to_le_bytes());
    if compressed.is_some() {
        a[0x0C..0x0E].copy_from_slice(&0x0001u16.to_le_bytes());
        a[0x22..0x24].copy_from_slice(&4u16.to_le_bytes());
    }
    a[0x0E..0x10].copy_from_slice(&id.to_le_bytes());
    a[0x10..0x18].copy_from_slice(&vcn.to_le_bytes());
    a[0x20..0x22].copy_from_slice(&(head as u16).to_le_bytes());
    a[0x28..0x30].copy_from_slice(&alloc.to_le_bytes());
    a[0x30..0x38].copy_from_slice(&len.to_le_bytes());
    a[0x38..0x40].copy_from_slice(&len.to_le_bytes());
    if let Some(c) = compressed {
        a[0x40..0x48].copy_from_slice(&c.to_le_bytes());
    }
    a.extend_from_slice(runs);
    pad8(&mut a);
    let n = a.len() as u32;
    a[4..8].copy_from_slice(&n.to_le_bytes());
    a
}

fn standard(attrs: u32) -> Vec<u8> {
    let mut v = vec![0u8; 0x48];
    for at in [0x00, 0x08, 0x10, 0x18] {
        v[at..at + 8].copy_from_slice(&MTIME_FT.to_le_bytes());
    }
    v[0x20..0x24].copy_from_slice(&attrs.to_le_bytes());
    v
}

fn file_name(parent: u64, parent_seq: u16, ns: u8, name: &str, dir: bool) -> Vec<u8> {
    let units: Vec<u16> = name.encode_utf16().collect();
    let mut v = vec![0u8; 0x42];
    let r = (parent & 0x0000_FFFF_FFFF_FFFF) | ((parent_seq as u64) << 48);
    v[0..8].copy_from_slice(&r.to_le_bytes());
    v[0x38..0x3C].copy_from_slice(&(if dir { 0x1000_0000u32 } else { 0x20 }).to_le_bytes());
    v[0x40] = units.len() as u8;
    v[0x41] = ns;
    for u in units {
        v.extend_from_slice(&u.to_le_bytes());
    }
    v
}

pub fn record(spec: &Spec, number: u32) -> Vec<u8> {
    let size = RECORD;
    let usa_count = size / STRIDE + 1;
    let usa_off = 0x30usize;
    let first = (usa_off + usa_count * 2).div_ceil(8) * 8;
    let mut r = vec![0u8; size];
    r[0..4].copy_from_slice(b"FILE");
    r[4..6].copy_from_slice(&(usa_off as u16).to_le_bytes());
    r[6..8].copy_from_slice(&(usa_count as u16).to_le_bytes());
    r[0x10..0x12].copy_from_slice(&spec.seq.to_le_bytes());
    r[0x12..0x14].copy_from_slice(&(spec.names.len() as u16).to_le_bytes());
    r[0x14..0x16].copy_from_slice(&(first as u16).to_le_bytes());
    let flags = u16::from(spec.in_use) | if spec.dir { 2 } else { 0 };
    r[0x16..0x18].copy_from_slice(&flags.to_le_bytes());
    r[0x1C..0x20].copy_from_slice(&(size as u32).to_le_bytes());
    let base = if spec.base != 0 { spec.base | (1u64 << 48) } else { 0 };
    r[0x20..0x28].copy_from_slice(&base.to_le_bytes());
    r[0x2C..0x30].copy_from_slice(&number.to_le_bytes());
    let mut attrs: Vec<Vec<u8>> = Vec::new();
    let mut id = 0u16;
    let mut next_id = || {
        id += 1;
        id
    };
    if let Some(a) = spec.attrs {
        attrs.push(resident(ATTR_STANDARD, next_id(), &standard(a)));
    }
    if spec.list {
        attrs.push(resident(ATTR_LIST, next_id(), &[0u8; 0x20]));
    }
    for (parent, pseq, ns, name) in &spec.names {
        attrs.push(resident(ATTR_FILE_NAME, next_id(), &file_name(*parent, *pseq, *ns, name, spec.dir)));
    }
    match &spec.data {
        Some(Data::Resident(n)) => attrs.push(resident(ATTR_DATA, next_id(), &vec![b'x'; *n])),
        Some(Data::Runs { alloc, len, compressed, vcn, runs }) => attrs.push(non_resident(next_id(), *alloc, *len, *compressed, *vcn, runs)),
        None => {}
    }
    if let Some(tag) = spec.reparse {
        let mut v = tag.to_le_bytes().to_vec();
        v.extend_from_slice(&[0u8; 4]);
        attrs.push(resident(ATTR_REPARSE, next_id(), &v));
    }
    let mut at = first;
    for a in attrs {
        r[at..at + a.len()].copy_from_slice(&a);
        at += a.len();
    }
    r[at..at + 4].copy_from_slice(&ATTR_END.to_le_bytes());
    at += 8;
    r[0x18..0x1C].copy_from_slice(&(at as u32).to_le_bytes());
    let usn = [0x07u8, 0x00];
    r[usa_off..usa_off + 2].copy_from_slice(&usn);
    for i in 1..usa_count {
        let end = i * STRIDE - 2;
        r[usa_off + i * 2] = r[end];
        r[usa_off + i * 2 + 1] = r[end + 1];
        r[end..end + 2].copy_from_slice(&usn);
    }
    r
}

pub fn records() -> Vec<(u32, Spec)> {
    let mut deleted = Spec::file(17, "ghost.txt").alloc(4096, 10);
    deleted.in_use = false;
    let mut dos = Spec::file(17, "Ảnh dài tên.jpg").alloc(4096, 4000);
    dos.names[0].2 = NS_WIN32;
    dos.names.push((17, 1, NS_DOS, "ANHDAI~1.JPG".into()));
    let mut linked = Spec::file(17, "link1.dat").alloc(65536, 60000);
    linked.names.push((16, 1, NS_BOTH, "link2.dat".into()));
    let mut cloud = Spec::file(16, "cloud.docx").alloc(4096, 3000);
    cloud.attrs = Some(0x0040_0020);
    let mut junction = Spec::dir(5, "Link");
    junction.reparse = Some(0xA000_0003);
    let mut ext = Spec::file(0, "");
    ext.names.clear();
    ext.attrs = None;
    ext.base = 27;
    ext.data = Some(Data::Runs { alloc: 1 << 20, len: 1_000_000, compressed: None, vcn: 0, runs: vec![0x11, 0x10, 0x40, 0x00] });
    let mut big = Spec::file(5, "big.iso");
    big.list = true;
    let mut orphan = Spec::file(17, "orphan.txt").alloc(4096, 1);
    orphan.names[0].1 = 9;
    let mut packed = Spec::file(5, "packed.log");
    packed.data = Some(Data::Runs { alloc: 65536, len: 100_000, compressed: Some(16384), vcn: 0, runs: vec![0x11, 0x10, 0x50, 0x00] });
    let mut posix = Spec::file(5, "Case");
    posix.names[0].2 = NS_POSIX;
    posix.attrs = Some(0x26);
    posix.data = Some(Data::Resident(10));
    let mut tiny = Spec::file(5, "tiny.txt");
    tiny.data = Some(Data::Resident(100));
    let mut root = Spec::dir(5, ".");
    root.attrs = Some(0x16);
    vec![
        (0, Spec::file(5, "$MFT").alloc(1 << 18, 1 << 18)),
        (1, Spec::file(5, "$MFTMirr").alloc(4096, 4096)),
        (5, root),
        (11, Spec::dir(5, "$Extend")),
        (16, Spec::dir(5, "Users")),
        (17, Spec::dir(16, "jin")),
        (18, Spec::file(17, "a.bin").alloc(8192, 5000)),
        (19, dos),
        (20, linked),
        (21, deleted),
        (22, tiny),
        (23, cloud),
        (24, junction),
        (25, Spec::file(11, "$UsnJrnl").alloc(1 << 20, 1 << 20)),
        (26, ext),
        (27, big),
        (28, orphan),
        (29, packed),
        (30, posix),
    ]
}

pub const EXPECTED_BYTES: u64 = 8192 + 4096 + 65536 + (1 << 20) + 16384;

pub fn image() -> Vec<u8> {
    let list = records();
    let count = list.iter().map(|(n, _)| *n as usize + 1).max().unwrap_or(0);
    let mut out = vec![0u8; count * RECORD];
    for (n, spec) in &list {
        let r = record(spec, *n);
        out[*n as usize * RECORD..(*n as usize + 1) * RECORD].copy_from_slice(&r);
    }
    out
}
