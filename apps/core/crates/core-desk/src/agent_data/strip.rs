use super::tree::{aborted, copy_times, Halt};
use crate::fs_tree::{walk_tree, EntryType, Signal};
use crate::js;
use crate::util::{mtime_ms, path_str, to_fixed1};
use core_transcript::extract::extract_signals;
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::fs;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StripCount {
    pub images: f64,
    pub bytes: f64,
}

fn size_text(chars: f64) -> String {
    let bytes = (chars * 3.0 / 4.0).round();
    if bytes >= 1024.0 * 1024.0 {
        format!("{} MB", to_fixed1(bytes / 1024.0 / 1024.0))
    } else {
        format!("{} KB", js::number((bytes / 1024.0).round().max(1.0)))
    }
}

pub fn strip_placeholder(media: &str, chars: f64) -> String {
    format!("[Image removed by Orrery to save disk space ({media}, {}). The original transcript is in the backup folder chosen in Orrery.]", size_text(chars))
}

fn visit(v: &mut Value, out: &mut StripCount) {
    match v {
        Value::Array(a) => a.iter_mut().for_each(|x| visit(x, out)),
        Value::Object(m) => {
            let image = m.get("type").and_then(Value::as_str) == Some("image")
                && m.get("source").and_then(Value::as_object).is_some_and(|s| s.get("type").and_then(Value::as_str) == Some("base64") && s.get("data").is_some_and(Value::is_string));
            if image {
                let src = &m["source"];
                let len = js::len(src["data"].as_str().unwrap_or("")) as f64;
                let media = src.get("media_type").and_then(Value::as_str).unwrap_or("image").to_string();
                out.images += 1.0;
                out.bytes += len;
                let mut text = Map::new();
                text.insert("type".into(), json!("text"));
                text.insert("text".into(), json!(strip_placeholder(&media, len)));
                *v = Value::Object(text);
                return;
            }
            for (_, x) in m.iter_mut() {
                visit(x, out);
            }
        }
        _ => {}
    }
}

pub fn strip_record(rec: &mut Value) -> StripCount {
    let mut out = StripCount::default();
    visit(rec, &mut out);
    if let Some(file) = rec.get_mut("toolUseResult").and_then(|t| t.get_mut("file")).and_then(Value::as_object_mut) {
        if let Some(data) = file.get("base64").and_then(Value::as_str) {
            let n = js::len(data) as f64;
            if n > 0.0 {
                out.bytes += n;
                file.insert("base64".into(), json!(""));
            }
        }
    }
    out
}

pub struct StrippedLine {
    pub line: String,
    pub count: StripCount,
}

pub fn strip_line(line: &str) -> Result<StrippedLine, Halt> {
    let same = || Ok(StrippedLine { line: line.to_string(), count: StripCount::default() });
    if !line.contains("\"base64\"") {
        return same();
    }
    let Ok(mut rec) = serde_json::from_str::<Value>(line) else { return same() };
    let before = extract_signals(&rec).len();
    let n = strip_record(&mut rec);
    if n.bytes == 0.0 {
        return same();
    }
    let next = js::stringify(&rec);
    let again: Value = serde_json::from_str(&next).map_err(|_| Halt::Failed)?;
    if extract_signals(&again).len() != before {
        return Err(Halt::Failed);
    }
    Ok(StrippedLine { line: next, count: n })
}

fn each_line(file: &Path, signal: Option<&Signal>, mut f: impl FnMut(&str) -> Result<(), Halt>) -> Result<(), Halt> {
    let mut reader = BufReader::new(fs::File::open(file).map_err(|_| Halt::Failed)?);
    let mut buf = Vec::new();
    loop {
        buf.clear();
        let n = reader.read_until(b'\n', &mut buf).map_err(|_| Halt::Failed)?;
        if n == 0 {
            return Ok(());
        }
        if buf.last() == Some(&b'\n') {
            buf.pop();
        }
        if buf.last() == Some(&b'\r') {
            buf.pop();
        }
        let text = String::from_utf8_lossy(&buf);
        for part in text.split(['\r', '\u{2028}', '\u{2029}']) {
            if aborted(signal) {
                return Err(Halt::Cancelled);
            }
            f(part)?;
        }
    }
}

pub fn count_file(file: &Path, signal: Option<&Signal>) -> Result<StripCount, Halt> {
    let mut out = StripCount::default();
    each_line(file, signal, |line| {
        let r = strip_line(line)?;
        out.images += r.count.images;
        out.bytes += r.count.bytes;
        Ok(())
    })?;
    Ok(out)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StripFileResult {
    pub ok: bool,
    pub count: StripCount,
}

fn failed() -> Result<StripFileResult, Halt> {
    Ok(StripFileResult { ok: false, count: StripCount::default() })
}

pub fn strip_file(file: &Path, backup_file: &Path, signal: Option<&Signal>) -> Result<StripFileResult, Halt> {
    let before = fs::metadata(file).map_err(|_| Halt::Failed)?;
    let mut tmp = file.as_os_str().to_owned();
    tmp.push(".orrery-strip-tmp");
    let tmp = PathBuf::from(tmp);
    let mut total = StripCount::default();
    let mut lines = 0usize;
    let written = (|| -> Result<(), Halt> {
        let mut out = BufWriter::new(fs::File::create(&tmp).map_err(|_| Halt::Failed)?);
        each_line(file, signal, |line| {
            let r = strip_line(line)?;
            total.images += r.count.images;
            total.bytes += r.count.bytes;
            lines += 1;
            out.write_all(r.line.as_bytes()).and_then(|_| out.write_all(b"\n")).map_err(|_| Halt::Failed)
        })?;
        out.flush().map_err(|_| Halt::Failed)
    })();
    if let Err(e) = written {
        let _ = fs::remove_file(&tmp);
        return if e == Halt::Cancelled { Err(e) } else { failed() };
    }
    if total.bytes == 0.0 {
        let _ = fs::remove_file(&tmp);
        return Ok(StripFileResult { ok: true, count: StripCount::default() });
    }
    let mut check = 0usize;
    let mut bad = false;
    let checked = each_line(&tmp, None, |line| {
        check += 1;
        if !line.is_empty() && serde_json::from_str::<Value>(line).is_err() {
            bad = true;
        }
        Ok(())
    });
    let now = fs::metadata(file).map_err(|_| Halt::Failed);
    let unchanged = now.as_ref().is_ok_and(|n| n.len() == before.len() && mtime_ms(n) == mtime_ms(&before));
    if checked.is_err() || bad || check != lines || !unchanged {
        let _ = fs::remove_file(&tmp);
        return failed();
    }
    let copied = (|| -> std::io::Result<u64> {
        if let Some(dir) = backup_file.parent() {
            fs::create_dir_all(dir)?;
        }
        let mut from = fs::File::open(file)?;
        let mut to = fs::OpenOptions::new().write(true).create_new(true).open(backup_file)?;
        let n = std::io::copy(&mut from, &mut to)?;
        to.flush()?;
        drop(to);
        copy_times(&before, backup_file);
        Ok(n)
    })();
    match copied {
        Ok(n) if n == before.len() => {}
        _ => {
            let _ = fs::remove_file(&tmp);
            return failed();
        }
    }
    if fs::rename(&tmp, file).is_err() {
        let _ = fs::remove_file(&tmp);
        return failed();
    }
    copy_times(&before, file);
    Ok(StripFileResult { ok: true, count: total })
}

#[derive(Debug, Clone, PartialEq)]
pub struct StripCandidate {
    pub file: PathBuf,
    pub rel: PathBuf,
    pub session_id: String,
    pub size: f64,
}

fn is_uuid(id: &str) -> bool {
    let parts: Vec<&str> = id.split('-').collect();
    let lens = [8, 4, 4, 4, 12];
    parts.len() == 5 && parts.iter().zip(lens).all(|(p, n)| p.len() == n && p.bytes().all(|b| b.is_ascii_hexdigit()))
}

pub fn session_of_rel(rel: &str) -> Option<String> {
    let parts: Vec<&str> = rel.split(['\\', '/']).collect();
    if parts.len() < 2 {
        return None;
    }
    let id = if parts.len() == 2 { parts[1].strip_suffix(".jsonl").unwrap_or(parts[1]) } else { parts[1] };
    is_uuid(id).then(|| id.to_string())
}

pub fn strip_candidates(projects: &Path, cutoff_ms: f64, running: &HashSet<String>, signal: Option<&Signal>) -> Result<Vec<StripCandidate>, Halt> {
    let mut out = Vec::new();
    walk_tree(
        projects,
        |e| {
            let rel = path_str(&e.rel);
            if e.kind != EntryType::File || !rel.ends_with(".jsonl") || e.mtime_ms >= cutoff_ms {
                return Ok(());
            }
            let Some(session_id) = session_of_rel(&rel) else { return Ok(()) };
            if running.contains(&session_id) {
                return Ok(());
            }
            out.push(StripCandidate { file: projects.join(&e.rel), rel: e.rel.clone(), session_id, size: e.size });
            Ok(())
        },
        signal,
    )?;
    Ok(out)
}

pub fn running_sessions(config_dir: &Path, alive: &dyn Fn(f64) -> bool) -> HashSet<String> {
    let mut out = HashSet::new();
    let dir = config_dir.join("sessions");
    for name in crate::fs_tree::read_names(&dir) {
        if !name.ends_with(".json") {
            continue;
        }
        let Some(rec) = fs::read_to_string(dir.join(&name)).ok().and_then(|t| serde_json::from_str::<Value>(&t).ok()) else { continue };
        if let (Some(pid), Some(id)) = (rec.get("pid").and_then(Value::as_f64), rec.get("sessionId").and_then(Value::as_str)) {
            if alive(pid) {
                out.insert(id.to_string());
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SID: &str = "11111111-2222-4333-8444-555555555555";

    fn b64() -> String {
        format!("{:A<4000}", "iVBORw0KGgo")
    }

    #[test]
    fn strips_images_and_keeps_other_lines() {
        let pasted = json!({"type": "user", "sessionId": SID, "message": {"role": "user", "content": [{"type": "text", "text": "look"}, {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": b64()}}]}});
        let r = strip_line(&pasted.to_string()).unwrap();
        assert_eq!((r.count.images, r.count.bytes), (1.0, 4000.0));
        assert!(r.line.contains("[Image removed by Orrery to save disk space (image/png, 3 KB)."));
        assert!(!r.line.contains("iVBOR"));
        let plain = r#"{"type":"user","message":{"content":"x"}}"#;
        assert_eq!(strip_line(plain).unwrap().line, plain);
        assert_eq!(strip_line("not \"base64\" json").unwrap().line, "not \"base64\" json");
        let mut tool = json!({"toolUseResult": {"file": {"base64": "QUJD", "type": "image/png"}}});
        assert_eq!(strip_record(&mut tool).bytes, 4.0);
        assert_eq!(tool["toolUseResult"]["file"]["base64"], json!(""));
    }

    #[test]
    fn sessions_from_paths() {
        assert_eq!(session_of_rel(&format!("slug/{SID}.jsonl")).as_deref(), Some(SID));
        assert_eq!(session_of_rel(&format!("slug/{SID}/subagents/a.jsonl")).as_deref(), Some(SID));
        assert_eq!(session_of_rel("slug/nope.jsonl"), None);
        assert_eq!(session_of_rel("x.jsonl"), None);
        assert_eq!(size_text(4000.0), "3 KB");
        assert_eq!(size_text(4.0), "1 KB");
        assert_eq!(size_text(4.0 * 1024.0 * 1024.0), "3.0 MB");
    }

    #[test]
    fn rewrites_a_file_with_a_backup() {
        let dir = std::env::temp_dir().join(format!("core-desk-ad-strip-{}", crate::util::uuid()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("t.jsonl");
        let img = json!({"type": "user", "message": {"content": [{"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": b64()}}]}});
        fs::write(&file, format!("{}\n{{\"a\":1}}\n", img)).unwrap();
        assert_eq!(count_file(&file, None).unwrap().images, 1.0);
        let backup = dir.join("b").join("t.jsonl");
        let r = strip_file(&file, &backup, None).unwrap();
        assert!(r.ok && r.count.images == 1.0);
        assert!(fs::read_to_string(&backup).unwrap().contains("iVBOR"));
        assert!(!fs::read_to_string(&file).unwrap().contains("iVBOR"));
        let again = strip_file(&file, &backup, None).unwrap();
        assert!(again.ok && again.count.bytes == 0.0);
        fs::remove_dir_all(&dir).unwrap();
    }
}
