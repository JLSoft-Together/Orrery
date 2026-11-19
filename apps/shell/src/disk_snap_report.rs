use std::io::Write;
use std::path::PathBuf;

use serde_json::{json, Value};
use tauri::AppHandle;

use super::{build_compared, e2e_allowed, top_growth, Compared, Snapshot};
use crate::disk_guard::{path_key, Plat};

pub const FREE_DROP_BYTES: u64 = 5 * 1024 * 1024 * 1024;
pub const ALERT_GAP_MS: u64 = 6 * 3_600_000;
const TOP_N: usize = 5;
const DAY_MS: i64 = 86_400_000;

pub struct FreeDrop {
    pub label: String,
    pub free: u64,
    pub total: u64,
    pub drop: u64,
}

fn group(int: u64, en: bool) -> String {
    let digits = int.to_string();
    let sep = if en { ',' } else { '.' };
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(sep);
        }
        out.push(c);
    }
    out
}

pub fn fmt_bytes(n: u64, en: bool) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    let s = if i == 0 {
        v.round().to_string()
    } else if v >= 100.0 {
        group(v.round() as u64, en)
    } else {
        let tenths = (v * 10.0).round() as u64;
        let (int, frac) = (tenths / 10, tenths % 10);
        if frac == 0 {
            group(int, en)
        } else {
            format!("{}{}{}", group(int, en), if en { '.' } else { ',' }, frac)
        }
    };
    format!("{s} {}", UNITS[i])
}

fn cut(s: &str) -> &str {
    s.trim_end_matches(['\\', '/'])
}

pub fn short_path(p: &str, home: &str, plat: Plat) -> String {
    let sep = if plat == Plat::Win { "\\" } else { "/" };
    let hk = path_key(home, plat);
    let pk = path_key(p, plat);
    let mut shown = p.to_owned();
    if !hk.is_empty() && (pk == hk || pk.starts_with(&format!("{hk}\\")) || pk.starts_with(&format!("{hk}/"))) {
        let rest = cut(p).get(cut(home).len()..).unwrap_or("").trim_start_matches(['\\', '/']);
        shown = if rest.is_empty() { "~".to_owned() } else { format!("~{sep}{rest}") };
    }
    let parts: Vec<&str> = cut(&shown).split(['\\', '/']).filter(|x| !x.is_empty()).collect();
    let lead = if shown.starts_with(['\\', '/']) { sep } else { "" };
    if parts.len() <= 4 {
        let joined = format!("{lead}{}", parts.join(sep));
        return if joined.is_empty() { shown } else { joined };
    }
    format!("{lead}{}{sep}...{sep}{}", parts[0], parts[parts.len() - 2..].join(sep))
}

pub fn day(at: u64, offset_ms: i64) -> String {
    let local = at as i64 + offset_ms;
    let (y, m, d) = core_transcript::time::civil_from_days(local.div_euclid(DAY_MS));
    format!("{d:02}/{m:02}/{y:04}")
}

fn local_day(at: u64) -> String {
    day(at, core_transcript::time::local_offset_ms(at as i64))
}

pub struct Report<'a> {
    pub alert: bool,
    pub en: bool,
    pub plat: Plat,
    pub home: &'a str,
    pub since: String,
    pub drops: Vec<FreeDrop>,
    pub growth: Vec<Value>,
    pub largest: Vec<(String, u64)>,
}

pub fn build_report(r: &Report) -> String {
    let en = r.en;
    let title = match (r.alert, en) {
        (true, true) => "Disk almost full",
        (true, false) => "Ổ đĩa sắp đầy",
        (false, true) => "Background disk scan finished",
        (false, false) => "Quét nền ổ đĩa xong",
    };
    let mut lines = vec![title.to_owned()];
    for d in &r.drops {
        let (free, total, drop) = (fmt_bytes(d.free, en), fmt_bytes(d.total, en), fmt_bytes(d.drop, en));
        lines.push(if en {
            format!("Drive {} has {free} free of {total}, down {drop} since the scan on {}.", d.label, r.since)
        } else {
            format!("Ổ {} còn trống {free} / {total}, giảm {drop} so với lần quét ngày {}.", d.label, r.since)
        });
    }
    if !r.growth.is_empty() {
        lines.push(String::new());
        lines.push(format!("{}:", if en { "Grew the most" } else { "Phình nhiều nhất" }));
        for (n, row) in r.growth.iter().take(TOP_N).enumerate() {
            let delta = row["delta"].as_i64().unwrap_or(0).max(0) as u64;
            let after = row["after"].as_i64().unwrap_or(0).max(0) as u64;
            lines.push(format!("{}. {}  +{} ({})", n + 1, short_path(row["path"].as_str().unwrap_or(""), r.home, r.plat), fmt_bytes(delta, en), fmt_bytes(after, en)));
        }
    }
    if !r.largest.is_empty() {
        lines.push(String::new());
        lines.push(format!("{}:", if en { "Largest" } else { "Lớn nhất" }));
        for (n, (p, size)) in r.largest.iter().take(TOP_N).enumerate() {
            lines.push(format!("{}. {}  {}", n + 1, short_path(p, r.home, r.plat), fmt_bytes(*size, en)));
        }
    }
    lines.join("\n")
}

pub fn free_drops(base: &Snapshot, cur: &Snapshot, plat: Plat, min_drop: u64) -> Vec<FreeDrop> {
    let num = |v: &Value| v.as_u64().unwrap_or(0);
    let mut out: Vec<FreeDrop> = cur
        .volumes
        .iter()
        .filter_map(|v| {
            let mount = v["mount"].as_str()?;
            let key = path_key(mount, plat);
            let b = base.volumes.iter().find(|b| b["mount"].as_str().is_some_and(|m| path_key(m, plat) == key))?;
            let (free, total) = (num(&v["free"]), num(&v["total"]));
            if num(&b["total"]) == 0 || total == 0 {
                return None;
            }
            let drop = num(&b["free"]).checked_sub(free)?;
            let label = cut(mount);
            (drop > min_drop).then(|| FreeDrop { label: if label.is_empty() { mount.to_owned() } else { label.to_owned() }, free, total, drop })
        })
        .collect();
    out.sort_by(|a, b| b.drop.cmp(&a.drop));
    out
}

pub fn largest_dirs(c: &Compared, limit: usize) -> Vec<(String, u64)> {
    let size = |k: &str| c.nodes.get(k).and_then(|n| n.after).unwrap_or(0);
    let explained = |k: &str| c.children.get(k).into_iter().flatten().any(|ck| size(ck) as f64 >= size(k) as f64 * 0.8);
    let mut cands: Vec<(&String, u64)> = c.nodes.iter().filter(|(k, n)| !n.root && n.after.unwrap_or(0) > 0 && !explained(k)).map(|(k, _)| (k, size(k))).collect();
    cands.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(b.0)));
    let mut out: Vec<(&String, String, u64)> = Vec::new();
    for (key, sz) in cands {
        let nested = out.iter().any(|(o, _, _)| {
            [format!("{key}\\"), format!("{key}/")].iter().any(|k| o.starts_with(k.as_str())) || [format!("{o}\\"), format!("{o}/")].iter().any(|ok| key.starts_with(ok.as_str()))
        });
        if nested {
            continue;
        }
        let path = c.nodes.get(key).map(|n| n.path.clone()).unwrap_or_default();
        out.push((key, path, sz));
        if out.len() >= limit {
            break;
        }
    }
    out.into_iter().map(|(_, p, s)| (p, s)).collect()
}

pub fn compose(base: &Snapshot, cur: &Snapshot, alert: bool, en: bool, plat: Plat, home: &str, min_drop: u64, since: impl Fn(u64) -> String) -> Option<String> {
    let drops = free_drops(base, cur, plat, min_drop);
    if drops.is_empty() {
        return None;
    }
    let diff = build_compared(base, cur, plat);
    let own = build_compared(cur, cur, plat);
    let mut growth = top_growth(&diff);
    growth.truncate(TOP_N);
    Some(build_report(&Report { alert, en, plat, home, since: since(base.at), drops, growth, largest: largest_dirs(&own, TOP_N) }))
}

pub fn compose_local(base: &Snapshot, cur: &Snapshot, alert: bool, en: bool, plat: Plat, home: &str) -> Option<String> {
    compose(base, cur, alert, en, plat, home, FREE_DROP_BYTES, local_day)
}

fn sink() -> Option<PathBuf> {
    if !e2e_allowed() {
        return None;
    }
    std::env::var("CCM_E2E_DISKSNAP_TG_SINK").ok().filter(|v| !v.is_empty()).map(PathBuf::from)
}

fn tg_state(app: &AppHandle) -> Value {
    crate::desk::call_inside(app, "desk:tg-state", json!([null])).unwrap_or(Value::Null)
}

pub fn telegram_on(app: &AppHandle) -> bool {
    if sink().is_some() {
        return std::env::var("CCM_E2E_DISKSNAP_TG_OFF").ok().as_deref() != Some("1");
    }
    let st = tg_state(app);
    st["enabled"] == json!(true) && st["configured"] == json!(true)
}

pub fn send(app: &AppHandle, text: &str) -> bool {
    if let Some(file) = sink() {
        let line = format!("{}\n", json!({ "text": text }));
        return std::fs::OpenOptions::new().create(true).append(true).open(file).and_then(|mut f| f.write_all(line.as_bytes())).is_ok();
    }
    let st = tg_state(app);
    let Some(project) = st["routes"][0]["project"].as_str().filter(|p| !p.is_empty()) else { return false };
    crate::desk::call_inside(app, "desk:tg-report", json!([{ "cwd": project, "text": text }])).is_ok_and(|r| r["ok"] == json!(true))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::disk_snap::Root;

    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;

    fn snap(at: u64, free: u64, dirs: &[(&str, u64)]) -> Snapshot {
        let size = dirs.iter().filter(|(p, _)| p.matches('\\').count() == 4).map(|(_, n)| n).sum();
        Snapshot {
            at,
            roots: vec![Root { path: "C:\\Users\\me\\w".into(), size, dirs: dirs.iter().map(|(p, n)| ((*p).to_owned(), *n)).collect() }],
            volumes: vec![json!({ "mount": "C:\\", "label": "", "free": free, "total": 500 * GB }), json!({ "mount": "E:\\", "label": "", "free": 0, "total": 0 })],
        }
    }

    #[test]
    fn bytes_read_like_the_electron_report() {
        assert_eq!(fmt_bytes(512, true), "512 B");
        assert_eq!(fmt_bytes(80 * GB, false), "80 GB");
        assert_eq!(fmt_bytes(2 * MB + MB / 2, true), "2.5 MB");
        assert_eq!(fmt_bytes(2 * MB + MB / 2, false), "2,5 MB");
        assert_eq!(fmt_bytes(1000 * MB, true), "1,000 MB");
        assert_eq!(fmt_bytes(1000 * MB, false), "1.000 MB");
        assert_eq!(fmt_bytes(150 * GB + GB / 3, true), "150 GB");
    }

    #[test]
    fn paths_shorten_the_home_folder_and_long_chains() {
        assert_eq!(short_path("C:\\Users\\Me\\a", "c:\\users\\me", Plat::Win), "~\\a");
        assert_eq!(short_path("C:\\Users\\me", "C:\\Users\\me\\", Plat::Win), "~");
        assert_eq!(short_path("C:\\Users\\me\\AppData\\Local\\Temp\\x\\a", "C:\\Users\\me", Plat::Win), "~\\...\\x\\a");
        assert_eq!(short_path("D:\\w\\a", "C:\\Users\\me", Plat::Win), "D:\\w\\a");
        assert_eq!(short_path("/opt/a/b/c/d", "/home/me", Plat::Linux), "/opt/.../c/d");
        assert_eq!(short_path("/home/me/x", "/home/me", Plat::Mac), "~/x");
    }

    #[test]
    fn dates_are_day_month_year_in_local_time() {
        assert_eq!(day(1_791_331_200_000, 0), "07/10/2026");
        assert_eq!(day(1_791_331_200_000 - 1, 7 * 3_600_000), "07/10/2026");
    }

    #[test]
    fn a_free_space_drop_builds_the_full_report() {
        let base = snap(1, 100 * GB, &[("C:\\Users\\me\\w\\a", 3 * MB), ("C:\\Users\\me\\w\\b", 2 * MB), ("C:\\Users\\me\\w\\c", 1 * MB)]);
        let cur = snap(2, 90 * GB, &[("C:\\Users\\me\\w\\a", 6 * MB), ("C:\\Users\\me\\w\\b", 2 * MB), ("C:\\Users\\me\\w\\d", 2 * MB)]);
        let text = compose(&base, &cur, false, false, Plat::Win, "C:\\Users\\me", FREE_DROP_BYTES, |_| "15/11/2026".into()).unwrap();
        assert_eq!(
            text,
            [
                "Quét nền ổ đĩa xong",
                "Ổ C: còn trống 90 GB / 500 GB, giảm 10 GB so với lần quét ngày 15/11/2026.",
                "",
                "Phình nhiều nhất:",
                "1. ~\\w\\a  +3 MB (6 MB)",
                "2. ~\\w\\d  +2 MB (2 MB)",
                "",
                "Lớn nhất:",
                "1. ~\\w\\a  6 MB",
                "2. ~\\w\\b  2 MB",
                "3. ~\\w\\d  2 MB",
            ]
            .join("\n")
        );
        let en = compose(&base, &cur, true, true, Plat::Win, "C:\\Users\\me", FREE_DROP_BYTES, |_| "15/11/2026".into()).unwrap();
        assert!(en.starts_with("Disk almost full\nDrive C: has 90 GB free of 500 GB, down 10 GB since the scan on 15/11/2026.\n\nGrew the most:"));
        assert!(compose(&base, &snap(2, 96 * GB, &[]), false, true, Plat::Win, "C:\\Users\\me", FREE_DROP_BYTES, |_| String::new()).is_none());
    }

    #[test]
    fn largest_skips_parents_explained_by_one_child_and_nested_folders() {
        let s = snap(1, 0, &[("C:\\Users\\me\\w\\a", 10 * MB), ("C:\\Users\\me\\w\\a\\x", 9 * MB), ("C:\\Users\\me\\w\\b", 4 * MB), ("C:\\Users\\me\\w\\b\\y", 2 * MB), ("C:\\Users\\me\\w\\b\\z", 2 * MB)]);
        let c = build_compared(&s, &s, Plat::Win);
        let got: Vec<String> = largest_dirs(&c, 5).into_iter().map(|(p, _)| p).collect();
        assert_eq!(got, vec!["C:\\Users\\me\\w\\a\\x", "C:\\Users\\me\\w\\b"]);
    }
}
