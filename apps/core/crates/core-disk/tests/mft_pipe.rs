#[cfg(windows)]
mod run {
    use core_disk::mft::sample::EXPECTED_BYTES;
    use core_disk::serve::Engine;
    use serde_json::{json, Value};
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};
    use std::time::{Duration, Instant};

    type Events = Arc<Mutex<Vec<(String, Value)>>>;

    fn engine(fixture: PathBuf, data: Option<PathBuf>) -> (Arc<Engine>, Events) {
        let events: Events = Arc::default();
        let sink = events.clone();
        let e = Engine::with_mft_fixture(data, Arc::new(move |ev: &str, p: Value| sink.lock().unwrap().push((ev.to_string(), p))), fixture);
        (e, events)
    }

    fn done(events: &Events) -> Value {
        let t = Instant::now();
        while t.elapsed() < Duration::from_secs(30) {
            if let Some((_, p)) = events.lock().unwrap().iter().find(|(e, _)| e == "disk:done") {
                return p.clone();
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("no disk:done");
    }

    fn fixture() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests").join("fixtures").join("mft-sample.bin")
    }

    fn full_path_through_a_separate_helper_process() {
        let data = std::env::temp_dir().join(format!("orrery-mft-pipe-{}", std::process::id()));
        let (e, events) = engine(fixture(), Some(data.clone()));
        let v = e.dispatch("disk.mft", &json!({"drive": "q:", "save": true})).unwrap();
        assert_eq!(v["root"], "Q:\\");
        assert_eq!(e.dispatch("disk.mft", &json!({"drive": "Q"})).unwrap_err(), "busy");
        let d = done(&events);
        assert_eq!(d["ok"], true, "{d}");
        assert_eq!(d["mft"], true);
        assert_eq!(d["bytes"], EXPECTED_BYTES);
        assert!(d["saved"].is_object(), "{d}");
        assert!(events.lock().unwrap().iter().any(|(e, p)| e == "disk:progress" && p["phase"] == "mft"));
        let kids = e.dispatch("disk.children", &json!({"path": r"Q:\Users\jin"})).unwrap();
        let names: Vec<&str> = kids["items"].as_array().unwrap().iter().map(|i| i["name"].as_str().unwrap()).collect();
        assert_eq!(names, vec!["a.bin", "Ảnh dài tên.jpg", "link1.dat"]);
        let found = e.dispatch("disk.search", &json!({"query": "anh dai"})).unwrap();
        assert_eq!(found["total"], 1);
        assert!(!e.scanning());
        let _ = std::fs::remove_dir_all(data);
    }

    fn helper_failure_comes_back_as_done_error() {
        let bad = std::env::temp_dir().join(format!("orrery-mft-bad-{}.bin", std::process::id()));
        std::fs::write(&bad, vec![0u8; 4096]).unwrap();
        let (e, events) = engine(bad.clone(), None);
        e.dispatch("disk.mft", &json!({"drive": "Q"})).unwrap();
        let d = done(&events);
        assert_eq!(d["ok"], false);
        assert_eq!(d["error"], "not an MFT image", "{d}");
        assert!(e.index().is_none());
        let _ = std::fs::remove_file(bad);
    }

    fn missing_fixture_file() {
        let (e, events) = engine(PathBuf::from(r"Z:\does-not-exist\mft.bin"), None);
        e.dispatch("disk.mft", &json!({"drive": "Q"})).unwrap();
        let d = done(&events);
        assert_eq!(d["ok"], false);
        assert!(d["error"].as_str().unwrap().starts_with("fixture:"), "{d}");
    }

    fn mft_merges_with_an_existing_index() {
        let (e, events) = engine(fixture(), None);
        e.dispatch("disk.mft", &json!({"drive": "Q"})).unwrap();
        assert_eq!(done(&events)["ok"], true);
        events.lock().unwrap().clear();
        e.dispatch("disk.mft", &json!({"drive": "R"})).unwrap();
        let d = done(&events);
        assert_eq!(d["roots"], json!(["Q:\\", "R:\\"]));
        assert_eq!(d["bytes"], EXPECTED_BYTES * 2);
    }

    fn bad_drive_arguments() {
        let (e, _) = engine(fixture(), None);
        assert_eq!(e.dispatch("disk.mft", &json!({})).unwrap_err(), "drive required");
        assert_eq!(e.dispatch("disk.mft", &json!({"drive": r"\\srv\x"})).unwrap_err(), "drive required");
        let plain = Engine::new(None, Arc::new(|_: &str, _: Value| {}));
        assert_eq!(plain.dispatch("disk.mft", &json!({"drive": "B"})).unwrap_err(), "unsupported");
    }

    pub fn main() {
        let args: Vec<String> = std::env::args().collect();
        if args.get(1).map(String::as_str) == Some("disk") {
            std::process::exit(core_disk::cli::run(&args[2..]));
        }
        let tests: [(&str, fn()); 5] = [
            ("full_path_through_a_separate_helper_process", full_path_through_a_separate_helper_process),
            ("helper_failure_comes_back_as_done_error", helper_failure_comes_back_as_done_error),
            ("missing_fixture_file", missing_fixture_file),
            ("mft_merges_with_an_existing_index", mft_merges_with_an_existing_index),
            ("bad_drive_arguments", bad_drive_arguments),
        ];
        for (name, f) in tests {
            f();
            println!("test {name} ... ok");
        }
        println!("test result: ok. {} passed", tests.len());
    }
}

fn main() {
    #[cfg(windows)]
    run::main();
}
