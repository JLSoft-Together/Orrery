use super::changes::base_rev;
use super::exec::{exec_git, ExecOpts};
use super::paths::{is_absolute, relative, safe_repo_path};
use regex::Regex;
use serde_json::{json, Map, Value};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

pub const MAX_IMAGE_BYTES: usize = 20 * 1024 * 1024;
const MAX_BLAME_BYTES: usize = 16 * 1024 * 1024;

pub fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (u32::from(c[0]) << 16) | (u32::from(*c.get(1).unwrap_or(&0)) << 8) | u32::from(*c.get(2).unwrap_or(&0));
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

pub fn image_mime(rel: &str) -> Option<&'static str> {
    let ext = rel.rsplit_once('.').map(|(_, e)| e.to_lowercase())?;
    if ext.is_empty() || !ext.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    Some(match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "bmp" => "image/bmp",
        _ => return None,
    })
}

fn err(e: &str) -> Value {
    json!({"ok": false, "error": e})
}

pub fn head_image(root: &str, rel: &Value) -> Value {
    let Some(safe) = safe_repo_path(root, rel) else { return err("denied") };
    let Some(mime) = image_mime(&safe.rel) else { return err("unsupported") };
    if base_rev(root).as_deref() != Some("HEAD") {
        return err("missing");
    }
    let r = Path::new(root);
    let spec = format!("HEAD:{}", safe.rel);
    let kind = exec_git(r, &["cat-file", "-t", &spec], &ExecOpts::max(64).with_timeout(10_000));
    if !kind.ok() || kind.stdout.trim() != "blob" {
        return err("missing");
    }
    let size = exec_git(r, &["cat-file", "-s", &spec], &ExecOpts::max(64).with_timeout(10_000));
    let bytes: Option<f64> = size.stdout.trim().parse().ok().or(if size.stdout.trim().is_empty() { Some(0.0) } else { None });
    let Some(bytes) = bytes.filter(|_| size.ok()) else { return err("failed") };
    if bytes > MAX_IMAGE_BYTES as f64 {
        return err("too-large");
    }
    let mut o = ExecOpts::max(MAX_IMAGE_BYTES).with_timeout(20_000);
    o.binary = true;
    let blob = exec_git(r, &["cat-file", "blob", &spec], &o);
    let good = blob.ok() && !blob.truncated;
    let Some(data) = blob.bytes.filter(|_| good) else { return err("failed") };
    json!({"ok": true, "dataUrl": format!("data:{mime};base64,{}", base64(&data)), "size": data.len()})
}

pub fn parse_blame(out: &str) -> (Map<String, Value>, Vec<String>) {
    static HEAD: OnceLock<Regex> = OnceLock::new();
    let head_re = HEAD.get_or_init(|| Regex::new(r"^([0-9a-f]{40,64}) [0-9]+ ([0-9]+)(?: [0-9]+)?$").expect("regex"));
    let mut commits = Map::new();
    let mut lines: Vec<Option<String>> = Vec::new();
    let mut hash = String::new();
    let mut line = 0usize;
    for raw in out.split('\n') {
        if let Some(c) = head_re.captures(raw) {
            let h = &c[1];
            hash = if h.len() == 40 && h.bytes().all(|b| b == b'0') { String::new() } else { h.to_string() };
            line = c[2].parse().unwrap_or(0);
            if !hash.is_empty() && !commits.contains_key(&hash) {
                commits.insert(hash.clone(), json!({"author": "", "time": 0, "summary": ""}));
            }
            continue;
        }
        if raw.starts_with('\t') {
            if line >= 1 {
                if lines.len() < line {
                    lines.resize(line, None);
                }
                lines[line - 1] = Some(hash.clone());
            }
            continue;
        }
        if hash.is_empty() {
            continue;
        }
        let Some(c) = commits.get_mut(&hash) else { continue };
        if let Some(a) = raw.strip_prefix("author-time ") {
            let n: f64 = a.trim().parse::<f64>().unwrap_or(f64::NAN) * 1000.0;
            c["time"] = json!(if n.is_finite() && n != 0.0 { n as i64 } else { 0 });
        } else if let Some(a) = raw.strip_prefix("author ") {
            c["author"] = json!(crate::js::slice(a, 80));
        } else if let Some(s) = raw.strip_prefix("summary ") {
            c["summary"] = json!(crate::js::slice(s, 200));
        }
    }
    (commits, lines.into_iter().map(Option::unwrap_or_default).collect())
}

pub fn blame(root: &str, abs: &str, signal: Arc<AtomicBool>) -> Value {
    let rel = relative(root, abs).split(std::path::MAIN_SEPARATOR).collect::<Vec<_>>().join("/");
    if rel.is_empty() || rel.starts_with("..") || is_absolute(&rel) {
        return err("denied");
    }
    let mut o = ExecOpts::timeout(30_000).with_max(MAX_BLAME_BYTES);
    o.signal = Some(signal.clone());
    let res = exec_git(Path::new(root), &["blame", "--porcelain", "--", &rel], &o);
    if signal.load(Ordering::SeqCst) {
        return err("cancelled");
    }
    if res.truncated {
        return err("too-large");
    }
    if !res.ok() {
        let low = res.stderr.to_lowercase();
        let kind = if low.contains("no such path") || low.contains("not in head") || low.contains("no such ref") { "untracked" } else { "failed" };
        return json!({"ok": false, "error": kind, "detail": crate::js::slice(res.stderr.trim(), 300)});
    }
    let (commits, lines) = parse_blame(&res.stdout);
    json!({"ok": true, "root": root, "commits": commits, "lines": lines})
}

pub fn merge_state(root: &str) -> Result<Option<&'static str>, &'static str> {
    let r = Path::new(root);
    let res = exec_git(r, &["rev-parse", "--absolute-git-dir"], &ExecOpts::max(8192).with_timeout(10_000));
    if !res.ok() {
        return Err("failed");
    }
    let dir = Path::new(res.stdout.trim());
    let exists = |n: &str| std::fs::metadata(dir.join(n)).is_ok();
    if exists("rebase-merge") || exists("rebase-apply") {
        return Ok(Some("rebase"));
    }
    if exists("MERGE_HEAD") {
        return Ok(Some("merge"));
    }
    if exists("CHERRY_PICK_HEAD") {
        return Ok(Some("cherry-pick"));
    }
    if exists("REVERT_HEAD") {
        return Ok(Some("revert"));
    }
    if exists("SQUASH_MSG") {
        let u = exec_git(r, &["ls-files", "-u", "-z"], &ExecOpts::max(4096).with_timeout(10_000));
        if !u.stdout.is_empty() {
            return Ok(Some("squash"));
        }
    }
    Ok(None)
}

pub fn merge_state_json(root: &str) -> Value {
    match merge_state(root) {
        Ok(k) => json!({"ok": true, "kind": k}),
        Err(e) => err(e),
    }
}

pub fn abort_args(kind: &str) -> &'static [&'static str] {
    match kind {
        "merge" => &["merge", "--abort"],
        "squash" => &["reset", "--merge"],
        "rebase" => &["rebase", "--abort"],
        "cherry-pick" => &["cherry-pick", "--abort"],
        _ => &["revert", "--abort"],
    }
}

pub fn abort_merge(root: &str, confirm: &dyn Fn(&str, &str) -> bool) -> Value {
    let kind = match merge_state(root) {
        Err(e) => return err(e),
        Ok(None) => return err("none"),
        Ok(Some(k)) => k,
    };
    let args = abort_args(kind);
    if !confirm(kind, &format!("git {}", args.join(" "))) {
        return err("cancelled");
    }
    let res = exec_git(Path::new(root), args, &ExecOpts::timeout(60_000));
    if res.ok() {
        json!({"ok": true})
    } else {
        json!({"ok": false, "error": "failed", "detail": crate::js::slice(res.stderr.trim(), 400)})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_and_blame() {
        assert_eq!(base64(b"Man"), "TWFu");
        assert_eq!(base64(b"Ma"), "TWE=");
        assert_eq!(base64(b"M"), "TQ==");
        assert_eq!(image_mime("a/B.PNG"), Some("image/png"));
        assert_eq!(image_mime("a.txt"), None);
        let h = "a".repeat(40);
        let z = "0".repeat(40);
        let out = format!("{h} 1 1 1\nauthor Ann\nauthor-time 10\nsummary s\n\tline\n{z} 2 2 1\n\tnew\n");
        let (c, l) = parse_blame(&out);
        assert_eq!(c[&h]["time"], 10000);
        assert_eq!(l, vec![h.clone(), String::new()]);
    }
}
