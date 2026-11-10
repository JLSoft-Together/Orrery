use super::paths::{dirname, is_absolute, real_or_null, resolve_str, root_of};
use crate::util::mtime_ms;
use serde_json::{json, Value};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

pub const MAX_VIEW_BYTES: u64 = 2 * 1024 * 1024;
pub const MAX_WRITE_BYTES: usize = 4 * 1024 * 1024;
const SNIFF: usize = 8 * 1024;
const LOG_CHUNK_BYTES: f64 = 2.0 * 1024.0 * 1024.0;
const MAX_LOG_BYTES: u64 = 200 * 1024 * 1024;
const IMAGE_EXT: [&str; 9] = ["png", "jpg", "jpeg", "gif", "webp", "svg", "ico", "bmp", "avif"];

pub fn eol_of(text: &str) -> &'static str {
    let b = text.as_bytes();
    let (mut crlf, mut lf, mut cr) = (0, 0, 0);
    let mut i = 0;
    while i < b.len() {
        if b[i] == 13 {
            if b.get(i + 1) == Some(&10) {
                crlf += 1;
                i += 1;
            } else {
                cr += 1;
            }
        } else if b[i] == 10 {
            lf += 1;
        }
        i += 1;
    }
    if crlf > 0 && crlf >= lf && crlf >= cr {
        return "crlf";
    }
    if cr > lf {
        "cr"
    } else {
        "lf"
    }
}

pub fn has_bom(b: &[u8]) -> bool {
    b.len() >= 3 && b[0] == 0xef && b[1] == 0xbb && b[2] == 0xbf
}

pub fn decode_utf8(b: &[u8]) -> Option<String> {
    let body = if has_bom(b) { &b[3..] } else { b };
    std::str::from_utf8(body).ok().map(String::from)
}

pub fn sniff_binary(b: &[u8]) -> bool {
    b[..b.len().min(SNIFF)].contains(&0)
}

pub fn ext_of(file: &str) -> String {
    let base = crate::util::basename(file).to_lowercase();
    match base.rfind('.') {
        Some(i) if i > 0 => base[i + 1..].to_string(),
        _ => String::new(),
    }
}

pub fn lang_of(file: &str) -> Option<String> {
    let base = crate::util::basename(file).to_lowercase();
    if base == "dockerfile" || base == "makefile" {
        return Some(base);
    }
    Some(ext_of(file)).filter(|e| !e.is_empty())
}

pub fn real_roots(roots: &[String]) -> Vec<String> {
    roots.iter().filter(|r| is_absolute(r)).filter_map(|r| real_or_null(r)).collect()
}

fn empty(kind: &str, file: &str) -> Value {
    json!({"kind": kind, "path": file, "size": 0, "mtimeMs": 0})
}

pub fn read_file_for_view(file: &Value, allowed: &[String]) -> Value {
    let Some(file) = file.as_str().filter(|f| is_absolute(f)) else {
        return empty("denied", &super::ipc_wt::js_string(file));
    };
    let roots = real_roots(allowed);
    let Some(real) = real_or_null(file) else {
        return empty(if root_of(&resolve_str(file), &roots).is_some() { "missing" } else { "denied" }, file);
    };
    if root_of(&real, &roots).is_none() {
        return empty("denied", file);
    }
    let Some(meta) = std::fs::metadata(&real).ok().filter(|m| m.is_file()) else { return empty("missing", &real) };
    let mut base = json!({"path": real, "size": meta.len(), "mtimeMs": mtime_ms(&meta)});
    if let Some(l) = lang_of(&real) {
        base["lang"] = json!(l);
    }
    let with = |kind: &str, base: &Value| {
        let mut v = json!({"kind": kind});
        for (k, x) in base.as_object().expect("obj") {
            v[k] = x.clone();
        }
        v
    };
    if IMAGE_EXT.contains(&ext_of(&real).as_str()) {
        return with("image", &base);
    }
    if meta.len() > MAX_VIEW_BYTES {
        return with("too-large", &base);
    }
    let Ok(buf) = std::fs::read(&real) else { return empty("missing", &real) };
    if sniff_binary(&buf) {
        return with("binary", &base);
    }
    let strict = decode_utf8(&buf);
    let text = strict.clone().unwrap_or_else(|| {
        let s = String::from_utf8_lossy(&buf).to_string();
        s.strip_prefix('\u{feff}').map(String::from).unwrap_or(s)
    });
    let mut v = with("text", &base);
    v["size"] = json!(buf.len());
    v["text"] = json!(text);
    v["bom"] = json!(has_bom(&buf));
    v["eol"] = json!(eol_of(&text));
    if strict.is_none() {
        v["invalidUtf8"] = json!(true);
    }
    v
}

fn offset(v: &Value) -> Option<f64> {
    v.as_f64().filter(|f| f.fract() == 0.0 && *f >= 0.0 && *f <= 9_007_199_254_740_991.0)
}

pub fn read_log_chunk(req: &Value, allowed: &[String]) -> Value {
    let fail = |e: &str| json!({"ok": false, "error": e});
    let Some(file) = req.get("path").and_then(Value::as_str).filter(|f| is_absolute(f)) else { return fail("denied") };
    let roots = real_roots(allowed);
    let Some(real) = real_or_null(file) else {
        let mut all = roots.clone();
        all.extend(allowed.iter().filter(|r| is_absolute(r)).cloned());
        return fail(if root_of(&resolve_str(file), &all).is_some() { "missing" } else { "denied" });
    };
    if root_of(&real, &roots).is_none() {
        return fail("denied");
    }
    let Some(meta) = std::fs::metadata(&real).ok().filter(|m| m.is_file()) else { return fail("missing") };
    if meta.len() > MAX_LOG_BYTES {
        return fail("too-large");
    }
    let size = meta.len() as f64;
    let want = LOG_CHUNK_BYTES.min(offset(req.get("bytes").unwrap_or(&Value::Null)).unwrap_or(LOG_CHUNK_BYTES).max(1.0));
    let from = offset(req.get("from").unwrap_or(&Value::Null));
    let forward = from.map(|f| f <= size).unwrap_or(false);
    let (mut start, mut stop) = if forward {
        let f = from.unwrap_or(0.0);
        (f, size.min(f + want))
    } else {
        let s = size.min(offset(req.get("end").unwrap_or(&Value::Null)).unwrap_or(size));
        ((s - want).max(0.0), s)
    };
    let Ok(mut fh) = std::fs::File::open(&real) else { return fail("failed") };
    let mut buf = vec![0u8; (stop - start).max(0.0) as usize];
    let mut read = 0usize;
    if stop > start {
        if fh.seek(SeekFrom::Start(start as u64)).is_err() {
            return fail("failed");
        }
        while read < buf.len() {
            match fh.read(&mut buf[read..]) {
                Ok(0) => break,
                Ok(n) => read += n,
                Err(_) => return fail("failed"),
            }
        }
    }
    let mut bytes = &buf[..read];
    stop = start + read as f64;
    if forward {
        if let Some(last) = bytes.iter().rposition(|b| *b == 10) {
            bytes = &bytes[..last + 1];
            stop = start + last as f64 + 1.0;
        } else if (read as f64) < want {
            bytes = &bytes[..0];
            stop = start;
        }
    } else if start > 0.0 {
        if let Some(first) = bytes.iter().position(|b| *b == 10) {
            if first + 1 < bytes.len() {
                bytes = &bytes[first + 1..];
                start += first as f64 + 1.0;
            }
        }
    }
    let mut v = json!({"ok": true, "path": real, "text": String::from_utf8_lossy(bytes), "start": start as u64, "end": stop as u64, "size": size as u64, "mtimeMs": mtime_ms(&meta)});
    if from.is_some() && !forward {
        v["reset"] = json!(true);
    }
    v
}

pub fn encode_for_disk(text: &str, eol: &str, bom: bool) -> Vec<u8> {
    let nl = match eol {
        "crlf" => "\r\n",
        "cr" => "\r",
        _ => "\n",
    };
    let mut out = String::with_capacity(text.len());
    let mut it = text.chars().peekable();
    while let Some(c) = it.next() {
        match c {
            '\r' => {
                if it.peek() == Some(&'\n') {
                    it.next();
                }
                out.push_str(nl);
            }
            '\n' => out.push_str(nl),
            other => out.push(other),
        }
    }
    let mut b = if bom { vec![0xef, 0xbb, 0xbf] } else { Vec::new() };
    b.extend_from_slice(out.as_bytes());
    b
}

fn is_busy(e: &std::io::Error) -> bool {
    matches!(e.kind(), std::io::ErrorKind::PermissionDenied) || matches!(e.raw_os_error(), Some(5) | Some(32) | Some(33)) || (!cfg!(windows) && matches!(e.raw_os_error(), Some(1) | Some(13) | Some(16)))
}

pub fn random_hex(n: usize) -> String {
    let mut b = vec![0u8; n];
    let _ = getrandom::fill(&mut b);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn rename_retry(from: &Path, to: &Path, tries: usize) -> Result<(), std::io::Error> {
    let mut i = 0;
    loop {
        match std::fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) if is_busy(&e) && i < tries - 1 => {
                std::thread::sleep(std::time::Duration::from_millis(40 * (i as u64 + 1)));
                i += 1;
            }
            Err(e) => return Err(e),
        }
    }
}

pub fn write_text_file(req: &Value, allowed: &[String], on_saved: &dyn Fn(&str, &str)) -> Value {
    let fail = |e: &str| json!({"ok": false, "error": e});
    let file = req.get("path").and_then(Value::as_str).filter(|f| is_absolute(f));
    let text = req.get("text").and_then(Value::as_str);
    let (Some(file), Some(text)) = (file, text) else { return fail("denied") };
    if crate::js::len(text) > MAX_WRITE_BYTES || text.len() > MAX_WRITE_BYTES {
        return fail("too-large");
    }
    let roots = real_roots(allowed);
    let Some(real) = real_or_null(file) else {
        let mut all = roots.clone();
        all.extend(allowed.iter().cloned());
        return fail(if root_of(&resolve_str(file), &all).is_some() { "missing" } else { "denied" });
    };
    if root_of(&real, &roots).is_none() {
        return fail("denied");
    }
    let Some(meta) = std::fs::metadata(&real).ok().filter(|m| m.is_file()) else { return fail("missing") };
    if let Some(expected) = req.get("expectedMtime").and_then(Value::as_f64) {
        if (mtime_ms(&meta) - expected).abs() > 0.5 {
            return fail("conflict");
        }
    }
    if meta.len() as usize > MAX_WRITE_BYTES {
        return fail("too-large");
    }
    let Ok(current) = std::fs::read(&real) else { return fail("missing") };
    if sniff_binary(&current) {
        return fail("encoding");
    }
    let Some(decoded) = decode_utf8(&current) else { return fail("encoding") };
    let bytes = encode_for_disk(text, eol_of(&decoded), has_bom(&current));
    let dir = dirname(&real);
    let temp = Path::new(&dir).join(format!(".{}.{}.ccm-tmp", crate::util::basename(&real), random_hex(4)));
    let mut opts = std::fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        opts.mode(meta.permissions().mode() & 0o777);
    }
    let written = opts.open(&temp).and_then(|mut f| std::io::Write::write_all(&mut f, &bytes));
    if let Err(e) = written {
        let _ = std::fs::remove_file(&temp);
        return fail(if is_busy(&e) { "denied" } else { "failed" });
    }
    if let Err(e) = rename_retry(&temp, Path::new(&real), 6) {
        let _ = std::fs::remove_file(&temp);
        return fail(if is_busy(&e) { "busy" } else { "failed" });
    }
    if bytes != current {
        on_saved(&real, &decoded);
    }
    let after = std::fs::metadata(&real).ok();
    json!({
        "ok": true, "path": real,
        "mtimeMs": after.as_ref().map(mtime_ms).unwrap_or_else(crate::util::now_ms),
        "size": after.map(|m| m.len()).unwrap_or(bytes.len() as u64),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eols_and_encoding() {
        assert_eq!(eol_of("a\r\nb\r\nc\n"), "crlf");
        assert_eq!(eol_of("a\rb\rc\n"), "cr");
        assert_eq!(eol_of("abc"), "lf");
        assert_eq!(encode_for_disk("a\nb\r\nc\rd", "crlf", true), b"\xef\xbb\xbfa\r\nb\r\nc\r\nd".to_vec());
        assert_eq!(lang_of("/x/Dockerfile").as_deref(), Some("dockerfile"));
        assert_eq!(lang_of("/x/.env"), None);
        assert_eq!(lang_of("/x/a.TS").as_deref(), Some("ts"));
    }
}
