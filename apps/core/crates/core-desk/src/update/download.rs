use super::github::is_allowed_download;
use super::net;
use super::pick::FeedAsset;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;
use url::Url;

const PROGRESS_MS: u128 = 250;
const SPEED_WINDOW_MS: u128 = 3_000;
const MAX_HOPS: usize = 6;

pub struct Progress {
    pub received: f64,
    pub total: f64,
    pub bps: f64,
}

pub fn local_name(name: &str) -> Option<String> {
    let base = name.rsplit(if cfg!(windows) { &['/', '\\'][..] } else { &['/'][..] }).next().unwrap_or("");
    let ok = !base.is_empty() && base.chars().count() <= 120 && base.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '-')) && !base.starts_with('.');
    ok.then(|| base.to_string())
}

fn fetch(url: &str, allow_local: bool, cancel: &AtomicBool) -> Result<ureq::http::Response<ureq::Body>, &'static str> {
    let agent = net::agent(None, 0);
    let mut current = url.to_string();
    for _ in 0..MAX_HOPS {
        if cancel.load(Ordering::SeqCst) {
            return Err("cancelled");
        }
        if !is_allowed_download(&current, allow_local) {
            return Err("download");
        }
        let res = match agent.get(&current).call() {
            Ok(r) => r,
            Err(_) => return Err(if cancel.load(Ordering::SeqCst) { "cancelled" } else { "offline" }),
        };
        let status = res.status().as_u16();
        if (300..400).contains(&status) {
            let Some(loc) = res.headers().get("location").and_then(|v| v.to_str().ok()) else { return Err("download") };
            let Ok(base) = Url::parse(&current) else { return Err("download") };
            let Ok(next) = base.join(loc) else { return Err("download") };
            current = next.to_string();
            continue;
        }
        if !(200..300).contains(&status) {
            return Err("download");
        }
        return Ok(res);
    }
    Err("download")
}

pub fn download_asset(asset: &FeedAsset, dir: &Path, allow_local: bool, cancel: &AtomicBool, on_progress: &mut dyn FnMut(Progress)) -> Result<PathBuf, &'static str> {
    let Some(name) = local_name(&asset.name) else { return Err("no-asset") };
    let Some(expected) = asset.sha256.as_deref() else { return Err("no-digest") };
    if !is_allowed_download(&asset.url, allow_local) {
        return Err("download");
    }
    let fin = dir.join(&name);
    let part = dir.join(format!("{name}.part"));
    if std::fs::create_dir_all(dir).is_err() || (part.exists() && std::fs::remove_file(&part).is_err()) {
        return Err("disk");
    }
    let mut res = fetch(&asset.url, allow_local, cancel)?;
    let total = asset.size;
    let mut out = std::fs::File::create(&part).map_err(|_| "disk")?;
    let mut hash = Sha256::new();
    let mut reader = res.body_mut().with_config().limit(u64::MAX).reader();
    let start = Instant::now();
    let mut samples: Vec<(u128, f64)> = vec![(0, 0.0)];
    let mut received = 0.0f64;
    let mut last_emit = 0u128;
    let mut buf = vec![0u8; 64 * 1024];
    let mut fail: Option<&'static str> = None;
    loop {
        if cancel.load(Ordering::SeqCst) {
            fail = Some("cancelled");
            break;
        }
        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(_) => {
                fail = Some(if cancel.load(Ordering::SeqCst) { "cancelled" } else { "download" });
                break;
            }
        };
        received += n as f64;
        if received > total {
            fail = Some("download");
            break;
        }
        hash.update(&buf[..n]);
        if out.write_all(&buf[..n]).is_err() {
            fail = Some("disk");
            break;
        }
        let t = start.elapsed().as_millis();
        if t - last_emit >= PROGRESS_MS {
            last_emit = t;
            samples.push((t, received));
            while samples.len() > 2 && t - samples[0].0 > SPEED_WINDOW_MS {
                samples.remove(0);
            }
            let (t0, r0) = samples[0];
            let bps = if t > t0 { ((received - r0) * 1000.0 / (t - t0) as f64).round() } else { 0.0 };
            on_progress(Progress { received, total, bps });
        }
    }
    if let Some(e) = fail {
        drop(out);
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
    if out.flush().is_err() || out.sync_all().is_err() {
        let _ = std::fs::remove_file(&part);
        return Err("disk");
    }
    drop(out);
    on_progress(Progress { received, total, bps: 0.0 });
    let digest: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
    if received != total || digest != expected {
        let _ = std::fs::remove_file(&part);
        return Err(if received != total { "download" } else { "digest" });
    }
    let _ = std::fs::remove_file(&fin);
    if std::fs::rename(&part, &fin).is_err() {
        let _ = std::fs::remove_file(&part);
        return Err("disk");
    }
    Ok(fin)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_stay_inside_the_folder() {
        assert_eq!(local_name("Orrery-Setup-2.5.0.exe").as_deref(), Some("Orrery-Setup-2.5.0.exe"));
        assert_eq!(local_name("../x.exe").as_deref(), Some("x.exe"));
        assert_eq!(local_name(".hidden"), None);
        assert_eq!(local_name("a b.exe"), None);
        assert_eq!(local_name(&"a".repeat(121)), None);
        assert_eq!(local_name("dir/"), None);
    }
}
