use super::zip::{extract_zip, ZipError};
use super::LAYOUTLIB_VERSION;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

pub const READY_MARKER: &str = ".ccm-ready";
pub const KIT_JAR: &str = "layout-daemon.jar";
pub const KIT_DIR: &str = "loose-kit";
const MAVEN: &str = "https://dl.google.com/android/maven2/com/android/tools/layoutlib";
const MAX_UNZIPPED: u64 = 600 * 1024 * 1024;
const MAX_REDIRECTS: usize = 5;
const PARTIAL_STALE_MS: f64 = 10.0 * 60_000.0;
const KIT_HOSTS: [&str; 4] = ["github.com", "objects.githubusercontent.com", "release-assets.githubusercontent.com", "github-releases.githubusercontent.com"];

pub fn jar_name() -> String {
    format!("layoutlib-{LAYOUTLIB_VERSION}.jar")
}

pub fn join_native(win: bool, a: &str, b: &str) -> String {
    crate::run::android::pathx::join(win, &[a, b])
}

#[derive(Debug, Clone, Copy)]
pub struct Artifact {
    pub artifact: &'static str,
    pub classifier: Option<&'static str>,
    pub size: u64,
    pub sha256: &'static str,
}

pub const JAR: Artifact = Artifact { artifact: "layoutlib", classifier: None, size: 60_286_750, sha256: "5dbdbd3e35abb8f8bab4c3019869642f515afd31c43d230ece0be082486c3ec8" };
pub const WIN: Artifact = Artifact { artifact: "layoutlib-runtime", classifier: Some("win"), size: 76_544_623, sha256: "adadac21f5a9bfde4336d834ce0f9db83a478c65f27afd48751c470b3d708c6f" };
pub const MAC: Artifact = Artifact { artifact: "layoutlib-runtime", classifier: Some("mac"), size: 80_275_609, sha256: "2b4462089c3976735cd79570e3ce35b4fe24d1c70112a8c75f422dac477ef732" };
pub const MAC_ARM: Artifact = Artifact { artifact: "layoutlib-runtime", classifier: Some("mac-arm"), size: 79_334_417, sha256: "2ca430114a7ec0c718b998ef12af915a724f7a2530dc3c382c616a2ca9076e38" };
pub const RESOURCES: Artifact = Artifact { artifact: "layoutlib-resources", classifier: None, size: 34_854_638, sha256: "b4c4582e3680cbc8e0248223dc6ccf393bdedecd38cfccd03a33014b9238c559" };

pub fn runtime_for(platform: &str, arch: &str) -> Option<Artifact> {
    match platform {
        "win32" => Some(WIN),
        "darwin" => Some(if arch == "arm64" { MAC_ARM } else { MAC }),
        _ => None,
    }
}

pub fn artifact_url(a: &Artifact) -> String {
    let file = format!("{}-{LAYOUTLIB_VERSION}{}.jar", a.artifact, a.classifier.map(|c| format!("-{c}")).unwrap_or_default());
    format!("{MAVEN}/{}/{LAYOUTLIB_VERSION}/{file}", a.artifact)
}

fn parsed(url: &str) -> Option<url::Url> {
    let u = url::Url::parse(url).ok()?;
    (u.scheme() == "https" && u.username().is_empty() && u.password().is_none() && u.port().is_none_or(|p| p == 443)).then_some(u)
}

pub fn is_allowed_url(url: &str) -> bool {
    parsed(url).is_some_and(|u| u.host_str() == Some("dl.google.com"))
}

pub fn is_allowed_kit_url(url: &str) -> bool {
    parsed(url).is_some_and(|u| u.host_str().is_some_and(|h| KIT_HOSTS.contains(&h)))
}

#[derive(Debug, Clone)]
pub struct KitSpec {
    pub version: String,
    pub size: u64,
    pub sha256: String,
    pub url: String,
}

impl KitSpec {
    pub fn from_value(v: &Value) -> Option<KitSpec> {
        Some(KitSpec { version: v.get("version")?.as_str()?.to_string(), size: v.get("size")?.as_u64()?, sha256: v.get("sha256")?.as_str()?.to_string(), url: v.get("url")?.as_str()?.to_string() })
    }
}

pub fn kit_dir_for(base: &str, version: &str) -> String {
    Path::new(base).join("daemon").join(version).to_string_lossy().to_string()
}

pub fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| s.split(['.', '-']).map(|x| x.parse::<i64>().unwrap_or(0)).collect::<Vec<_>>();
    let (pa, pb) = (parse(a), parse(b));
    for i in 0..pa.len().max(pb.len()) {
        let (x, y) = (pa.get(i).copied().unwrap_or(0), pb.get(i).copied().unwrap_or(0));
        if x != y {
            return x.cmp(&y);
        }
    }
    std::cmp::Ordering::Equal
}

pub fn kits_to_prune(names: &[String], current: &str) -> Vec<String> {
    let mut others: Vec<&String> = names.iter().filter(|n| n.as_str() != current && !n.contains(".partial-")).collect();
    others.sort_by(|x, y| compare_versions(y, x));
    others.into_iter().skip(1).filter(|n| compare_versions(n, current).is_lt()).cloned().collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct DlError(pub String);

impl DlError {
    fn new(code: &str) -> Self {
        DlError(code.to_string())
    }
}

pub struct HttpBody {
    pub status: u16,
    pub length: Option<u64>,
    pub location: Option<String>,
    pub body: Box<dyn Read + Send>,
}

pub trait Fetcher: Send + Sync {
    fn get(&self, url: &str) -> Result<HttpBody, DlError>;
}

pub struct UreqFetcher {
    agent: ureq::Agent,
}

impl Default for UreqFetcher {
    fn default() -> Self {
        let tls = ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build();
        let config = ureq::Agent::config_builder().timeout_connect(Some(Duration::from_secs(30))).timeout_recv_response(Some(Duration::from_secs(30))).max_redirects(0).max_redirects_will_error(false).http_status_as_error(false).tls_config(tls).build();
        UreqFetcher { agent: config.into() }
    }
}

impl Fetcher for UreqFetcher {
    fn get(&self, url: &str) -> Result<HttpBody, DlError> {
        let res = self.agent.get(url).call().map_err(|_| DlError::new("network"))?;
        let status = res.status().as_u16();
        let header = |n: &str| res.headers().get(n).and_then(|v| v.to_str().ok()).map(String::from);
        let length = header("content-length").and_then(|v| v.parse().ok());
        let location = header("location");
        let (_, body) = res.into_parts();
        Ok(HttpBody { status, length, location, body: Box::new(body.into_with_config().limit(u64::MAX).reader()) })
    }
}

struct Follow<'a> {
    base: &'a dyn Fetcher,
    allow: &'a (dyn Fn(&str) -> bool + Sync),
}

impl Fetcher for Follow<'_> {
    fn get(&self, url: &str) -> Result<HttpBody, DlError> {
        let mut cur = url.to_string();
        for _ in 0..=MAX_REDIRECTS {
            if !(self.allow)(&cur) {
                return Err(DlError::new("host-not-allowed"));
            }
            let res = self.base.get(&cur)?;
            if (300..400).contains(&res.status) {
                if let Some(loc) = &res.location {
                    cur = url::Url::parse(&cur).and_then(|b| b.join(loc)).map_err(|_| DlError::new("network"))?.to_string();
                    continue;
                }
            }
            return Ok(res);
        }
        Err(DlError::new("too-many-redirects"))
    }
}

pub fn download_pinned(size: u64, sha256: &str, file: &Path, fetcher: &dyn Fetcher, on_bytes: &mut dyn FnMut(u64), url: &str, allow: &dyn Fn(&str) -> bool) -> Result<(), DlError> {
    if !allow(url) {
        return Err(DlError::new("host-not-allowed"));
    }
    let res = fetcher.get(url)?;
    if res.status != 200 {
        return Err(DlError(format!("http-{}", res.status)));
    }
    if res.length.is_some_and(|l| l != size) {
        return Err(DlError::new("size-mismatch"));
    }
    let result = stream_to(res.body, size, sha256, file, on_bytes);
    if result.is_err() {
        let _ = fs::remove_file(file);
    }
    result
}

fn stream_to(mut body: Box<dyn Read + Send>, size: u64, sha256: &str, file: &Path, on_bytes: &mut dyn FnMut(u64)) -> Result<(), DlError> {
    let mut out = File::create(file).map_err(|_| DlError::new("network"))?;
    let mut hash = Sha256::new();
    let mut got = 0u64;
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = body.read(&mut buf).map_err(|_| DlError::new("network"))?;
        if n == 0 {
            break;
        }
        got += n as u64;
        if got > size {
            return Err(DlError::new("size-mismatch"));
        }
        hash.update(&buf[..n]);
        out.write_all(&buf[..n]).map_err(|_| DlError::new("network"))?;
        on_bytes(n as u64);
    }
    drop(out);
    if got != size {
        return Err(DlError::new("size-mismatch"));
    }
    let hex: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if hex != sha256 {
        return Err(DlError::new("sha256-mismatch"));
    }
    Ok(())
}

pub fn rename_with_retry(from: &Path, to: &Path) -> std::io::Result<()> {
    let mut i = 0;
    loop {
        match fs::rename(from, to) {
            Ok(()) => return Ok(()),
            Err(e) if i < 4 && e.kind() == std::io::ErrorKind::PermissionDenied => {
                std::thread::sleep(Duration::from_millis(100 * (1u64 << i)));
                i += 1;
            }
            Err(e) => return Err(e),
        }
    }
}

fn rand_hex() -> String {
    let mut b = [0u8; 4];
    let _ = getrandom::fill(&mut b);
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub struct BundleCfg {
    pub base: String,
    pub platform: String,
    pub arch: String,
    pub override_dir: Option<String>,
    pub kit: Option<KitSpec>,
    pub fetcher: Arc<dyn Fetcher>,
    pub on_status: Arc<dyn Fn(Value) + Send + Sync>,
}

#[derive(Default)]
struct DlState {
    downloading: bool,
    pct: i64,
    last_error: Option<String>,
}

pub struct Bundle {
    cfg: BundleCfg,
    state: Mutex<DlState>,
    done: Condvar,
}

impl Bundle {
    pub fn new(cfg: BundleCfg) -> Self {
        Bundle { cfg, state: Mutex::new(DlState { pct: 0, ..Default::default() }), done: Condvar::new() }
    }

    fn version_dir(&self) -> PathBuf {
        Path::new(&self.cfg.base).join(LAYOUTLIB_VERSION)
    }

    pub fn dir(&self) -> String {
        if let Some(o) = self.cfg.override_dir.as_deref().filter(|o| Path::new(o).join(jar_name()).exists()) {
            return o.to_string();
        }
        self.version_dir().to_string_lossy().to_string()
    }

    fn kit_dir(&self) -> Option<PathBuf> {
        self.cfg.kit.as_ref().map(|k| PathBuf::from(kit_dir_for(&self.cfg.base, &k.version)))
    }

    pub fn hosts(&self) -> String {
        let mut parts = Vec::new();
        if !self.layoutlib_ready() {
            parts.push("dl.google.com");
        }
        if !self.kit_ready() {
            parts.push("github.com (GitHub release assets)");
        }
        if parts.is_empty() {
            "dl.google.com".into()
        } else {
            parts.join(", ")
        }
    }

    pub fn download_bytes(&self) -> u64 {
        let runtime = runtime_for(&self.cfg.platform, &self.cfg.arch);
        let lib = if self.layoutlib_ready() { 0 } else { JAR.size + RESOURCES.size + runtime.map(|r| r.size).unwrap_or(0) };
        lib + if self.kit_ready() { 0 } else { self.cfg.kit.as_ref().map(|k| k.size).unwrap_or(0) }
    }

    fn layoutlib_ready(&self) -> bool {
        let dir = PathBuf::from(self.dir());
        if dir != self.version_dir() {
            return true;
        }
        dir.join(READY_MARKER).exists() && dir.join(jar_name()).exists()
    }

    fn kit_ready(&self) -> bool {
        match self.kit_dir() {
            None => true,
            Some(d) => d.join(READY_MARKER).exists() && d.join(KIT_JAR).exists() && d.join(KIT_DIR).join("kit.json").exists(),
        }
    }

    pub fn kit_pending(&self) -> bool {
        !self.kit_ready()
    }

    pub fn is_ready(&self) -> bool {
        self.layoutlib_ready() && self.kit_ready()
    }

    pub fn status(&self) -> Value {
        let st = self.state.lock().unwrap();
        if st.downloading {
            return json!({"state": "downloading", "pct": st.pct});
        }
        if self.is_ready() {
            return json!({"state": "ready", "dir": self.dir()});
        }
        match &st.last_error {
            Some(code) => json!({"state": "error", "code": code}),
            None => json!({"state": "missing"}),
        }
    }

    pub fn download(&self) -> Value {
        if self.is_ready() {
            return self.status();
        }
        {
            let mut st = self.state.lock().unwrap();
            if st.downloading {
                while st.downloading {
                    st = self.done.wait(st).unwrap();
                }
                drop(st);
                return self.status();
            }
            st.downloading = true;
        }
        self.run();
        self.state.lock().unwrap().downloading = false;
        self.done.notify_all();
        (self.cfg.on_status)(self.status());
        self.status()
    }

    fn emit(&self, pct: f64) {
        let next = pct.min(100.0).floor() as i64;
        {
            let mut st = self.state.lock().unwrap();
            if st.pct == next {
                return;
            }
            st.pct = next;
        }
        (self.cfg.on_status)(json!({"state": "downloading", "pct": next}));
    }

    fn fail(&self, code: &str) {
        self.state.lock().unwrap().last_error = Some(code.to_string());
    }

    fn run(&self) {
        let need_lib = !self.layoutlib_ready();
        let need_kit = !self.kit_ready();
        let runtime = runtime_for(&self.cfg.platform, &self.cfg.arch);
        if need_lib && runtime.is_none() {
            return self.fail("unsupported-os");
        }
        {
            let mut st = self.state.lock().unwrap();
            st.last_error = None;
            st.pct = 0;
        }
        (self.cfg.on_status)(json!({"state": "downloading", "pct": 0}));
        let total = self.download_bytes().max(1) as f64;
        let mut done = 0u64;
        let mut on_bytes = |n: u64| {
            done += n;
            self.emit(done as f64 / total * 85.0);
        };
        let result: Result<(), String> = (|| {
            fs::create_dir_all(&self.cfg.base).map_err(|_| "io".to_string())?;
            if need_kit {
                self.fetch_kit(&mut on_bytes).map_err(|e| format!("daemon-{e}"))?;
            }
            if let (true, Some(rt)) = (need_lib, runtime) {
                self.fetch_layoutlib(&rt, &mut on_bytes)?;
            }
            Ok(())
        })();
        match result {
            Ok(()) => self.emit(100.0),
            Err(code) => self.fail(&code),
        }
    }

    fn fetch_kit(&self, on_bytes: &mut dyn FnMut(u64)) -> Result<(), String> {
        let (Some(kit), Some(dest)) = (&self.cfg.kit, self.kit_dir()) else { return Ok(()) };
        let kit_base = dest.parent().map(Path::to_path_buf).unwrap_or_default();
        fs::create_dir_all(&kit_base).map_err(|_| "io".to_string())?;
        let now = crate::util::now_ms();
        for e in fs::read_dir(&kit_base).into_iter().flatten().flatten() {
            let n = e.file_name().to_string_lossy().to_string();
            if !n.contains(".partial-") {
                continue;
            }
            let age = e.metadata().ok().map(|m| now - crate::util::mtime_ms(&m)).unwrap_or(f64::MAX);
            if n.starts_with(&format!("{}.partial-", kit.version)) || age > PARTIAL_STALE_MS {
                let _ = fs::remove_dir_all(e.path());
            }
        }
        let tmp = kit_base.join(format!("{}.partial-{}", kit.version, rand_hex()));
        let res = (|| -> Result<(), String> {
            fs::create_dir_all(&tmp).map_err(|_| "io".to_string())?;
            let zip_file = tmp.join("kit.download.zip");
            let follow = Follow { base: self.cfg.fetcher.as_ref(), allow: &is_allowed_kit_url };
            download_pinned(kit.size, &kit.sha256, &zip_file, &follow, on_bytes, &kit.url, &is_allowed_kit_url).map_err(|e| e.0)?;
            let content = tmp.join("content");
            extract_zip(&zip_file, &content, MAX_UNZIPPED).map_err(|e: ZipError| e.code().to_string())?;
            if !content.join(KIT_JAR).exists() || !content.join(KIT_DIR).join("kit.json").exists() {
                return Err("kit-incomplete".into());
            }
            fs::write(content.join(READY_MARKER), format!("{}\n", kit.version)).map_err(|_| "io".to_string())?;
            let _ = fs::remove_dir_all(&dest);
            rename_with_retry(&content, &dest).map_err(|_| "io".to_string())
        })();
        let _ = fs::remove_dir_all(&tmp);
        res?;
        let names: Vec<String> = fs::read_dir(&kit_base).into_iter().flatten().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
        for n in kits_to_prune(&names, &kit.version) {
            let _ = fs::remove_dir_all(kit_base.join(n));
        }
        Ok(())
    }

    fn fetch_layoutlib(&self, runtime: &Artifact, on_bytes: &mut dyn FnMut(u64)) -> Result<(), String> {
        let base = Path::new(&self.cfg.base);
        for e in fs::read_dir(base).into_iter().flatten().flatten() {
            if e.file_name().to_string_lossy().starts_with(&format!("{LAYOUTLIB_VERSION}.partial-")) {
                let _ = fs::remove_dir_all(e.path());
            }
        }
        let tmp = base.join(format!("{LAYOUTLIB_VERSION}.partial-{}", rand_hex()));
        let res = (|| -> Result<(), String> {
            fs::create_dir_all(&tmp).map_err(|_| "io".to_string())?;
            let (runtime_jar, resources_jar) = (tmp.join("runtime.download.jar"), tmp.join("resources.download.jar"));
            let fetch = |a: &Artifact, file: &Path, on: &mut dyn FnMut(u64)| download_pinned(a.size, a.sha256, file, self.cfg.fetcher.as_ref(), on, &artifact_url(a), &is_allowed_url).map_err(|e| e.0);
            fetch(&JAR, &tmp.join(jar_name()), on_bytes)?;
            fetch(runtime, &runtime_jar, on_bytes)?;
            fetch(&RESOURCES, &resources_jar, on_bytes)?;
            let zerr = |e: ZipError| e.code().to_string();
            extract_zip(&runtime_jar, &tmp.join("runtime"), MAX_UNZIPPED).map_err(zerr)?;
            self.emit(92.0);
            extract_zip(&resources_jar, &tmp.join("resources"), MAX_UNZIPPED).map_err(zerr)?;
            let _ = fs::remove_file(&runtime_jar);
            let _ = fs::remove_file(&resources_jar);
            fs::write(tmp.join(READY_MARKER), format!("{LAYOUTLIB_VERSION}\n")).map_err(|_| "io".to_string())?;
            let dest = self.version_dir();
            let _ = fs::remove_dir_all(&dest);
            rename_with_retry(&tmp, &dest).map_err(|_| "io".to_string())
        })();
        if res.is_err() {
            let _ = fs::remove_dir_all(&tmp);
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_rules() {
        assert!(is_allowed_url(&artifact_url(&JAR)));
        assert!(!is_allowed_url("http://dl.google.com/x"));
        assert!(!is_allowed_url("https://dl.google.com.evil.com/x"));
        assert!(!is_allowed_url("https://u:p@dl.google.com/x"));
        assert!(!is_allowed_url("https://dl.google.com:8443/x"));
        assert!(is_allowed_kit_url("https://github.com/a/b/releases/download/v1/x.zip"));
        assert!(!is_allowed_kit_url("https://evil.com/x.zip"));
        assert_eq!(runtime_for("darwin", "arm64").unwrap().classifier, Some("mac-arm"));
        assert!(runtime_for("linux", "x64").is_none());
    }

    #[test]
    fn prunes_old_kits() {
        let names: Vec<String> = ["1.0.0", "1.1.0", "1.2.0", "1.3.0", "1.3.0.partial-ab"].iter().map(|s| s.to_string()).collect();
        assert_eq!(kits_to_prune(&names, "1.3.0"), vec!["1.1.0".to_string(), "1.0.0".to_string()]);
        assert_eq!(compare_versions("1.10.0", "1.9.0"), std::cmp::Ordering::Greater);
    }
}
