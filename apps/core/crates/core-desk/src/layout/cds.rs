use super::LAYOUTLIB_VERSION;
use regex::Regex;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;

const MIN_CLASSES: usize = 1000;
const MIN_ARCHIVE_BYTES: u64 = 1_000_000;
const LOG_TO_STDERR: [&str; 2] = ["-Xlog:disable", "-Xlog:all=warning:stderr"];

pub struct CdsInput<'a> {
    pub jdk_home: &'a str,
    pub cache_dir: &'a str,
    pub daemon_jar: &'a str,
    pub layoutlib_dir: &'a str,
    pub win: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CdsPlan {
    Use { jvm_args: Vec<String> },
    Record(Record),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    pub jvm_args: Vec<String>,
    pub class_list: String,
    pub snapshot: String,
    pub archive: String,
    pub tmp: String,
    pub fail_marker: String,
    pub dump_args: Vec<String>,
}

impl CdsPlan {
    pub fn jvm_args(&self) -> &[String] {
        match self {
            CdsPlan::Use { jvm_args } => jvm_args,
            CdsPlan::Record(r) => &r.jvm_args,
        }
    }
}

pub fn parse_jdk_release(text: &str) -> Option<String> {
    let re = Regex::new(r#"(?m)^JAVA_VERSION="([^"]+)""#).expect("regex");
    re.captures(text).map(|m| m[1].trim().to_string()).filter(|s| !s.is_empty())
}

pub fn cds_key(jdk_version: &str, size: u64, mtime_ms: f64, daemon_jar: &str, layoutlib_dir: &str) -> String {
    let safe: String = {
        let mut out = String::new();
        let mut gap = false;
        for c in jdk_version.chars() {
            if c.is_ascii_alphanumeric() || c == '.' {
                out.push(c);
                gap = false;
            } else if !gap {
                out.push('_');
                gap = true;
            }
        }
        out.chars().take(40).collect()
    };
    let rounded = (mtime_ms + 0.5).floor() as i64;
    let digest = Sha256::digest(format!("{size}:{rounded}:{daemon_jar}:{layoutlib_dir}").as_bytes());
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    format!("daemon-{safe}-{LAYOUTLIB_VERSION}-{}", &hex[..12])
}

fn own(name: &str) -> bool {
    let Some(rest) = name.strip_prefix("daemon-") else { return false };
    [".jsa", ".classlist", ".tmp", ".nocds"].iter().any(|e| rest.len() > e.len() && rest.ends_with(e))
}

fn rm(file: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(file, fs::Permissions::from_mode(0o666));
    }
    #[cfg(not(unix))]
    if let Ok(m) = fs::metadata(file) {
        let mut p = m.permissions();
        p.set_readonly(false);
        let _ = fs::set_permissions(file, p);
    }
    let _ = fs::remove_file(file);
}

pub fn plan_cds(opts: &CdsInput) -> Option<CdsPlan> {
    let release = fs::read_to_string(Path::new(opts.jdk_home).join("release")).ok()?;
    let version = parse_jdk_release(&release)?;
    let jar = fs::metadata(opts.daemon_jar).ok()?;
    let key = cds_key(&version, jar.len(), crate::util::mtime_ms(&jar), opts.daemon_jar, opts.layoutlib_dir);
    let file = |ext: &str| Path::new(opts.cache_dir).join(format!("{key}.{ext}")).to_string_lossy().to_string();
    fs::create_dir_all(opts.cache_dir).ok()?;
    let names: Vec<String> = fs::read_dir(opts.cache_dir).ok()?.filter_map(|e| e.ok()?.file_name().into_string().ok()).collect();
    for f in &names {
        if own(f) && !f.starts_with(&format!("{key}.")) {
            rm(&Path::new(opts.cache_dir).join(f));
        }
    }
    if names.contains(&format!("{key}.nocds")) {
        return None;
    }
    let archive = names.contains(&format!("{key}.jsa")).then(|| fs::metadata(file("jsa")).ok()).flatten();
    if let Some(a) = archive {
        if a.len() < MIN_ARCHIVE_BYTES {
            rm(Path::new(&file("jsa")));
        } else {
            let mut jvm_args = vec![format!("-XX:SharedArchiveFile={}", file("jsa")), "-Xshare:auto".to_string()];
            jvm_args.extend(LOG_TO_STDERR.iter().map(|s| s.to_string()));
            return Some(CdsPlan::Use { jvm_args });
        }
    }
    let sep = if opts.win { ";" } else { ":" };
    let mut jvm_args = vec![format!("-XX:DumpLoadedClassList={}", file("classlist"))];
    jvm_args.extend(LOG_TO_STDERR.iter().map(|s| s.to_string()));
    let mut dump_args = vec!["-Xshare:dump".to_string(), format!("-XX:SharedClassListFile={}", file("snap.classlist")), format!("-XX:SharedArchiveFile={}", file("tmp"))];
    dump_args.extend(LOG_TO_STDERR.iter().map(|s| s.to_string()));
    let jar_path = super::download::join_native(opts.win, opts.layoutlib_dir, &super::download::jar_name());
    dump_args.extend(["-cp".to_string(), format!("{}{sep}{jar_path}", opts.daemon_jar)]);
    Some(CdsPlan::Record(Record { jvm_args, class_list: file("classlist"), snapshot: file("snap.classlist"), archive: file("jsa"), tmp: file("tmp"), fail_marker: file("nocds"), dump_args }))
}

pub fn snapshot_class_list(plan: &Record) -> bool {
    let Ok(text) = fs::read_to_string(&plan.class_list) else { return false };
    let Some(end) = text.rfind('\n') else { return false };
    if text[..end].split('\n').count() < MIN_CLASSES {
        return false;
    }
    fs::write(&plan.snapshot, &text[..=end]).is_ok()
}

pub fn finish_dump(plan: &Record, ok: bool) -> bool {
    if ok && fs::metadata(&plan.tmp).map(|m| m.len() > 0).unwrap_or(false) && fs::rename(&plan.tmp, &plan.archive).is_ok() {
        rm(Path::new(&plan.snapshot));
        return true;
    }
    rm(Path::new(&plan.tmp));
    rm(Path::new(&plan.snapshot));
    let _ = fs::write(&plan.fail_marker, "");
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("ccm-cds-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn plans_record_then_use() {
        let d = tmp("plan");
        let jdk = d.join("jdk");
        fs::create_dir_all(&jdk).unwrap();
        fs::write(jdk.join("release"), "JAVA_VERSION=\"21.0.10\"\n").unwrap();
        let jar = d.join("daemon.jar");
        fs::write(&jar, "x").unwrap();
        let cache = d.join("cache");
        let s = |p: &Path| p.to_string_lossy().to_string();
        let (jdk_s, cache_s, jar_s) = (s(&jdk), s(&cache), s(&jar));
        let input = CdsInput { jdk_home: &jdk_s, cache_dir: &cache_s, daemon_jar: &jar_s, layoutlib_dir: "/ll", win: false };
        let Some(CdsPlan::Record(rec)) = plan_cds(&input) else { panic!("record expected") };
        assert!(rec.jvm_args[0].starts_with("-XX:DumpLoadedClassList="));
        assert_eq!(rec.dump_args[0], "-Xshare:dump");
        assert!(rec.dump_args.last().unwrap().contains("layoutlib-16.2.3.jar"));
        assert!(!snapshot_class_list(&rec));
        fs::write(&rec.class_list, "a\n".repeat(1200) + "partial").unwrap();
        assert!(snapshot_class_list(&rec));
        assert_eq!(fs::read_to_string(&rec.snapshot).unwrap().lines().count(), 1200);
        fs::write(&rec.tmp, vec![1u8; 10]).unwrap();
        assert!(finish_dump(&rec, true));
        assert!(!finish_dump(&rec, false));
        assert!(Path::new(&rec.fail_marker).exists());
        assert!(plan_cds(&input).is_none());
        fs::remove_file(&rec.fail_marker).unwrap();
        fs::write(&rec.archive, vec![0u8; 1_100_000]).unwrap();
        let Some(CdsPlan::Use { jvm_args }) = plan_cds(&input) else { panic!("use expected") };
        assert_eq!(jvm_args[1], "-Xshare:auto");
        fs::write(d.join("cache").join("daemon-old.jsa"), "x").unwrap();
        plan_cds(&input);
        assert!(!d.join("cache").join("daemon-old.jsa").exists());
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn key_is_stable() {
        assert!(cds_key("21.0.10+7", 5, 1000.4, "/a.jar", "/ll").starts_with("daemon-21.0.10_7-16.2.3-"));
        assert_eq!(parse_jdk_release("X=1\nJAVA_VERSION=\"17.0.9\"\n"), Some("17.0.9".into()));
    }
}
