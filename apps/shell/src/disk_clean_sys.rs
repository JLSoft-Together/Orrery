use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::disk_clean::{is_link, lstat, mtime_ms, Ctx, Item, Member};
use crate::disk_clean_dev::run;
use crate::disk_guard::{self as guard, Plat};

pub const HOUR_MS: f64 = 3_600_000.0;
pub const DAY_MS: f64 = 24.0 * HOUR_MS;
pub const TEMP_AGE_MS: f64 = DAY_MS;
pub const LOG_AGE_MS: f64 = 7.0 * DAY_MS;
pub const INSTALLER_AGE_MS: f64 = 30.0 * DAY_MS;
pub const MEMBERS_MAX: usize = 20_000;

#[derive(Clone, Debug, PartialEq)]
pub struct SysRoots {
    pub plat: Plat,
    pub home: String,
    pub local: String,
    pub temp: String,
    pub windir: String,
    pub program_data: String,
    pub downloads: String,
    pub mac_tmp: String,
    pub bin: Option<String>,
}

pub trait SysIo: Send + Sync {
    fn procs(&self) -> Vec<String>;
    fn apps(&self) -> Vec<String> {
        Vec::new()
    }
    fn bin_size(&self) -> Option<(u64, u64)>;
    fn bin_empty(&self) -> Result<(), String>;
    fn busy(&self, file: &str) -> bool;
}

pub struct SysDeps {
    pub roots: SysRoots,
    pub io: Box<dyn SysIo>,
}

fn j(r: &SysRoots, parts: &[&str]) -> String {
    parts.iter().skip(1).fold(parts[0].to_owned(), |acc, p| guard::join(&acc, p, r.plat))
}

pub fn sys_roots(plat: Plat, home: &str, env: &dyn Fn(&str) -> Option<String>, downloads: Option<String>, tmpdir: &str, fake: Option<&str>) -> SysRoots {
    let jn = |parts: &[&str]| parts.iter().skip(1).fold(parts[0].to_owned(), |acc, p| guard::join(&acc, p, plat));
    if let Some(f) = fake {
        let h = jn(&[f, "home"]);
        return SysRoots {
            plat,
            home: h.clone(),
            local: jn(&[&h, "AppData", "Local"]),
            temp: jn(&[f, "Temp"]),
            windir: jn(&[f, "Windows"]),
            program_data: jn(&[f, "ProgramData"]),
            downloads: jn(&[&h, "Downloads"]),
            mac_tmp: jn(&[f, "var", "T"]),
            bin: Some(jn(&[f, "Bin"])),
        };
    }
    let get = |k: &str| env(k).filter(|v| !v.is_empty());
    let local = get("LOCALAPPDATA").unwrap_or_else(|| jn(&[home, "AppData", "Local"]));
    SysRoots {
        plat,
        home: home.to_owned(),
        temp: get("TEMP").or_else(|| get("TMP")).unwrap_or_else(|| jn(&[&local, "Temp"])),
        local,
        windir: get("SystemRoot").or_else(|| get("windir")).unwrap_or_else(|| "C:\\Windows".into()),
        program_data: get("ProgramData").unwrap_or_else(|| "C:\\ProgramData".into()),
        downloads: downloads.filter(|d| !d.is_empty()).unwrap_or_else(|| jn(&[home, "Downloads"])),
        mac_tmp: get("TMPDIR").unwrap_or_else(|| tmpdir.to_owned()),
        bin: None,
    }
}

const WIN_FORBIDDEN: [&str; 4] = ["C:\\Windows", "C:\\Program Files", "C:\\Program Files (x86)", "C:\\ProgramData"];
const MAC_FORBIDDEN: [&str; 8] = ["/System", "/Library", "/usr", "/bin", "/sbin", "/Applications", "/private/etc", "/etc"];
const HOME_FORBIDDEN: [&str; 11] = ["Documents", "Desktop", "Pictures", "Music", "Videos", "Movies", ".ssh", ".config", ".gnupg", "OneDrive", "iCloudDrive"];
const MAC_HOME_FORBIDDEN: [&[&str]; 4] = [&["Library", "Keychains"], &["Library", "Mobile Documents"], &["Library", "CloudStorage"], &["Library", "Application Support", "MobileSync"]];

pub fn forbidden_bases(r: &SysRoots) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if r.plat == Plat::Win {
        out.extend(WIN_FORBIDDEN.iter().map(|s| s.to_string()));
        out.push(r.windir.clone());
        out.push(r.program_data.clone());
    } else {
        out.extend(MAC_FORBIDDEN.iter().map(|s| s.to_string()));
        for parts in MAC_HOME_FORBIDDEN {
            let mut all = vec![r.home.as_str()];
            all.extend_from_slice(parts);
            out.push(j(r, &all));
        }
    }
    for n in HOME_FORBIDDEN {
        out.push(j(r, &[&r.home, n]));
    }
    out.into_iter().filter(|s| !s.is_empty()).collect()
}

pub fn is_forbidden(p: &str, r: &SysRoots, extra: &[String]) -> bool {
    if guard::is_under(&r.home, p, r.plat) {
        return true;
    }
    forbidden_bases(r).iter().chain(extra.iter()).any(|b| !b.is_empty() && guard::is_under(p, b, r.plat))
}

pub fn tree_info(root: &str, abort: &AtomicBool, stop_above: f64) -> Option<(u64, f64, bool)> {
    let mut bytes = 0u64;
    let mut newest = 0f64;
    let mut readable = true;
    let mut first = true;
    let mut stack = vec![root.to_owned()];
    while let Some(dir) = stack.pop() {
        if abort.load(Ordering::SeqCst) {
            return None;
        }
        let rd = match std::fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(_) => {
                if first {
                    readable = false;
                }
                first = false;
                continue;
            }
        };
        first = false;
        for e in rd.flatten() {
            let Ok(m) = e.metadata() else { continue };
            if is_link(&m) {
                continue;
            }
            newest = newest.max(mtime_ms(&m));
            if m.is_dir() {
                stack.push(guard::join(&dir, &e.file_name().to_string_lossy(), Plat::host()));
            } else if m.is_file() {
                bytes += m.len();
            }
        }
        if newest > stop_above {
            return Some((bytes, newest, readable));
        }
    }
    Some((bytes, newest, readable))
}

fn is_dir(p: &str) -> bool {
    lstat(p).is_some_and(|m| m.is_dir() && !is_link(&m))
}

pub struct Rule {
    pub older_ms: f64,
    pub files_only: bool,
    pub matcher: Option<fn(&str, bool) -> bool>,
}

fn rule(older_ms: f64) -> Rule {
    Rule { older_ms, files_only: false, matcher: None }
}

pub fn members_of(ctx: &Ctx, s: &SysDeps, dir: &str, rule: &Rule) -> Option<(Vec<Member>, bool)> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Some((Vec::new(), false)) };
    let cutoff = ctx.now - rule.older_ms;
    let mut list = Vec::new();
    for e in rd.flatten() {
        if list.len() >= MEMBERS_MAX {
            break;
        }
        if ctx.aborted() {
            return None;
        }
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_symlink() {
            continue;
        }
        let file = ft.is_file();
        if !file && !ft.is_dir() {
            continue;
        }
        if rule.files_only && !file {
            continue;
        }
        let name = e.file_name().to_string_lossy().into_owned();
        if rule.matcher.is_some_and(|m| !m(&name, file)) {
            continue;
        }
        let p = guard::join(dir, &name, ctx.plat);
        if is_forbidden(&p, &s.roots, &[]) || !ctx.keep_member(&p) {
            continue;
        }
        let Some(st) = lstat(&p) else { continue };
        if is_link(&st) || mtime_ms(&st) > cutoff {
            continue;
        }
        if file {
            if s.io.busy(&p) {
                continue;
            }
            list.push(Member { path: p, size: st.len(), mtime: mtime_ms(&st), file: true });
            continue;
        }
        let (bytes, newest, _) = tree_info(&p, ctx.abort, cutoff)?;
        if newest.max(mtime_ms(&st)) > cutoff {
            continue;
        }
        list.push(Member { path: p, size: bytes, mtime: mtime_ms(&st), file: false });
    }
    Some((list, true))
}

fn files_item(ctx: &Ctx, s: &SysDeps, category: &str, kind: &str, dir: &str, rule: &Rule) {
    if !is_dir(dir) || is_forbidden(dir, &s.roots, &[]) {
        return;
    }
    let Some((list, readable)) = members_of(ctx, s, dir, rule) else { return };
    if !readable {
        let mut it = Item::new(dir, 0, "system", category).kind(kind);
        it.locked = Some("denied");
        it.unknown = true;
        ctx.emit(it);
        return;
    }
    if list.is_empty() {
        return;
    }
    let mut it = Item::new(dir, list.iter().map(|m| m.size).sum(), "system", category).kind(kind);
    it.action = Some("files");
    it.files = Some(list.len() as u64);
    ctx.attach(dir, list);
    ctx.emit(it);
}

fn admin_item(ctx: &Ctx, category: &str, kind: &str, dir: &str) {
    if !is_dir(dir) {
        return;
    }
    let Some((bytes, _, readable)) = tree_info(dir, ctx.abort, f64::INFINITY) else { return };
    let mut it = Item::new(dir, bytes, "system", category).kind(kind);
    it.locked = Some("admin");
    it.unknown = !readable;
    ctx.emit(it);
}

fn skip_apple(name: &str, _file: bool) -> bool {
    !name.to_lowercase().starts_with("com.apple.")
}

pub fn scan_temp(ctx: &Ctx) {
    let Some(s) = ctx.system else { return };
    let r = &s.roots;
    if r.plat == Plat::Win {
        files_item(ctx, s, "temp", "userTemp", &r.temp, &rule(TEMP_AGE_MS));
        if !ctx.aborted() {
            admin_item(ctx, "temp", "winTemp", &j(r, &[&r.windir, "Temp"]));
        }
        return;
    }
    if r.mac_tmp.is_empty() {
        return;
    }
    let apple = Rule { older_ms: TEMP_AGE_MS, files_only: false, matcher: Some(skip_apple) };
    files_item(ctx, s, "temp", "userTemp", &r.mac_tmp, &apple);
    if !ctx.aborted() {
        let t = r.mac_tmp.trim_end_matches('/');
        files_item(ctx, s, "temp", "userCache", &j(r, &[&guard::parent(t, Plat::Mac), "C"]), &apple);
    }
}

pub struct BrowserSpot {
    pub id: &'static str,
    pub name: &'static str,
    pub bases: Vec<String>,
    pub caches: &'static [&'static str],
    pub procs: &'static [&'static str],
}

pub const CHROMIUM_CACHES: [&str; 3] = ["Cache", "Code Cache", "GPUCache"];
pub const FIREFOX_CACHES: [&str; 1] = ["cache2"];

pub fn browser_spots(r: &SysRoots) -> Vec<BrowserSpot> {
    if r.plat == Plat::Win {
        return vec![
            BrowserSpot { id: "chrome", name: "Chrome", bases: vec![j(r, &[&r.local, "Google", "Chrome", "User Data"])], caches: &CHROMIUM_CACHES, procs: &["chrome.exe"] },
            BrowserSpot { id: "edge", name: "Edge", bases: vec![j(r, &[&r.local, "Microsoft", "Edge", "User Data"])], caches: &CHROMIUM_CACHES, procs: &["msedge.exe"] },
            BrowserSpot { id: "brave", name: "Brave", bases: vec![j(r, &[&r.local, "BraveSoftware", "Brave-Browser", "User Data"])], caches: &CHROMIUM_CACHES, procs: &["brave.exe"] },
            BrowserSpot { id: "firefox", name: "Firefox", bases: vec![j(r, &[&r.local, "Mozilla", "Firefox", "Profiles"])], caches: &FIREFOX_CACHES, procs: &["firefox.exe"] },
        ];
    }
    if r.plat != Plat::Mac {
        return Vec::new();
    }
    let lc = j(r, &[&r.home, "Library", "Caches"]);
    let support = j(r, &[&r.home, "Library", "Application Support"]);
    vec![
        BrowserSpot { id: "chrome", name: "Chrome", bases: vec![j(r, &[&lc, "Google", "Chrome"]), j(r, &[&support, "Google", "Chrome"])], caches: &CHROMIUM_CACHES, procs: &["google chrome"] },
        BrowserSpot { id: "edge", name: "Edge", bases: vec![j(r, &[&lc, "Microsoft Edge"]), j(r, &[&support, "Microsoft Edge"])], caches: &CHROMIUM_CACHES, procs: &["microsoft edge"] },
        BrowserSpot { id: "brave", name: "Brave", bases: vec![j(r, &[&lc, "BraveSoftware", "Brave-Browser"]), j(r, &[&support, "BraveSoftware", "Brave-Browser"])], caches: &CHROMIUM_CACHES, procs: &["brave browser"] },
        BrowserSpot { id: "firefox", name: "Firefox", bases: vec![j(r, &[&lc, "Firefox", "Profiles"])], caches: &FIREFOX_CACHES, procs: &["firefox"] },
    ]
}

pub fn proc_name(p: &str) -> String {
    p.replace('\\', "/").rsplit('/').next().unwrap_or("").to_lowercase()
}

pub fn running_browsers(s: &SysDeps) -> HashSet<&'static str> {
    let names: HashSet<String> = s.io.procs().iter().map(|p| proc_name(p)).collect();
    browser_spots(&s.roots).into_iter().filter(|b| b.procs.iter().any(|p| names.contains(*p))).map(|b| b.id).collect()
}

pub fn browser_cache_ok(p: &str, spot: &BrowserSpot, r: &SysRoots) -> bool {
    if !spot.caches.contains(&guard::base_name(p, r.plat).as_str()) {
        return false;
    }
    let profile = guard::parent(p, r.plat);
    let up = guard::parent(&profile, r.plat).to_lowercase();
    spot.bases.iter().any(|b| guard::normalize(b, r.plat).to_lowercase() == up)
}

pub fn scan_browser(ctx: &Ctx) {
    let Some(s) = ctx.system else { return };
    let running = running_browsers(s);
    for spot in browser_spots(&s.roots) {
        for base in &spot.bases {
            let Ok(rd) = std::fs::read_dir(base) else { continue };
            for prof in rd.flatten() {
                if ctx.aborted() {
                    return;
                }
                let Ok(ft) = prof.file_type() else { continue };
                if !ft.is_dir() || ft.is_symlink() {
                    continue;
                }
                let pname = prof.file_name().to_string_lossy().into_owned();
                for c in spot.caches {
                    let p = j(&s.roots, &[base, &pname, c]);
                    if !is_dir(&p) || !browser_cache_ok(&p, &spot, &s.roots) || is_forbidden(&p, &s.roots, &[]) {
                        continue;
                    }
                    let mut it = Item::new(&p, ctx.size(&p), "system", "browser").kind(spot.id);
                    it.detail = Some(format!("{}, {}, {}", spot.name, pname, c));
                    if running.contains(spot.id) {
                        it.locked = Some("browser");
                    }
                    ctx.emit(it);
                }
            }
        }
    }
}

pub fn scan_crash(ctx: &Ctx) {
    let Some(s) = ctx.system else { return };
    let r = &s.roots;
    if r.plat == Plat::Win {
        files_item(ctx, s, "crash", "crashDumps", &j(r, &[&r.local, "CrashDumps"]), &rule(0.0));
        if !ctx.aborted() {
            admin_item(ctx, "crash", "minidump", &j(r, &[&r.windir, "Minidump"]));
        }
        return;
    }
    files_item(ctx, s, "crash", "diagnostic", &j(r, &[&r.home, "Library", "Logs", "DiagnosticReports"]), &rule(0.0));
}

fn not_diag(name: &str, _file: bool) -> bool {
    name != "DiagnosticReports"
}

pub fn scan_logs(ctx: &Ctx) {
    let Some(s) = ctx.system else { return };
    let r = &s.roots;
    if r.plat == Plat::Win {
        let wer = |base: &str, n: &str| j(r, &[base, "Microsoft", "Windows", "WER", n]);
        files_item(ctx, s, "logs", "werArchive", &wer(&r.local, "ReportArchive"), &rule(0.0));
        if !ctx.aborted() {
            files_item(ctx, s, "logs", "werQueue", &wer(&r.local, "ReportQueue"), &rule(0.0));
        }
        if !ctx.aborted() {
            admin_item(ctx, "logs", "werArchiveAll", &wer(&r.program_data, "ReportArchive"));
        }
        if !ctx.aborted() {
            admin_item(ctx, "logs", "werQueueAll", &wer(&r.program_data, "ReportQueue"));
        }
        return;
    }
    files_item(ctx, s, "logs", "userLogs", &j(r, &[&r.home, "Library", "Logs"]), &Rule { older_ms: LOG_AGE_MS, files_only: false, matcher: Some(not_diag) });
}

fn thumbcache(name: &str, _file: bool) -> bool {
    let n = name.to_lowercase();
    n.starts_with("thumbcache_") && n.ends_with(".db")
}

pub fn scan_thumbs(ctx: &Ctx) {
    let Some(s) = ctx.system else { return };
    let r = &s.roots;
    files_item(ctx, s, "thumbs", "thumbcache", &j(r, &[&r.local, "Microsoft", "Windows", "Explorer"]), &Rule { older_ms: 0.0, files_only: true, matcher: Some(thumbcache) });
}

pub fn installer_name(name: &str, plat: Plat) -> bool {
    let n = name.to_lowercase();
    if plat == Plat::Mac {
        return n.ends_with(".dmg") || n.ends_with(".pkg");
    }
    if [".exe", ".msi", ".dmg", ".pkg"].iter().any(|e| n.ends_with(e)) {
        return true;
    }
    n.ends_with(".zip") && (n.contains("setup") || n.contains("install"))
}

pub fn scan_installers(ctx: &Ctx) {
    let Some(s) = ctx.system else { return };
    let r = &s.roots;
    let Ok(rd) = std::fs::read_dir(&r.downloads) else { return };
    let cutoff = ctx.now - INSTALLER_AGE_MS;
    for e in rd.flatten() {
        if ctx.aborted() {
            break;
        }
        let Ok(ft) = e.file_type() else { continue };
        let name = e.file_name().to_string_lossy().into_owned();
        if !ft.is_file() || ft.is_symlink() || !installer_name(&name, r.plat) {
            continue;
        }
        let p = j(r, &[&r.downloads, &name]);
        if is_forbidden(&p, r, &[]) || !ctx.keep_member(&p) {
            continue;
        }
        let Some(st) = lstat(&p) else { continue };
        if !st.is_file() || mtime_ms(&st) > cutoff {
            continue;
        }
        let mut it = Item::new(&p, st.len(), "system", "installers").kind("installer");
        it.file = true;
        it.last_touched = Some(mtime_ms(&st));
        ctx.emit(it);
    }
}

pub fn bin_place(r: &SysRoots) -> String {
    if let Some(b) = &r.bin {
        return b.clone();
    }
    if r.plat == Plat::Win {
        let b = r.windir.as_bytes();
        let drive = if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' { r.windir[..2].to_owned() } else { "C:".into() };
        return format!("{drive}\\$Recycle.Bin");
    }
    j(r, &[&r.home, ".Trash"])
}

pub fn scan_bin(ctx: &Ctx) {
    let Some(s) = ctx.system else { return };
    let r = &s.roots;
    let kind = if r.plat == Plat::Mac { "trash" } else { "recycle" };
    let mut it = Item::new(&bin_place(r), 0, "system", "bin").kind(kind);
    it.action = Some("bin");
    match s.io.bin_size() {
        Some((bytes, count)) => {
            it.size = bytes;
            it.files = Some(count);
        }
        None => {
            it.locked = Some("denied");
            it.unknown = true;
        }
    }
    ctx.emit(it);
}

pub fn scan_update(ctx: &Ctx) {
    let Some(s) = ctx.system else { return };
    let r = &s.roots;
    admin_item(ctx, "update", "winUpdate", &j(r, &[&r.windir, "SoftwareDistribution", "Download"]));
    if !ctx.aborted() {
        admin_item(ctx, "update", "delivery", &j(r, &[&r.windir, "ServiceProfiles", "NetworkService", "AppData", "Local", "Microsoft", "Windows", "DeliveryOptimization", "Cache"]));
    }
}

const APP_CACHE_SKIP: [&str; 9] = ["google", "microsoft edge", "bravesoftware", "firefox", "yarn", "pip", "cocoapods", "org.swift.swiftpm", "com.apple.dt.xcode"];

pub fn running_apps(s: &SysDeps) -> HashSet<String> {
    let mut out: HashSet<String> = s.io.apps().into_iter().map(|a| a.to_lowercase()).collect();
    out.extend(s.io.procs().iter().map(|p| proc_name(p)));
    out
}

pub fn app_cache_lock(name: &str, running: &HashSet<String>) -> bool {
    let n = name.to_lowercase();
    if running.contains(&n) {
        return true;
    }
    let last = n.rsplit('.').next().unwrap_or(&n).to_owned();
    n.contains('.') && running.contains(&last)
}

pub fn scan_appcache(ctx: &Ctx) {
    let Some(s) = ctx.system else { return };
    let r = &s.roots;
    let base = j(r, &[&r.home, "Library", "Caches"]);
    let running = running_apps(s);
    let Ok(rd) = std::fs::read_dir(&base) else { return };
    for e in rd.flatten() {
        if ctx.aborted() {
            break;
        }
        let Ok(ft) = e.file_type() else { continue };
        if !ft.is_dir() || ft.is_symlink() {
            continue;
        }
        let name = e.file_name().to_string_lossy().into_owned();
        let low = name.to_lowercase();
        if low.starts_with("com.apple.") || APP_CACHE_SKIP.contains(&low.as_str()) {
            continue;
        }
        let p = j(r, &[&base, &name]);
        if is_forbidden(&p, r, &[]) {
            continue;
        }
        let mut it = Item::new(&p, ctx.size(&p), "system", "appcache").kind("app");
        it.detail = Some(name.clone());
        if app_cache_lock(&name, &running) {
            it.locked = Some("app");
        }
        ctx.emit(it);
    }
}

pub fn busy_win(file: &str) -> bool {
    match std::fs::OpenOptions::new().read(true).write(true).open(file) {
        Ok(_) => false,
        Err(e) => matches!(e.raw_os_error(), Some(32 | 33 | 5)) || e.kind() == std::io::ErrorKind::PermissionDenied,
    }
}

enum BinKind {
    Win,
    Mac(String),
    Dir(String),
}

pub struct RealSysIo {
    plat: Plat,
    procs_file: Option<String>,
    bin: BinKind,
}

fn dir_bin_size(dir: &str) -> Option<(u64, u64)> {
    let names: Vec<String> = std::fs::read_dir(dir).ok()?.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    let (bytes, _, _) = tree_info(dir, &AtomicBool::new(false), f64::INFINITY)?;
    Some((bytes, names.len() as u64))
}

#[cfg(windows)]
fn win_bin_size() -> Option<(u64, u64)> {
    use windows::Win32::UI::Shell::{SHQueryRecycleBinW, SHQUERYRBINFO};
    let mut q = SHQUERYRBINFO { cbSize: std::mem::size_of::<SHQUERYRBINFO>() as u32, i64Size: 0, i64NumItems: 0 };
    unsafe { SHQueryRecycleBinW(windows::core::PCWSTR::null(), &mut q) }.ok()?;
    Some((q.i64Size.max(0) as u64, q.i64NumItems.max(0) as u64))
}

#[cfg(not(windows))]
fn win_bin_size() -> Option<(u64, u64)> {
    None
}

#[cfg(windows)]
fn win_bin_empty() -> Result<(), String> {
    use windows::Win32::UI::Shell::SHEmptyRecycleBinW;
    match unsafe { SHEmptyRecycleBinW(None, windows::core::PCWSTR::null(), 7) } {
        Ok(()) => Ok(()),
        Err(e) if e.code().0 == -2147418113 => Ok(()),
        Err(_) => Err("EBIN".into()),
    }
}

#[cfg(not(windows))]
fn win_bin_empty() -> Result<(), String> {
    Err("EBIN".into())
}

impl SysIo for RealSysIo {
    fn procs(&self) -> Vec<String> {
        if let Some(f) = &self.procs_file {
            return std::fs::read_to_string(f).unwrap_or_default().lines().map(str::trim).filter(|s| !s.is_empty()).map(str::to_owned).collect();
        }
        core_procs::table().map(|rows| rows.into_iter().map(|r| proc_name(&r.name)).collect()).unwrap_or_default()
    }

    fn apps(&self) -> Vec<String> {
        if self.procs_file.is_some() || self.plat != Plat::Mac {
            return Vec::new();
        }
        let out = run("lsappinfo", &["list".to_owned()], Duration::from_secs(30)).stdout;
        parse_lsappinfo(&out)
    }

    fn bin_size(&self) -> Option<(u64, u64)> {
        match &self.bin {
            BinKind::Win => win_bin_size(),
            BinKind::Mac(trash) => {
                let names: Vec<String> = std::fs::read_dir(trash).ok()?.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
                let (bytes, _, _) = tree_info(trash, &AtomicBool::new(false), f64::INFINITY)?;
                Some((bytes, names.iter().filter(|n| *n != ".DS_Store").count() as u64))
            }
            BinKind::Dir(d) => dir_bin_size(d),
        }
    }

    fn bin_empty(&self) -> Result<(), String> {
        match &self.bin {
            BinKind::Win => win_bin_empty(),
            BinKind::Mac(_) => {
                let r = run("osascript", &["-e".to_owned(), "tell application \"Finder\" to empty trash".to_owned()], Duration::from_secs(300));
                if r.ok {
                    Ok(())
                } else {
                    Err("EBIN".into())
                }
            }
            BinKind::Dir(d) => {
                for e in std::fs::read_dir(d).map_err(|_| "EBIN".to_owned())?.flatten() {
                    let p = e.path();
                    let r = match e.file_type() {
                        Ok(ft) if ft.is_dir() && !ft.is_symlink() => std::fs::remove_dir_all(&p),
                        _ => std::fs::remove_file(&p).or_else(|_| std::fs::remove_dir(&p)),
                    };
                    r.map_err(|_| "EBIN".to_owned())?;
                }
                Ok(())
            }
        }
    }

    fn busy(&self, file: &str) -> bool {
        self.plat == Plat::Win && busy_win(file)
    }
}

pub fn parse_lsappinfo(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("bundleID=\"") {
        let after = &rest[i + 10..];
        let Some(end) = after.find('"') else { break };
        out.push(after[..end].to_owned());
        rest = &after[end..];
    }
    for line in text.lines() {
        let t = line.trim_start();
        let digits = t.chars().take_while(char::is_ascii_digit).count();
        if digits == 0 {
            continue;
        }
        let Some(r) = t[digits..].strip_prefix(')') else { continue };
        let r = r.trim_start();
        let Some(r) = r.strip_prefix('"') else { continue };
        if let Some(end) = r.find('"') {
            out.push(r[..end].to_owned());
        }
    }
    out
}

pub fn system_deps(plat: Plat, home: &str, env: &dyn Fn(&str) -> Option<String>, downloads: Option<String>, fake: Option<String>) -> SysDeps {
    let tmp = std::env::temp_dir().to_string_lossy().into_owned();
    let roots = sys_roots(plat, home, env, downloads, &tmp, fake.as_deref());
    let procs_file = fake.as_ref().map(|f| guard::join(f, "procs.txt", plat));
    let bin = match (&roots.bin, plat) {
        (Some(b), _) => BinKind::Dir(b.clone()),
        (None, Plat::Win) => BinKind::Win,
        (None, _) => BinKind::Mac(guard::join(home, ".Trash", plat)),
    };
    SysDeps { roots, io: Box::new(RealSysIo { plat, procs_file, bin }) }
}
