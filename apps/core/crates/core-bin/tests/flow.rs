mod common;

use common::*;
use serde_json::{json, Value};
use std::process::Command;
use std::time::{Duration, Instant};
use tungstenite::Message;

const RING: usize = 256 * 1024;

#[derive(Debug)]
enum Msg {
    Text(Value),
    Bin(Vec<u8>),
    Close(u16),
}

fn read_ordered(c: &mut Client, ms: u64, stop: impl Fn(&[Msg]) -> bool) -> Vec<Msg> {
    let end = Instant::now() + Duration::from_millis(ms);
    let mut out = Vec::new();
    loop {
        match c.ws.read() {
            Ok(Message::Text(t)) => out.push(Msg::Text(serde_json::from_str(t.as_str()).unwrap())),
            Ok(Message::Binary(b)) => out.push(Msg::Bin(b.to_vec())),
            Ok(Message::Close(f)) => {
                out.push(Msg::Close(f.map(|f| u16::from(f.code)).unwrap_or(1005)));
                return out;
            }
            Ok(_) => {}
            Err(tungstenite::Error::Io(e)) if matches!(e.kind(), std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut) => {}
            Err(e) => panic!("ws error {e}"),
        }
        if stop(&out) {
            return out;
        }
        assert!(Instant::now() < end, "timeout; got {:?}", out.iter().map(kind).collect::<Vec<_>>());
    }
}

fn kind(m: &Msg) -> String {
    match m {
        Msg::Text(v) => v["t"].as_str().unwrap_or("?").to_owned(),
        Msg::Bin(b) => format!("bin({})", b.len()),
        Msg::Close(c) => format!("close({c})"),
    }
}

fn spawn_with(core: &mut Core, c: &mut Client, id: &str, cwd: &str, cols: u16, args: &[&str]) -> u64 {
    c.send(json!({ "t": "spawn", "preset": "shell", "cols": cols, "rows": 30, "cwd": cwd }));
    let req = core.next(|m| m["t"] == "resolve", 5_000);
    core.resolved(&req["rid"], id, &node(), args, cwd);
    let ready = c.text("ready");
    assert_eq!(ready["id"], id);
    ready["pid"].as_u64().unwrap()
}

fn rss_kb(pid: u32) -> u64 {
    #[cfg(windows)]
    {
        let out = Command::new("tasklist").args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"]).output().unwrap();
        let s = String::from_utf8_lossy(&out.stdout);
        let field = s.trim().rsplit("\",\"").next().unwrap_or("");
        field.chars().filter(char::is_ascii_digit).collect::<String>().parse().unwrap_or(0)
    }
    #[cfg(unix)]
    {
        let out = Command::new("ps").args(["-o", "rss=", "-p", &pid.to_string()]).output().unwrap();
        String::from_utf8_lossy(&out.stdout).trim().parse().unwrap_or(0)
    }
}

fn strip_csi(b: &[u8]) -> String {
    let s = String::from_utf8_lossy(b);
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(ch) = it.next() {
        if ch == '\x1b' {
            match it.peek() {
                Some('[') => {
                    it.next();
                    for c in it.by_ref() {
                        if ('@'..='~').contains(&c) {
                            break;
                        }
                    }
                }
                Some(']') => {
                    it.next();
                    while let Some(c) = it.next() {
                        if c == '\x07' {
                            break;
                        }
                        if c == '\x1b' {
                            it.next();
                            break;
                        }
                    }
                }
                _ => {
                    it.next();
                }
            }
            continue;
        }
        out.push(ch);
    }
    out
}

fn line_numbers(text: &str) -> Vec<u64> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 8 <= b.len() {
        if b[i] == b'L' && b[i + 1..i + 8].iter().all(u8::is_ascii_digit) && (i + 8 == b.len() || !b[i + 8].is_ascii_digit()) {
            out.push(std::str::from_utf8(&b[i + 1..i + 8]).unwrap().parse().unwrap());
            i += 8;
        } else {
            i += 1;
        }
    }
    out
}

fn read_progress(path: &std::path::Path) -> String {
    std::fs::read_to_string(path).unwrap_or_default()
}

const FLOOD: &str = "if(process.stdin.isTTY)process.stdin.setRawMode(true);const fs=require('fs');const p=process.argv[1];const TOTAL=Number(process.argv[2]);const line=(n)=>'L'+String(n).padStart(7,'0')+' '+'y'.repeat(110)+'\\n';let sent=0,n=0,started=false;const pump=()=>{let batch=0;while(batch<65536&&sent<TOTAL){const l=line(n++);sent+=l.length;batch+=l.length;process.stdout.write(l)}const wait=()=>{fs.writeFileSync(p,String(sent-process.stdout.writableLength));if(process.stdout.writableLength>0)return setTimeout(wait,20);if(sent<TOTAL)setImmediate(pump);else process.stdout.write('FLOOD-END '+n+'\\n',()=>fs.writeFileSync(p,'done '+sent))};wait()};process.stdout.write('flood-ready\\n');process.stdin.on('data',(d)=>{const s=String(d);if(s.includes('g')&&!started){started=true;pump()}if(s.includes('q'))process.exit(0)});if(process.argv[3]==='auto'){started=true;setTimeout(pump,300)}";

fn progress_now(path: &std::path::Path) -> u64 {
    read_progress(path).trim_start_matches("done ").trim().parse().unwrap_or(0)
}

fn wait_progress_stall(path: &std::path::Path, ms: u64) -> u64 {
    let end = Instant::now() + Duration::from_millis(ms);
    let mut last = progress_now(path);
    let mut stable_since = Instant::now();
    while Instant::now() < end {
        std::thread::sleep(Duration::from_millis(100));
        let now = progress_now(path);
        if now != last {
            last = now;
            stable_since = Instant::now();
        } else if stable_since.elapsed() > Duration::from_millis(1200) {
            return now;
        }
    }
    last
}

#[test]
fn backpressure_8mb_slow_client_blocks_the_child_keeps_rss_bounded_and_loses_nothing() {
    let mut core = Core::start();
    let core_pid = core.child.id();
    let cwd = temp_dir("bp");
    let progress = std::path::PathBuf::from(&cwd).join("progress.txt");
    let _ = std::fs::remove_file(&progress);
    let total = 8 * 1024 * 1024;
    let mut c = Client::connect(core.port);
    let id = "6f1c1d2e-0000-4000-8000-0000000000d1";
    spawn_with(&mut core, &mut c, id, &cwd, 200, &["-e", FLOOD, progress.to_str().unwrap(), &total.to_string()]);
    c.out_has("flood-ready", 10_000);
    std::thread::sleep(Duration::from_millis(500));
    let base = rss_kb(core_pid);
    c.send_bytes(b"g");

    let stalled = wait_progress_stall(&progress, 20_000);
    let peak_stalled = rss_kb(core_pid);
    assert!(stalled < total as u64, "child never blocked while the client did not read: {stalled}");
    assert!(stalled > 0, "child did not start");

    let mut peak = peak_stalled;
    let mut last_sample = Instant::now();
    let end = Instant::now() + Duration::from_secs(120);
    loop {
        c.pump();
        if c.out_str().contains("FLOOD-END") {
            break;
        }
        if last_sample.elapsed() > Duration::from_millis(250) {
            peak = peak.max(rss_kb(core_pid));
            last_sample = Instant::now();
        }
        std::thread::sleep(Duration::from_millis(2));
        assert!(Instant::now() < end, "flood did not finish; progress={}", read_progress(&progress));
    }
    std::thread::sleep(Duration::from_millis(300));
    c.pump();
    let text = strip_csi(&c.output);
    let nums = line_numbers(&text);
    let expected = (total as u64).div_ceil(120);
    let missing: Vec<u64> = (0..expected).filter(|n| nums.binary_search(n).is_err()).take(10).collect();
    assert!(nums.windows(2).all(|w| w[0] < w[1]), "lines out of order");
    assert!(missing.is_empty(), "lost lines {missing:?} of {expected}");
    assert!(read_progress(&progress).starts_with("done"));
    let grow = peak.saturating_sub(base);
    eprintln!("backpressure: stalled at {stalled} of {total} bytes, core rss base={base} KB peak={peak} KB (+{grow} KB), lines={}", nums.len());
    assert!(grow < 12 * 1024, "core rss grew {grow} KB");
    c.send_bytes(b"q");
    c.text("exit");
}

#[test]
fn socket_closed_while_paused_lets_the_child_run_on_into_the_ring() {
    let mut core = Core::start();
    let cwd = temp_dir("bpclose");
    let progress = std::path::PathBuf::from(&cwd).join("progress.txt");
    let _ = std::fs::remove_file(&progress);
    let total = 6 * 1024 * 1024;
    let mut c = Client::connect(core.port);
    let id = "6f1c1d2e-0000-4000-8000-0000000000d2";
    spawn_with(&mut core, &mut c, id, &cwd, 200, &["-e", FLOOD, progress.to_str().unwrap(), &total.to_string()]);
    c.out_has("flood-ready", 10_000);
    c.send_bytes(b"g");
    let stalled = wait_progress_stall(&progress, 20_000);
    assert!(stalled < total as u64, "child never blocked: {stalled}");
    drop(c);
    let end = Instant::now() + Duration::from_secs(60);
    while !read_progress(&progress).starts_with("done") {
        assert!(Instant::now() < end, "child stayed blocked after the socket closed: {}", read_progress(&progress));
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(500));
    let mut b = Client::connect(core.port);
    b.send(json!({ "t": "attach", "id": id }));
    let msgs = read_ordered(&mut b, 10_000, |m| m.iter().any(|x| matches!(x, Msg::Bin(_))));
    let replay: Vec<u8> = msgs.iter().filter_map(|m| if let Msg::Bin(b) = m { Some(b.clone()) } else { None }).next().unwrap();
    assert!(replay.len() <= RING && replay.len() >= RING - 3, "replay {}", replay.len());
    assert!(strip_csi(&replay).contains("FLOOD-END"));
    b.send_bytes(b"q");
    b.text("exit");
}

#[test]
fn kill_while_paused_still_delivers_exit_after_the_backlog() {
    let mut core = Core::start();
    let cwd = temp_dir("bpkill");
    let progress = std::path::PathBuf::from(&cwd).join("progress.txt");
    let _ = std::fs::remove_file(&progress);
    let total = 6 * 1024 * 1024;
    let mut c = Client::connect(core.port);
    let id = "6f1c1d2e-0000-4000-8000-0000000000d3";
    spawn_with(&mut core, &mut c, id, &cwd, 200, &["-e", FLOOD, progress.to_str().unwrap(), &total.to_string()]);
    c.out_has("flood-ready", 10_000);
    c.send_bytes(b"g");
    let stalled = wait_progress_stall(&progress, 20_000);
    assert!(stalled < total as u64);
    let mut k = Client::connect(core.port);
    k.send(json!({ "t": "kill", "id": id }));
    core.next(|m| m["t"] == "removed" && m["id"] == id, 5_000);
    let exit = c.until(30_000, |c| c.texts.iter().find(|m| m["t"] == "exit").cloned());
    #[cfg(windows)]
    assert_eq!(exit["code"], 1);
    #[cfg(unix)]
    assert_eq!(exit["code"], 0);
    assert!(!c.output.is_empty());
    core.send(json!({ "t": "census" }));
    let census = core.next(|m| m["t"] == "census-reply", 5_000);
    assert_eq!(census["sessions"], json!([]));
}

#[test]
fn flood_without_any_socket_is_read_into_the_ring_and_replayed() {
    let mut core = Core::start();
    let cwd = temp_dir("nosock");
    let progress = std::path::PathBuf::from(&cwd).join("progress.txt");
    let _ = std::fs::remove_file(&progress);
    let total = 3 * 1024 * 1024;
    let mut c = Client::connect(core.port);
    let id = "6f1c1d2e-0000-4000-8000-0000000000d4";
    spawn_with(&mut core, &mut c, id, &cwd, 200, &["-e", FLOOD, progress.to_str().unwrap(), &total.to_string(), "auto"]);
    c.out_has("flood-ready", 10_000);
    drop(c);
    let end = Instant::now() + Duration::from_secs(60);
    while !read_progress(&progress).starts_with("done") {
        assert!(Instant::now() < end, "child blocked with no socket: {}", read_progress(&progress));
        std::thread::sleep(Duration::from_millis(100));
    }
    std::thread::sleep(Duration::from_millis(500));
    let mut b = Client::connect(core.port);
    b.send(json!({ "t": "attach", "id": id }));
    let msgs = read_ordered(&mut b, 10_000, |m| m.iter().any(|x| matches!(x, Msg::Bin(_))));
    let replay = msgs.iter().find_map(|m| if let Msg::Bin(b) = m { Some(b.clone()) } else { None }).unwrap();
    let text = strip_csi(&replay);
    let last = (total as u64).div_ceil(120) - 1;
    assert!(text.contains(&format!("L{last:07}")), "tail line missing");
    assert!(text.contains("FLOOD-END"));
    b.send_bytes(b"q");
    b.text("exit");
}

const MULTI: &str = "if(process.stdin.isTTY)process.stdin.setRawMode(true);let s='';for(let i=0;i<20000;i++)s+='R'+String(i).padStart(5,'0')+' \\u00e9\\u4e2d\\u{1F600} abc\\n';process.stdout.write(s);process.stdout.write('MULTI-END\\n');process.stdin.on('data',(d)=>{process.stdout.write('typed:'+d);if(String(d).includes('q'))process.exit(0)})";

#[test]
fn replay_over_256k_is_the_exact_tail_on_a_utf8_boundary_and_replace_closes_4000() {
    let mut core = Core::start();
    let cwd = temp_dir("ring");
    let mut a = Client::connect(core.port);
    let id = "6f1c1d2e-0000-4000-8000-0000000000d5";
    spawn_with(&mut core, &mut a, id, &cwd, 200, &["-e", MULTI]);
    a.out_has("MULTI-END", 30_000);
    let mut quiet = a.output.len();
    loop {
        std::thread::sleep(Duration::from_millis(400));
        a.pump();
        if a.output.len() == quiet {
            break;
        }
        quiet = a.output.len();
    }
    assert!(a.output.len() > RING, "live output only {} bytes", a.output.len());
    let mut b = Client::connect(core.port);
    b.send(json!({ "t": "attach", "id": id }));
    let msgs = read_ordered(&mut b, 10_000, |m| m.iter().any(|x| matches!(x, Msg::Bin(_))));
    assert!(matches!(&msgs[0], Msg::Text(v) if v["t"] == "ready" && v["replay"] == true));
    let replay = msgs.iter().find_map(|m| if let Msg::Bin(b) = m { Some(b.clone()) } else { None }).unwrap();
    assert!(replay.len() <= RING && replay.len() >= RING - 3, "replay len {}", replay.len());
    assert_ne!(replay[0] & 0xC0, 0x80, "replay starts inside a UTF-8 character");
    assert!(a.output.ends_with(&replay), "replay is not the exact tail of the live stream");
    a.until(5_000, |a| (a.close == Some(4000)).then_some(()));
    b.send_bytes(b"x");
    let more = read_ordered(&mut b, 10_000, |m| m.iter().any(|x| matches!(x, Msg::Bin(d) if String::from_utf8_lossy(d).contains("typed:x"))));
    assert!(!more.is_empty());
    b.send_bytes(b"q");
    read_ordered(&mut b, 10_000, |m| m.iter().any(|x| matches!(x, Msg::Text(v) if v["t"] == "exit")));
}

#[test]
fn attach_after_exit_orders_ready_cwd_replay_branch_exit() {
    let mut core = Core::start();
    let cwd = temp_dir("order");
    let mut a = Client::connect(core.port);
    let id = "6f1c1d2e-0000-4000-8000-0000000000d6";
    let script = "if(process.stdin.isTTY)process.stdin.setRawMode(true);process.stdout.write('hi from child\\n');process.stdin.on('data',(d)=>{process.stdout.write('got:'+d+'\\n');if(String(d).includes('q'))setTimeout(()=>process.exit(3),50)})";
    spawn_with(&mut core, &mut a, id, &cwd, 100, &["-e", script]);
    a.out_has("hi from child", 10_000);
    a.send(json!({ "t": "cwd", "cwd": cwd }));
    let ask = core.next(|m| m["t"] == "cwd", 5_000);
    let sub = format!("{cwd}/sub");
    core.send(json!({ "t": "cwd-ok", "rid": ask["rid"], "cwd": sub }));
    a.text("cwd");
    core.send(json!({ "t": "branch", "id": id, "branch": "main" }));
    a.text("branch");
    a.send_bytes(b"q");
    a.text("exit");
    core.next(|m| m["t"] == "exited" && m["id"] == id, 5_000);
    let mut b = Client::connect(core.port);
    b.send(json!({ "t": "attach", "id": id }));
    let msgs = read_ordered(&mut b, 10_000, |m| m.iter().any(|x| matches!(x, Msg::Text(v) if v["t"] == "exit")));
    let kinds: Vec<String> = msgs.iter().map(kind).map(|k| if k.starts_with("bin(") { "bin".into() } else { k }).collect();
    assert_eq!(kinds, ["ready", "cwd", "bin", "branch", "exit"]);
    let Msg::Bin(replay) = &msgs[2] else { unreachable!() };
    assert_eq!(replay, &a.output, "replay after exit equals everything the first socket saw");
    let Msg::Text(exit) = &msgs[4] else { unreachable!() };
    assert_eq!(exit["code"], 3);
    a.until(5_000, |a| (a.close == Some(4000)).then_some(()));
}

fn cheap_pane() -> (String, Vec<String>) {
    #[cfg(windows)]
    return ("cmd.exe".into(), vec!["/d".into(), "/q".into(), "/k".into()]);
    #[cfg(unix)]
    return ("/bin/sh".into(), vec!["-c".into(), "sleep 120".into()]);
}

#[test]
fn live_limit_16_refuses_the_17th_with_limit_until_one_exits() {
    let mut core = Core::start();
    let cwd = temp_dir("limit");
    let (file, args) = cheap_pane();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let mut clients = Vec::new();
    for i in 0..16 {
        let mut c = Client::connect(core.port);
        c.send(json!({ "t": "spawn", "preset": "shell", "cols": 80, "rows": 24, "cwd": cwd }));
        let req = core.next(|m| m["t"] == "resolve", 5_000);
        core.resolved(&req["rid"], &format!("6f1c1d2e-0000-4000-8000-0000000001{i:02}"), &file, &args, &cwd);
        c.text("ready");
        clients.push(c);
    }
    let mut x = Client::connect(core.port);
    x.send(json!({ "t": "spawn", "preset": "shell", "cols": 80, "rows": 24, "cwd": cwd }));
    let req = core.next(|m| m["t"] == "resolve", 5_000);
    let over = "6f1c1d2e-0000-4000-8000-000000000199";
    core.resolved(&req["rid"], over, &file, &args, &cwd);
    assert_eq!(x.text("error"), json!({ "t": "error", "message": "limit" }));
    core.next(|m| m["t"] == "removed" && m["id"] == over, 5_000);
    assert!(!core.seen(|m| m["t"] == "spawned" && m["id"] == over));

    clients[0].send(json!({ "t": "kill" }));
    core.next(|m| m["t"] == "removed" && m["id"] == "6f1c1d2e-0000-4000-8000-000000000100", 5_000);
    clients[0].text("exit");
    x.texts.clear();
    x.send(json!({ "t": "spawn", "preset": "shell", "cols": 80, "rows": 24, "cwd": cwd }));
    let req = core.next(|m| m["t"] == "resolve", 5_000);
    core.resolved(&req["rid"], over, &file, &args, &cwd);
    assert_eq!(x.text("ready")["id"], over);
    core.send(json!({ "t": "kill-all" }));
    for _ in 0..16 {
        core.next(|m| m["t"] == "removed", 5_000);
    }
}

const TREE: &str = "const {spawn}=require('child_process');const c=spawn(process.execPath,['-e',\"const {spawn}=require('child_process');const g=spawn(process.execPath,['-e','setInterval(()=>{},1000)'],{stdio:'ignore'});console.log('gc='+g.pid);setInterval(()=>{},1000)\"],{stdio:['ignore','pipe','ignore']});c.stdout.on('data',(d)=>{const m=/gc=(\\d+)/.exec(String(d));if(m)process.stdout.write('tree:'+c.pid+':'+m[1]+';\\n')});setInterval(()=>{},1000)";

#[test]
fn owners_resolves_three_levels_and_ignores_unrelated_pids() {
    let mut core = Core::start();
    let cwd = temp_dir("owners");
    let mut c = Client::connect(core.port);
    let mut q = Client::connect(core.port);
    q.send(json!({ "t": "owners", "pids": [std::process::id()] }));
    assert_eq!(q.text("owners"), json!({ "t": "owners", "owners": {} }));
    q.texts.clear();
    let id = "6f1c1d2e-0000-4000-8000-0000000000d7";
    let pane = spawn_with(&mut core, &mut c, id, &cwd, 100, &["-e", TREE]);
    let (child, gc) = c.until(15_000, |c| {
        let s = c.out_str();
        let i = s.find("tree:")?;
        let rest = &s[i + 5..];
        let end = rest.find(';')?;
        let mut parts = rest[..end].split(':');
        Some((parts.next()?.parse::<u64>().ok()?, parts.next()?.parse::<u64>().ok()?))
    });
    q.send(json!({ "t": "owners", "pids": [gc, child, pane, std::process::id(), 0, -1, "x", 2.5] }));
    let got = q.text("owners");
    let mut want = serde_json::Map::new();
    want.insert(gc.to_string(), json!(id));
    want.insert(child.to_string(), json!(id));
    want.insert(pane.to_string(), json!(id));
    assert_eq!(got["owners"], Value::Object(want));
    q.texts.clear();
    q.send(json!({ "t": "owners", "pids": [] }));
    assert_eq!(q.text("owners")["owners"], json!({}));
    c.send(json!({ "t": "kill" }));
    core.next(|m| m["t"] == "removed" && m["id"] == id, 5_000);
    assert!(wait_dead(gc, 5_000), "grandchild survived pane kill");
}

#[test]
#[ignore]
fn measure_rss_with_8_shell_panes() {
    let mut core = Core::start();
    let pid = core.child.id();
    let cwd = temp_dir("rss8");
    std::thread::sleep(Duration::from_millis(500));
    let idle = rss_kb(pid);
    let (file, args) = cheap_pane();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let mut clients = Vec::new();
    for i in 0..8 {
        let mut c = Client::connect(core.port);
        c.send(json!({ "t": "spawn", "preset": "shell", "cols": 120, "rows": 40, "cwd": cwd }));
        let req = core.next(|m| m["t"] == "resolve", 5_000);
        core.resolved(&req["rid"], &format!("6f1c1d2e-0000-4000-8000-0000000002{i:02}"), &file, &args, &cwd);
        c.text("ready");
        clients.push(c);
    }
    std::thread::sleep(Duration::from_secs(3));
    for c in clients.iter_mut() {
        c.send_bytes(b"echo hi\r");
    }
    std::thread::sleep(Duration::from_secs(2));
    let eight = rss_kb(pid);
    eprintln!("MEASURE rss orrery-core: idle={idle} KB, 8 panes={eight} KB");
    core.send(json!({ "t": "kill-all" }));
}

#[test]
#[ignore]
fn measure_echo_p50_over_200_bytes() {
    let mut core = Core::start();
    let cwd = temp_dir("echo200");
    let mut c = Client::connect(core.port);
    c.ws.get_ref().set_read_timeout(Some(Duration::from_millis(1))).unwrap();
    let id = "6f1c1d2e-0000-4000-8000-0000000000d8";
    let script = "process.stdin.setRawMode(true);process.stdout.write('echo-ready\\n');process.stdin.on('data',(d)=>process.stdout.write(d))";
    spawn_with(&mut core, &mut c, id, &cwd, 100, &["-e", script]);
    c.out_has("echo-ready", 10_000);
    std::thread::sleep(Duration::from_millis(500));
    c.pump();
    let mut times = Vec::new();
    for i in 0..200u32 {
        let byte = b'a' + (i % 26) as u8;
        c.output.clear();
        let t = Instant::now();
        c.send_bytes(&[byte]);
        c.until(5_000, |c| c.output.contains(&byte).then_some(()));
        times.push(t.elapsed().as_secs_f64() * 1000.0);
        std::thread::sleep(Duration::from_millis(5));
    }
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    eprintln!("MEASURE echo 1 byte over WS (n=200): p50={:.2} ms p90={:.2} ms max={:.2} ms", times[100], times[180], times[199]);
    c.send(json!({ "t": "kill" }));
}
