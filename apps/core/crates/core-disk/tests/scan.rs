use core_disk::index::SEP;
use core_disk::serve::Engine;
use core_disk::{flag, scan, search, store, Control, Index, Only, Progress, Query, ScanError, ScanOptions, Sort};
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
    let p = std::env::temp_dir().join(format!("core-disk-{tag}-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    Tmp(p)
}

fn write(p: &Path, bytes: usize) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, vec![7u8; bytes]).unwrap();
}

fn set_mtime(p: &Path, t: SystemTime) {
    fs::File::options().write(true).open(p).unwrap().set_modified(t).unwrap();
}

fn tree() -> Tmp {
    let t = tmp("tree");
    let r = &t.0;
    write(&r.join("a.txt"), 1000);
    write(&r.join("docs").join("Báo cáo Tiếng Việt.pdf"), 5000);
    write(&r.join("docs").join("notes.md"), 300);
    write(&r.join("media").join("Holiday.MP4"), 200_000);
    write(&r.join("media").join("clip.mkv"), 50_000);
    write(&r.join("media").join("photos").join("Ảnh Đẹp.JPG"), 20_000);
    write(&r.join("media").join("photos").join("b.png"), 10_000);
    write(&r.join("node_modules").join("x").join("index.js"), 4000);
    fs::create_dir_all(r.join("empty")).unwrap();
    t
}

fn run(root: &Path, f: impl FnOnce(&mut ScanOptions)) -> Index {
    let mut o = ScanOptions::new(vec![root.to_string_lossy().to_string()]);
    o.threads = Some(3);
    f(&mut o);
    scan(&o, &Control::default(), &Progress::default()).unwrap()
}

fn names(ix: &Index, items: &[u32]) -> Vec<String> {
    items.iter().map(|i| ix.name(*i).to_string()).collect()
}

fn check_sums(ix: &Index) {
    for (i, n) in ix.nodes.iter().enumerate() {
        if n.is_dir() {
            let kids: u64 = ix.children(i as u32).map(|c| ix.nodes[c as usize].size).sum();
            assert_eq!(n.size, kids, "dir {}", ix.path(i as u32));
            let lens: u64 = ix.children(i as u32).map(|c| ix.nodes[c as usize].len).sum();
            assert_eq!(n.len, lens);
        }
    }
}

#[test]
fn sizes_add_up() {
    let t = tree();
    let ix = run(&t.0, |_| {});
    check_sums(&ix);
    assert_eq!(ix.stats.files, 8);
    assert_eq!(ix.stats.dirs, 7);
    let root = ix.roots[0];
    assert_eq!(ix.nodes[root as usize].len, 1000 + 5000 + 300 + 200_000 + 50_000 + 20_000 + 10_000 + 4000);
    assert!(ix.nodes[root as usize].size >= ix.nodes[root as usize].len);
    let photos = ix.find(&t.0.join("media").join("photos").to_string_lossy()).unwrap();
    assert_eq!(ix.nodes[photos as usize].len, 30_000);
    assert_eq!(ix.nodes[photos as usize].count, 2);
    let pic = ix.find(&t.0.join("media").join("photos").join("ảnh đẹp.jpg").to_string_lossy());
    if cfg!(any(windows, target_os = "macos")) {
        assert_eq!(pic.map(|p| ix.name(p)), Some("Ảnh Đẹp.JPG"));
    }
    let empty = ix.find(&t.0.join("empty").to_string_lossy()).unwrap();
    assert_eq!(ix.nodes[empty as usize].count, 0);
    assert!(ix.nodes[empty as usize].is_dir());
}

#[test]
fn hardlinks_count_once() {
    let t = tmp("hard");
    write(&t.0.join("big.bin"), 300_000);
    fs::create_dir_all(t.0.join("sub")).unwrap();
    fs::hard_link(t.0.join("big.bin"), t.0.join("sub").join("same.bin")).unwrap();
    let ix = run(&t.0, |o| o.hardlinks = true);
    check_sums(&ix);
    let root = &ix.nodes[ix.roots[0] as usize];
    let dup = ix.nodes.iter().filter(|n| n.flags & flag::HARDLINK != 0).count();
    assert_eq!(dup, 1);
    assert!(root.size >= 300_000 && root.size < 600_000, "{}", root.size);
    let off = run(&t.0, |o| o.hardlinks = false);
    assert!(off.nodes[off.roots[0] as usize].size >= 600_000);
}

fn make_dir_link(target: &Path, link: &Path) -> bool {
    #[cfg(windows)]
    {
        std::process::Command::new("cmd").args(["/C", "mklink", "/J"]).arg(link).arg(target).output().map(|o| o.status.success()).unwrap_or(false)
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
}

#[test]
fn links_are_not_followed() {
    let t = tmp("link");
    let outside = tmp("outside");
    write(&outside.0.join("huge.bin"), 400_000);
    write(&t.0.join("real").join("f.bin"), 1000);
    assert!(make_dir_link(&outside.0, &t.0.join("jump")));
    assert!(make_dir_link(&t.0.join("real"), &t.0.join("loop")));
    let ix = run(&t.0, |_| {});
    check_sums(&ix);
    let jump = ix.find(&t.0.join("jump").to_string_lossy()).unwrap();
    let n = ix.nodes[jump as usize];
    assert!(n.flags & flag::LINK != 0);
    assert_eq!((n.count, n.size), (0, 0));
    assert!(ix.nodes[ix.roots[0] as usize].len < 400_000);
    assert_eq!(ix.stats.files, 3);
}

#[test]
fn excludes_by_name_and_path() {
    let t = tree();
    let media = t.0.join("media").join("photos").to_string_lossy().to_string();
    let ix = run(&t.0, |o| o.excludes = vec!["node_modules".into(), "*.md".into(), media.clone()]);
    check_sums(&ix);
    let all: Vec<&str> = (0..ix.len() as u32).map(|i| ix.name(i)).collect();
    assert!(!all.contains(&"node_modules"));
    assert!(!all.contains(&"notes.md"));
    assert!(!all.contains(&"photos"));
    assert!(all.contains(&"clip.mkv"));
}

#[test]
fn cancel_mid_scan() {
    let t = tree();
    let opts = ScanOptions::new(vec![t.0.to_string_lossy().to_string()]);
    let control = Arc::new(Control::default());
    control.pause();
    let c = control.clone();
    let h = std::thread::spawn(move || scan(&opts, &c, &Progress::default()));
    std::thread::sleep(Duration::from_millis(100));
    assert!(!h.is_finished());
    control.cancel();
    assert_eq!(h.join().unwrap().err(), Some(ScanError::Cancelled));

    let opts = ScanOptions::new(vec![t.0.to_string_lossy().to_string()]);
    let control = Arc::new(Control::default());
    control.pause();
    let c = control.clone();
    let h = std::thread::spawn(move || scan(&opts, &c, &Progress::default()));
    std::thread::sleep(Duration::from_millis(50));
    control.resume();
    assert_eq!(h.join().unwrap().unwrap().stats.files, 8);
}

#[test]
fn missing_root_is_an_error() {
    let opts = ScanOptions::new(vec!["Z:\\definitely\\missing\\dir".into(), "/definitely/missing/dir".into()]);
    assert_eq!(scan(&opts, &Control::default(), &Progress::default()).err(), Some(ScanError::NoRoots));
}

#[test]
fn long_paths_and_unicode_names() {
    let t = tmp("long");
    let mut deep = t.0.clone();
    for i in 0..8 {
        deep = deep.join(format!("thư mục rất dài số {i} {}", "x".repeat(30)));
    }
    write(&deep.join("tệp cuối 😀.txt"), 123);
    assert!(deep.to_string_lossy().len() > 300);
    let ix = run(&t.0, |_| {});
    assert_eq!((ix.stats.errors, ix.stats.denied), (0, 0));
    let f = search(&ix, &Query::new("tep cuoi"));
    assert_eq!(f.total, 1);
    let i = f.items[0];
    assert_eq!(ix.nodes[i as usize].len, 123);
    assert_eq!(ix.find(&ix.path(i)), Some(i));
    assert!(ix.path(i).ends_with("tệp cuối 😀.txt"));
}

#[test]
fn save_load_round_trip() {
    let t = tree();
    let ix = run(&t.0, |_| {});
    let d = tmp("store");
    let (file, bytes) = store::save(&ix, &d.0).unwrap();
    assert!(file.exists() && bytes > 12);
    let back = store::load(&d.0).unwrap().unwrap();
    assert_eq!(back, ix);
    assert!(store::load(&d.0.join("nothing")).unwrap().is_none());
    let mut raw = fs::read(&file).unwrap();
    raw[8] = 99;
    fs::write(&file, &raw).unwrap();
    assert!(store::load(&d.0).is_err());
    let mut body = store::encode(&ix);
    let n = body.len();
    body.truncate(n - 3);
    assert!(store::decode(&body).is_err());
}

#[test]
fn search_filters() {
    let t = tree();
    let old = SystemTime::UNIX_EPOCH + Duration::from_secs(1_500_000_000);
    set_mtime(&t.0.join("media").join("clip.mkv"), old);
    let ix = run(&t.0, |_| {});
    let q = |text: &str| Query::new(text);

    let f = search(&ix, &q("holiday"));
    assert_eq!(names(&ix, &f.items), vec!["Holiday.MP4"]);

    let f = search(&ix, &q("tieng viet"));
    assert_eq!(names(&ix, &f.items), vec!["Báo cáo Tiếng Việt.pdf"]);
    let f = search(&ix, &q("ANH dep"));
    assert_eq!(names(&ix, &f.items), vec!["Ảnh Đẹp.JPG"]);
    let f = search(&ix, &q("tiếng"));
    assert_eq!(f.total, 1);

    let f = search(&ix, &q("*.mp4"));
    assert_eq!(names(&ix, &f.items), vec!["Holiday.MP4"]);

    let mut s = q("");
    s.min_size = Some(15_000);
    s.only = Some(Only::Files);
    let f = search(&ix, &s);
    assert_eq!(names(&ix, &f.items), vec!["Holiday.MP4", "clip.mkv", "Ảnh Đẹp.JPG"]);
    s.max_size = Some(60_000);
    assert_eq!(search(&ix, &s).total, 2);

    let mut s = q("");
    s.before = Some(1_600_000_000);
    assert_eq!(names(&ix, &search(&ix, &s).items), vec!["clip.mkv"]);
    s.before = None;
    s.after = Some(1_600_000_000);
    s.only = Some(Only::Files);
    assert_eq!(search(&ix, &s).total, 7);

    let mut s = q("");
    s.group = Some(1);
    s.sort = Sort::Name;
    assert_eq!(names(&ix, &search(&ix, &s).items), vec!["Ảnh Đẹp.JPG", "b.png"]);

    let mut s = q("");
    s.only = Some(Only::Dirs);
    s.sort = Sort::Name;
    assert_eq!(names(&ix, &search(&ix, &s).items), vec!["docs", "empty", "media", "node_modules", "photos", "x"]);

    let mut s = q("");
    s.under = ix.find(&t.0.join("media").to_string_lossy());
    s.only = Some(Only::Files);
    s.limit = 2;
    let f = search(&ix, &s);
    assert_eq!(f.total, 4);
    assert_eq!(names(&ix, &f.items), vec!["Holiday.MP4", "clip.mkv"]);

    let mut s = q("a");
    s.limit = 0;
    assert!(search(&ix, &s).total > 0 && search(&ix, &s).items.is_empty());
}

#[test]
fn search_parallel_matches_serial() {
    let mut b = core_disk::index::Builder::new();
    let root = b.push(core_disk::NONE, if cfg!(windows) { "C:\\big" } else { "/big" }, flag::DIR, 0, 0, 0);
    let n = 300_000u32;
    b.nodes[root as usize].first = 1;
    b.nodes[root as usize].count = n;
    for i in 0..n {
        let name = if i % 7 == 0 { format!("Ảnh {i}.jpg") } else { format!("file{i}.bin") };
        b.push(root, &name, 0, (i as u64 * 7919) % 100_000, 1, i);
    }
    let mut ix = b.finish(vec![root], Default::default());
    ix.sum_sizes();
    let mut q = Query::new("anh");
    q.limit = 50;
    let f = search(&ix, &q);
    assert_eq!(f.total, (n as usize).div_ceil(7));
    let mut sizes: Vec<u64> = (1..=n).filter(|i| (i - 1) % 7 == 0).map(|i| ix.nodes[i as usize].size).collect();
    sizes.sort_unstable_by(|a, b| b.cmp(a));
    assert_eq!(f.items.iter().map(|i| ix.nodes[*i as usize].size).collect::<Vec<_>>(), sizes[..50].to_vec());
}

#[test]
fn engine_ops_over_ndjson() {
    let t = tree();
    let events: Arc<Mutex<Vec<(String, Value)>>> = Arc::default();
    let sink = events.clone();
    let data = tmp("engine");
    let engine = Engine::new(Some(data.0.clone()), Arc::new(move |e, p| sink.lock().unwrap().push((e.to_string(), p))));
    let root = t.0.to_string_lossy().to_string();
    let r = engine.dispatch("disk.scan", &json!([{"roots": [root], "level": "light", "save": true}])).unwrap();
    assert_eq!(r["started"], json!(true));
    assert!(engine.dispatch("disk.scan", &json!([{"roots": [root]}])).is_err() || !engine.scanning());
    let deadline = std::time::Instant::now() + Duration::from_secs(20);
    while engine.scanning() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    let done = events.lock().unwrap().iter().find(|(e, _)| e == "disk:done").map(|(_, p)| p.clone()).unwrap();
    assert_eq!(done["ok"], json!(true));
    assert_eq!(done["files"], json!(8));
    assert!(done["saved"]["bytes"].as_u64().unwrap() > 0);

    let kids = engine.dispatch("disk.children", &json!({"path": root, "sort": "size", "limit": 2})).unwrap();
    assert_eq!(kids["total"], json!(5));
    assert_eq!(kids["items"][0]["name"], json!("media"));
    assert_eq!(kids["items"].as_array().unwrap().len(), 2);
    let top = engine.dispatch("disk.children", &json!({})).unwrap();
    assert_eq!(top["items"][0]["path"], json!(root));

    let found = engine.dispatch("disk.search", &json!({"query": "*.jpg", "kind": "image"})).unwrap();
    assert_eq!(found["total"], json!(1));
    assert_eq!(found["items"][0]["path"], json!(format!("{root}{SEP}media{SEP}photos{SEP}Ảnh Đẹp.JPG")));
    assert!(engine.dispatch("disk.search", &json!({"kind": "nope"})).is_err());

    engine.dispatch("disk.clear", &json!({})).unwrap();
    assert!(engine.dispatch("disk.search", &json!({})).is_err());
    let loaded = engine.dispatch("disk.load", &json!({})).unwrap();
    assert_eq!(loaded["loaded"], json!(true));
    assert_eq!(loaded["files"], json!(8));
    let st = engine.dispatch("disk.status", &json!({})).unwrap();
    assert_eq!(st["scanning"], json!(false));
    assert_eq!(engine.dispatch("disk.pause", &json!({})).unwrap()["ok"], json!(false));
    let vols = engine.dispatch("disk.volumes", &json!({})).unwrap();
    assert!(!vols.as_array().unwrap().is_empty());
    assert!(engine.dispatch("disk.nope", &json!({})).is_err());
}

#[test]
fn stdio_answers_requests() {
    #[derive(Clone, Default)]
    struct Buf(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for Buf {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let buf = Buf::default();
    let out = Arc::new(core_disk::serve::Out::new(Box::new(buf.clone())));
    let engine = Engine::new(None, Arc::new(|_, _| {}));
    let input = std::io::Cursor::new("{\"id\":1,\"channel\":\"disk.status\",\"args\":[]}\n{\"id\":2,\"channel\":\"disk.save\",\"args\":[]}\n{\"type\":\"shutdown\"}\n");
    core_disk::serve::serve(engine, input, out);
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        let text = String::from_utf8(buf.0.lock().unwrap().clone()).unwrap();
        if (text.contains("\"id\":1") && text.contains("\"id\":2")) || std::time::Instant::now() > deadline {
            assert!(text.lines().next().unwrap().contains("\"ready\":true"), "{text}");
            assert!(text.contains("\"id\":1,\"ok\":true"), "{text}");
            assert!(text.contains("\"error\":\"no index\""), "{text}");
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn engine_forget_updates_index() {
    let t = tree();
    let engine = Engine::new(None, Arc::new(|_, _| {}));
    assert!(engine.dispatch("disk.forget", &json!({"paths": ["x"]})).is_err());
    let ix = run(&t.0, |_| {});
    let root = t.0.to_string_lossy().to_string();
    let media = t.0.join("media").to_string_lossy().to_string();
    let clip = t.0.join("media").join("clip.mkv").to_string_lossy().to_string();
    let a = t.0.join("a.txt").to_string_lossy().to_string();
    let before = ix.nodes[ix.roots[0] as usize].size;
    let clip_size = ix.nodes[ix.find(&clip).unwrap() as usize].size;
    let a_size = ix.nodes[ix.find(&a).unwrap() as usize].size;
    let media_size = ix.nodes[ix.find(&media).unwrap() as usize].size;
    engine.set_index(Some(ix));
    let r = engine.dispatch("disk.forget", &json!([{"paths": [clip, a, t.0.join("nope").to_string_lossy()]}])).unwrap();
    assert_eq!(r["removed"], json!(2));
    assert_eq!(r["freed"], json!(clip_size + a_size));
    let ix = engine.index().unwrap();
    check_sums(&ix);
    assert_eq!(ix.nodes[ix.roots[0] as usize].size, before - clip_size - a_size);
    assert_eq!(ix.nodes[ix.find(&media).unwrap() as usize].size, media_size - clip_size);
    assert_eq!(ix.stats.files, 6);
    let kids = engine.dispatch("disk.children", &json!({"path": media})).unwrap();
    assert_eq!(kids["total"], json!(2));
    assert_eq!(engine.dispatch("disk.search", &json!({"query": "clip"})).unwrap()["total"], json!(0));
    let again = engine.dispatch("disk.forget", &json!({"paths": [t.0.join("media").join("clip.mkv").to_string_lossy()]})).unwrap();
    assert_eq!(again["removed"], json!(0));
    let all = engine.dispatch("disk.forget", &json!({"paths": [root]})).unwrap();
    assert_eq!(all["entries"], json!(0));
    assert_eq!(engine.dispatch("disk.children", &json!({})).unwrap()["total"], json!(0));
}
