use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use serde_json::{json, Value};
use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

use crate::state::Shell;

pub const LIMIT: u64 = 52_428_800;
const MAX_ERRORS: usize = 1_048_576;
const MAX_AGE_DAYS: u64 = 30;
#[cfg(windows)]
const EVENTS_TIMEOUT: Duration = Duration::from_secs(10);
const PROCESS_NAMES: [&str; 3] = ["orrery.exe", "orrery-core.exe", "orrery-shell.exe"];

pub enum Body {
    Text(String),
    File(PathBuf, u64),
}

pub struct Candidate {
    pub name: String,
    pub kind: &'static str,
    pub body: Body,
}

pub enum Chosen {
    Text(String),
    File(PathBuf),
}

pub struct Entry {
    pub name: String,
    pub bytes: u64,
    pub original: u64,
    pub cut: bool,
    pub chosen: Chosen,
}

pub struct Selection {
    pub entries: Vec<Entry>,
    pub cut: Vec<String>,
    pub skipped: Vec<String>,
    pub skipped_sizes: Vec<u64>,
    pub total: u64,
}

fn body_len(b: &Body) -> u64 {
    match b {
        Body::Text(t) => t.len() as u64,
        Body::File(_, n) => *n,
    }
}

pub fn select(cands: Vec<Candidate>, limit: u64) -> Selection {
    let mut sel = Selection { entries: vec![], cut: vec![], skipped: vec![], skipped_sizes: vec![], total: 0 };
    for c in cands {
        let len = body_len(&c.body);
        let left = limit - sel.total;
        match c.body {
            Body::Text(t) if len <= left => {
                sel.total += len;
                sel.entries.push(Entry { name: c.name, bytes: len, original: len, cut: false, chosen: Chosen::Text(t) });
            }
            Body::Text(t) => {
                let mut start = (len - left) as usize;
                while start < t.len() && !t.is_char_boundary(start) {
                    start += 1;
                }
                let kept = t[start..].to_owned();
                if kept.is_empty() {
                    sel.skipped.push(c.name);
                    sel.skipped_sizes.push(len);
                    continue;
                }
                let n = kept.len() as u64;
                sel.total += n;
                sel.cut.push(c.name.clone());
                sel.entries.push(Entry { name: c.name, bytes: n, original: len, cut: true, chosen: Chosen::Text(kept) });
            }
            Body::File(p, n) if n <= left => {
                sel.total += n;
                sel.entries.push(Entry { name: c.name, bytes: n, original: n, cut: false, chosen: Chosen::File(p) });
            }
            Body::File(..) => {
                sel.skipped.push(c.name);
                sel.skipped_sizes.push(len);
            }
        }
    }
    sel
}

pub fn manifest(sel: &Selection, limit: u64) -> String {
    let mut lines = vec![format!("Orrery bug report, budget {limit} bytes uncompressed, used {}", sel.total)];
    for e in &sel.entries {
        if e.cut {
            lines.push(format!("{}\t{}\tcut (kept last {} of {} bytes)", e.name, e.bytes, e.bytes, e.original));
        } else {
            lines.push(format!("{}\t{}", e.name, e.bytes));
        }
    }
    for (n, s) in sel.skipped.iter().zip(&sel.skipped_sizes) {
        lines.push(format!("{n}\t{s}\tskipped (over budget)"));
    }
    format!("{}\n", lines.join("\n"))
}

pub fn decode_text(bytes: &[u8]) -> String {
    if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
        let units: Vec<u16> = bytes[2..].chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
        return String::from_utf16_lossy(&units);
    }
    String::from_utf8_lossy(bytes.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(bytes)).into_owned()
}

pub fn filter_events(text: &str) -> String {
    let mut blocks: Vec<String> = vec![];
    for line in text.lines() {
        if line.starts_with("Event[") || blocks.is_empty() {
            blocks.push(String::new());
        }
        let last = blocks.last_mut().unwrap();
        last.push_str(line);
        last.push('\n');
    }
    blocks
        .into_iter()
        .filter(|b| {
            let low = b.to_lowercase();
            PROCESS_NAMES.iter().any(|n| low.contains(n))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn wer_folder_ok(name: &str) -> bool {
    name.starts_with("AppCrash_Orrery") || name.starts_with("AppCrash_orrery")
}

pub fn dump_ok(name: &str) -> bool {
    let low = name.to_ascii_lowercase();
    low.starts_with("orrery") && low.ends_with(".dmp")
}

pub fn mac_report_ok(name: &str) -> bool {
    let low = name.to_ascii_lowercase();
    ["orrery", "orrery-core", "orrery-shell"].iter().any(|p| low.starts_with(p)) && (low.ends_with(".ips") || low.ends_with(".crash"))
}

pub struct Roots {
    pub logs: PathBuf,
    pub home: PathBuf,
    pub local_app_data: Option<PathBuf>,
    pub platform: String,
}

fn recent(path: &Path, now: SystemTime) -> bool {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .is_ok_and(|t| now.duration_since(t).map_or(true, |d| d <= Duration::from_secs(MAX_AGE_DAYS * 86_400)))
}

fn mtime(path: &Path) -> SystemTime {
    std::fs::metadata(path).and_then(|m| m.modified()).unwrap_or(SystemTime::UNIX_EPOCH)
}

fn read_tail(path: &Path, max: u64) -> Option<Vec<u8>> {
    let mut f = std::fs::File::open(path).ok()?;
    let len = f.metadata().ok()?.len();
    f.seek(SeekFrom::Start(len.saturating_sub(max))).ok()?;
    let mut buf = vec![];
    f.read_to_end(&mut buf).ok()?;
    Some(buf)
}

fn mask(text: &str, roots: &Roots) -> String {
    core_desk::claude::mask::mask_hook_text(text, &roots.home.to_string_lossy(), &roots.platform)
}

fn read_masked(path: &Path, roots: &Roots) -> Option<String> {
    Some(mask(&decode_text(&read_tail(path, LIMIT)?), roots))
}

fn list_dir(dir: &Path) -> Vec<PathBuf> {
    std::fs::read_dir(dir).map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).collect()).unwrap_or_default()
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

pub fn log_candidates(roots: &Roots) -> Vec<Candidate> {
    let mut files: Vec<PathBuf> = list_dir(&roots.logs).into_iter().filter(|p| p.is_file() && p.extension().is_some_and(|e| e.eq_ignore_ascii_case("log"))).collect();
    files.sort_by_key(|p| std::cmp::Reverse(mtime(p)));
    files
        .into_iter()
        .filter_map(|p| Some(Candidate { name: format!("logs/{}", file_name(&p)), kind: "log", body: Body::Text(read_masked(&p, roots)?) }))
        .collect()
}

pub fn crash_candidates(roots: &Roots, events: Option<String>, now: SystemTime) -> Vec<Candidate> {
    let mut out = vec![];
    if let Some(text) = events.map(|t| mask(&filter_events(&t), roots)).filter(|t| !t.trim().is_empty()) {
        out.push(Candidate { name: "crashes/events.txt".into(), kind: "crash", body: Body::Text(text) });
    }
    if let Some(local) = &roots.local_app_data {
        for sub in ["ReportArchive", "ReportQueue"] {
            let mut dirs: Vec<PathBuf> = list_dir(&local.join("Microsoft").join("Windows").join("WER").join(sub))
                .into_iter()
                .filter(|d| d.is_dir() && wer_folder_ok(&file_name(d)))
                .collect();
            dirs.sort_by_key(|p| std::cmp::Reverse(mtime(p)));
            for d in dirs {
                let wer = d.join("Report.wer");
                if !recent(&wer, now) {
                    continue;
                }
                if let Some(text) = read_masked(&wer, roots) {
                    out.push(Candidate { name: format!("crashes/{}/Report.wer", file_name(&d)), kind: "crash", body: Body::Text(text) });
                }
            }
        }
    }
    if roots.platform == "darwin" {
        let mut files: Vec<PathBuf> = list_dir(&roots.home.join("Library").join("Logs").join("DiagnosticReports"))
            .into_iter()
            .filter(|p| p.is_file() && mac_report_ok(&file_name(p)) && recent(p, now))
            .collect();
        files.sort_by_key(|p| std::cmp::Reverse(mtime(p)));
        for p in files {
            if let Some(text) = read_masked(&p, roots) {
                out.push(Candidate { name: format!("crashes/{}", file_name(&p)), kind: "crash", body: Body::Text(text) });
            }
        }
    }
    out
}

pub fn dump_candidates(roots: &Roots) -> Vec<Candidate> {
    let Some(local) = &roots.local_app_data else { return vec![] };
    let mut files: Vec<PathBuf> = list_dir(&local.join("CrashDumps")).into_iter().filter(|p| p.is_file() && dump_ok(&file_name(p))).collect();
    files.sort_by_key(|p| std::cmp::Reverse(mtime(p)));
    files
        .into_iter()
        .filter_map(|p| {
            let n = std::fs::metadata(&p).ok()?.len();
            Some(Candidate { name: format!("dumps/{}", file_name(&p)), kind: "dump", body: Body::File(p.clone(), n) })
        })
        .collect()
}

pub fn assemble(report: String, errors: &str, crashes: Vec<Candidate>, logs: Vec<Candidate>, dumps: Vec<Candidate>, roots: &Roots) -> Vec<Candidate> {
    let mut out = vec![Candidate { name: "report.txt".into(), kind: "report", body: Body::Text(report) }];
    let errors: String = errors.chars().take(MAX_ERRORS).collect();
    if !errors.trim().is_empty() {
        out.push(Candidate { name: "page-errors.txt".into(), kind: "report", body: Body::Text(mask(&errors, roots)) });
    }
    out.extend(crashes);
    out.extend(logs);
    out.extend(dumps);
    out
}

#[cfg(windows)]
fn events_text() -> Option<String> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};
    let query = "*[System[(EventID=1000 or EventID=1001) and TimeCreated[timediff(@SystemTime) <= 2592000000]]]";
    let mut child = Command::new("wevtutil")
        .args(["qe", "Application", &format!("/q:{query}"), "/f:text", "/rd:true", "/c:200"])
        .creation_flags(0x0800_0000)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut buf = vec![];
        let _ = stdout.read_to_end(&mut buf);
        buf
    });
    let start = std::time::Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() < EVENTS_TIMEOUT => std::thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break;
            }
        }
    }
    Some(decode_text(&reader.join().ok()?))
}

#[cfg(not(windows))]
fn events_text() -> Option<String> {
    None
}

fn roots(app: &AppHandle) -> Roots {
    let shell = app.state::<Shell>();
    Roots {
        logs: shell.data_dir.clone(),
        home: PathBuf::from(std::env::var(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).unwrap_or_default()),
        local_app_data: std::env::var_os("LOCALAPPDATA").map(PathBuf::from),
        platform: crate::files_ipc::platform().to_owned(),
    }
}

fn gather(app: &AppHandle, page: &str, errors: &str, dumps: bool) -> Vec<Candidate> {
    let r = roots(app);
    let report = crate::files_ipc::report(app, page);
    let crashes = crash_candidates(&r, events_text(), SystemTime::now());
    let dump_list = if dumps { dump_candidates(&r) } else { vec![] };
    assemble(report, errors, crashes, log_candidates(&r), dump_list, &r)
}

pub fn write_zip(sel: &Selection, manifest_text: &str, out: &Path) -> std::io::Result<u64> {
    let file = std::fs::File::create(out)?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let io = |e: zip::result::ZipError| std::io::Error::other(e.to_string());
    for e in &sel.entries {
        zip.start_file(e.name.as_str(), opts).map_err(io)?;
        match &e.chosen {
            Chosen::Text(t) => zip.write_all(t.as_bytes())?,
            Chosen::File(p) => {
                std::io::copy(&mut std::fs::File::open(p)?, &mut zip)?;
            }
        }
    }
    zip.start_file("manifest.txt", opts).map_err(io)?;
    zip.write_all(manifest_text.as_bytes())?;
    zip.finish().map_err(io)?;
    Ok(std::fs::metadata(out)?.len())
}

pub fn preview(app: &AppHandle) -> Value {
    let items: Vec<Value> = gather(app, "", "", true).iter().map(|c| json!({"name": c.name, "bytes": body_len(&c.body), "kind": c.kind})).collect();
    json!({"ok": true, "items": items, "limit": LIMIT})
}

pub fn save(app: &AppHandle, req: &Value) -> Value {
    let page = crate::files_ipc::page_report(req.get("page").unwrap_or(&Value::Null));
    let errors = req.get("errors").and_then(Value::as_str).unwrap_or("");
    let dumps = req.get("dumps").and_then(Value::as_bool).unwrap_or(false);
    let stamp = core_desk::util::local_file_stamp();
    let Some(path) = crate::files_ipc::save_dialog(app, &format!("orrery-bugreport-{stamp}.zip"), "Zip", &["zip"]) else { return json!({"ok": false, "code": "cancel"}) };
    let sel = select(gather(app, &page, errors, dumps), LIMIT);
    let text = manifest(&sel, LIMIT);
    let Ok(bytes) = write_zip(&sel, &text, &path) else { return json!({"ok": false, "code": "io"}) };
    let _ = app.opener().reveal_item_in_dir(&path);
    json!({"ok": true, "path": path.to_string_lossy(), "bytes": bytes, "files": sel.entries.len() + 1, "cut": sel.cut, "skipped": sel.skipped})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(name: &str, body: &str) -> Candidate {
        Candidate { name: name.into(), kind: "log", body: Body::Text(body.into()) }
    }

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("orrery-bug-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn everything_fits_under_the_budget() {
        let sel = select(vec![text("report.txt", "abcd"), text("logs/a.log", "xyz")], 100);
        assert_eq!(sel.total, 7);
        assert!(sel.cut.is_empty() && sel.skipped.is_empty());
        assert!(manifest(&sel, 100).contains("logs/a.log\t3\n"));
    }

    #[test]
    fn a_text_that_does_not_fit_keeps_its_last_bytes_and_later_ones_are_skipped() {
        let sel = select(vec![text("report.txt", "abcd"), text("logs/a.log", "0123456789"), text("logs/b.log", "zz")], 10);
        assert_eq!(sel.cut, vec!["logs/a.log"]);
        assert_eq!(sel.skipped, vec!["logs/b.log"]);
        let Chosen::Text(t) = &sel.entries[1].chosen else { panic!() };
        assert_eq!(t, "456789");
        assert_eq!(sel.total, 10);
        let m = manifest(&sel, 10);
        assert!(m.contains("logs/a.log\t6\tcut (kept last 6 of 10 bytes)"), "{m}");
        assert!(m.contains("logs/b.log\t2\tskipped"), "{m}");
    }

    #[test]
    fn a_cut_never_splits_a_character() {
        let sel = select(vec![text("logs/a.log", "aéé")], 3);
        let Chosen::Text(t) = &sel.entries[0].chosen else { panic!() };
        assert_eq!(t, "é");
    }

    #[test]
    fn dumps_that_do_not_fit_are_skipped_whole() {
        let cands = vec![text("report.txt", "abcd"), Candidate { name: "dumps/Orrery.exe.1.dmp".into(), kind: "dump", body: Body::File(PathBuf::from("x"), 50) }];
        let sel = select(cands, 20);
        assert_eq!(sel.skipped, vec!["dumps/Orrery.exe.1.dmp"]);
        assert_eq!(sel.entries.len(), 1);
    }

    #[test]
    fn names_are_filtered() {
        assert!(wer_folder_ok("AppCrash_Orrery.exe_abc") && wer_folder_ok("AppCrash_orrery-core.exe_1") && !wer_folder_ok("AppCrash_chrome.exe_1"));
        assert!(dump_ok("Orrery.exe.1234.dmp") && dump_ok("orrery-core.exe.9.dmp") && !dump_ok("chrome.exe.1.dmp") && !dump_ok("Orrery.txt"));
        assert!(mac_report_ok("Orrery-2026.ips") && mac_report_ok("orrery-core_1.crash") && mac_report_ok("orrery-shell-1.ips") && !mac_report_ok("Safari.ips") && !mac_report_ok("Orrery.txt"));
    }

    #[test]
    fn events_keep_only_orrery_blocks() {
        let raw = "Event[0]:\n  Application: Chrome.exe\n\nEvent[1]:\n  Faulting application name: ORRERY.EXE, version 4\n\nEvent[2]:\n  orrery-core.exe crashed\n";
        let out = filter_events(raw);
        assert!(out.contains("ORRERY.EXE") && out.contains("orrery-core.exe") && !out.contains("Chrome"), "{out}");
        assert_eq!(filter_events("nothing here\n"), "");
    }

    #[test]
    fn utf16_reports_are_decoded() {
        let mut b = vec![0xFF, 0xFE];
        for u in "Sig=ok é".encode_utf16() {
            b.extend_from_slice(&u.to_le_bytes());
        }
        assert_eq!(decode_text(&b), "Sig=ok é");
        assert_eq!(decode_text(b"\xEF\xBB\xBFhi"), "hi");
    }

    #[test]
    fn collection_masks_orders_and_zips() {
        let base = tmp("zip");
        let logs = base.join("logs");
        let local = base.join("local");
        let home = base.join("home");
        let wer = local.join("Microsoft/Windows/WER/ReportArchive/AppCrash_Orrery.exe_1");
        std::fs::create_dir_all(&logs).unwrap();
        std::fs::create_dir_all(&wer).unwrap();
        std::fs::create_dir_all(local.join("CrashDumps")).unwrap();
        std::fs::create_dir_all(&home).unwrap();
        std::fs::write(logs.join("shell.log"), format!("start {}/p\nAPI_KEY=abcdef123456\n", home.display())).unwrap();
        std::fs::write(logs.join("notes.txt"), "no").unwrap();
        std::fs::write(wer.join("Report.wer"), "EventType=APPCRASH\n").unwrap();
        std::fs::create_dir_all(local.join("Microsoft/Windows/WER/ReportArchive/AppCrash_Other_2")).unwrap();
        std::fs::write(local.join("Microsoft/Windows/WER/ReportArchive/AppCrash_Other_2/Report.wer"), "other").unwrap();
        std::fs::write(local.join("CrashDumps/Orrery.exe.1234.dmp"), vec![7u8; 64]).unwrap();
        std::fs::write(local.join("CrashDumps/chrome.exe.1.dmp"), vec![7u8; 8]).unwrap();
        let r = Roots { logs, home: home.clone(), local_app_data: Some(local), platform: "win32".into() };
        let crashes = crash_candidates(&r, Some("Event[0]:\n orrery-shell.exe fault\n".into()), SystemTime::now());
        let names: Vec<_> = crashes.iter().map(|c| c.name.clone()).collect();
        assert_eq!(names, vec!["crashes/events.txt", "crashes/AppCrash_Orrery.exe_1/Report.wer"]);
        let all = assemble("REPORT".into(), "err at X", crashes, log_candidates(&r), dump_candidates(&r), &r);
        let order: Vec<_> = all.iter().map(|c| c.name.clone()).collect();
        assert_eq!(order, vec!["report.txt", "page-errors.txt", "crashes/events.txt", "crashes/AppCrash_Orrery.exe_1/Report.wer", "logs/shell.log", "dumps/Orrery.exe.1234.dmp"]);
        let sel = select(all, LIMIT);
        let out = base.join("out.zip");
        let m = manifest(&sel, LIMIT);
        write_zip(&sel, &m, &out).unwrap();
        let mut z = zip::ZipArchive::new(std::fs::File::open(&out).unwrap()).unwrap();
        let mut log = String::new();
        z.by_name("logs/shell.log").unwrap().read_to_string(&mut log).unwrap();
        assert!(log.contains("start ~/p") || log.contains("start ~\\p"), "{log}");
        assert!(!log.contains("abcdef123456"), "{log}");
        let mut dmp = vec![];
        z.by_name("dumps/Orrery.exe.1234.dmp").unwrap().read_to_end(&mut dmp).unwrap();
        assert_eq!(dmp.len(), 64);
        let mut man = String::new();
        z.by_name("manifest.txt").unwrap().read_to_string(&mut man).unwrap();
        assert!(man.contains("report.txt\t6"), "{man}");
        assert_eq!(z.len(), 7);
        let _ = std::fs::remove_dir_all(base);
    }

    #[test]
    fn old_wer_reports_are_left_out() {
        let base = tmp("old");
        let local = base.join("local");
        let wer = local.join("Microsoft/Windows/WER/ReportQueue/AppCrash_orrery-core.exe_9");
        std::fs::create_dir_all(&wer).unwrap();
        std::fs::write(wer.join("Report.wer"), "x").unwrap();
        let r = Roots { logs: base.join("none"), home: base.clone(), local_app_data: Some(local), platform: "win32".into() };
        let future = SystemTime::now() + Duration::from_secs(40 * 86_400);
        assert!(crash_candidates(&r, None, future).is_empty());
        assert_eq!(crash_candidates(&r, None, SystemTime::now()).len(), 1);
        let _ = std::fs::remove_dir_all(base);
    }
}
