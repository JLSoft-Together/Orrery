use super::types::{BuildError, RunDevice};
use regex::Regex;
use serde_json::{json, Map, Value};
use std::sync::OnceLock;

#[derive(Debug, Clone, PartialEq)]
pub enum MachineMessage {
    Event { event: String, params: Map<String, Value> },
    Response { id: i64, result: Option<Value>, error: Option<Value> },
    Text(String),
}

pub fn parse_machine_line(line: &str) -> MachineMessage {
    let t = line.trim();
    if !t.starts_with("[{") || !t.ends_with("}]") {
        return MachineMessage::Text(line.to_string());
    }
    let Ok(Value::Array(arr)) = serde_json::from_str::<Value>(t) else {
        return MachineMessage::Text(line.to_string());
    };
    let Some(Value::Object(msg)) = arr.into_iter().next() else {
        return MachineMessage::Text(line.to_string());
    };
    if let Some(Value::String(event)) = msg.get("event") {
        let params = msg.get("params").and_then(Value::as_object).cloned().unwrap_or_default();
        return MachineMessage::Event { event: event.clone(), params };
    }
    if let Some(id) = msg.get("id").and_then(Value::as_f64) {
        let present = |k: &str| msg.get(k).filter(|v| !v.is_null()).cloned();
        return MachineMessage::Response { id: id as i64, result: present("result"), error: present("error") };
    }
    MachineMessage::Text(line.to_string())
}

pub fn machine_request(id: i64, method: &str, params: Value) -> String {
    format!("{}\n", crate::js::stringify(&json!([{"id": id, "method": method, "params": params}])))
}

fn re(cell: &'static OnceLock<Regex>, src: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(src).expect("regex"))
}

fn decode_uri(s: &str) -> String {
    let b = s.as_bytes();
    let mut out: Vec<u8> = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let hex = s.get(i + 1..i + 3).and_then(|h| u8::from_str_radix(h, 16).ok());
            match hex {
                Some(v) => {
                    out.push(v);
                    i += 3;
                    continue;
                }
                None => return s.to_string(),
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_string())
}

fn absolute(file: &str, cwd: &str) -> String {
    let f = file.trim();
    let drive = f.len() >= 3 && f.as_bytes()[0].is_ascii_alphabetic() && f.as_bytes()[1] == b':' && (f.as_bytes()[2] == b'\\' || f.as_bytes()[2] == b'/');
    if drive || f.starts_with('/') {
        return super::super::util::path_str(&super::super::util::resolve(f));
    }
    super::super::util::path_str(&super::super::util::resolve(&format!("{cwd}/{f}")))
}

pub fn parse_build_error(line: &str, cwd: &str) -> Option<BuildError> {
    static KOTLIN_URI: OnceLock<Regex> = OnceLock::new();
    static KOTLIN_OLD: OnceLock<Regex> = OnceLock::new();
    static JAVAC: OnceLock<Regex> = OnceLock::new();
    static AAPT: OnceLock<Regex> = OnceLock::new();
    static DART: OnceLock<Regex> = OnceLock::new();
    let text = line.trim_end();
    let mk = |file: &str, ln: &str, col: &str, message: &str, error: bool| BuildError {
        file: absolute(file, cwd),
        line: ln.parse().unwrap_or(0),
        col: col.parse().unwrap_or(0),
        message: message.to_string(),
        severity: if error { "error" } else { "warning" }.to_string(),
    };
    if let Some(m) = re(&KOTLIN_URI, r"^([ew]): file://(/?[^:]+(?::[^:]+)?):(\d+):(\d+) (.+)$").captures(text) {
        let file = decode_uri(&m[2]);
        let file = match file.strip_prefix('/') {
            Some(rest) if rest.len() >= 2 && rest.as_bytes()[0].is_ascii_alphabetic() && rest.as_bytes()[1] == b':' => rest.to_string(),
            _ => file,
        };
        return Some(mk(&file, &m[3], &m[4], &m[5], &m[1] == "e"));
    }
    if let Some(m) = re(&KOTLIN_OLD, r"^([ew]): (.+?): \((\d+), (\d+)\): (.+)$").captures(text) {
        return Some(mk(&m[2], &m[3], &m[4], &m[5], &m[1] == "e"));
    }
    if let Some(m) = re(&JAVAC, r"^(.+?\.java):(\d+): (error|warning): (.+)$").captures(text) {
        return Some(mk(&m[1], &m[2], "1", &m[4], &m[3] == "error"));
    }
    if let Some(m) = re(&DART, r"^(.+?\.dart):(\d+):(\d+): (Error|Warning): (.+)$").captures(text) {
        return Some(mk(&m[1], &m[2], &m[3], &m[5], &m[4] == "Error"));
    }
    if let Some(m) = re(&AAPT, r"(?i)^(?:ERROR|WARNING):\s*(.+?\.xml):(\d+):\s*(?:AAPT: )?(?:error|warning)?:?\s*(.+)$").captures(text) {
        let is_warning = text.len() >= 7 && text[..7].eq_ignore_ascii_case("warning");
        return Some(mk(&m[1], &m[2], "1", &m[3], !is_warning));
    }
    None
}

pub struct LogcatLine {
    pub level: String,
    pub tag: String,
    pub text: String,
}

pub fn parse_logcat(line: &str) -> Option<LogcatLine> {
    static R: OnceLock<Regex> = OnceLock::new();
    let m = re(&R, r"^\d\d-\d\d \d\d:\d\d:\d\d\.\d{3}\s+\d+\s+\d+\s+([VDIWEF])\s+(.*?)\s*: (.*)$").captures(line)?;
    Some(LogcatLine { level: m[1].to_string(), tag: m[2].to_string(), text: m[3].to_string() })
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdbRow {
    pub id: String,
    pub state: String,
    pub model: Option<String>,
}

pub fn parse_adb_rows(out: &str) -> Vec<AdbRow> {
    static ROW: OnceLock<Regex> = OnceLock::new();
    static FIELD: OnceLock<Regex> = OnceLock::new();
    let row = re(&ROW, r"^(\S+)\s+(no permissions|\S+)(.*)$");
    let field = re(&FIELD, r"\b(usb|product|model|device|transport_id):(\S+)");
    let mut rows = Vec::new();
    for raw in out.split('\n') {
        let line = raw.trim();
        if line.is_empty() || line.starts_with("List of devices") || line.starts_with('*') {
            continue;
        }
        let Some(m) = row.captures(line) else { continue };
        if m[1].ends_with(':') {
            continue;
        }
        let model = field.captures_iter(m.get(3).map(|x| x.as_str()).unwrap_or("")).find(|f| &f[1] == "model").map(|f| f[2].replace('_', " "));
        let state = match &m[2] {
            "device" => "ready",
            "unauthorized" => "unauthorized",
            "offline" => "offline",
            "authorizing" => "authorizing",
            "recovery" => "recovery",
            "sideload" => "sideload",
            "bootloader" => "bootloader",
            "no permissions" => "no-permissions",
            _ => "unknown",
        };
        rows.push(AdbRow { id: m[1].to_string(), state: state.to_string(), model });
    }
    rows
}

pub fn parse_adb_devices(out: &str) -> Vec<RunDevice> {
    parse_adb_rows(out)
        .into_iter()
        .filter(|r| matches!(r.state.as_str(), "ready" | "unauthorized" | "offline"))
        .map(|r| RunDevice { name: r.model.clone().unwrap_or_else(|| r.id.clone()), emulator: r.id.starts_with("emulator-"), platform: "android".into(), state: r.state, source: "adb".into(), id: r.id })
        .collect()
}

pub fn read_adb_frames(buf: &str) -> (Vec<String>, String) {
    let mut frames = Vec::new();
    let mut i = 0;
    while buf.len() - i >= 4 {
        let Some(head) = buf.get(i..i + 4) else { break };
        let Ok(len) = usize::from_str_radix(head, 16) else { break };
        if buf.len() - i - 4 < len {
            break;
        }
        let Some(body) = buf.get(i + 4..i + 4 + len) else { break };
        frames.push(body.to_string());
        i += 4 + len;
    }
    (frames, buf[i..].to_string())
}

pub fn parse_output_metadata(json: &str, dir: &str) -> Option<(String, String)> {
    let v: Value = serde_json::from_str(json).ok()?;
    let app = v.get("applicationId")?.as_str()?.to_string();
    let file = v.get("elements")?.as_array()?.iter().find_map(|e| e.get("outputFile").and_then(Value::as_str))?;
    Some((super::super::util::path_str(&super::super::util::resolve(&format!("{dir}/{file}"))), app))
}

pub fn parse_resolved_activity(out: &str, pkg: &str) -> Option<String> {
    let last = out.trim().split('\n').next_back().map(|l| l.trim().trim_end_matches('\r')).unwrap_or("");
    last.starts_with(&format!("{pkg}/")).then(|| last.to_string())
}

fn lowest(pids: Vec<&str>) -> Option<String> {
    let mut v: Vec<&str> = pids.into_iter().filter(|t| !t.is_empty() && t.bytes().all(|b| b.is_ascii_digit())).collect();
    v.sort_by_key(|p| p.parse::<f64>().unwrap_or(0.0) as u64);
    v.first().map(|s| s.to_string())
}

pub fn first_pid(out: &str) -> Option<String> {
    lowest(out.split_whitespace().collect())
}

pub fn pidof_missing(code: Option<i32>, out: &str) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    code == Some(127) || re(&R, r"\bpidof\b.*(?:not found|inaccessible)").is_match(out)
}

pub fn ps_pid(out: &str, pkg: &str) -> Option<String> {
    let lines: Vec<&str> = out.split('\n').map(str::trim).filter(|l| !l.is_empty()).collect();
    let header: Vec<&str> = lines.first().map(|l| l.split_whitespace().collect()).unwrap_or_default();
    let col = header.iter().position(|h| *h == "PID")?;
    let mut pids = Vec::new();
    for line in lines.iter().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.last() == Some(&pkg) && cols.get(col).is_some_and(|c| !c.is_empty() && c.bytes().all(|b| b.is_ascii_digit())) {
            pids.push(cols[col]);
        }
    }
    lowest(pids)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InstallFailure {
    Signature,
    Storage,
    Abi,
    VersionDowngrade,
    Other,
}

pub fn install_failure(out: &str) -> Option<InstallFailure> {
    static OK: OnceLock<Regex> = OnceLock::new();
    if re(&OK, r"(?m)^Success\s*$").is_match(out) {
        return None;
    }
    if out.contains("INSTALL_FAILED_UPDATE_INCOMPATIBLE") || out.contains("INSTALL_PARSE_FAILED_INCONSISTENT_CERTIFICATES") {
        return Some(InstallFailure::Signature);
    }
    if out.contains("INSUFFICIENT_STORAGE") {
        return Some(InstallFailure::Storage);
    }
    if out.contains("NO_MATCHING_ABIS") {
        return Some(InstallFailure::Abi);
    }
    if out.contains("INSTALL_FAILED_VERSION_DOWNGRADE") {
        return Some(InstallFailure::VersionDowngrade);
    }
    Some(InstallFailure::Other)
}

pub fn flutter_device(raw: &Value) -> Option<RunDevice> {
    let d = raw.as_object()?;
    let id = d.get("id")?.as_str()?;
    let name = d.get("name")?.as_str()?;
    let platform = d.get("platform").and_then(Value::as_str).or_else(|| d.get("targetPlatform").and_then(Value::as_str)).unwrap_or("unknown");
    Some(RunDevice {
        id: id.into(),
        name: name.into(),
        platform: platform.into(),
        emulator: d.get("emulator") == Some(&Value::Bool(true)),
        state: if d.get("isSupported") == Some(&Value::Bool(false)) { "offline" } else { "ready" }.into(),
        source: "flutter".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn machine_lines() {
        assert!(matches!(parse_machine_line(r#"[{"event":"app.started","params":{"appId":"x"}}]"#), MachineMessage::Event { .. }));
        assert!(matches!(parse_machine_line(r#"[{"id":3,"result":{"code":0}}]"#), MachineMessage::Response { id: 3, .. }));
        assert!(matches!(parse_machine_line("[{not json}]"), MachineMessage::Text(_)));
        assert!(matches!(parse_machine_line("plain"), MachineMessage::Text(_)));
    }

    #[test]
    fn build_errors() {
        let e = parse_build_error("e: file:///tmp/a%20b/Main.kt:12:5 Unresolved reference: x", "/p").unwrap();
        assert_eq!((e.file.as_str(), e.line, e.col, e.severity.as_str()), ("/tmp/a b/Main.kt", 12, 5, "error"));
        let d = parse_build_error("lib/main.dart:3:4: Error: bad", "/p").unwrap();
        assert_eq!(d.file, "/p/lib/main.dart");
        let w = parse_build_error("WARNING: res/a.xml:2: AAPT: warning: odd", "/p").unwrap();
        assert_eq!(w.severity, "warning");
        assert!(parse_build_error("nothing", "/p").is_none());
    }

    #[test]
    fn adb_and_pids() {
        let out = "List of devices attached\nemulator-5554 device product:sdk model:Pixel_6 device:x transport_id:1\nabc unauthorized\nxyz recovery\n";
        let d = parse_adb_devices(out);
        assert_eq!(d.len(), 2);
        assert_eq!(d[0].name, "Pixel 6");
        assert!(d[0].emulator);
        assert_eq!(first_pid("300 12 55"), Some("12".into()));
        assert_eq!(ps_pid("USER PID PPID NAME\nu 20 1 com.a\nu 7 1 com.a\n", "com.a"), Some("7".into()));
        let (frames, rest) = read_adb_frames("0003abc0002x");
        assert_eq!((frames, rest.as_str()), (vec!["abc".to_string()], "0002x"));
        assert_eq!(install_failure("Success\n"), None);
        assert_eq!(install_failure("INSTALL_FAILED_VERSION_DOWNGRADE"), Some(InstallFailure::VersionDowngrade));
    }
}
