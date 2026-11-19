use core_disk::dupes::{self, DupeProgress, Error, Options, QUICK};
use core_disk::serve::Engine;
use core_disk::{flag, scan, Control, Index, Progress, ScanOptions};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

struct Tmp(PathBuf);

impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn tmp(tag: &str) -> Tmp {
    static N: AtomicU64 = AtomicU64::new(0);
    let p = std::env::temp_dir().join(format!("core-dupes-{tag}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    Tmp(p)
}

fn blob(len: usize, seed: u64) -> Vec<u8> {
    let mut x = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    (0..len)
        .map(|_| {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            (x >> 24) as u8
        })
        .collect()
}

fn put(p: &Path, data: &[u8]) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, data).unwrap();
}

fn index(root: &Path) -> Index {
    scan(&ScanOptions::new(vec![root.to_string_lossy().to_string()]), &Control::default(), &Progress::default()).unwrap()
}

fn opts(root: &Path) -> Options {
    let mut o = Options::new(vec![root.to_string_lossy().to_string()]);
    o.min_size = 1000;
    o
}

fn run(ix: &Index, o: &Options) -> dupes::Outcome {
    dupes::find(ix, o, &Control::default(), &DupeProgress::default()).unwrap()
}

fn names(g: &dupes::Group) -> Vec<String> {
    g.files.iter().map(|f| Path::new(&f.path).file_name().unwrap().to_string_lossy().to_string()).collect()
}

#[test]
fn groups_by_content_not_by_name() {
    let t = tmp("groups");
    let r = &t.0;
    let a = blob(300_000, 1);
    put(&r.join("one").join("a.bin"), &a);
    put(&r.join("two").join("renamed.dat"), &a);
    put(&r.join("two").join("deep").join("copy3"), &a);
    let b = blob(5_000, 2);
    put(&r.join("b1.txt"), &b);
    put(&r.join("sub").join("b2.txt"), &b);
    put(&r.join("unique.bin"), &blob(300_000, 3));
    put(&r.join("same-size-other-head.bin"), &blob(300_000, 4));
    let ix = index(r);
    let out = run(&ix, &opts(r));
    assert_eq!(out.total_groups, 2);
    assert_eq!(out.groups[0].size, 300_000);
    assert_eq!(out.groups[0].count, 3);
    assert_eq!(out.groups[1].size, 5_000);
    assert_eq!(out.groups[1].count, 2);
    assert_eq!(out.wasted, 300_000 * 2 + 5_000);
    assert_eq!(out.candidates, 7);
    assert!(out.groups[0].wasted() >= out.groups[1].wasted());
}

#[test]
fn difference_only_in_the_middle_needs_full_hash() {
    let t = tmp("middle");
    let r = &t.0;
    let len = 600_000;
    let a = blob(len, 9);
    let mut b = a.clone();
    b[len / 2] ^= 0xFF;
    assert_eq!(a[..QUICK], b[..QUICK]);
    assert_eq!(a[len - QUICK..], b[len - QUICK..]);
    put(&r.join("a.bin"), &a);
    put(&r.join("b.bin"), &b);
    put(&r.join("c.bin"), &a);
    let ix = index(r);
    let out = run(&ix, &opts(r));
    assert_eq!(out.total_groups, 1);
    assert_eq!(out.groups[0].count, 2);
    let n = names(&out.groups[0]);
    assert!(n.contains(&"a.bin".to_string()) && n.contains(&"c.bin".to_string()));
    assert!(out.hashed_bytes > 2 * len as u64);
}

#[test]
fn hardlinks_are_not_duplicates() {
    let t = tmp("hardlink");
    let r = &t.0;
    let a = blob(200_000, 5);
    put(&r.join("orig.bin"), &a);
    fs::hard_link(r.join("orig.bin"), r.join("link.bin")).unwrap();
    let ix = index(r);
    let out = run(&ix, &opts(r));
    assert_eq!(out.total_groups, 0);
    put(&r.join("real-copy.bin"), &a);
    let ix = index(r);
    let out = run(&ix, &opts(r));
    assert_eq!(out.total_groups, 1);
    assert_eq!(out.groups[0].count, 2);
}

#[test]
fn cloud_flagged_and_small_files_are_never_candidates() {
    let t = tmp("cloud");
    let r = &t.0;
    let a = blob(200_000, 6);
    put(&r.join("a.bin"), &a);
    put(&r.join("b.bin"), &a);
    put(&r.join("c.bin"), &a);
    put(&r.join("tiny1.txt"), b"hello");
    put(&r.join("tiny2.txt"), b"hello");
    let mut ix = index(r);
    let c = ix.find(&r.join("c.bin").to_string_lossy()).unwrap();
    ix.nodes[c as usize].flags |= flag::CLOUD;
    let out = run(&ix, &opts(r));
    assert_eq!(out.candidates, 2);
    assert_eq!(out.groups[0].count, 2);
    assert!(!names(&out.groups[0]).contains(&"c.bin".to_string()));
    let mut small = opts(r);
    small.min_size = 1;
    let out = run(&ix, &small);
    assert_eq!(out.total_groups, 2);
    let mut big = opts(r);
    big.min_size = 1_000_000;
    assert_eq!(run(&ix, &big).total_groups, 0);
}

#[test]
fn default_and_custom_excludes_prune_subtrees() {
    let t = tmp("excl");
    let r = &t.0;
    let a = blob(100_000, 7);
    put(&r.join("keep1.bin"), &a);
    put(&r.join("node_modules").join("pkg").join("x.bin"), &a);
    put(&r.join(".git").join("objects").join("y.bin"), &a);
    put(&r.join("AppData").join("z.bin"), &a);
    put(&r.join("skipme").join("w.bin"), &a);
    put(&r.join("keep2.bin"), &a);
    let ix = index(r);
    let mut o = opts(r);
    let out = run(&ix, &o);
    assert_eq!(out.groups[0].count, 3);
    o.excludes = vec!["skipme".into()];
    let out = run(&ix, &o);
    assert_eq!(out.groups[0].count, 2);
    let mut inside = opts(&r.join("node_modules"));
    inside.min_size = 1;
    assert!(dupes::find(&ix, &inside, &Control::default(), &DupeProgress::default()).is_ok());
}

#[test]
fn unicode_paths_and_files_changed_since_scan() {
    let t = tmp("uni");
    let r = &t.0;
    let a = blob(150_000, 8);
    put(&r.join("Tài liệu").join("Báo cáo Ảnh Đẹp.bin"), &a);
    put(&r.join("日本語").join("コピー.bin"), &a);
    put(&r.join("stale1.bin"), &blob(150_000, 10));
    put(&r.join("stale2.bin"), &blob(150_000, 10));
    let ix = index(r);
    fs::write(r.join("stale2.bin"), blob(150_001, 11)).unwrap();
    let out = run(&ix, &opts(r));
    assert_eq!(out.total_groups, 1);
    assert_eq!(out.groups[0].count, 2);
    assert!(out.groups[0].files.iter().any(|f| f.path.contains("Báo cáo Ảnh Đẹp")));
    assert!(out.groups[0].files.iter().any(|f| f.path.contains("コピー")));
    assert_eq!(out.skipped.stale, 1);
}

#[test]
fn mtime_change_since_scan_is_stale() {
    let t = tmp("mtime");
    let r = &t.0;
    let a = blob(150_000, 12);
    put(&r.join("a.bin"), &a);
    put(&r.join("b.bin"), &a);
    let ix = index(r);
    let f = fs::File::options().write(true).open(r.join("b.bin")).unwrap();
    f.set_modified(SystemTime::now() + Duration::from_secs(3600)).unwrap();
    drop(f);
    let out = run(&ix, &opts(r));
    assert_eq!(out.total_groups, 0);
    assert_eq!(out.skipped.stale, 1);
}

#[cfg(windows)]
#[test]
fn locked_file_is_skipped() {
    use std::os::windows::fs::OpenOptionsExt;
    let t = tmp("locked");
    let r = &t.0;
    let a = blob(150_000, 13);
    put(&r.join("a.bin"), &a);
    put(&r.join("b.bin"), &a);
    put(&r.join("c.bin"), &a);
    let ix = index(r);
    let _held = fs::OpenOptions::new().read(true).share_mode(0).open(r.join("c.bin")).unwrap();
    let out = run(&ix, &opts(r));
    assert_eq!(out.total_groups, 1);
    assert_eq!(out.groups[0].count, 2);
    assert_eq!(out.skipped.locked, 1);
}

#[test]
fn old_files_first_and_group_cap() {
    let t = tmp("order");
    let r = &t.0;
    let a = blob(2_000, 14);
    for i in 0..300 {
        put(&r.join(format!("d{:03}", i % 7)).join(format!("f{i:03}.bin")), &a);
    }
    let old = r.join("d003").join("f003.bin");
    let f = fs::File::options().write(true).open(&old).unwrap();
    f.set_modified(SystemTime::now() - Duration::from_secs(86_400 * 400)).unwrap();
    drop(f);
    let ix = index(r);
    let mut o = opts(r);
    o.max_files = 50;
    let out = run(&ix, &o);
    assert_eq!(out.groups[0].count, 300);
    assert_eq!(out.groups[0].files.len(), 50);
    assert!(out.groups[0].files[0].path.ends_with("f003.bin"));
    assert_eq!(out.wasted, 2_000 * 299);
    let mut capped = opts(r);
    capped.max_groups = 0;
    assert!(run(&ix, &capped).groups.is_empty());
}

#[test]
fn cancel_stops_without_result() {
    let t = tmp("cancel");
    let r = &t.0;
    for i in 0..40 {
        let a = blob(400_000, 100 + (i / 2));
        put(&r.join(format!("f{i}.bin")), &a);
    }
    let ix = index(r);
    let c = Control::default();
    c.cancel();
    assert_eq!(dupes::find(&ix, &opts(r), &c, &DupeProgress::default()).unwrap_err(), Error::Cancelled);

    let c = Arc::new(Control::default());
    let p = Arc::new(DupeProgress::default());
    let (c2, p2) = (c.clone(), p.clone());
    let h = std::thread::spawn(move || {
        while p2.bytes_read() == 0 {
            std::thread::sleep(Duration::from_millis(1));
        }
        c2.cancel();
    });
    let r = dupes::find(&ix, &opts(&t.0), &c, &p);
    h.join().unwrap();
    assert!(matches!(r, Err(Error::Cancelled)) || r.is_ok());
}

#[test]
fn total_file_budget_keeps_two_per_group() {
    let t = tmp("budget");
    let r = &t.0;
    for g in 0..6u64 {
        let a = blob(3_000, 200 + g);
        for i in 0..10 {
            put(&r.join(format!("g{g}")).join(format!("f{i}.bin")), &a);
        }
    }
    let ix = index(r);
    let mut o = opts(r);
    o.max_total_files = 20;
    let out = run(&ix, &o);
    assert_eq!(out.total_groups, 6);
    assert!(out.groups.iter().all(|g| g.count == 10 && g.files.len() >= 2));
    assert!(out.groups.iter().map(|g| g.files.len()).sum::<usize>() <= 20 + 2 * 6);
}

#[test]
fn not_in_index_is_an_error() {
    let t = tmp("missing");
    put(&t.0.join("a.bin"), &blob(5_000, 1));
    let ix = index(&t.0);
    let o = Options::new(vec![t.0.join("nope").to_string_lossy().to_string()]);
    assert!(matches!(dupes::find(&ix, &o, &Control::default(), &DupeProgress::default()), Err(Error::NotInIndex(_))));
}

#[test]
fn engine_runs_dupes_in_background() {
    let t = tmp("engine");
    let a = blob(120_000, 21);
    put(&t.0.join("x").join("one.bin"), &a);
    put(&t.0.join("y").join("two.bin"), &a);
    put(&t.0.join("z").join("three.bin"), &blob(120_000, 22));
    let events: Arc<Mutex<Vec<(String, Value)>>> = Arc::default();
    let sink = events.clone();
    let engine = Engine::new(None, Arc::new(move |e, p| sink.lock().unwrap().push((e.to_string(), p))));
    let root = t.0.to_string_lossy().to_string();
    assert!(engine.dispatch("disk.dupes", &json!({"minSize": 1000})).is_err());
    engine.dispatch("disk.scan", &json!({"roots": [root], "level": "light"})).unwrap();
    wait(&engine);
    let r = engine.dispatch("disk.dupes", &json!({"under": [root], "minSize": 1000, "level": "light"})).unwrap();
    assert_eq!(r["started"], json!(true));
    assert!(engine.dispatch("disk.dupes", &json!({"under": [root]})).is_err());
    wait_done(&events, 1);
    let ev = events.lock().unwrap();
    let done = ev.iter().find(|(e, _)| e == "disk:dupes-done").map(|(_, p)| p.clone()).unwrap();
    assert_eq!(done["ok"], json!(true));
    assert_eq!(done["totalGroups"], json!(1));
    assert_eq!(done["groups"][0]["count"], json!(2));
    assert_eq!(done["wasted"], json!(120_000));
    assert!(done["groups"][0]["files"][0]["mtime"].as_u64().unwrap() > 0);
    drop(ev);
    let err = engine.dispatch("disk.dupes", &json!({"under": [format!("{root}{}nope", core_disk::index::SEP)]})).unwrap_err();
    assert!(err.starts_with("not in index"), "{err}");
}

fn wait(engine: &Arc<Engine>) {
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while engine.scanning() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(!engine.scanning());
}

fn wait_done(events: &Arc<Mutex<Vec<(String, Value)>>>, n: usize) {
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    while events.lock().unwrap().iter().filter(|(e, _)| e == "disk:dupes-done").count() < n && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(10));
    }
}
