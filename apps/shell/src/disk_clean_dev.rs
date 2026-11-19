use std::collections::{HashMap, HashSet, VecDeque};
use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::disk_clean::{is_link, lstat, mtime_ms, Ctx, Item};
use crate::disk_guard::{self as guard, GuardContext, Plat};
use crate::disk_move::{is_tool, parse_proc_lines, ProcInfo};

pub const WALK_DEPTH: usize = 6;
const MAX_DIRS: usize = 20_000;
const READ_MAX: u64 = 512 * 1024;
pub const ARCHIVE_KEEP_MS: f64 = 90.0 * 86_400_000.0;
const EXEC_MAX: usize = 4 * 1024 * 1024;
pub const DOCKER_PROBE: Duration = Duration::from_secs(3);
pub const DOCKER_DF: Duration = Duration::from_secs(20);

pub const MARKERS: [&str; 12] = ["package.json", "build.gradle", "build.gradle.kts", "settings.gradle", "settings.gradle.kts", "gradlew", "pubspec.yaml", "cargo.toml", "pom.xml", "pyproject.toml", "setup.py", "requirements.txt"];
const JUNK_NAMES: [&str; 13] = ["node_modules", ".pnpm-store", "build", ".gradle", ".dart_tool", "dist", "out", "target", ".next", ".nuxt", ".turbo", ".parcel-cache", ".venv"];
const LOCKFILES: [&str; 6] = ["pnpm-lock.yaml", "yarn.lock", "package-lock.json", "bun.lockb", "bun.lock", "npm-shrinkwrap.json"];
const SKIP: [&str; 11] = [".git", ".hg", ".svn", ".idea", ".vscode", ".cxx", ".externalNativeBuild", "__pycache__", ".pytest_cache", "Pods", ".symlinks"];
const GRADLE: [&str; 5] = ["build.gradle", "build.gradle.kts", "settings.gradle", "settings.gradle.kts", "gradlew"];
const TRACK_CHECK: [&str; 4] = ["build", "dist", "out", "target"];
const BUILDER_FILES: [&str; 12] = [
    "electron-builder.yml",
    "electron-builder.yaml",
    "electron-builder.json",
    "electron-builder.json5",
    "electron-builder.toml",
    "electron-builder.js",
    "electron-builder.cjs",
    "electron-builder.mjs",
    "electron-builder.ts",
    "electron-builder.config.js",
    "electron-builder.config.cjs",
    "electron-builder.config.ts",
];

#[derive(Clone, Debug, PartialEq)]
pub struct JunkHit {
    pub path: String,
    pub name: String,
    pub kind: &'static str,
    pub owner: String,
    pub root: String,
}

fn has(s: &HashSet<String>, names: &[&str]) -> bool {
    names.iter().any(|n| s.contains(*n))
}

pub fn build_dir_kind(name: &str, siblings: &HashSet<String>, git: bool) -> Option<&'static str> {
    match name {
        "node_modules" => Some("node"),
        ".dart_tool" => Some("flutter"),
        ".next" | ".nuxt" | ".turbo" | ".parcel-cache" => Some("web"),
        "__pycache__" | ".pytest_cache" | ".venv" => Some("python"),
        ".gradle" => (git || has(siblings, &GRADLE)).then_some("gradle"),
        "build" => {
            if has(siblings, &GRADLE) {
                Some("gradle")
            } else if siblings.contains("pubspec.yaml") {
                Some("flutter")
            } else if siblings.contains("package.json") {
                Some("web")
            } else {
                git.then_some("other")
            }
        }
        "dist" | "out" => {
            if siblings.contains("package.json") {
                Some("web")
            } else {
                git.then_some("other")
            }
        }
        "target" => {
            if siblings.contains("cargo.toml") {
                Some("rust")
            } else if siblings.contains("pom.xml") {
                Some("maven")
            } else {
                git.then_some("other")
            }
        }
        _ => None,
    }
}

pub fn junk_kind(name: &str, siblings: &HashSet<String>) -> Option<&'static str> {
    if !has(siblings, &MARKERS) {
        return None;
    }
    match name {
        "node_modules" => siblings.contains("package.json").then_some("node"),
        ".pnpm-store" => has(siblings, &["package.json", "pnpm-lock.yaml"]).then_some("pnpm"),
        ".dart_tool" => siblings.contains("pubspec.yaml").then_some("flutter"),
        ".next" | ".nuxt" | ".turbo" | ".parcel-cache" => siblings.contains("package.json").then_some("web"),
        ".venv" => has(siblings, &["pyproject.toml", "setup.py", "requirements.txt"]).then_some("python"),
        "build" | ".gradle" | "dist" | "out" | "target" => build_dir_kind(name, siblings, false).filter(|k| matches!(*k, "gradle" | "flutter" | "rust" | "maven" | "web")),
        _ => None,
    }
}

pub fn join(base: &str, name: &str) -> String {
    guard::join(base, name, Plat::host())
}

pub fn builder_resources(owner: &str, siblings: &HashSet<String>) -> Option<String> {
    let pkg: Option<Value> = if siblings.contains("package.json") { std::fs::read_to_string(join(owner, "package.json")).ok().and_then(|t| serde_json::from_str(&t).ok()) } else { None };
    let in_deps = pkg.as_ref().is_some_and(|p| p["dependencies"].get("electron-builder").is_some() || p["devDependencies"].get("electron-builder").is_some());
    let build = pkg.as_ref().map(|p| &p["build"]).filter(|b| b.is_object());
    let uses = build.is_some() || in_deps || BUILDER_FILES.iter().any(|f| siblings.contains(*f));
    if !uses {
        return None;
    }
    let res = match build.and_then(|b| b["directories"]["buildResources"].as_str()) {
        Some(s) => {
            let t = s.strip_prefix("./").or_else(|| s.strip_prefix(".\\")).unwrap_or(s);
            t.trim_end_matches(['\\', '/']).to_owned()
        }
        None => "build".into(),
    };
    Some(if res.is_empty() { "build".into() } else { res })
}

pub fn repo_of(owner: &str, root: &str) -> Option<(String, String, bool)> {
    let plat = Plat::host();
    let mut dir = guard::normalize(owner, plat);
    let top = guard::normalize(root, plat);
    for _ in 0..12 {
        let git = join(&dir, ".git");
        if let Ok(m) = std::fs::metadata(&git) {
            return Some((dir, git, m.is_dir()));
        }
        if guard::path_key(&dir, plat) == guard::path_key(&top, plat) {
            break;
        }
        let up = guard::parent(&dir, plat);
        if up == dir {
            break;
        }
        dir = up;
    }
    None
}

pub fn junk_allowed(owner: &str, name: &str, siblings: &HashSet<String>, tracked: &dyn Fn(&str, &str) -> Option<bool>) -> bool {
    if !TRACK_CHECK.contains(&name) {
        return true;
    }
    if builder_resources(owner, siblings).is_some_and(|r| r.eq_ignore_ascii_case(name)) {
        return false;
    }
    let plat = Plat::host();
    if repo_of(owner, &guard::root_of(&guard::normalize(owner, plat), plat)).is_none() {
        return true;
    }
    tracked(owner, name) == Some(false)
}

pub fn find_junk(root: &str, abort: &AtomicBool, depth_max: usize, tracked: &dyn Fn(&str, &str) -> Option<bool>) -> Option<Vec<JunkHit>> {
    let mut out = Vec::new();
    let mut queue: VecDeque<(String, usize)> = VecDeque::from([(root.to_owned(), 0)]);
    let mut seen = 0;
    while let Some((dir, depth)) = queue.pop_front() {
        if seen >= MAX_DIRS {
            break;
        }
        if abort.load(Ordering::SeqCst) {
            return None;
        }
        seen += 1;
        let entries: Vec<(String, Option<std::fs::FileType>)> = match std::fs::read_dir(&dir) {
            Ok(rd) => rd.flatten().map(|e| (e.file_name().to_string_lossy().into_owned(), e.file_type().ok())).collect(),
            Err(_) => Vec::new(),
        };
        let siblings: HashSet<String> = entries.iter().map(|(n, _)| n.to_lowercase()).collect();
        for (name, ft) in &entries {
            let Some(ft) = ft else { continue };
            if ft.is_symlink() || !ft.is_dir() {
                continue;
            }
            let child = join(&dir, name);
            if JUNK_NAMES.contains(&name.as_str()) {
                if let Some(kind) = junk_kind(name, &siblings) {
                    if junk_allowed(&dir, name, &siblings, tracked) {
                        out.push(JunkHit { path: child, name: name.clone(), kind, owner: dir.clone(), root: root.to_owned() });
                    }
                }
                continue;
            }
            if SKIP.contains(&name.as_str()) || depth + 1 >= depth_max {
                continue;
            }
            queue.push_back((child, depth + 1));
        }
    }
    Some(out)
}

pub struct RunOut {
    pub ok: bool,
    pub code: Option<String>,
    pub stdout: String,
}

pub fn run(bin: &str, args: &[String], timeout: Duration) -> RunOut {
    let mut cmd = Command::new(bin);
    cmd.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(e) => return RunOut { ok: false, code: Some(if e.kind() == std::io::ErrorKind::NotFound { "ENOENT".into() } else { "EXIT".into() }), stdout: String::new() },
    };
    let mut pipe = child.stdout.take();
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(p) = pipe.as_mut() {
            let _ = p.take(EXEC_MAX as u64 + 1).read_to_end(&mut buf);
        }
        buf
    });
    let end = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if Instant::now() < end => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
        }
    };
    let buf = reader.join().unwrap_or_default();
    let over = buf.len() > EXEC_MAX;
    let stdout = String::from_utf8_lossy(&buf[..buf.len().min(EXEC_MAX)]).into_owned();
    match status {
        None => RunOut { ok: false, code: Some("ETIMEDOUT".into()), stdout },
        Some(_) if over => RunOut { ok: false, code: Some("MAXBUFFER".into()), stdout },
        Some(s) if s.success() => RunOut { ok: true, code: None, stdout },
        Some(_) => RunOut { ok: false, code: Some("EXIT".into()), stdout },
    }
}

pub fn git_tracked(dir: &str, name: &str) -> Option<bool> {
    let args: Vec<String> = ["-C", dir, "-c", "core.quotepath=false", "ls-files", "--", name].iter().map(|s| s.to_string()).collect();
    let r = run("git", &args, Duration::from_secs(15));
    match r.code.as_deref() {
        None => Some(!r.stdout.trim().is_empty()),
        Some("MAXBUFFER") => Some(true),
        _ => None,
    }
}

fn mtime_of(p: &str) -> f64 {
    std::fs::metadata(p).map(|m| mtime_ms(&m)).unwrap_or(0.0)
}

fn names_of(dir: &str) -> Vec<String> {
    std::fs::read_dir(dir).map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default()
}

pub fn project_of(owner: &str, root: &str) -> String {
    repo_of(owner, root).map_or_else(|| owner.to_owned(), |r| r.0)
}

pub fn project_touched(owner: &str, root: &str) -> Option<f64> {
    let mut best = 0f64;
    for n in names_of(owner) {
        let low = n.to_lowercase();
        if MARKERS.contains(&low.as_str()) || LOCKFILES.contains(&low.as_str()) {
            best = best.max(mtime_of(&join(owner, &n)));
        }
    }
    match repo_of(owner, root) {
        Some((_, git, true)) => best = best.max(mtime_of(&join(&git, "HEAD"))).max(mtime_of(&join(&git, "index"))),
        Some((_, git, false)) => best = best.max(mtime_of(&git)),
        None => {}
    }
    (best > 0.0).then_some(best)
}

pub fn node_manager(owner: &str, root: &str) -> &'static str {
    let plat = Plat::host();
    let mut dir = owner.to_owned();
    for _ in 0..12 {
        let names: HashSet<String> = names_of(&dir).into_iter().map(|n| n.to_lowercase()).collect();
        if names.contains("pnpm-lock.yaml") {
            return "pnpm";
        }
        if names.contains("yarn.lock") {
            return "yarn";
        }
        if names.contains("bun.lockb") || names.contains("bun.lock") {
            return "bun";
        }
        if names.contains("package-lock.json") || names.contains("npm-shrinkwrap.json") {
            return "npm";
        }
        if guard::path_key(&dir, plat) == guard::path_key(root, plat) {
            break;
        }
        let up = guard::parent(&dir, plat);
        if up == dir {
            break;
        }
        dir = up;
    }
    "npm"
}

pub fn restore_of(hit: &JunkHit) -> String {
    match hit.kind {
        "node" => match node_manager(&hit.owner, &hit.root) {
            "yarn" => "yarn".into(),
            m => format!("{m} install"),
        },
        "pnpm" => "pnpm install".into(),
        "web" => match node_manager(&hit.owner, &hit.root) {
            "npm" => "npm run build".into(),
            m => format!("{m} run build"),
        },
        "gradle" => "./gradlew build".into(),
        "flutter" => "flutter pub get".into(),
        "rust" => "cargo build".into(),
        "maven" => "mvn package".into(),
        _ => {
            let names: HashSet<String> = names_of(&hit.owner).into_iter().map(|n| n.to_lowercase()).collect();
            if names.contains("requirements.txt") {
                "python -m venv .venv && pip install -r requirements.txt".into()
            } else if names.contains("pyproject.toml") || names.contains("setup.py") {
                "python -m venv .venv && pip install -e .".into()
            } else {
                "python -m venv .venv".into()
            }
        }
    }
}

fn from_hits(ctx: &Ctx, category: &str, hits: Vec<JunkHit>) {
    for h in hits {
        if ctx.aborted() {
            break;
        }
        let mut item = Item::new(&h.path, ctx.size(&h.path), "dev", category).kind(h.kind);
        item.project = Some(project_of(&h.owner, &h.root));
        item.restore = Some(restore_of(&h));
        item.last_touched = project_touched(&h.owner, &h.root);
        ctx.emit(item);
    }
}

pub fn scan_node_modules(ctx: &Ctx) {
    let hits: Vec<JunkHit> = ctx.junk().iter().filter(|h| matches!(h.kind, "node" | "pnpm")).cloned().collect();
    from_hits(ctx, "node_modules", hits);
}

pub fn scan_build(ctx: &Ctx) {
    let hits: Vec<JunkHit> = ctx.junk().iter().filter(|h| !matches!(h.kind, "node" | "pnpm")).cloned().collect();
    from_hits(ctx, "build", hits);
}

pub struct Sub {
    pub name: String,
    pub path: String,
    pub mtime: f64,
}

pub fn subdirs(dir: &str) -> Vec<Sub> {
    let Ok(rd) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut out = Vec::new();
    for e in rd.flatten() {
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_symlink() || !ft.is_dir() {
            continue;
        }
        let name = e.file_name().to_string_lossy().into_owned();
        let path = join(dir, &name);
        let mtime = lstat(&path).map_or(0.0, |m| mtime_ms(&m));
        out.push(Sub { name, path, mtime });
    }
    out
}

pub fn scan_xcode(ctx: &Ctx) {
    let base = join(&join(&join(&ctx.home, "Library"), "Developer"), "Xcode");
    for d in subdirs(&join(&base, "DerivedData")) {
        if ctx.aborted() {
            return;
        }
        let mut it = Item::new(&d.path, ctx.size(&d.path), "dev", "xcode").kind("derived");
        it.last_touched = Some(d.mtime);
        it.restore = Some("xcodebuild".into());
        ctx.emit(it);
    }
    for day in subdirs(&join(&base, "Archives")) {
        for a in subdirs(&day.path) {
            if ctx.aborted() {
                return;
            }
            if !a.path.ends_with(".xcarchive") {
                continue;
            }
            let mut it = Item::new(&a.path, ctx.size(&a.path), "dev", "xcode").kind("archive");
            it.last_touched = Some(a.mtime);
            it.hold = ctx.now - a.mtime < ARCHIVE_KEEP_MS;
            ctx.emit(it);
        }
    }
}

pub fn cache_spots(home: &str, plat: Plat, env: &dyn Fn(&str) -> Option<String>) -> Vec<(&'static str, String)> {
    let j = |parts: &[&str]| parts.iter().skip(1).fold(parts[0].to_owned(), |acc, p| guard::join(&acc, p, plat));
    let get = |k: &str| env(k).filter(|v| !v.is_empty());
    let win = plat == Plat::Win;
    let mac = plat == Plat::Mac;
    let local = get("LOCALAPPDATA").unwrap_or_else(|| j(&[home, "AppData", "Local"]));
    let lc = j(&[home, "Library", "Caches"]);
    let xdg = get("XDG_CACHE_HOME").unwrap_or_else(|| j(&[home, ".cache"]));
    let mut out = Vec::new();
    let npm = get("npm_config_cache").unwrap_or_else(|| if win { j(&[&local, "npm-cache"]) } else { j(&[home, ".npm"]) });
    out.push(("npm", j(&[&npm, "_cacache"])));
    out.push(("yarn", if win { j(&[&local, "Yarn", "Cache"]) } else if mac { j(&[&lc, "Yarn"]) } else { j(&[&xdg, "yarn"]) }));
    out.push(("yarn", j(&[home, ".yarn", "berry", "cache"])));
    out.push(("pnpm", if win { j(&[&local, "pnpm", "store"]) } else if mac { j(&[home, "Library", "pnpm", "store"]) } else { j(&[home, ".local", "share", "pnpm", "store"]) }));
    let pub_cache = get("PUB_CACHE").unwrap_or_else(|| if win { j(&[&local, "Pub", "Cache"]) } else { j(&[home, ".pub-cache"]) });
    out.push(("pub", j(&[&pub_cache, "hosted"])));
    let gradle = get("GRADLE_USER_HOME").unwrap_or_else(|| j(&[home, ".gradle"]));
    out.push(("gradle", j(&[&gradle, "caches"])));
    let cargo = get("CARGO_HOME").unwrap_or_else(|| j(&[home, ".cargo"]));
    out.push(("cargo", j(&[&cargo, "registry", "cache"])));
    out.push(("pip", if win { j(&[&local, "pip", "Cache"]) } else if mac { j(&[&lc, "pip"]) } else { j(&[&xdg, "pip"]) }));
    if mac {
        out.push(("cocoapods", j(&[&lc, "CocoaPods"])));
    }
    out
}

pub fn real_dir(p: &str) -> Option<String> {
    let real = std::fs::canonicalize(p).ok()?;
    let s = guard::plain_canonical(&real.to_string_lossy(), Plat::host())?;
    let m = lstat(&s)?;
    (m.is_dir() && !is_link(&m)).then_some(s)
}

pub fn scan_cache(ctx: &Ctx) {
    let mut seen = HashSet::new();
    for (kind, spot) in cache_spots(&ctx.home, ctx.plat, ctx.env) {
        if ctx.aborted() {
            break;
        }
        let Some(real) = real_dir(&spot) else { continue };
        let g = GuardContext { home: ctx.home.clone(), keep: ctx.keep.clone(), roots: vec![real.clone()], volumes: Vec::new() };
        if guard::check_deletable(&real, &g, ctx.plat).is_some() || ctx.keep.iter().any(|k| !k.is_empty() && guard::is_under(&real, k, ctx.plat)) {
            continue;
        }
        let key = if ctx.plat == Plat::Linux { real.clone() } else { real.to_lowercase() };
        if !seen.insert(key) {
            continue;
        }
        ctx.emit(Item::new(&real, ctx.size(&real), "dev", "cache").kind(kind));
    }
}

pub struct SdkDeps {
    pub sdk_root: Option<String>,
    pub avd_home: String,
    pub gradle_home: String,
    pub xcode_base: String,
    pub procs: Box<dyn Fn() -> Vec<ProcInfo> + Send + Sync>,
    pub simctl: Option<Box<dyn Fn(&[String]) -> Option<String> + Send + Sync>>,
}

pub fn tool_procs() -> Vec<ProcInfo> {
    let Some(rows) = core_procs::table() else { return Vec::new() };
    let tools: Vec<(u32, String)> = rows.into_iter().filter(|r| is_tool(&r.name)).map(|r| (r.pid, r.name.to_lowercase())).collect();
    if tools.is_empty() {
        return Vec::new();
    }
    let pids: Vec<u32> = tools.iter().map(|t| t.0).collect();
    let cmds: HashMap<u32, String> = core_procs::cmdline(&pids).into_iter().map(|c| (c.pid, c.cmd)).collect();
    tools.into_iter().map(|(pid, name)| ProcInfo { name, cmd: cmds.get(&pid).cloned().unwrap_or_default() }).collect()
}

pub fn sdk_deps(plat: Plat, home: &str, env: &dyn Fn(&str) -> Option<String>, fake: Option<String>) -> SdkDeps {
    let j = |base: &str, p: &str| guard::join(base, p, plat);
    if let Some(f) = fake {
        let procs_file = j(&f, "procs.txt");
        return SdkDeps {
            sdk_root: Some(j(&f, "sdk")),
            avd_home: j(&f, "avd"),
            gradle_home: j(&f, "gradle"),
            xcode_base: j(&f, "Xcode"),
            procs: Box::new(move || parse_proc_lines(&std::fs::read_to_string(&procs_file).unwrap_or_default())),
            simctl: None,
        };
    }
    let (sdk, avd) = crate::disk_move::sdk_spots(plat, home, env);
    let gradle = env("GRADLE_USER_HOME").filter(|v| !v.is_empty()).unwrap_or_else(|| j(home, ".gradle"));
    let xcode = j(&j(&j(home, "Library"), "Developer"), "Xcode");
    let simctl: Option<Box<dyn Fn(&[String]) -> Option<String> + Send + Sync>> =
        if plat == Plat::Mac { Some(Box::new(|args: &[String]| Some(run("xcrun", args, Duration::from_secs(15)).stdout).filter(|s| !s.is_empty()))) } else { None };
    SdkDeps { sdk_root: Some(sdk), avd_home: avd, gradle_home: gradle, xcode_base: xcode, procs: Box::new(tool_procs), simctl }
}

#[derive(Default, Debug)]
pub struct AndroidUse {
    pub compile: HashSet<u32>,
    pub tools: HashSet<String>,
    pub gradle: HashSet<String>,
    pub android: bool,
    pub compile_unsure: bool,
    pub tools_unsure: bool,
    pub gradle_unsure: bool,
}

fn read_small(p: &str) -> Option<String> {
    let m = lstat(p)?;
    if !m.is_file() || m.len() > READ_MAX {
        return None;
    }
    std::fs::read_to_string(p).ok()
}

fn skip_ws(s: &[char], mut i: usize) -> usize {
    while i < s.len() && s[i].is_whitespace() {
        i += 1;
    }
    i
}

fn after_key(chars: &[char], at: usize, key_len: usize) -> Option<usize> {
    let start = at + key_len;
    let i = skip_ws(chars, start);
    match chars.get(i) {
        Some('=') | Some('(') => Some(skip_ws(chars, i + 1)),
        _ if i > start => Some(i),
        _ => None,
    }
}

fn find_all(chars: &[char], key: &str) -> Vec<usize> {
    let k: Vec<char> = key.chars().collect();
    (0..chars.len().saturating_sub(k.len() - 1)).filter(|&i| chars[i..].starts_with(&k)).collect()
}

fn android_block(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    find_all(&chars, "android").into_iter().any(|i| chars.get(skip_ws(&chars, i + 7)) == Some(&'{'))
}

pub fn read_build_file(text: &str, u: &mut AndroidUse) {
    let chars: Vec<char> = text.chars().collect();
    let is_app = text.contains("com.android.application") || text.contains("com.android.library") || android_block(text);
    let mut saw_compile = false;
    for at in find_all(&chars, "compileSdk") {
        let len = if chars[at + 10..].starts_with(&['V', 'e', 'r', 's', 'i', 'o', 'n']) { 17 } else { 10 };
        let Some(i) = after_key(&chars, at, len) else { continue };
        let raw: String = chars[i..].iter().take_while(|c| !c.is_whitespace() && **c != ')').collect();
        if raw.is_empty() {
            continue;
        }
        saw_compile = true;
        let clean: String = raw.chars().filter(|c| !matches!(c, '"' | '\'' | ')')).collect();
        let clean = clean.strip_prefix("android-").unwrap_or(&clean).to_owned();
        match clean.parse::<u32>() {
            Ok(n) if clean.chars().all(|c| c.is_ascii_digit()) => {
                u.compile.insert(n);
            }
            _ => u.compile_unsure = true,
        }
    }
    if find_all(&chars, "compileSdk").into_iter().any(|at| chars.get(skip_ws(&chars, at + 10)) == Some(&'{')) {
        u.compile_unsure = true;
    }
    let mut saw_tools = false;
    for at in find_all(&chars, "buildToolsVersion") {
        let Some(i) = after_key(&chars, at, 17) else { continue };
        if !chars.get(i).is_some_and(|c| matches!(c, '"' | '\'')) {
            continue;
        }
        let body: String = chars[i + 1..].iter().take_while(|c| !matches!(c, '"' | '\'')).collect();
        let digits = body.chars().take_while(|c| c.is_ascii_digit() || *c == '.').count();
        let ok = digits > 0 && body[digits..].chars().all(|c| c == '-' || c.is_alphanumeric() || c == '_') && chars.get(i + 1 + body.chars().count()).is_some();
        if ok {
            saw_tools = true;
            u.tools.insert(body);
        }
    }
    if is_app {
        u.android = true;
        let block = android_block(text);
        if !saw_compile && block {
            u.compile_unsure = true;
        }
        if !saw_tools && block {
            u.tools_unsure = true;
        }
    }
}

pub fn read_wrapper(text: &str) -> Option<String> {
    let at = text.find("distributionUrl")?;
    let rest = text[at + 15..].trim_start();
    let rest = rest.strip_prefix('=')?.trim_start();
    let token: String = rest.chars().take_while(|c| !c.is_whitespace()).collect();
    let seg = token.rsplit('/').next()?;
    let stem = seg.strip_suffix(".zip")?;
    let ok = stem.starts_with("gradle-") && (stem.ends_with("-bin") || stem.ends_with("-all")) && stem.len() > 11 && stem.chars().all(|c| c.is_alphanumeric() || matches!(c, '_' | '.' | '-'));
    ok.then(|| stem.to_owned())
}

const BUILD_FILES: [&[&str]; 8] = [
    &["build.gradle"],
    &["build.gradle.kts"],
    &["app", "build.gradle"],
    &["app", "build.gradle.kts"],
    &["android", "build.gradle"],
    &["android", "build.gradle.kts"],
    &["android", "app", "build.gradle"],
    &["android", "app", "build.gradle.kts"],
];
const WRAPPER_FILES: [&[&str]; 2] = [&["gradle", "wrapper", "gradle-wrapper.properties"], &["android", "gradle", "wrapper", "gradle-wrapper.properties"]];

fn join_all(root: &str, parts: &[&str]) -> String {
    parts.iter().fold(root.to_owned(), |acc, p| join(&acc, p))
}

pub fn android_use(roots: &[String]) -> AndroidUse {
    let mut u = AndroidUse::default();
    let mut wrappers = 0;
    for root in roots {
        for parts in BUILD_FILES {
            if let Some(t) = read_small(&join_all(root, parts)) {
                read_build_file(&t, &mut u);
            }
        }
        for parts in WRAPPER_FILES {
            let Some(t) = read_small(&join_all(root, parts)) else { continue };
            wrappers += 1;
            match read_wrapper(&t) {
                Some(d) => {
                    u.gradle.insert(d);
                }
                None => u.gradle_unsure = true,
            }
        }
    }
    if !u.android {
        u.compile_unsure = true;
        u.tools_unsure = true;
    }
    if wrappers == 0 {
        u.gradle_unsure = true;
    }
    u
}

pub fn version_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering as O;
    let pa: Vec<&str> = a.split(['.', '-']).collect();
    let pb: Vec<&str> = b.split(['.', '-']).collect();
    let num = |s: &str| (!s.is_empty() && s.chars().all(|c| c.is_ascii_digit())).then(|| s.parse::<f64>().unwrap_or(0.0));
    for i in 0..pa.len().max(pb.len()) {
        let x = pa.get(i).copied().unwrap_or("");
        let y = pb.get(i).copied().unwrap_or("");
        let (nx, ny) = (num(x), num(y));
        if let (Some(nx), Some(ny)) = (nx, ny) {
            if nx != ny {
                return nx.partial_cmp(&ny).unwrap_or(O::Equal);
            }
            continue;
        }
        if x == y {
            continue;
        }
        if x.is_empty() {
            return if ny.is_none() { O::Greater } else { O::Less };
        }
        if y.is_empty() {
            return if nx.is_none() { O::Less } else { O::Greater };
        }
        if nx.is_some() {
            return O::Greater;
        }
        if ny.is_some() {
            return O::Less;
        }
        return x.cmp(y);
    }
    O::Equal
}

fn push(ctx: &Ctx, mut item: Item) {
    if ctx.aborted() {
        return;
    }
    item.size = if item.file { lstat(&item.path).map_or(0, |m| m.len()) } else { ctx.size(&item.path) };
    item.group = "dev";
    ctx.emit(item);
}

fn image_key(s: &str) -> String {
    let n = s.replace('\\', "/");
    let n = match n.find("system-images/") {
        Some(i) => n[i + 14..].to_owned(),
        None => n,
    };
    n.trim_end_matches('/').to_lowercase()
}

pub fn avd_images(avd_home: &str) -> (HashSet<String>, bool) {
    let mut used = HashSet::new();
    let Ok(rd) = std::fs::read_dir(avd_home) else { return (used, false) };
    for e in rd.flatten() {
        let n = e.file_name().to_string_lossy().into_owned();
        if !n.ends_with(".avd") {
            continue;
        }
        let Some(text) = read_small(&join(&join(avd_home, &n), "config.ini")) else { continue };
        for line in text.lines() {
            let Some(rest) = line.strip_prefix("image.sysdir.") else { continue };
            let digits = rest.chars().take_while(char::is_ascii_digit).count();
            if digits == 0 {
                continue;
            }
            let Some(v) = rest[digits..].trim_start().strip_prefix('=') else { continue };
            let v = v.trim();
            if !v.is_empty() {
                used.insert(image_key(v));
            }
        }
    }
    (used, true)
}

fn dev_item(path: &str, category: &str, kind: &str, detail: &str, mtime: f64) -> Item {
    let mut it = Item::new(path, 0, "dev", category).kind(kind);
    it.detail = Some(detail.to_owned());
    it.last_touched = Some(mtime);
    it
}

pub fn scan_sdk(ctx: &Ctx) {
    let Some(sdk) = ctx.sdk else { return };
    let Some(root_raw) = &sdk.sdk_root else { return };
    let Some(root) = real_dir(root_raw) else {
        ctx.set_note("noSdk");
        return;
    };
    let u = android_use(&ctx.roots);
    let platforms: Vec<Sub> = subdirs(&join(&root, "platforms")).into_iter().filter(|d| d.name.starts_with("android-")).collect();
    let level = |d: &Sub| d.name[8..].parse::<u32>().ok().filter(|_| d.name[8..].chars().all(|c| c.is_ascii_digit()));
    let newest = platforms.iter().filter_map(level).max();
    for d in &platforms {
        let lv = level(d);
        if lv.is_some_and(|l| Some(l) == newest || u.compile.contains(&l)) {
            continue;
        }
        let mut it = dev_item(&d.path, "sdk", "platform", &d.name, d.mtime);
        it.restore = Some(format!("sdkmanager \"platforms;{}\"", d.name));
        it.hold = u.compile_unsure || lv.is_none();
        push(ctx, it);
    }
    let tools = subdirs(&join(&root, "build-tools"));
    let mut names: Vec<String> = tools.iter().map(|d| d.name.clone()).collect();
    names.sort_by(|a, b| version_cmp(a, b));
    let top = names.pop();
    for d in &tools {
        if Some(&d.name) == top.as_ref() || u.tools.contains(&d.name) {
            continue;
        }
        let mut it = dev_item(&d.path, "sdk", "buildTools", &d.name, d.mtime);
        it.restore = Some(format!("sdkmanager \"build-tools;{}\"", d.name));
        it.hold = u.tools_unsure;
        push(ctx, it);
    }
    let (used, readable) = avd_images(&sdk.avd_home);
    for api in subdirs(&join(&root, "system-images")) {
        for tag in subdirs(&api.path) {
            for abi in subdirs(&tag.path) {
                let key = format!("{}/{}/{}", api.name, tag.name, abi.name).to_lowercase();
                if used.contains(&key) {
                    continue;
                }
                let detail = format!("{};{};{}", api.name, tag.name, abi.name);
                let mut it = dev_item(&abi.path, "sdk", "sysImage", &detail, abi.mtime);
                it.restore = Some(format!("sdkmanager \"system-images;{detail}\""));
                it.hold = !readable;
                push(ctx, it);
            }
        }
    }
}

pub fn avd_running(name: &str, procs: &[ProcInfo]) -> bool {
    let n = name.to_lowercase();
    procs.iter().any(|p| {
        let base = p.name.strip_suffix(".exe").unwrap_or(&p.name);
        if !(base.starts_with("emulator") || base.starts_with("qemu-system")) {
            return false;
        }
        let c = format!(" {} ", p.cmd.to_lowercase());
        c.contains(&format!(" @{n} ")) || c.contains(&format!(" -avd {n} ")) || c.contains(&format!("\"@{n}\""))
    })
}

fn newest_inside(dir: &str) -> f64 {
    let Ok(rd) = std::fs::read_dir(dir) else { return 0.0 };
    rd.flatten().filter_map(|e| lstat(&join(dir, &e.file_name().to_string_lossy())).map(|m| mtime_ms(&m))).fold(0.0, f64::max)
}

fn has_lock(dir: &str) -> bool {
    names_of(dir).iter().any(|n| n.ends_with(".lock"))
}

pub fn avd_busy(item: &Item, sdk: &SdkDeps) -> bool {
    let plat = Plat::host();
    let base = guard::base_name(&item.path, plat);
    let ini = item.kind.as_deref() == Some("avdIni");
    let strip = |s: &str, suf: &str| if s.to_lowercase().ends_with(suf) { s[..s.len() - suf.len()].to_owned() } else { s.to_owned() };
    let name = if ini { strip(&base, ".ini") } else { strip(&base, ".avd") };
    let dir = if ini { join(&guard::parent(&item.path, plat), &format!("{name}.avd")) } else { item.path.clone() };
    has_lock(&dir) || avd_running(&name, &(sdk.procs)())
}

pub fn scan_avd(ctx: &Ctx) {
    let Some(sdk) = ctx.sdk else { return };
    let Some(home) = real_dir(&sdk.avd_home) else { return };
    let procs = (sdk.procs)();
    for d in subdirs(&home) {
        if !d.name.ends_with(".avd") {
            continue;
        }
        let name = d.name[..d.name.len() - 4].to_owned();
        let running = has_lock(&d.path) || avd_running(&name, &procs);
        let last = Some(newest_inside(&d.path)).filter(|v| *v > 0.0).unwrap_or(d.mtime);
        let mut it = dev_item(&d.path, "avd", "avd", &name, last);
        it.restore = Some("Android Studio > Device Manager".into());
        it.hold = true;
        if running {
            it.locked = Some("app");
        }
        push(ctx, it);
        let ini = join(&home, &format!("{name}.ini"));
        if lstat(&ini).is_some_and(|m| m.is_file()) {
            let mut it = dev_item(&ini, "avd", "avdIni", &format!("{name}.ini"), last);
            it.file = true;
            it.hold = true;
            if running {
                it.locked = Some("app");
            }
            push(ctx, it);
        }
    }
}

pub fn scan_gradle_dists(ctx: &Ctx) {
    let Some(sdk) = ctx.sdk else { return };
    let Some(home) = real_dir(&sdk.gradle_home) else { return };
    let dists: Vec<Sub> = subdirs(&join(&join(&home, "wrapper"), "dists")).into_iter().filter(|d| d.name.starts_with("gradle-") && (d.name.ends_with("-bin") || d.name.ends_with("-all")) && d.name.len() > 11).collect();
    if dists.is_empty() {
        return;
    }
    let u = android_use(&ctx.roots);
    let ver = |n: &str| n.strip_prefix("gradle-").unwrap_or(n).strip_suffix("-bin").or_else(|| n.strip_prefix("gradle-").unwrap_or(n).strip_suffix("-all")).unwrap_or(n).to_owned();
    let mut vs: Vec<String> = dists.iter().map(|d| ver(&d.name)).collect();
    vs.sort_by(|a, b| version_cmp(a, b));
    let top = vs.pop();
    for d in &dists {
        if Some(ver(&d.name)) == top || u.gradle.contains(&d.name) {
            continue;
        }
        let mut it = dev_item(&d.path, "gradle_dists", "gradleDist", &d.name, d.mtime);
        it.restore = Some("./gradlew --version".into());
        it.hold = u.gradle_unsure;
        push(ctx, it);
    }
}

pub fn parse_simctl_unavailable(json: Option<&str>) -> Vec<String> {
    let Some(v) = json.and_then(|t| serde_json::from_str::<Value>(t).ok()) else { return Vec::new() };
    let mut out = Vec::new();
    for list in v["devices"].as_object().into_iter().flat_map(|o| o.values()) {
        for d in list.as_array().into_iter().flatten() {
            if d["isAvailable"] == Value::Bool(false) {
                if let Some(p) = d["dataPath"].as_str() {
                    out.push(p.to_owned());
                }
            }
        }
    }
    out
}

fn ios_ver(n: &str) -> String {
    let chars: Vec<char> = n.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        if c.is_ascii_digit() {
            let mut end = i;
            while end < chars.len() && (chars[end].is_ascii_digit() || (chars[end] == '.' && chars.get(end + 1).is_some_and(char::is_ascii_digit))) {
                end += 1;
            }
            return chars[i..end].iter().collect();
        }
        if !(c.is_alphanumeric() || *c == '_' || *c == ' ') {
            break;
        }
    }
    n.to_owned()
}

pub fn scan_ios(ctx: &Ctx) {
    let Some(sdk) = ctx.sdk else { return };
    for fam in ["iOS DeviceSupport", "watchOS DeviceSupport", "tvOS DeviceSupport"] {
        let list = subdirs(&join(&sdk.xcode_base, fam));
        let mut names: Vec<String> = list.iter().map(|d| d.name.clone()).collect();
        names.sort_by(|a, b| version_cmp(&ios_ver(a), &ios_ver(b)));
        let top = names.pop();
        for d in &list {
            if Some(&d.name) == top.as_ref() {
                continue;
            }
            let mut it = dev_item(&d.path, "ios", "deviceSupport", &d.name, d.mtime);
            it.restore = Some("Xcode > Window > Devices and Simulators".into());
            push(ctx, it);
        }
    }
    let Some(sim) = &sdk.simctl else { return };
    if ctx.aborted() {
        return;
    }
    let args: Vec<String> = ["simctl", "list", "devices", "unavailable", "-j"].iter().map(|s| s.to_string()).collect();
    let dirs = parse_simctl_unavailable(sim(&args).as_deref());
    if dirs.is_empty() {
        return;
    }
    let size = dirs.iter().map(|d| ctx.size(d)).sum();
    let mut it = Item::new("simctl:unavailable", size, "dev", "ios").kind("simUnavailable");
    it.action = Some("command");
    it.files = Some(dirs.len() as u64);
    it.restore = Some("xcrun simctl delete unavailable".into());
    ctx.emit(it);
}

pub struct DockerDeps {
    pub run: Box<dyn Fn(&[String], Duration) -> RunOut + Send + Sync>,
}

pub fn docker_deps(fake: Option<String>) -> DockerDeps {
    match fake {
        Some(script) => DockerDeps {
            run: Box::new(move |args: &[String], t: Duration| {
                let mut all = vec![script.clone()];
                all.extend(args.iter().cloned());
                run("node", &all, t)
            }),
        },
        None => DockerDeps { run: Box::new(|args: &[String], t: Duration| run("docker", args, t)) },
    }
}

pub const DOCKER_KINDS: [&str; 4] = ["images", "containers", "volumes", "buildCache"];

pub fn docker_prune(kind: &str) -> Option<&'static [&'static str]> {
    match kind {
        "images" => Some(&["image", "prune", "-a", "-f"]),
        "containers" => Some(&["container", "prune", "-f"]),
        "volumes" => Some(&["volume", "prune", "-f"]),
        "buildCache" => Some(&["builder", "prune", "-f"]),
        _ => None,
    }
}

pub fn docker_bytes(v: &Value) -> u64 {
    let Some(s) = v.as_str() else { return 0 };
    let s = s.trim_start();
    let num: String = s.chars().take_while(|c| c.is_ascii_digit() || *c == '.').collect();
    let unit: String = s[num.len()..].trim_start().chars().take_while(char::is_ascii_alphabetic).collect();
    let Ok(n) = num.parse::<f64>() else { return 0 };
    let mul = match unit.to_lowercase().as_str() {
        "" | "b" => 1.0,
        "kb" => 1e3,
        "mb" => 1e6,
        "gb" => 1e9,
        "tb" => 1e12,
        "kib" => 1024.0,
        "mib" => 1024f64.powi(2),
        "gib" => 1024f64.powi(3),
        "tib" => 1024f64.powi(4),
        _ => 0.0,
    };
    (n * mul).round() as u64
}

#[derive(Debug, Clone, PartialEq)]
pub struct DockerRow {
    pub kind: &'static str,
    pub total: u64,
    pub active: u64,
    pub size: u64,
    pub reclaimable: u64,
}

fn count(v: &Value) -> u64 {
    match v {
        Value::Number(n) => n.as_f64().filter(|f| *f >= 0.0).map_or(0, |f| f as u64),
        Value::String(s) => s.trim().parse::<f64>().ok().filter(|f| *f >= 0.0).map_or(0, |f| f as u64),
        _ => 0,
    }
}

pub fn parse_docker_df(stdout: &str) -> Vec<DockerRow> {
    let mut out = Vec::new();
    for line in stdout.lines() {
        let s = line.trim();
        if !s.starts_with('{') {
            continue;
        }
        let Ok(o) = serde_json::from_str::<Value>(s) else { continue };
        let kind = match o["Type"].as_str().unwrap_or("").to_lowercase().as_str() {
            "images" => "images",
            "containers" => "containers",
            "local volumes" => "volumes",
            "build cache" => "buildCache",
            _ => continue,
        };
        out.push(DockerRow { kind, total: count(&o["TotalCount"]), active: count(&o["Active"]), size: docker_bytes(&o["Size"]), reclaimable: docker_bytes(&o["Reclaimable"]) });
    }
    out
}

fn strs(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

pub fn docker_probe(d: &DockerDeps) -> Option<&'static str> {
    let r = (d.run)(&strs(&["info", "--format", "{{.ServerVersion}}"]), DOCKER_PROBE);
    if r.ok && !r.stdout.trim().is_empty() {
        return None;
    }
    match r.code.as_deref() {
        Some("ENOENT") => Some("noCli"),
        Some("ETIMEDOUT") => Some("timeout"),
        _ => Some("noDaemon"),
    }
}

pub fn docker_rows(d: &DockerDeps) -> Option<Vec<DockerRow>> {
    let r = (d.run)(&strs(&["system", "df", "--format", "{{json .}}"]), DOCKER_DF);
    r.ok.then(|| parse_docker_df(&r.stdout))
}

pub fn scan_docker(ctx: &Ctx) {
    let Some(d) = ctx.docker else { return };
    if let Some(note) = docker_probe(d) {
        ctx.set_note(note);
        return;
    }
    let Some(rows) = docker_rows(d) else {
        ctx.set_note("noDaemon");
        return;
    };
    for kind in DOCKER_KINDS {
        let Some(row) = rows.iter().find(|r| r.kind == kind) else { continue };
        let prune = docker_prune(kind).unwrap_or(&[]);
        let mut it = Item::new(&format!("docker:{kind}"), row.reclaimable, "dev", "docker").kind(kind);
        it.action = Some("command");
        it.files = Some(row.total.saturating_sub(row.active));
        it.detail = Some(format!("{}/{}", row.total, row.active));
        it.restore = Some(format!("docker {}", prune.iter().filter(|a| **a != "-f").copied().collect::<Vec<_>>().join(" ")));
        if kind == "volumes" {
            it.warn = true;
            it.hold = true;
        }
        ctx.emit(it);
    }
}
