use regex::Regex;
use std::sync::OnceLock;

fn re(cell: &'static OnceLock<Regex>, pat: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pat).expect("regex"))
}

pub fn parse_version(out: &str) -> Option<String> {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, r"(?i)\bcodewhale\s+v?([0-9]+\.[0-9]+\.[0-9]+[A-Za-z0-9_.-]*)").captures(out).map(|c| c[1].to_string())
}

pub fn parse_auth_status(out: &str, provider: &str) -> Option<bool> {
    for line in out.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let cols = split_ws(line);
        if cols.first().copied() != Some(provider) || cols.len() < 5 {
            continue;
        }
        let status = cols[4].to_lowercase();
        return Some(!(status.starts_with("unset") || status.starts_with("missing")));
    }
    None
}

fn split_ws(line: &str) -> Vec<&str> {
    core_transcript::jsstr::js_trim(line).split(core_transcript::jsstr::js_space).filter(|s| !s.is_empty()).collect()
}

pub fn parse_telemetry(out: &str) -> Option<bool> {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, r"(?i)usage reporting:\s*(on|off)\b").captures(out).map(|c| c[1].eq_ignore_ascii_case("on"))
}

#[derive(Debug, Clone, PartialEq)]
pub struct DownloadProgress {
    pub mb: u64,
    pub total_mb: Option<u64>,
    pub pct: Option<u64>,
}

pub fn parse_download_progress(line: &str) -> Option<DownloadProgress> {
    static FULL: OnceLock<Regex> = OnceLock::new();
    static OPEN: OnceLock<Regex> = OnceLock::new();
    if let Some(c) = re(&FULL, r"(?i)downloading\s+\S+:\s*([0-9]+)\s*/\s*([0-9]+)\s*MB\s*\(([0-9]+)%\)").captures(line) {
        let n = |i: usize| c[i].parse::<u64>().unwrap_or(0);
        return Some(DownloadProgress { mb: n(1), total_mb: Some(n(2)), pct: Some(n(3).min(100)) });
    }
    re(&OPEN, r"(?i)downloading\s+\S+:\s*([0-9]+)\s*MB downloaded").captures(line).map(|c| DownloadProgress { mb: c[1].parse().unwrap_or(0), total_mb: None, pct: None })
}

pub fn parse_doctor_updates(out: &str) -> Option<(String, String)> {
    let bytes = out.as_bytes();
    let mut sections: Vec<&str> = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\n' {
            let rest = &out[i + 1..];
            if rest.chars().next().is_some_and(|c| !core_transcript::jsstr::js_space(c)) {
                let end = if i > start && bytes[i - 1] == b'\r' { i - 1 } else { i };
                sections.push(&out[start..end]);
                start = i + 1;
            }
        }
        i += 1;
    }
    sections.push(&out[start..]);
    static HEAD: OnceLock<Regex> = OnceLock::new();
    let section = sections.into_iter().find(|s| re(&HEAD, r"(?i)^Updates:").is_match(s))?;
    let field = |name: &str| -> Option<String> {
        let pat = format!(r"(?im)^\s*\S?\s*{name}:\s*v?([0-9]+\.[0-9]+\.[0-9]+[A-Za-z0-9_.-]*)");
        Regex::new(&pat).ok()?.captures(section).map(|c| c[1].to_string())
    };
    Some((field("current")?, field("latest")?))
}

pub fn compare_versions(a: &str, b: &str) -> i32 {
    let split = |v: &str| -> (Vec<i64>, String) {
        let mut it = v.splitn(2, '-');
        let core = it.next().unwrap_or("");
        let pre = it.next().unwrap_or("");
        (core.split('.').map(leading_int).collect(), pre.to_string())
    };
    let (ca, pa) = split(a);
    let (cb, pb) = split(b);
    for i in 0..ca.len().max(cb.len()) {
        let d = ca.get(i).copied().unwrap_or(0) - cb.get(i).copied().unwrap_or(0);
        if d != 0 {
            return d.signum() as i32;
        }
    }
    if pa == pb {
        return 0;
    }
    if pa.is_empty() {
        return 1;
    }
    if pb.is_empty() {
        return -1;
    }
    if core_transcript::jsstr::utf16_cmp(&pa, &pb).is_lt() {
        -1
    } else {
        1
    }
}

fn leading_int(s: &str) -> i64 {
    let t = core_transcript::jsstr::js_trim(s);
    let (neg, digits) = match t.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, t.strip_prefix('+').unwrap_or(t)),
    };
    let n: String = digits.chars().take_while(char::is_ascii_digit).collect();
    let v = n.parse::<i64>().unwrap_or(0);
    if neg {
        -v
    } else {
        v
    }
}

pub fn clean_api_key(raw: &serde_json::Value) -> Option<String> {
    let key = core_transcript::jsstr::js_trim(raw.as_str()?);
    let len = core_transcript::jsstr::units(key);
    if !(8..=512).contains(&len) {
        return None;
    }
    key.bytes().all(|b| (0x21..=0x7e).contains(&b)).then(|| key.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_outputs() {
        assert_eq!(parse_version("codewhale 0.10.2 (ff466b2ed0bf)").as_deref(), Some("0.10.2"));
        assert_eq!(parse_version("nope"), None);
        let table = "provider config keyring env status\ndeepseek unset unset unset unset unset\n";
        assert_eq!(parse_auth_status(table, "deepseek"), Some(false));
        assert_eq!(parse_auth_status("deepseek x x x env\r\n", "deepseek"), Some(true));
        assert_eq!(parse_auth_status("other 1 2 3 4", "deepseek"), None);
        assert_eq!(parse_telemetry("Usage reporting: On (default)"), Some(true));
        assert_eq!(parse_telemetry("usage reporting: off"), Some(false));
        let p = parse_download_progress("codewhale: downloading a.zip: 12 / 128 MB (9%)").unwrap();
        assert_eq!((p.mb, p.total_mb, p.pct), (12, Some(128), Some(9)));
        let p = parse_download_progress("codewhale: downloading a.zip: 12 MB downloaded").unwrap();
        assert_eq!((p.mb, p.total_mb, p.pct), (12, None, None));
        assert!(parse_download_progress("hello").is_none());
    }

    #[test]
    fn doctor_and_versions() {
        let out = "Doctor\n  ok\nUpdates:\n  \u{b7} current: v0.10.2\n  \u{b7} latest: v0.11.0\nOther:\n  x\n";
        assert_eq!(parse_doctor_updates(out), Some(("0.10.2".into(), "0.11.0".into())));
        assert_eq!(parse_doctor_updates("Doctor\n"), None);
        assert_eq!(compare_versions("0.11.0", "0.10.2"), 1);
        assert_eq!(compare_versions("1.0.0-beta", "1.0.0"), -1);
        assert_eq!(compare_versions("1.0", "1.0.0"), 0);
        assert_eq!(compare_versions("1.0.0-a", "1.0.0-b"), -1);
    }

    #[test]
    fn api_key_rules() {
        use serde_json::json;
        assert_eq!(clean_api_key(&json!("  sk-12345678  ")).as_deref(), Some("sk-12345678"));
        assert!(clean_api_key(&json!("short")).is_none());
        assert!(clean_api_key(&json!("has space 12345")).is_none());
        assert!(clean_api_key(&json!(5)).is_none());
    }
}
