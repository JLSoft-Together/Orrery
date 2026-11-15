use crate::index::{Index, Sort};
use crate::procstat::usage;
use crate::search::{search, Only, Query};
use crate::store;
use crate::walk::{scan, Control, Level, Progress, ScanOptions};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const MB: f64 = 1024.0 * 1024.0;

fn usage_text() -> &'static str {
    "usage: orrery-core disk [serve]\n       orrery-core disk bench <path> [--level light|balanced|max] [--hardlinks] [--exclude <name>] [--data <dir>]
       orrery-core disk bench-dupes <path> [--level light|balanced|max] [--min-mb N] [--exclude <name>]"
}

pub fn run(args: &[String]) -> i32 {
    match args.first().map(String::as_str) {
        None | Some("serve") => crate::serve::run(),
        Some("bench") => bench(&args[1..]),
        Some("bench-dupes") => bench_dupes(&args[1..]),
        Some("mft-helper") => crate::mft::helper_main(&args[1..]),
        Some("mft-fixture") => crate::mft::fixture_main(&args[1..]),
        _ => {
            eprintln!("{}", usage_text());
            2
        }
    }
}

fn samples(ix: &Index) -> Vec<(&'static str, Query)> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as u32;
    let mut by_name = Query::new("a");
    by_name.sort = Sort::Name;
    let glob = Query::new("*.mp4");
    let mut big = Query::new("");
    big.min_size = Some(100 * 1024 * 1024);
    big.only = Some(Only::Files);
    let mut recent = Query::new("");
    recent.group = Some(1);
    recent.after = Some(now.saturating_sub(30 * 86_400));
    recent.sort = Sort::Mtime;
    let mut folded = Query::new("tiếng việt");
    folded.under = ix.roots.first().copied();
    vec![("name contains \"a\", sort name", by_name), ("glob *.mp4", glob), ("files >= 100 MB, sort size", big), ("images last 30 days, sort mtime", recent), ("\"tiếng việt\" (folded) under root", folded)]
}

fn bench(args: &[String]) -> i32 {
    let mut path = None;
    let mut opts = ScanOptions::new(Vec::new());
    let mut data = std::env::temp_dir().join("orrery-disk-bench");
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--level" => opts.level = Level::parse(it.next().map(String::as_str).unwrap_or("balanced")),
            "--hardlinks" => opts.hardlinks = true,
            "--exclude" => opts.excludes.extend(it.next().cloned()),
            "--data" => data = it.next().map(PathBuf::from).unwrap_or(data),
            p if path.is_none() => path = Some(p.to_string()),
            _ => {
                eprintln!("{}", usage_text());
                return 2;
            }
        }
    }
    let Some(path) = path else {
        eprintln!("{}", usage_text());
        return 2;
    };
    opts.roots = vec![path.clone()];
    let control = Control::default();
    let progress = Arc::new(Progress::default());
    let before = usage();
    let t = Instant::now();
    let done = Arc::new(AtomicBool::new(false));
    let sampler = {
        let done = done.clone();
        std::thread::spawn(move || {
            let mut peak = 0f64;
            let mut last = (Instant::now(), usage().cpu_ms);
            while !done.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(500));
                let now = (Instant::now(), usage().cpu_ms);
                let wall = now.0.duration_since(last.0).as_millis().max(1) as f64;
                peak = peak.max((now.1 - last.1) as f64 / wall);
                last = now;
            }
            peak
        })
    };
    let r = scan(&opts, &control, &progress);
    let wall = t.elapsed();
    done.store(true, Ordering::SeqCst);
    let peak_cores = sampler.join().unwrap_or(0.0);
    let after = usage();
    let ix = match r {
        Ok(ix) => ix,
        Err(e) => {
            eprintln!("scan failed: {e}");
            return 1;
        }
    };
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as f64;
    let cpu = (after.cpu_ms - before.cpu_ms) as f64;
    let wall_ms = wall.as_millis().max(1) as f64;
    println!("path        {path}");
    println!("level       {} ({} threads of {})", opts.level.name(), opts.level.threads(), cores);
    println!("scan        {:.2} s", wall.as_secs_f64());
    println!("entries     {} ({} files, {} dirs)", ix.len(), ix.stats.files, ix.stats.dirs);
    println!("on disk     {:.2} GB", ix.stats.bytes as f64 / MB / 1024.0);
    println!("denied      {}  errors {}", ix.stats.denied, ix.stats.errors);
    println!("rate        {:.0} entries/s", ix.len() as f64 / wall.as_secs_f64().max(0.001));
    println!("cpu         avg {:.1}% of machine ({:.2} cores), peak {:.1}% ({:.2} cores)", cpu / wall_ms / cores * 100.0, cpu / wall_ms, peak_cores / cores * 100.0, peak_cores);
    println!("index ram   {:.1} MB ({} B/entry)", ix.heap_bytes() as f64 / MB, ix.heap_bytes() / ix.len().max(1));
    println!("process     rss {:.1} MB, peak {:.1} MB", after.rss as f64 / MB, after.peak_rss as f64 / MB);
    for (label, q) in samples(&ix) {
        let t = Instant::now();
        let found = search(&ix, &q);
        let first = t.elapsed().as_secs_f64() * 1000.0;
        let t = Instant::now();
        for _ in 0..5 {
            search(&ix, &q);
        }
        let avg = t.elapsed().as_secs_f64() * 1000.0 / 5.0;
        println!("search      {label}: {} hits, first {:.2} ms, avg {:.2} ms", found.total, first, avg);
    }
    let t = Instant::now();
    match store::save(&ix, &data) {
        Ok((file, bytes)) => {
            let save_ms = t.elapsed().as_millis();
            let t = Instant::now();
            let loaded = store::load(&data);
            let load_ms = t.elapsed().as_millis();
            let same = matches!(&loaded, Ok(Some(l)) if l.nodes == ix.nodes && l.pool == ix.pool);
            println!("save        {:.1} MB in {} ms ({})", bytes as f64 / MB, save_ms, file.display());
            println!("load        {} ms, round-trip {}", load_ms, if same { "ok" } else { "MISMATCH" });
        }
        Err(e) => println!("save        failed: {e}"),
    }
    0
}

fn bench_dupes(args: &[String]) -> i32 {
    let mut path = None;
    let mut opts = ScanOptions::new(Vec::new());
    let mut dopts = crate::dupes::Options::new(Vec::new());
    let mut it = args.iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--level" => {
                opts.level = Level::parse(it.next().map(String::as_str).unwrap_or("balanced"));
                dopts.level = opts.level;
            }
            "--min-mb" => dopts.min_size = (it.next().and_then(|v| v.parse::<f64>().ok()).unwrap_or(1.0) * MB) as u64,
            "--exclude" => {
                let e = it.next().cloned();
                opts.excludes.extend(e.clone());
                dopts.excludes.extend(e);
            }
            p if path.is_none() => path = Some(p.to_string()),
            _ => {
                eprintln!("{}", usage_text());
                return 2;
            }
        }
    }
    let Some(path) = path else {
        eprintln!("{}", usage_text());
        return 2;
    };
    opts.roots = vec![path.clone()];
    dopts.under = vec![path.clone()];
    let ix = match scan(&opts, &Control::default(), &Progress::default()) {
        Ok(ix) => ix,
        Err(e) => {
            eprintln!("scan failed: {e}");
            return 1;
        }
    };
    let cores = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1) as f64;
    let control = Control::default();
    let progress = Arc::new(crate::dupes::DupeProgress::default());
    let before = usage();
    let t = Instant::now();
    let done = Arc::new(AtomicBool::new(false));
    let sampler = {
        let done = done.clone();
        std::thread::spawn(move || {
            let mut peak = 0f64;
            let mut last = (Instant::now(), usage().cpu_ms);
            while !done.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(500));
                let now = (Instant::now(), usage().cpu_ms);
                let wall = now.0.duration_since(last.0).as_millis().max(1) as f64;
                peak = peak.max((now.1 - last.1) as f64 / wall);
                last = now;
            }
            peak
        })
    };
    let r = crate::dupes::find(&ix, &dopts, &control, &progress);
    let wall = t.elapsed();
    done.store(true, Ordering::SeqCst);
    let peak_cores = sampler.join().unwrap_or(0.0);
    let after = usage();
    let out = match r {
        Ok(o) => o,
        Err(e) => {
            eprintln!("dupes failed: {e}");
            return 1;
        }
    };
    let cpu = (after.cpu_ms - before.cpu_ms) as f64;
    let wall_ms = wall.as_millis().max(1) as f64;
    println!("path        {path}");
    println!("level       {} ({} threads of {})", dopts.level.name(), dopts.level.threads(), cores);
    println!("index       {} files", ix.stats.files);
    println!("min size    {:.1} MB", dopts.min_size as f64 / MB);
    println!("candidates  {}", out.candidates);
    println!("hashed      {} files, {:.2} GB read", out.hashed_files, out.hashed_bytes as f64 / MB / 1024.0);
    println!("time        {:.2} s", wall.as_secs_f64());
    println!("rate        {:.0} MB/s", out.hashed_bytes as f64 / MB / wall.as_secs_f64().max(0.001));
    println!("cpu         avg {:.1}% of machine ({:.2} cores), peak {:.1}% ({:.2} cores)", cpu / wall_ms / cores * 100.0, cpu / wall_ms, peak_cores / cores * 100.0, peak_cores);
    println!("groups      {} (wasted {:.2} GB)", out.total_groups, out.wasted as f64 / MB / 1024.0);
    println!("skipped     {} (stale {}, cloud {}, locked {}, changed {}, gone {})", out.skipped.total(), out.skipped.stale, out.skipped.cloud, out.skipped.locked, out.skipped.changed, out.skipped.gone);
    println!("process     peak rss {:.1} MB", after.peak_rss as f64 / MB);
    0
}
