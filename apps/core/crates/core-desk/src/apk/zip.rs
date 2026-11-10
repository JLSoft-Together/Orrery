use flate2::{Decompress, FlushDecompress};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

pub const MAX_ENTRIES: usize = 200_000;
pub const MAX_CD_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApkError {
    NotZip,
    Zip64,
    TooLarge,
    ReadFailed,
}

impl ApkError {
    pub fn code(self) -> &'static str {
        match self {
            ApkError::NotZip => "not-zip",
            ApkError::Zip64 => "zip64",
            ApkError::TooLarge => "too-large",
            ApkError::ReadFailed => "read-failed",
        }
    }
}

pub struct Entry {
    pub name: String,
    pub method: u16,
    pub compressed: u64,
    pub size: u64,
    pub local_offset: u64,
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

fn read_at(f: &mut File, at: u64, len: usize) -> Result<Vec<u8>, ApkError> {
    let mut buf = vec![0u8; len];
    f.seek(SeekFrom::Start(at)).map_err(|_| ApkError::ReadFailed)?;
    let mut got = 0;
    while got < len {
        match f.read(&mut buf[got..]) {
            Ok(0) => break,
            Ok(n) => got += n,
            Err(_) => return Err(ApkError::ReadFailed),
        }
    }
    buf.truncate(got);
    Ok(buf)
}

fn find_eocd(buf: &[u8]) -> Option<usize> {
    if buf.len() < 22 {
        return None;
    }
    (0..=buf.len() - 22).rev().find(|&i| u32_at(buf, i) == 0x0605_4b50)
}

fn central_directory(buf: &[u8], file_size: u64) -> Result<Vec<Entry>, ApkError> {
    let eocd = find_eocd(buf).ok_or(ApkError::NotZip)?;
    let count = u16_at(buf, eocd + 10) as usize;
    let cd_size = u32_at(buf, eocd + 12) as u64;
    let cd_offset = u32_at(buf, eocd + 16) as u64;
    if count == 0xffff || cd_offset == 0xffff_ffff {
        return Err(ApkError::Zip64);
    }
    let tail_start = file_size - buf.len() as u64;
    if cd_offset < tail_start || cd_offset - tail_start + cd_size > buf.len() as u64 {
        return Err(ApkError::NotZip);
    }
    let mut at = (cd_offset - tail_start) as usize;
    let mut out = Vec::new();
    for _ in 0..count {
        if at + 46 > buf.len() || u32_at(buf, at) != 0x0201_4b50 {
            return Err(ApkError::NotZip);
        }
        let name_len = u16_at(buf, at + 28) as usize;
        let extra = u16_at(buf, at + 30) as usize;
        let comment = u16_at(buf, at + 32) as usize;
        let compressed = u32_at(buf, at + 20);
        let size = u32_at(buf, at + 24);
        let local_offset = u32_at(buf, at + 42);
        if compressed == 0xffff_ffff || size == 0xffff_ffff || local_offset == 0xffff_ffff {
            return Err(ApkError::Zip64);
        }
        let name_end = (at + 46 + name_len).min(buf.len());
        out.push(Entry { name: String::from_utf8_lossy(&buf[at + 46..name_end]).to_string(), method: u16_at(buf, at + 10), compressed: compressed as u64, size: size as u64, local_offset: local_offset as u64 });
        at += 46 + name_len + extra + comment;
    }
    Ok(out)
}

pub fn read_entries(f: &mut File, file_size: u64) -> Result<Vec<Entry>, ApkError> {
    let tail_len = file_size.min(22 + 65535) as usize;
    if tail_len < 22 {
        return Err(ApkError::NotZip);
    }
    let tail = read_at(f, file_size - tail_len as u64, tail_len)?;
    let eocd = find_eocd(&tail).ok_or(ApkError::NotZip)?;
    let count = u16_at(&tail, eocd + 10);
    let cd_size = u32_at(&tail, eocd + 12) as u64;
    let cd_offset = u32_at(&tail, eocd + 16) as u64;
    if count == 0xffff || cd_offset == 0xffff_ffff || cd_size == 0xffff_ffff {
        return Err(ApkError::Zip64);
    }
    if cd_size > MAX_CD_BYTES {
        return Err(ApkError::TooLarge);
    }
    if cd_offset + cd_size > file_size {
        return Err(ApkError::NotZip);
    }
    let cd_len = (file_size - cd_offset) as usize;
    if cd_len as u64 > MAX_CD_BYTES + 22 + 65535 {
        return Err(ApkError::TooLarge);
    }
    let cd = read_at(f, cd_offset, cd_len)?;
    let entries = central_directory(&cd, file_size)?;
    if entries.len() > MAX_ENTRIES {
        return Err(ApkError::TooLarge);
    }
    Ok(entries)
}

pub fn raw_data(f: &mut File, e: &Entry, file_size: u64, max: u64) -> Result<Option<Vec<u8>>, ApkError> {
    let head = read_at(f, e.local_offset, 30)?;
    if head.len() < 30 || u32_at(&head, 0) != 0x0403_4b50 {
        return Ok(None);
    }
    let at = e.local_offset + 30 + u16_at(&head, 26) as u64 + u16_at(&head, 28) as u64;
    let len = e.compressed.min(max);
    if at + len > file_size {
        return Ok(None);
    }
    if len == 0 {
        return Ok(Some(Vec::new()));
    }
    Ok(Some(read_at(f, at, len as usize)?))
}

pub fn inflate_exact(raw: &[u8], max_out: usize) -> Option<Vec<u8>> {
    let mut d = Decompress::new(false);
    let mut out = Vec::with_capacity(max_out.min(1 << 20));
    let cap = max_out.max(1);
    let mut buf = vec![0u8; 32 * 1024];
    loop {
        let (in_before, out_before) = (d.total_in(), d.total_out());
        let status = d.decompress(&raw[d.total_in() as usize..], &mut buf, FlushDecompress::None).ok()?;
        let produced = (d.total_out() - out_before) as usize;
        out.extend_from_slice(&buf[..produced]);
        if out.len() > cap {
            return None;
        }
        match status {
            flate2::Status::StreamEnd => return Some(out),
            _ if produced == 0 && d.total_in() == in_before => return None,
            _ => {}
        }
    }
}

pub fn inflate_head(raw: &[u8]) -> Option<Vec<u8>> {
    let mut d = Decompress::new(false);
    let mut out = Vec::new();
    let mut buf = vec![0u8; 32 * 1024];
    loop {
        let (in_before, out_before) = (d.total_in(), d.total_out());
        let status = d.decompress(&raw[d.total_in() as usize..], &mut buf, FlushDecompress::Sync);
        let produced = (d.total_out() - out_before) as usize;
        out.extend_from_slice(&buf[..produced]);
        match status {
            Ok(flate2::Status::StreamEnd) => return Some(out),
            Ok(_) if produced == 0 && d.total_in() == in_before => return Some(out),
            Ok(_) => {}
            Err(_) => return None,
        }
    }
}
