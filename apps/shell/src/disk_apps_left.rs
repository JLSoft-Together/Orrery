use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;
use std::time::Duration;

use regex::Regex;
use serde_json::{json, Value};

use crate::disk_apps_reg::{hidden, wait_with_timeout, App};
use crate::disk_guard::{self as guard, Plat};

#[derive(Clone, Debug, PartialEq)]
pub struct Root {
    pub base: &'static str,
    pub dir: String,
    pub files: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LeftApp {
    pub name: String,
    pub publisher: Option<String>,
    pub bundle_id: Option<String>,
    pub location: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Hit {
    pub path: String,
    pub base: &'static str,
    pub exact: bool,
    pub file: bool,
    pub size: Option<f64>,
}

impl Hit {
    pub fn to_json(&self) -> Value {
        json!({ "path": self.path, "base": self.base, "match": if self.exact { "exact" } else { "loose" }, "size": self.size, "file": self.file })
    }
}

pub struct Entry {
    pub name: String,
    pub dir: bool,
    pub link: bool,
}

const GENERIC: &[&str] = &[
    "microsoft", "microsoftcorporation", "google", "googlellc", "apple", "appleinc", "adobe", "mozilla", "intel", "nvidia", "nvidiacorporation", "amd", "oracle", "packages", "programs", "temp", "tmp", "cache", "caches", "commonfiles", "windows", "system",
    "crashdumps", "crashreports", "logs", "log", "data", "config", "settings", "publishers", "squirreltemp", "electron", "node", "npm", "pip", "python", "java", "app", "apps", "application", "applications", "updater", "update", "updates", "installer",
    "setup", "uninstall", "default", "user", "users", "shared", "common", "local", "roaming", "cloud", "sync", "helper", "tools", "software", "virtualstore", "connecteddevicesplatform", "d3dscache", "diagnostics", "documents", "downloads", "desktop",
    "pictures", "music", "videos", "library", "preferences", "containers", "groupcontainers", "savedapplicationstate", "applicationsupport", "comapple", "orrery", "agentmonitor", "ccm",
];

fn generic(k: &str) -> bool {
    !k.is_empty() && GENERIC.contains(&k)
}

fn re(cell: &'static OnceLock<Regex>, pat: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pat).expect("regex"))
}

pub fn norm(s: &str) -> String {
    s.to_lowercase().chars().filter(|c| c.is_alphanumeric()).collect()
}

pub fn app_key(name: &str) -> String {
    static A: OnceLock<Regex> = OnceLock::new();
    static B: OnceLock<Regex> = OnceLock::new();
    static C: OnceLock<Regex> = OnceLock::new();
    static D: OnceLock<Regex> = OnceLock::new();
    static E: OnceLock<Regex> = OnceLock::new();
    static F: OnceLock<Regex> = OnceLock::new();
    let s = re(&A, r"(?i)\.app$").replace(name, "");
    let s = re(&B, r"\([^)]*\)").replace_all(&s, " ");
    let s = re(&C, r"(?i)(?-u:\b)(x64|x86|amd64|arm64|64-bit|32-bit|64 bit|32 bit)(?-u:\b)").replace_all(&s, " ");
    let s = re(&D, r"(?i)\s+(v(ersion)?|phiên bản)\s*[0-9][A-Za-z0-9_.\-]*").replace_all(&s, " ");
    let s = re(&E, r"\s+v?[0-9]+(\.[0-9]+)+[A-Za-z0-9_.\-]*").replace_all(&s, " ");
    let s = re(&F, r"\s+[0-9]+$").replace_all(&s, " ");
    norm(&s)
}

pub fn publisher_key(p: Option<&str>) -> String {
    static A: OnceLock<Regex> = OnceLock::new();
    static B: OnceLock<Regex> = OnceLock::new();
    let Some(p) = p else { return String::new() };
    let s = re(&A, r"(?i)^công ty (tnhh|cổ phần)\s*").replace(p, "");
    let s = re(&B, r"(?i)(?-u:\b)(incorporated|inc|llc|ltd|limited|corp|corporation|co|company|gmbh|ag|sa|srl|bv|oy|ab|pty|plc|kk)(?-u:\b)\.?").replace_all(&s, " ");
    norm(&s)
}

fn lexical_under(child: &str, base: &str, plat: Plat) -> bool {
    let c = guard::normalize(child, plat).to_lowercase();
    let b = guard::normalize(base, plat).to_lowercase();
    let b = b.trim_end_matches(['\\', '/']);
    let sep = if plat == Plat::Win { '\\' } else { '/' };
    c == b || c.starts_with(&format!("{b}{sep}"))
}

pub fn read_entries(dir: &str) -> Vec<Entry> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    rd.flatten().filter_map(|e| e.file_type().ok().map(|ft| Entry { name: e.file_name().to_string_lossy().into_owned(), dir: ft.is_dir() && !ft.is_symlink(), link: ft.is_symlink() })).collect()
}

pub struct Query<'a> {
    pub app: &'a LeftApp,
    pub installed: bool,
    pub others: &'a [LeftApp],
}

pub fn find_leftovers(q: &Query, roots: &[Root], plat: Plat, readdir: &dyn Fn(&str) -> Vec<Entry>, exists: &dyn Fn(&str) -> bool) -> Vec<Hit> {
    let key = app_key(&q.app.name);
    let publisher = publisher_key(q.app.publisher.as_deref());
    let bundle = q.app.bundle_id.as_ref().map(|b| b.to_lowercase());
    let bundle_key = bundle.as_deref().map(norm).unwrap_or_default();
    let usable = |k: &str, min: usize| k.chars().count() >= min && !generic(k);
    let other_keys: HashSet<String> = q.others.iter().map(|o| app_key(&o.name)).filter(|k| !k.is_empty() && *k != key).collect();
    let other_bundles: HashSet<String> = q.others.iter().filter_map(|o| o.bundle_id.as_ref().map(|b| b.to_lowercase())).filter(|b| Some(b) != bundle.as_ref()).collect();
    let pub_shared = !publisher.is_empty() && q.others.iter().any(|o| publisher_key(o.publisher.as_deref()) == publisher);
    let mut locations: Vec<String> = q.others.iter().filter_map(|o| o.location.clone()).collect();
    if q.installed {
        if let Some(l) = &q.app.location {
            locations.push(l.clone());
        }
    }
    let owned = |p: &str| locations.iter().any(|l| lexical_under(p, l, plat) || lexical_under(l, p, plat));
    let mut order: Vec<String> = Vec::new();
    let mut hits: HashMap<String, Hit> = HashMap::new();
    let mut add = |p: String, base: &'static str, exact: bool, file: bool| {
        let k = p.to_lowercase();
        if let Some(prev) = hits.get(&k) {
            if prev.exact || !exact {
                return;
            }
        }
        if owned(&p) {
            return;
        }
        if !hits.contains_key(&k) {
            order.push(k.clone());
        }
        hits.insert(k, Hit { path: p, base, exact, file, size: None });
    };
    let strip_ext = |s: &str| -> String {
        let l = s.to_lowercase();
        for ext in [".plist", ".savedstate"] {
            if l.ends_with(ext) {
                return s[..s.len() - ext.len()].to_owned();
            }
        }
        s.to_owned()
    };
    let name_match = |raw: &str| -> Option<bool> {
        let lower = raw.to_lowercase();
        let n = norm(&strip_ext(raw));
        if n.is_empty() || other_keys.contains(&n) {
            return None;
        }
        if bundle.is_some() && other_bundles.contains(&strip_ext(&lower)) {
            return None;
        }
        if let Some(b) = &bundle {
            if lower == *b || lower == format!("{b}.plist") || lower == format!("{b}.savedstate") {
                return Some(true);
            }
            if lower.starts_with(&format!("{b}.")) {
                return Some(false);
            }
        }
        if generic(&n) {
            return None;
        }
        if usable(&key, 3) && n == key {
            return Some(true);
        }
        if usable(&bundle_key, 6) && n == bundle_key {
            return Some(true);
        }
        if usable(&key, 4) && n.chars().count() >= 4 && (n.starts_with(&key) || key.starts_with(&n)) {
            if other_keys.iter().any(|o| o.chars().count() >= 4 && (o.starts_with(&n) || n.starts_with(o.as_str()))) {
                return None;
            }
            return Some(false);
        }
        None
    };
    for root in roots {
        for e in readdir(&root.dir) {
            if e.link || (!e.dir && !root.files) {
                continue;
            }
            let full = guard::join(&root.dir, &e.name, plat);
            let n = norm(&e.name);
            if e.dir && !publisher.is_empty() && n == publisher && usable(&publisher, 3) {
                let mut any = false;
                for c in readdir(&full) {
                    if c.link || !c.dir {
                        continue;
                    }
                    if let Some(exact) = name_match(&c.name) {
                        any = true;
                        add(guard::join(&full, &c.name, plat), root.base, exact, false);
                    }
                }
                if !any && !pub_shared && n != key {
                    add(full.clone(), root.base, false, false);
                }
                if n != key {
                    continue;
                }
            }
            if let Some(exact) = name_match(&e.name) {
                add(full, root.base, exact, !e.dir);
            }
        }
    }
    if !q.installed {
        if let Some(l) = q.app.location.as_ref().filter(|l| exists(l)) {
            add(l.clone(), "location", true, false);
        }
    }
    let list: Vec<Hit> = order.into_iter().filter_map(|k| hits.remove(&k)).collect();
    list.iter().filter(|h| !list.iter().any(|o| !std::ptr::eq(*h, o) && lexical_under(&h.path, &o.path, plat) && o.path.to_lowercase() != h.path.to_lowercase())).cloned().collect()
}

pub fn windows_roots(appdata: Option<String>, local: Option<String>, program_data: Option<String>) -> Vec<Root> {
    let mut out = Vec::new();
    if let Some(a) = appdata.filter(|s| !s.is_empty()) {
        out.push(Root { base: "appdata", dir: a, files: false });
    }
    if let Some(l) = local.filter(|s| !s.is_empty()) {
        out.push(Root { base: "localappdata", dir: l.clone(), files: false });
        out.push(Root { base: "localappdata", dir: guard::join(&l, "Programs", Plat::Win), files: false });
    }
    if let Some(p) = program_data.filter(|s| !s.is_empty()) {
        out.push(Root { base: "programdata", dir: p, files: false });
    }
    out
}

pub fn mac_roots(home: &str) -> Vec<Root> {
    let lib = format!("{}/Library", home.trim_end_matches('/'));
    [("support", "Application Support", false), ("caches", "Caches", false), ("preferences", "Preferences", true), ("containers", "Containers", false), ("savedState", "Saved Application State", false), ("logs", "Logs", false)]
        .into_iter()
        .map(|(base, sub, files)| Root { base, dir: format!("{lib}/{sub}"), files })
        .collect()
}

pub fn dir_size(p: &Path, deadline: std::time::Instant) -> Option<u64> {
    if std::time::Instant::now() > deadline {
        return None;
    }
    let mut total = 0u64;
    for e in std::fs::read_dir(p).ok()?.flatten() {
        let Ok(m) = e.path().symlink_metadata() else { continue };
        if m.file_type().is_symlink() {
            continue;
        }
        total += if m.is_dir() { dir_size(&e.path(), deadline)? } else { m.len() };
    }
    Some(total)
}

fn decode_xml(s: &str) -> String {
    static R: OnceLock<Regex> = OnceLock::new();
    let s = s.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'");
    let s = re(&R, r"(?i)&#(x[0-9a-f]+|[0-9]+);").replace_all(&s, |c: &regex::Captures| {
        let t = &c[1];
        let n = if let Some(h) = t.strip_prefix(['x', 'X']) { u32::from_str_radix(h, 16).ok() } else { t.parse().ok() };
        n.and_then(char::from_u32).map(String::from).unwrap_or_default()
    });
    s.replace("&amp;", "&")
}

pub fn parse_plist_strings(xml: &str) -> HashMap<String, String> {
    static BODY: OnceLock<Regex> = OnceLock::new();
    static TOK: OnceLock<Regex> = OnceLock::new();
    let mut out = HashMap::new();
    let Some(body) = re(&BODY, r"<plist[^>]*>\s*<dict>([\s\S]*)</dict>\s*</plist>").captures(xml).and_then(|c| c.get(1)) else { return out };
    let mut depth: i32 = 0;
    for m in re(&TOK, r"<(/?)(dict|array)\b[^>]*?(/?)>|<key>([^<]*)</key>\s*<string>([^<]*)</string>|<key>[^<]*</key>").captures_iter(body.as_str()) {
        if m.get(2).is_some() {
            if m.get(3).is_some_and(|x| x.as_str() == "/") {
                continue;
            }
            depth += if m.get(1).is_some_and(|x| x.as_str() == "/") { -1 } else { 1 };
            continue;
        }
        if depth == 0 {
            if let (Some(k), Some(v)) = (m.get(4), m.get(5)) {
                out.insert(decode_xml(k.as_str().trim()), decode_xml(v.as_str().trim()));
            }
        }
    }
    out
}

fn run(file: &str, args: &[&str], max: Duration) -> Option<String> {
    let mut child = hidden(Command::new(file).args(args).env("LC_ALL", "C").stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null())).spawn().ok()?;
    let mut out = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut s = Vec::new();
        let _ = std::io::Read::read_to_end(&mut out, &mut s);
        s
    });
    let ok = wait_with_timeout(&mut child, max).is_some_and(|s| s.success());
    let bytes = reader.join().ok()?;
    ok.then(|| String::from_utf8_lossy(&bytes).into_owned())
}

fn read_plist(file: &str) -> HashMap<String, String> {
    let Ok(buf) = std::fs::read(file) else { return HashMap::new() };
    if !buf.starts_with(b"bplist") {
        return parse_plist_strings(&String::from_utf8_lossy(&buf));
    }
    run("/usr/bin/plutil", &["-convert", "xml1", "-o", "-", file], Duration::from_secs(15)).map(|x| parse_plist_strings(&x)).unwrap_or_default()
}

pub fn blocked_mac(app_path: &str, bundle: Option<&str>, self_bundle: Option<&str>) -> Option<&'static str> {
    let n = |p: &str| guard::normalize(p, Plat::Mac);
    if self_bundle.is_some_and(|s| n(s) == n(app_path)) || bundle == Some("dev.ccm.monitor") {
        return Some("self");
    }
    if app_path.starts_with("/System/") || bundle.is_some_and(|b| b.to_lowercase().starts_with("com.apple.")) {
        return Some("system");
    }
    None
}

pub fn last_used_of(text: Option<&str>, count: usize) -> Vec<Option<f64>> {
    let parts: Vec<&str> = text.map(|t| t.split('\0').collect()).unwrap_or_default();
    (0..count)
        .map(|i| {
            let v = parts.get(i)?.trim();
            if v.is_empty() || v == "(null)" {
                return None;
            }
            parse_mdls_date(v)
        })
        .collect()
}

fn parse_mdls_date(v: &str) -> Option<f64> {
    let b = v.as_bytes();
    if b.len() < 19 {
        return None;
    }
    let num = |a: usize, z: usize| v.get(a..z)?.parse::<i64>().ok();
    let (y, mo, d, h, mi, s) = (num(0, 4)?, num(5, 7)?, num(8, 10)?, num(11, 13)?, num(14, 16)?, num(17, 19)?);
    let off = match v.get(19..).map(str::trim) {
        Some(z) if z.len() == 5 && (z.starts_with('+') || z.starts_with('-')) => {
            let sign = if z.starts_with('-') { -1 } else { 1 };
            sign * (z[1..3].parse::<i64>().ok()? * 60 + z[3..5].parse::<i64>().ok()?)
        }
        _ => 0,
    };
    let days = {
        let yy = if mo <= 2 { y - 1 } else { y };
        let era = yy.div_euclid(400);
        let yoe = yy - era * 400;
        let mp = (mo + 9) % 12;
        let doy = (153 * mp + 2) / 5 + d - 1;
        era * 146_097 + yoe * 365 + yoe / 4 - yoe / 100 + doy - 719_468
    };
    Some(((days * 86_400 + h * 3600 + mi * 60 + s - off * 60) * 1000) as f64)
}

fn du_bytes(dir: &str) -> Option<f64> {
    let out = run("/usr/bin/du", &["-sk", dir], Duration::from_secs(20))?;
    let kb: f64 = out.trim().split_whitespace().next()?.parse().ok()?;
    Some(kb * 1024.0)
}

pub fn read_mac_apps(roots: &[String], self_bundle: Option<&str>, index: &dyn Fn(&[String]) -> Option<HashMap<String, Option<f64>>>) -> Vec<App> {
    let mut found: Vec<(String, &'static str)> = Vec::new();
    for (i, root) in roots.iter().enumerate() {
        for e in read_entries(root) {
            if e.dir && e.name.ends_with(".app") {
                found.push((format!("{}/{}", root.trim_end_matches('/'), e.name), if i == 0 { "machine" } else { "user" }));
            }
        }
    }
    if found.is_empty() {
        return Vec::new();
    }
    let paths: Vec<String> = found.iter().map(|f| f.0.clone()).collect();
    let sizes = index(&paths);
    let mut mdls_args: Vec<&str> = vec!["-name", "kMDItemLastUsedDate", "-raw"];
    mdls_args.extend(paths.iter().map(String::as_str));
    let used = last_used_of(run("/usr/bin/mdls", &mdls_args, Duration::from_secs(15)).as_deref(), found.len());
    let next = std::sync::atomic::AtomicUsize::new(0);
    let slots: Vec<std::sync::Mutex<Option<App>>> = (0..found.len()).map(|_| std::sync::Mutex::new(None)).collect();
    std::thread::scope(|sc| {
        for _ in 0..found.len().min(4) {
            sc.spawn(|| loop {
                let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let Some((p, scope)) = found.get(i) else { break };
                let meta = std::fs::metadata(p).ok();
                let info = read_plist(&format!("{p}/Contents/Info.plist"));
                let pick = |keys: &[&str]| keys.iter().find_map(|k| info.get(*k).map(|v| v.trim().to_owned()).filter(|v| !v.is_empty()));
                let bundle = pick(&["CFBundleIdentifier"]);
                let from_index = sizes.as_ref().and_then(|s| s.get(p).copied().flatten());
                let size = from_index.or_else(|| du_bytes(p));
                let base = p.rsplit('/').next().unwrap_or(p).trim_end_matches(".app").to_owned();
                let installed = meta.and_then(|m| m.created().or_else(|_| m.modified()).ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_millis() as f64);
                *crate::state::lock(&slots[i]) = Some(App {
                    id: p.clone(),
                    name: pick(&["CFBundleDisplayName", "CFBundleName"]).unwrap_or(base),
                    publisher: None,
                    version: pick(&["CFBundleShortVersionString", "CFBundleVersion"]),
                    installed_at: installed,
                    size,
                    size_from: if from_index.is_some() { Some("index") } else if size.is_some() { Some("du") } else { None },
                    location: Some(p.clone()),
                    source: "bundle",
                    scope,
                    arch: None,
                    blocked: blocked_mac(p, bundle.as_deref(), self_bundle),
                    bundle_id: bundle,
                    last_used: used.get(i).copied().flatten(),
                    uninstall: None,
                });
            });
        }
    });
    slots.into_iter().filter_map(|s| s.into_inner().ok().flatten()).collect()
}
