use super::*;
use std::sync::atomic::AtomicUsize;

static SEQ: AtomicUsize = AtomicUsize::new(0);

struct TestDeps {
    root: PathBuf,
    vols: Vec<VolumeInfo>,
    procs: Mutex<Vec<ProcInfo>>,
    records: Mutex<Vec<Value>>,
    emits: Mutex<Vec<Value>>,
    asks: Mutex<Vec<&'static str>>,
    answer: AtomicBool,
    trash_ok: AtomicBool,
    pick: Mutex<Option<String>>,
}

impl Deps for TestDeps {
    fn plat(&self) -> Plat {
        Plat::host()
    }
    fn home(&self) -> String {
        self.root.join("users").join("me").to_string_lossy().into_owned()
    }
    fn data_dir(&self) -> PathBuf {
        self.root.join("data")
    }
    fn keep(&self) -> Vec<String> {
        Vec::new()
    }
    fn self_dirs(&self) -> Vec<String> {
        vec![self.root.join("app").to_string_lossy().into_owned()]
    }
    fn volumes(&self) -> Vec<VolumeInfo> {
        self.vols.clone()
    }
    fn suggest(&self) -> Vec<Suggestion> {
        vec![
            Suggestion { kind: "gradle", path: self.root.join("src").join(".gradle").to_string_lossy().into_owned(), guide: false },
            Suggestion { kind: "docker", path: self.root.join("src").join("Docker").to_string_lossy().into_owned(), guide: true },
        ]
    }
    fn holders(&self) -> Vec<ProcInfo> {
        lock(&self.procs).clone()
    }
    fn confirm(&self, ask: &MoveAsk) -> bool {
        lock(&self.asks).push(ask.kind);
        self.answer.load(Ordering::SeqCst)
    }
    fn trash(&self, p: &str) -> Result<(), String> {
        if !self.trash_ok.load(Ordering::SeqCst) {
            return Err("no bin".into());
        }
        let bin = self.root.join("bin");
        std::fs::create_dir_all(&bin).map_err(|e| e.to_string())?;
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        std::fs::rename(p, bin.join(format!("t{n}"))).map_err(|e| e.to_string())
    }
    fn record(&self, e: Value) {
        lock(&self.records).push(e);
    }
    fn emit(&self, p: Value) {
        lock(&self.emits).push(p);
    }
    fn index_size(&self, _p: &str) -> Option<u64> {
        None
    }
    fn clean_known(&self, p: &str) -> Option<String> {
        p.ends_with("known").then(|| p.to_owned())
    }
    fn pick(&self) -> Option<String> {
        lock(&self.pick).clone()
    }
    fn free_of(&self, _p: &str) -> Option<u64> {
        Some(1 << 40)
    }
    fn dev_of(&self, p: &str) -> Option<String> {
        tree::lstat(Path::new(p))?;
        let vol = self.root.join("vol").to_string_lossy().into_owned();
        Some(if guard::is_under(p, &vol, Plat::host()) { "vol".into() } else { "src".into() })
    }
    fn now(&self) -> u64 {
        1_700_000_000_000
    }
}

fn tmp_root() -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::SeqCst);
    let p = std::env::temp_dir().join(format!("orrery-move-{}-{}-{n}", std::process::id(), crate::config::random_hex(4)));
    std::fs::create_dir_all(&p).unwrap();
    std::fs::canonicalize(&p).map(|c| tree::plain_target(&c)).unwrap()
}

fn service_at(root: &Path) -> Service<TestDeps> {
    std::fs::create_dir_all(root.join("vol")).unwrap();
    std::fs::create_dir_all(root.join("net")).unwrap();
    let vol = |n: &str, network: bool| VolumeInfo { mount: root.join(n).to_string_lossy().into_owned(), label: n.into(), network, total: 1 << 41, free: 1 << 40 };
    Service::new(TestDeps {
        root: root.to_path_buf(),
        vols: vec![vol("vol", false), vol("net", true), vol("gone", false)],
        procs: Mutex::new(Vec::new()),
        records: Mutex::new(Vec::new()),
        emits: Mutex::new(Vec::new()),
        asks: Mutex::new(Vec::new()),
        answer: AtomicBool::new(true),
        trash_ok: AtomicBool::new(true),
        pick: Mutex::new(None),
    })
}

fn write(p: &Path, body: &[u8]) {
    std::fs::create_dir_all(p.parent().unwrap()).unwrap();
    std::fs::write(p, body).unwrap();
}

fn gradle_tree(root: &Path) -> PathBuf {
    let g = root.join("src").join(".gradle");
    write(&g.join("caches").join("big.jar"), &vec![7u8; 300_000]);
    write(&g.join("caches").join("tệp ✓.txt"), "xin chào".as_bytes());
    write(&g.join("wrapper").join("g.zip"), &vec![3u8; 50_000]);
    write(&root.join("outside").join("keep.txt"), b"outside");
    tree::make_dir_link(&root.join("outside"), &g.join("out-link")).unwrap();
    g
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

#[test]
fn parse_requests() {
    assert_eq!(parse_plan(&json!({ "candidate": "abc-1", "volume": "D:\\" })), Some(("abc-1".into(), "D:\\".into())));
    assert_eq!(parse_plan(&json!({ "candidate": "ABC", "volume": "D:\\" })), None);
    assert_eq!(parse_plan(&json!({ "candidate": "a", "volume": "" })), None);
    assert_eq!(parse_plan(&json!({ "candidate": "a", "volume": "x\ny" })), None);
    assert_eq!(parse_plan(&json!(["a"])), None);
    assert_eq!(parse_id(&json!({ "plan": "p-1" }), "plan"), Some("p-1".into()));
    assert_eq!(parse_id(&json!({ "plan": "x".repeat(65) }), "plan"), None);
    assert_eq!(parse_id(&json!({ "move": "m" }), "plan"), None);
    assert_eq!(parse_recover(&json!({ "pending": "e2e-crash-1", "action": "restore" })), Some(("e2e-crash-1".into(), "restore")));
    assert_eq!(parse_recover(&json!({ "pending": "a", "action": "delete" })), None);
    assert_eq!(parse_from_clean(&json!({ "path": "/x", "open": true })), Some(("/x".into(), true)));
    assert_eq!(parse_from_clean(&json!({ "path": "/x", "open": "yes" })), Some(("/x".into(), false)));
    assert_eq!(parse_from_clean(&json!({ "path": "" })), None);
    assert_eq!(move_need(1000), 1100);
    assert_eq!(move_need(10), (10.0f64 * 1.1).ceil() as u64);
    assert_eq!(move_need(0), 0);
}

#[test]
fn tools_and_holders() {
    for n in ["java.exe", "JAVA", "qemu-system-x86_64.exe", "com.docker.backend.exe", "kotlin-daemon", "studio64.exe"] {
        assert!(is_tool(n), "{n}");
    }
    for n in ["javac", "code.exe", "qemu", "com.docker."] {
        assert!(!is_tool(n), "{n}");
    }
    let procs = vec![
        ProcInfo { name: "java.exe".into(), cmd: "java -Dgradle.user.home=C:\\Users\\a\\.gradle org.gradle.Daemon".into() },
        ProcInfo { name: "code.exe".into(), cmd: "code C:\\Users\\a\\.npm".into() },
    ];
    assert_eq!(holder_of("C:\\Users\\a\\.gradle", &procs, Plat::Win), Some("java.exe".into()));
    assert_eq!(holder_of("c:/users/A/.gradle/", &procs, Plat::Win), Some("java.exe".into()));
    assert_eq!(holder_of("C:\\Users\\a\\.grad", &procs, Plat::Win), None);
    assert_eq!(holder_of("C:\\Users\\a\\.npm", &procs, Plat::Win), None);
    let lin = vec![ProcInfo { name: "node".into(), cmd: "node /home/a/.npm".into() }];
    assert_eq!(holder_of("/home/a/.npm", &lin, Plat::Linux), Some("node".into()));
    assert_eq!(holder_of("/home/A/.npm", &lin, Plat::Linux), None);
    let parsed = parse_proc_lines("Java.exe\tjava -jar x\r\n\nadb\n");
    assert_eq!(parsed, vec![ProcInfo { name: "java.exe".into(), cmd: "java -jar x".into() }, ProcInfo { name: "adb".into(), cmd: String::new() }]);
}

#[test]
fn suggestions_follow_env() {
    let env = |k: &str| match k {
        "GRADLE_USER_HOME" => Some("K:\\dev\\.gradle".to_owned()),
        "LOCALAPPDATA" => Some("C:\\Users\\a\\AppData\\Local".to_owned()),
        _ => None,
    };
    let list = move_suggestions(Plat::Win, "C:\\Users\\a", &env);
    let get = |k: &str| list.iter().find(|s| s.kind == k).unwrap().clone();
    assert_eq!(get("gradle").path, "K:\\dev\\.gradle");
    assert_eq!(get("pub").path, "C:\\Users\\a\\AppData\\Local\\Pub\\Cache");
    assert_eq!(get("sdk").path, "C:\\Users\\a\\AppData\\Local\\Android\\Sdk");
    assert_eq!(get("avd").path, "C:\\Users\\a\\.android\\avd");
    assert_eq!(get("npm").path, "C:\\Users\\a\\AppData\\Local\\npm-cache");
    assert!(get("docker").guide);
    let mac = move_suggestions(Plat::Mac, "/Users/a", &|_| None);
    assert_eq!(mac.iter().find(|s| s.kind == "sdk").unwrap().path, "/Users/a/Library/Android/sdk");
    assert_eq!(mac.iter().find(|s| s.kind == "docker").unwrap().path, "/Users/a/Library/Containers/com.docker.docker");
}

#[test]
fn samples_pick_largest_then_spread() {
    let files: Vec<(String, u64)> = (0..20).map(|i| (format!("f{i}"), i as u64)).collect();
    let out = tree::pick_samples(&files, 6);
    assert_eq!(out.len(), 6);
    assert_eq!(&out[..3], &["f19".to_owned(), "f18".to_owned(), "f17".to_owned()]);
    assert!(tree::pick_samples(&[("big".into(), 600 * 1024 * 1024)], 6).is_empty());
}

#[test]
fn journal_round_trip() {
    let root = tmp_root();
    let j = Journal::new(root.join("disk").join("moves.jsonl"));
    j.append(&json!({ "id": "a", "stage": "copying", "at": 1, "kind": "move", "source": "/s", "target": "/t", "bytes": 10, "files": 2 }));
    j.append(&json!({ "id": "a", "stage": "renamed", "at": 2 }));
    j.append(&json!({ "id": "a", "stage": "done", "at": 3, "leftover": "/s.orrery-moving" }));
    j.append(&json!({ "id": "b", "stage": "renamed", "at": 4 }));
    std::fs::OpenOptions::new().append(true).open(&j.file).unwrap().write_all(b"not json\n").unwrap();
    j.append(&json!({ "id": "c", "stage": "copying", "at": 5, "kind": "back", "source": "/s", "target": "/t" }));
    let e = j.entries();
    assert_eq!(e.len(), 2);
    assert_eq!(e[0], MoveEntry { id: "a".into(), kind: "move", source: "/s".into(), target: "/t".into(), bytes: 10, files: 2, at: 3, stage: "done".into(), leftover: Some("/s.orrery-moving".into()) });
    assert!(is_open(&e[0]));
    assert_eq!(e[1].kind, "back");
    j.append(&json!({ "id": "a", "stage": "cleaned", "at": 6, "leftover": "" }));
    let e = j.entries();
    assert_eq!(e[0].leftover, None);
    assert_eq!(e[0].at, 3);
    assert!(!is_open(&e[0]));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn candidate_blocks() {
    let root = tmp_root();
    let svc = service_at(&root);
    let g = gradle_tree(&root);
    let list = svc.list();
    let arr = list.as_array().unwrap();
    let gradle = arr.iter().find(|c| c["kind"] == "gradle").unwrap();
    assert!(gradle.get("block").is_none(), "{gradle}");
    let docker = arr.iter().find(|c| c["kind"] == "docker").unwrap();
    assert_eq!(docker["guide"], json!(true));
    assert!(docker.get("block").is_none());
    assert_eq!(svc.from_clean(&s(&root.join("nope")), true)["block"], json!("missing"));
    assert_eq!(svc.from_clean(&s(&g.join("caches").join("big.jar")), true)["block"], json!("notDir"));
    assert_eq!(svc.from_clean(&s(&g.join("out-link")), true)["block"], json!("link"));
    std::fs::create_dir_all(root.join("app").join("x")).unwrap();
    assert_eq!(svc.from_clean(&s(&root.join("app").join("x")), true)["block"], json!("self"));
    std::fs::create_dir_all(root.join("data").join("y")).unwrap();
    assert_eq!(svc.from_clean(&s(&root.join("data").join("y")), true)["block"], json!("self"));
    std::fs::create_dir_all(root.join("p")).unwrap();
    std::fs::create_dir_all(root.join(format!("p{MOVE_SUFFIX}"))).unwrap();
    assert_eq!(svc.from_clean(&s(&root.join("p")), true)["block"], json!("pending"));
    assert_eq!(svc.from_clean(&s(&root.join("users").join("me")), true)["block"], json!("missing"));
    std::fs::create_dir_all(root.join("users").join("me")).unwrap();
    assert_eq!(svc.from_clean(&s(&root.join("users").join("me")), true)["block"], json!("guard"));
    std::fs::create_dir_all(root.join("users").join("other")).unwrap();
    assert_eq!(svc.from_clean(&s(&root.join("users").join("other")), true)["block"], json!("guard"));
    assert_eq!(svc.from_clean("relative", true), Value::Null);
    assert_eq!(svc.from_clean(&s(&g), false), Value::Null);
    std::fs::create_dir_all(root.join("known")).unwrap();
    let k = svc.from_clean(&s(&root.join("known")), false);
    assert_eq!(k["kind"], json!("clean"));
    let again = svc.from_clean(&s(&g), true);
    assert_eq!(again["id"], gradle["id"]);
    assert_eq!(again["kind"], json!("gradle"));
    *lock(&svc.deps.pick) = Some(s(&root.join("known")));
    assert_eq!(svc.pick()["id"], k["id"]);
    let vols = svc.volumes(gradle["id"].as_str().unwrap());
    let reason = |n: &str| vols.as_array().unwrap().iter().find(|v| v["label"] == n).unwrap().get("reason").cloned();
    assert_eq!(reason("vol"), None);
    assert_eq!(reason("net"), Some(json!("network")));
    assert_eq!(reason("gone"), Some(json!("missing")));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn plan_locked_by_tool() {
    let root = tmp_root();
    let svc = service_at(&root);
    let g = gradle_tree(&root);
    let id = svc.from_clean(&s(&g), true)["id"].as_str().unwrap().to_owned();
    *lock(&svc.deps.procs) = vec![ProcInfo { name: "java.exe".into(), cmd: format!("java -Dgradle.user.home={} Daemon", s(&g)) }];
    let r = svc.plan(&id, &s(&root.join("vol")));
    assert_eq!(r["error"], json!("locked"));
    assert_eq!(r["holder"], json!("java.exe"));
    assert_eq!(svc.plan(&id, &s(&root.join("net")))["error"], json!("network"));
    assert_eq!(svc.plan(&id, "Z:\\nowhere")["error"], json!("invalid"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn move_link_and_back() {
    let root = tmp_root();
    let svc = service_at(&root);
    let g = gradle_tree(&root);
    let id = svc.from_clean(&s(&g), true)["id"].as_str().unwrap().to_owned();
    let plan = svc.plan(&id, &s(&root.join("vol")))["plan"].clone();
    let target = root.join("vol").join(MOVE_DIR).join(".gradle");
    assert_eq!(plan["target"], json!(s(&target)));
    assert_eq!(plan["files"], json!(3));
    assert_eq!(plan["bytes"], json!(350_000 + "xin chào".len()));
    assert_eq!(plan["need"], json!(move_need(plan["bytes"].as_u64().unwrap())));
    svc.deps.answer.store(false, Ordering::SeqCst);
    let no = svc.run(plan["id"].as_str().unwrap());
    assert_eq!(no["cancelled"], json!(true));
    assert_eq!(svc.run(plan["id"].as_str().unwrap())["error"], json!("invalid"));
    svc.deps.answer.store(true, Ordering::SeqCst);
    let plan = svc.plan(&id, &s(&root.join("vol")))["plan"].clone();
    let r = svc.run(plan["id"].as_str().unwrap());
    assert_eq!(r["ok"], json!(true), "{r}");
    assert!(tree::is_link(&g));
    assert_eq!(tree::realpath(&g), tree::realpath(&target));
    assert_eq!(std::fs::read_to_string(g.join("caches").join("tệp ✓.txt")).unwrap(), "xin chào");
    assert!(tree::is_link(&target.join("out-link")));
    assert_eq!(std::fs::read_to_string(root.join("outside").join("keep.txt")).unwrap(), "outside");
    assert!(tree::lstat(Path::new(&format!("{}{MOVE_SUFFIX}", s(&g)))).is_none());
    let recs = lock(&svc.deps.records).clone();
    assert!(recs.iter().any(|h| h["action"] == "trash" && h["paths"][0].as_str().unwrap().ends_with(MOVE_SUFFIX)));
    assert!(recs.iter().any(|h| h["action"] == "move" && h["paths"] == json!([s(&g), s(&target)])));
    assert!(lock(&svc.deps.emits).iter().any(|e| e["phase"] == "done"));
    let st = svc.state();
    assert_eq!(st["moves"].as_array().unwrap().len(), 1);
    assert_eq!(st["pending"], json!([]));
    assert_eq!(st["running"], Value::Null);
    assert_eq!(svc.from_clean(&s(&g), true)["block"], json!("moved"));
    let move_id = st["moves"][0]["id"].as_str().unwrap().to_owned();
    let b = svc.back(&move_id);
    assert_eq!(b["ok"], json!(true), "{b}");
    assert!(!tree::is_link(&g) && g.is_dir());
    assert!(!target.exists());
    assert!(tree::is_link(&g.join("out-link")));
    assert_eq!(std::fs::read_to_string(g.join("caches").join("tệp ✓.txt")).unwrap(), "xin chào");
    assert_eq!(svc.state()["moves"], json!([]));
    assert_eq!(svc.back(&move_id)["error"], json!("invalid"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn permanent_delete_needs_confirm() {
    let root = tmp_root();
    let svc = service_at(&root);
    svc.deps.trash_ok.store(false, Ordering::SeqCst);
    let g = gradle_tree(&root);
    let id = svc.from_clean(&s(&g), true)["id"].as_str().unwrap().to_owned();
    let plan = svc.plan(&id, &s(&root.join("vol")))["plan"].clone();
    let r = svc.run(plan["id"].as_str().unwrap());
    assert_eq!(r["ok"], json!(true), "{r}");
    assert_eq!(*lock(&svc.deps.asks), vec!["move", "permanent"]);
    assert!(lock(&svc.deps.records).iter().any(|h| h["action"] == "delete"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn verify_mismatch_rolls_back() {
    let root = tmp_root();
    let a = root.join("a");
    write(&a.join("x.bin"), &[1, 2, 3]);
    let b = root.join("b");
    tree::copy_tree(&a, &b, None, &mut |_, _| {}).unwrap();
    assert_eq!(tree::verify_copy(&a, &b, None).unwrap(), None);
    std::fs::write(b.join("x.bin"), [1, 2, 4]).unwrap();
    assert_eq!(tree::verify_copy(&a, &b, None).unwrap(), Some("hash"));
    std::fs::write(b.join("y.bin"), [0]).unwrap();
    assert_eq!(tree::verify_copy(&a, &b, None).unwrap(), Some("count"));
    let stop = AtomicBool::new(true);
    assert_eq!(tree::walk_tree(&a, Some(&stop), &mut |_, _| {}), Err(tree::CANCELLED.to_owned()));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn recover_restores_interrupted_move() {
    let root = tmp_root();
    let svc = service_at(&root);
    let npm = root.join("src").join("npm-cache");
    write(&npm.join("_cacache").join("x"), &vec![5u8; 5000]);
    let moving = PathBuf::from(format!("{}{MOVE_SUFFIX}", s(&npm)));
    std::fs::rename(&npm, &moving).unwrap();
    let tgt = root.join("vol").join(MOVE_DIR).join("npm-cache");
    write(&tgt.join("_cacache").join("x"), b"partial");
    svc.journal.append(&json!({ "id": "e2e-crash-1", "stage": "copying", "at": 1, "kind": "move", "source": s(&npm), "target": s(&tgt), "bytes": 5000, "files": 1 }));
    svc.journal.append(&json!({ "id": "e2e-crash-1", "stage": "renamed", "at": 2 }));
    let st = svc.state();
    assert_eq!(st["pending"][0]["stage"], json!("renamed"), "{st}");
    let r = svc.recover("e2e-crash-1", "restore");
    assert_eq!(r, json!({ "ok": true, "rolledBack": true }));
    assert!(npm.is_dir() && !tree::is_link(&npm));
    assert_eq!(std::fs::metadata(npm.join("_cacache").join("x")).unwrap().len(), 5000);
    assert!(!moving.exists() && !tgt.exists());
    assert_eq!(svc.state()["pending"], json!([]));
    assert_eq!(svc.recover("e2e-crash-1", "restore")["error"], json!("invalid"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn handle_rejects_bad_input() {
    let root = tmp_root();
    let svc = service_at(&root);
    assert_eq!(svc.handle("ccm:diskmove-run", &json!({ "plan": "BAD" })), json!({ "ok": false, "error": "invalid" }));
    assert_eq!(svc.handle("ccm:diskmove-plan", &json!(null)), json!({ "error": "invalid" }));
    assert_eq!(svc.handle("ccm:diskmove-volumes", &json!({ "candidate": "nope" })), json!([]));
    assert_eq!(svc.handle("ccm:diskmove-from-clean", &json!({})), Value::Null);
    assert_eq!(svc.handle("ccm:diskmove-cancel", &Value::Null), json!(false));
    assert_eq!(svc.handle("ccm:diskmove-state", &Value::Null), json!({ "moves": [], "pending": [], "running": null }));
    assert!(CHANNELS.iter().all(|c| handles(c)));
    assert!(!handles("ccm:diskmap-volumes"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn fake_dev_groups_by_first_folder() {
    let root = tmp_root();
    std::fs::create_dir_all(root.join("vols").join("D").join("x")).unwrap();
    std::fs::create_dir_all(root.join("vols").join("netE")).unwrap();
    std::fs::create_dir_all(root.join("home")).unwrap();
    let plat = Plat::host();
    let d = fake_dev(&root, &s(&root.join("vols").join("D")), plat);
    assert_eq!(d, fake_dev(&root, &s(&root.join("vols").join("D").join("x")), plat));
    assert_ne!(d, fake_dev(&root, &s(&root.join("vols").join("netE")), plat));
    assert_eq!(fake_dev(&root, &s(&root.join("home")), plat), Some("1".into()));
    assert_eq!(fake_dev(&root, &s(&root.join("missing")), plat), None);
    let vols = fake_volumes(&root);
    assert_eq!(vols.iter().map(|v| (v.label.as_str(), v.network)).collect::<Vec<_>>(), vec![("D", false), ("netE", true)]);
    let _ = std::fs::remove_dir_all(&root);
}
