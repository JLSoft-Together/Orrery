use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use serde_json::{json, Value};

#[derive(Clone, Debug, PartialEq)]
pub enum RegValue {
    Str(String),
    Num(f64),
    Multi(Vec<String>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct RegKey {
    pub key: String,
    pub values: HashMap<String, RegValue>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RegSource {
    pub key: &'static str,
    pub scope: &'static str,
    pub arch: Option<&'static str>,
}

pub const REG_SOURCES: [RegSource; 3] = [
    RegSource { key: "HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall", scope: "machine", arch: Some("64") },
    RegSource { key: "HKEY_LOCAL_MACHINE\\SOFTWARE\\WOW6432Node\\Microsoft\\Windows\\CurrentVersion\\Uninstall", scope: "machine", arch: Some("32") },
    RegSource { key: "HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall", scope: "user", arch: None },
];

#[derive(Clone, Debug, PartialEq)]
pub struct App {
    pub id: String,
    pub name: String,
    pub publisher: Option<String>,
    pub version: Option<String>,
    pub installed_at: Option<f64>,
    pub size: Option<f64>,
    pub size_from: Option<&'static str>,
    pub location: Option<String>,
    pub source: &'static str,
    pub scope: &'static str,
    pub arch: Option<&'static str>,
    pub bundle_id: Option<String>,
    pub last_used: Option<f64>,
    pub uninstall: Option<String>,
    pub blocked: Option<&'static str>,
}

impl App {
    pub fn to_json(&self) -> Value {
        json!({
            "id": self.id, "name": self.name, "publisher": self.publisher, "version": self.version, "installedAt": self.installed_at,
            "size": self.size, "sizeFrom": self.size_from, "location": self.location, "source": self.source, "scope": self.scope,
            "arch": self.arch, "bundleId": self.bundle_id, "lastUsed": self.last_used, "uninstall": self.uninstall, "blocked": self.blocked,
        })
    }
}

fn unquote(s: &str) -> Option<String> {
    let mut chars = s.strip_prefix('"')?.chars();
    let mut out = String::new();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some(n) => out.push(n),
                None => {
                    out.push('\\');
                }
            },
            '"' => return Some(out),
            _ => out.push(c),
        }
    }
    None
}

fn name_end(line: &[char]) -> Option<usize> {
    let mut i = 1;
    while i < line.len() {
        if line[i] == '\\' {
            i += 2;
            continue;
        }
        if line[i] == '"' {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn hex_bytes(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    for p in s.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        if p.len() > 2 || !p.bytes().all(|b| b.is_ascii_hexdigit()) {
            break;
        }
        out.push(u8::from_str_radix(p, 16).unwrap_or(0));
    }
    out
}

fn utf16(buf: &[u8]) -> String {
    let units: Vec<u16> = buf.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
    String::from_utf16_lossy(&units)
}

pub fn parse_reg_value(raw: &str) -> Option<RegValue> {
    if raw.starts_with('"') {
        return unquote(raw).map(RegValue::Str);
    }
    if let Some(d) = raw.get(..6).filter(|p| p.eq_ignore_ascii_case("dword:")).map(|_| &raw[6..]) {
        if (1..=8).contains(&d.len()) && d.bytes().all(|b| b.is_ascii_hexdigit()) {
            return u32::from_str_radix(d, 16).ok().map(|n| RegValue::Num(n as f64));
        }
        return None;
    }
    let lower = raw.to_lowercase();
    if !lower.starts_with("hex") {
        return None;
    }
    let rest = &raw[3..];
    let (ty, data) = if let Some(r) = rest.strip_prefix('(') {
        let close = r.find(')')?;
        let ty = &r[..close];
        if ty.is_empty() || !ty.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        (ty.to_lowercase(), r[close + 1..].strip_prefix(':')?)
    } else {
        ("3".to_owned(), rest.strip_prefix(':')?)
    };
    let buf = hex_bytes(data);
    match ty.as_str() {
        "1" | "2" => Some(RegValue::Str(utf16(&buf).trim_end_matches('\0').to_owned())),
        "7" => Some(RegValue::Multi(utf16(&buf).split('\0').filter(|s| !s.is_empty()).map(str::to_owned).collect())),
        "b" if buf.len() >= 8 => Some(RegValue::Num(u64::from_le_bytes(buf[..8].try_into().ok()?) as f64)),
        "4" if buf.len() >= 4 => Some(RegValue::Num(u32::from_le_bytes(buf[..4].try_into().ok()?) as f64)),
        _ => None,
    }
}

pub fn parse_reg_export(text: &str) -> Vec<RegKey> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let lines: Vec<&str> = text.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l)).collect();
    let mut keys: Vec<RegKey> = Vec::new();
    let mut cur: Option<usize> = None;
    let mut i = 0;
    while i < lines.len() {
        let mut line = lines[i].to_owned();
        while line.ends_with(",\\") && i + 1 < lines.len() {
            line.pop();
            i += 1;
            line.push_str(lines[i].trim());
        }
        i += 1;
        if line.starts_with('[') && line.ends_with(']') {
            let key = &line[1..line.len() - 1];
            cur = if key.starts_with('-') {
                None
            } else {
                keys.push(RegKey { key: key.to_owned(), values: HashMap::new() });
                Some(keys.len() - 1)
            };
            continue;
        }
        let Some(ci) = cur else { continue };
        if line.is_empty() {
            continue;
        }
        let (name, rest) = if let Some(r) = line.strip_prefix("@=") {
            (String::new(), r.to_owned())
        } else if line.starts_with('"') {
            let chars: Vec<char> = line.chars().collect();
            let Some(end) = name_end(&chars) else { continue };
            if chars.get(end + 1) != Some(&'=') {
                continue;
            }
            let quoted: String = chars[..=end].iter().collect();
            (unquote(&quoted).unwrap_or_default(), chars[end + 2..].iter().collect())
        } else {
            continue;
        };
        if let Some(v) = parse_reg_value(&rest) {
            keys[ci].values.insert(name.to_lowercase(), v);
        }
    }
    keys
}

fn s(v: Option<&RegValue>) -> Option<String> {
    match v {
        Some(RegValue::Str(t)) if !t.trim().is_empty() => Some(t.trim().to_owned()),
        _ => None,
    }
}

fn n(v: Option<&RegValue>) -> Option<f64> {
    match v {
        Some(RegValue::Num(x)) if x.is_finite() => Some(*x),
        Some(RegValue::Str(t)) if !t.trim().is_empty() && t.trim().bytes().all(|b| b.is_ascii_digit()) => t.trim().parse().ok(),
        _ => None,
    }
}

pub fn expand_env(text: &str, env: &HashMap<String, String>) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(start) = rest.find('%') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('%') {
            Some(end) if end > 0 && after[..end].chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '(' | ')')) => {
                let name = &after[..end];
                match env.get(&name.to_lowercase()) {
                    Some(v) => out.push_str(v),
                    None => out.push_str(&rest[start..start + end + 2]),
                }
                rest = &after[end + 1..];
            }
            _ => {
                out.push('%');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

pub fn env_map() -> HashMap<String, String> {
    std::env::vars_os().filter_map(|(k, v)| Some((k.into_string().ok()?.to_lowercase(), v.into_string().ok()?))).collect()
}

fn local_ms(y: i64, m: u32, d: u32) -> Option<f64> {
    let days = days_from_civil(y, m, d);
    let utc = days as f64 * 86_400_000.0;
    let probe = core_desk::telegram::clock::local(utc as i64);
    let shown = days_from_civil(probe.y, probe.mo, probe.d) as f64 * 86_400_000.0 + (probe.h as f64 * 3600.0 + probe.mi as f64 * 60.0 + probe.s as f64) * 1000.0;
    Some(utc - (shown - utc))
}

fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub fn install_date(v: Option<&str>) -> Option<f64> {
    let v = v?;
    if v.len() != 8 || !v.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let (y, mo, d): (i64, u32, u32) = (v[..4].parse().ok()?, v[4..6].parse().ok()?, v[6..].parse().ok()?);
    if !(1..=12).contains(&mo) || !(1..=31).contains(&d) || y < 1990 {
        return None;
    }
    local_ms(y, mo, d)
}

fn clean_location(v: Option<String>, env: &HashMap<String, String>) -> Option<String> {
    let v = v?;
    let t = v.trim();
    let t = if t.len() >= 2 && t.starts_with('"') && t.ends_with('"') { &t[1..t.len() - 1] } else { t };
    let mut p = expand_env(t, env);
    let b = p.as_bytes();
    let drive = b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'\\';
    if !drive && !p.starts_with("\\\\") {
        return None;
    }
    while p.chars().count() > 3 && (p.ends_with('\\') || p.ends_with('/')) {
        p.pop();
    }
    Some(p)
}

const UPDATE_TYPES: [&str; 5] = ["update", "hotfix", "security update", "service pack", "update rollup"];

fn kb_suffix(name: &str) -> bool {
    let upper = name.to_uppercase();
    let Some(rest) = upper.strip_suffix(')') else { return false };
    let Some(i) = rest.rfind("(KB") else { return false };
    let digits = &rest[i + 3..];
    digits.len() >= 6 && digits.bytes().all(|b| b.is_ascii_digit())
}

pub fn hidden_entry(values: &HashMap<String, RegValue>) -> bool {
    if n(values.get("systemcomponent")) == Some(1.0) || s(values.get("parentkeyname")).is_some() {
        return true;
    }
    if s(values.get("releasetype")).is_some_and(|r| UPDATE_TYPES.contains(&r.to_lowercase().as_str())) {
        return true;
    }
    let Some(name) = s(values.get("displayname")) else { return true };
    let lower = name.to_lowercase();
    lower.starts_with("update for ") || lower.starts_with("security update for ") || lower.starts_with("hotfix for ") || kb_suffix(&name)
}

pub fn apps_from_keys(keys: &[RegKey], src: &RegSource, env: &HashMap<String, String>) -> Vec<App> {
    let prefix = format!("{}\\", src.key).to_lowercase();
    let mut out = Vec::new();
    for k in keys {
        let lower = k.key.to_lowercase();
        let Some(tail) = lower.strip_prefix(&prefix) else { continue };
        if tail.contains('\\') || hidden_entry(&k.values) {
            continue;
        }
        let est = n(k.values.get("estimatedsize")).filter(|e| *e > 0.0);
        out.push(App {
            id: k.key.clone(),
            name: s(k.values.get("displayname")).unwrap_or_default(),
            publisher: s(k.values.get("publisher")),
            version: s(k.values.get("displayversion")),
            installed_at: install_date(s(k.values.get("installdate")).as_deref()),
            size: est.map(|e| e * 1024.0),
            size_from: est.map(|_| "estimate"),
            location: clean_location(s(k.values.get("installlocation")), env),
            source: "registry",
            scope: src.scope,
            arch: src.arch,
            bundle_id: None,
            last_used: None,
            uninstall: s(k.values.get("uninstallstring")),
            blocked: None,
        });
    }
    out
}

fn rank(a: &App) -> u8 {
    (if a.uninstall.is_some() { 0 } else { 4 }) + if a.scope == "machine" { if a.arch == Some("64") { 0 } else { 1 } } else { 2 }
}

pub fn dedupe(apps: Vec<App>) -> Vec<App> {
    let mut order: Vec<String> = Vec::new();
    let mut best: HashMap<String, App> = HashMap::new();
    for a in apps {
        let key = format!("{}|{}|{}", a.name.to_lowercase(), a.version.clone().unwrap_or_default().to_lowercase(), a.publisher.clone().unwrap_or_default().to_lowercase());
        match best.get(&key) {
            Some(prev) if rank(&a) >= rank(prev) => {}
            Some(_) => {
                best.insert(key, a);
            }
            None => {
                order.push(key.clone());
                best.insert(key, a);
            }
        }
    }
    order.into_iter().filter_map(|k| best.remove(&k)).collect()
}

pub fn decode_reg(buf: &[u8]) -> String {
    if buf.len() >= 2 && buf[0] == 0xff && buf[1] == 0xfe {
        return utf16(&buf[2..]);
    }
    String::from_utf8_lossy(buf).into_owned()
}

fn system_root(env: &HashMap<String, String>) -> String {
    env.get("systemroot").cloned().unwrap_or_else(|| "C:\\Windows".into())
}

pub fn wait_with_timeout(child: &mut std::process::Child, max: Duration) -> Option<std::process::ExitStatus> {
    let end = Instant::now() + max;
    loop {
        match child.try_wait() {
            Ok(Some(st)) => return Some(st),
            Ok(None) if Instant::now() < end => std::thread::sleep(Duration::from_millis(50)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    }
}

pub fn hidden(cmd: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

fn export_key(exe: &str, key: &str, file: &Path) -> bool {
    let Ok(mut child) = hidden(Command::new(exe).args(["export", key]).arg(file).arg("/y").stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())).spawn() else { return false };
    wait_with_timeout(&mut child, Duration::from_secs(30)).is_some_and(|s| s.success())
}

pub fn read_registry_text(env: &HashMap<String, String>) -> Option<Vec<(RegSource, String)>> {
    let dir: PathBuf = std::env::temp_dir().join(format!("orrery-apps-{}", crate::disk_organize_log::hex(6)));
    std::fs::create_dir_all(&dir).ok()?;
    let exe = format!("{}\\System32\\reg.exe", system_root(env));
    let handles: Vec<_> = REG_SOURCES
        .iter()
        .enumerate()
        .map(|(i, src)| {
            let (exe, file, src) = (exe.clone(), dir.join(format!("k{i}.reg")), *src);
            std::thread::spawn(move || export_key(&exe, src.key, &file).then(|| std::fs::read(&file).ok().map(|b| (src, decode_reg(&b)))).flatten())
        })
        .collect();
    let out: Vec<(RegSource, String)> = handles.into_iter().filter_map(|h| h.join().ok().flatten()).collect();
    let _ = std::fs::remove_dir_all(&dir);
    (!out.is_empty()).then_some(out)
}

pub fn read_fixture_text(dir: &Path) -> Option<Vec<(RegSource, String)>> {
    Some(REG_SOURCES.iter().enumerate().filter_map(|(i, src)| std::fs::read(dir.join(format!("k{i}.reg"))).ok().map(|b| (*src, decode_reg(&b)))).collect())
}

pub fn read_windows_apps(texts: Option<Vec<(RegSource, String)>>, env: &HashMap<String, String>) -> Option<Vec<App>> {
    let texts = texts?;
    let mut all = Vec::new();
    for (src, text) in &texts {
        all.extend(apps_from_keys(&parse_reg_export(text), src, env));
    }
    Some(dedupe(all))
}

#[derive(Clone, Debug, PartialEq)]
pub struct UninstallPlan {
    pub file: String,
    pub args: String,
    pub display: String,
}

const SHELLS: [&str; 14] = ["cmd.exe", "powershell.exe", "pwsh.exe", "wscript.exe", "cscript.exe", "mshta.exe", "bash.exe", "wsl.exe", "conhost.exe", "explorer.exe", "start.exe", "forfiles.exe", "schtasks.exe", "regsvr32.exe"];

fn unsafe_text(s: &str) -> bool {
    s.chars().any(|c| matches!(c, '&' | '|' | '<' | '>' | '^' | '`' | '\r' | '\n' | '\0'))
}

fn win_ext(p: &str) -> bool {
    let base = p.rsplit(['\\', '/']).next().unwrap_or(p);
    matches!(base.rfind('.'), Some(i) if i > 0 && i + 1 < base.len())
}

fn win_normalize(p: &str) -> String {
    crate::disk_guard::normalize(p, crate::disk_guard::Plat::Win)
}

fn resolve_exe(name: &str, env: &HashMap<String, String>, exists: &dyn Fn(&str) -> bool) -> Option<String> {
    let with_exe = |p: String| if win_ext(&p) { vec![p] } else { vec![p.clone(), format!("{p}.exe")] };
    let b = name.as_bytes();
    let abs = (b.len() >= 3 && b[0].is_ascii_alphabetic() && b[1] == b':' && b[2] == b'\\') || name.starts_with("\\\\");
    if abs {
        return with_exe(win_normalize(name)).into_iter().find(|c| exists(c));
    }
    if name.contains(['\\', '/']) {
        return None;
    }
    with_exe(format!("{}\\System32\\{name}", system_root(env))).into_iter().find(|c| exists(c))
}

fn msi_guid(args: &str) -> Option<String> {
    let a = args.strip_prefix('/')?;
    let first = a.chars().next()?;
    if !matches!(first, 'I' | 'i' | 'X' | 'x') {
        return None;
    }
    let g = a[1..].trim_start();
    let ok = g.len() == 38 && g.starts_with('{') && g.ends_with('}') && {
        let inner = &g[1..37];
        let parts: Vec<&str> = inner.split('-').collect();
        parts.len() == 5 && parts.iter().zip([8, 4, 4, 4, 12]).all(|(p, n)| p.len() == n && p.bytes().all(|b| b.is_ascii_hexdigit()))
    };
    ok.then(|| g.to_uppercase())
}

pub fn parse_uninstall(cmd: Option<&str>, env: &HashMap<String, String>, exists: &dyn Fn(&str) -> bool) -> Result<UninstallPlan, &'static str> {
    let raw = cmd.unwrap_or("").trim();
    if raw.is_empty() {
        return Err("empty");
    }
    if unsafe_text(raw) {
        return Err("unsafe");
    }
    let text = expand_env(raw, env);
    if unsafe_text(&text) {
        return Err("unsafe");
    }
    let (file, args) = if let Some(body) = text.strip_prefix('"') {
        let end = body.find('"').ok_or("unsafe")?;
        let rest = &body[end + 1..];
        if !rest.is_empty() && !rest.starts_with(char::is_whitespace) {
            return Err("unsafe");
        }
        let file = resolve_exe(&body[..end], env, exists).ok_or("missing")?;
        (file, rest.trim().to_owned())
    } else {
        let parts: Vec<&str> = text.split_whitespace().collect();
        let mut found = None;
        for i in 1..=parts.len() {
            let cand = parts[..i].join(" ");
            if i > 1 && !cand.to_lowercase().ends_with(".exe") {
                continue;
            }
            if let Some(hit) = resolve_exe(&cand, env, exists) {
                found = Some((hit, parts[i..].join(" ")));
                break;
            }
        }
        found.ok_or("missing")?
    };
    if args.matches('"').count() % 2 != 0 {
        return Err("unsafe");
    }
    let base = file.rsplit(['\\', '/']).next().unwrap_or(&file).to_lowercase();
    if SHELLS.contains(&base.as_str()) || !base.ends_with(".exe") {
        return Err("shell");
    }
    if base == "msiexec.exe" {
        let guid = msi_guid(&args).ok_or("msi")?;
        let msi = format!("{}\\System32\\msiexec.exe", system_root(env));
        if !exists(&msi) {
            return Err("missing");
        }
        return Ok(UninstallPlan { display: format!("\"{msi}\" /X{guid}"), file: msi, args: format!("/X{guid}") });
    }
    let display = if args.is_empty() { format!("\"{file}\"") } else { format!("\"{file}\" {args}") };
    Ok(UninstallPlan { file, args, display })
}

#[cfg(windows)]
pub fn launch(plan: &UninstallPlan, env: &HashMap<String, String>) -> bool {
    use std::os::windows::process::CommandExt;
    let mut cmd = Command::new(&plan.file);
    if !plan.args.is_empty() {
        cmd.raw_arg(&plan.args);
    }
    cmd.creation_flags(0x0000_0008 | 0x0000_0200).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null());
    match cmd.spawn() {
        Ok(_) => return true,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return false,
        Err(_) => {}
    }
    let ps = format!("{}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe", system_root(env));
    let script = "if ($env:ORRERY_UNINST_ARGS) { Start-Process -FilePath $env:ORRERY_UNINST_FILE -ArgumentList $env:ORRERY_UNINST_ARGS } else { Start-Process -FilePath $env:ORRERY_UNINST_FILE }";
    let Ok(mut child) = hidden(Command::new(ps).args(["-NoProfile", "-NonInteractive", "-Command", script]).env("ORRERY_UNINST_FILE", &plan.file).env("ORRERY_UNINST_ARGS", &plan.args).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())).spawn() else { return false };
    wait_with_timeout(&mut child, Duration::from_secs(120)).is_some_and(|s| s.success())
}

#[cfg(not(windows))]
pub fn launch(_plan: &UninstallPlan, _env: &HashMap<String, String>) -> bool {
    false
}
