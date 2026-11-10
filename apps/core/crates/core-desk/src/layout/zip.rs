use flate2::read::DeflateDecoder;
use std::fs::{self, File};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZipError {
    Corrupt,
    Zip64,
    UnsafePath,
    TooLarge,
    Method,
    Io,
}

impl ZipError {
    pub fn code(self) -> &'static str {
        match self {
            ZipError::Corrupt => "zip-corrupt",
            ZipError::Zip64 => "zip64",
            ZipError::UnsafePath => "zip-unsafe-path",
            ZipError::TooLarge => "zip-too-large",
            ZipError::Method => "zip-method",
            ZipError::Io => "io",
        }
    }
}

struct Entry {
    name: String,
    method: u16,
    compressed: u64,
    size: u64,
    offset: u64,
}

fn u16_at(b: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([b[at], b[at + 1]])
}

fn u32_at(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]])
}

pub fn safe_entry_name(name: &str) -> Option<String> {
    let drive = name.len() >= 2 && name.as_bytes()[0].is_ascii_alphabetic() && name.as_bytes()[1] == b':';
    if name.is_empty() || name.contains('\\') || name.contains('\0') || name.starts_with('/') || drive {
        return None;
    }
    let parts: Vec<&str> = name.split('/').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() || parts.iter().any(|s| *s == ".." || *s == ".") {
        return None;
    }
    Some(parts.join("/"))
}

fn read_at(f: &mut File, at: u64, len: usize) -> Result<Vec<u8>, ZipError> {
    let mut buf = vec![0u8; len];
    f.seek(SeekFrom::Start(at)).map_err(|_| ZipError::Io)?;
    f.read_exact(&mut buf).map_err(|_| ZipError::Corrupt)?;
    Ok(buf)
}

fn central_directory(f: &mut File, size: u64) -> Result<Vec<Entry>, ZipError> {
    let tail_len = size.min(22 + 65535) as usize;
    let tail = read_at(f, size - tail_len as u64, tail_len)?;
    let eocd = (0..=tail.len().saturating_sub(22)).rev().find(|&i| tail.len() >= 22 && u32_at(&tail, i) == 0x0605_4b50).ok_or(ZipError::Corrupt)?;
    let count = u16_at(&tail, eocd + 10) as usize;
    let cd_size = u32_at(&tail, eocd + 12) as u64;
    let cd_offset = u32_at(&tail, eocd + 16) as u64;
    if count == 0xffff || cd_offset == 0xffff_ffff {
        return Err(ZipError::Zip64);
    }
    if cd_offset + cd_size > size {
        return Err(ZipError::Corrupt);
    }
    let cd = read_at(f, cd_offset, cd_size as usize)?;
    let mut at = 0;
    let mut out = Vec::new();
    for _ in 0..count {
        if at + 46 > cd.len() || u32_at(&cd, at) != 0x0201_4b50 {
            return Err(ZipError::Corrupt);
        }
        let (name_len, extra, comment) = (u16_at(&cd, at + 28) as usize, u16_at(&cd, at + 30) as usize, u16_at(&cd, at + 32) as usize);
        let (compressed, usize_, offset) = (u32_at(&cd, at + 20), u32_at(&cd, at + 24), u32_at(&cd, at + 42));
        if compressed == 0xffff_ffff || usize_ == 0xffff_ffff || offset == 0xffff_ffff {
            return Err(ZipError::Zip64);
        }
        if at + 46 + name_len > cd.len() {
            return Err(ZipError::Corrupt);
        }
        out.push(Entry { name: String::from_utf8_lossy(&cd[at + 46..at + 46 + name_len]).into_owned(), method: u16_at(&cd, at + 10), compressed: compressed as u64, size: usize_ as u64, offset: offset as u64 });
        at += 46 + name_len + extra + comment;
    }
    Ok(out)
}

pub fn extract_zip(file: &Path, dest: &Path, max_total: u64) -> Result<(usize, u64), ZipError> {
    extract_zip_filtered(file, dest, max_total, &|_| true)
}

pub fn extract_zip_filtered(file: &Path, dest: &Path, max_total: u64, keep: &dyn Fn(&str) -> bool) -> Result<(usize, u64), ZipError> {
    let mut f = File::open(file).map_err(|_| ZipError::Io)?;
    let size = f.metadata().map_err(|_| ZipError::Io)?.len();
    let mut entries = central_directory(&mut f, size)?;
    entries.retain(|e| keep(&e.name));
    if entries.iter().map(|e| e.size).sum::<u64>() > max_total {
        return Err(ZipError::TooLarge);
    }
    let root: PathBuf = crate::util::resolve(&dest.to_string_lossy());
    let (mut files, mut bytes) = (0usize, 0u64);
    for e in entries {
        let name = safe_entry_name(&e.name).ok_or(ZipError::UnsafePath)?;
        let mut out = root.clone();
        out.extend(name.split('/'));
        if !out.starts_with(&root) || out == root {
            return Err(ZipError::UnsafePath);
        }
        if e.name.ends_with('/') {
            fs::create_dir_all(&out).map_err(|_| ZipError::Io)?;
            continue;
        }
        if e.method != 0 && e.method != 8 {
            return Err(ZipError::Method);
        }
        let head = read_at(&mut f, e.offset, 30)?;
        if u32_at(&head, 0) != 0x0403_4b50 {
            return Err(ZipError::Corrupt);
        }
        let data_at = e.offset + 30 + u16_at(&head, 26) as u64 + u16_at(&head, 28) as u64;
        if data_at + e.compressed > size {
            return Err(ZipError::Corrupt);
        }
        let raw = read_at(&mut f, data_at, e.compressed as usize)?;
        let data = if e.method == 8 {
            let mut v = Vec::with_capacity(e.size as usize);
            DeflateDecoder::new(raw.as_slice()).take(e.size + 1).read_to_end(&mut v).map_err(|_| ZipError::Corrupt)?;
            v
        } else {
            raw
        };
        if data.len() as u64 != e.size {
            return Err(ZipError::Corrupt);
        }
        if let Some(dir) = out.parent() {
            fs::create_dir_all(dir).map_err(|_| ZipError::Io)?;
        }
        fs::write(&out, &data).map_err(|_| ZipError::Io)?;
        files += 1;
        bytes += data.len() as u64;
    }
    Ok((files, bytes))
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use flate2::write::DeflateEncoder;
    use flate2::Compression;
    use std::io::Write;

    pub fn build_zip(entries: &[(&str, &[u8], bool)]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut cd = Vec::new();
        for (name, data, deflate) in entries {
            let comp = if *deflate {
                let mut e = DeflateEncoder::new(Vec::new(), Compression::default());
                e.write_all(data).unwrap();
                e.finish().unwrap()
            } else {
                data.to_vec()
            };
            let offset = out.len() as u32;
            out.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            out.extend_from_slice(&[20, 0, 0, 0]);
            out.extend_from_slice(&(if *deflate { 8u16 } else { 0 }).to_le_bytes());
            out.extend_from_slice(&[0; 8]);
            out.extend_from_slice(&(comp.len() as u32).to_le_bytes());
            out.extend_from_slice(&(data.len() as u32).to_le_bytes());
            out.extend_from_slice(&(name.len() as u16).to_le_bytes());
            out.extend_from_slice(&[0, 0]);
            out.extend_from_slice(name.as_bytes());
            out.extend_from_slice(&comp);
            cd.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            cd.extend_from_slice(&[20, 0, 20, 0, 0, 0]);
            cd.extend_from_slice(&(if *deflate { 8u16 } else { 0 }).to_le_bytes());
            cd.extend_from_slice(&[0; 8]);
            cd.extend_from_slice(&(comp.len() as u32).to_le_bytes());
            cd.extend_from_slice(&(data.len() as u32).to_le_bytes());
            cd.extend_from_slice(&(name.len() as u16).to_le_bytes());
            cd.extend_from_slice(&[0; 12]);
            cd.extend_from_slice(&offset.to_le_bytes());
            cd.extend_from_slice(name.as_bytes());
        }
        let cd_off = out.len() as u32;
        out.extend_from_slice(&cd);
        out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(cd.len() as u32).to_le_bytes());
        out.extend_from_slice(&cd_off.to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        out
    }

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("ccm-zip-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn extracts_stored_and_deflated() {
        let d = tmp("ok");
        let z = d.join("a.zip");
        fs::write(&z, build_zip(&[("dir/", b"", false), ("dir/a.txt", b"hello hello hello", true), ("b.bin", b"raw", false)])).unwrap();
        let (n, bytes) = extract_zip(&z, &d.join("out"), 1000).unwrap();
        assert_eq!((n, bytes), (2, 20));
        assert_eq!(fs::read_to_string(d.join("out/dir/a.txt")).unwrap(), "hello hello hello");
        assert_eq!(extract_zip(&z, &d.join("o2"), 10), Err(ZipError::TooLarge));
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn refuses_unsafe_and_corrupt() {
        let d = tmp("bad");
        let z = d.join("a.zip");
        fs::write(&z, build_zip(&[("../x", b"1", false)])).unwrap();
        assert_eq!(extract_zip(&z, &d.join("out"), 1000), Err(ZipError::UnsafePath));
        fs::write(&z, b"not a zip at all, long enough to pass the size check....").unwrap();
        assert_eq!(extract_zip(&z, &d.join("out"), 1000), Err(ZipError::Corrupt));
        assert_eq!(safe_entry_name("a//b/c"), Some("a/b/c".into()));
        assert_eq!(safe_entry_name("C:/x"), None);
        let _ = fs::remove_dir_all(&d);
    }
}
