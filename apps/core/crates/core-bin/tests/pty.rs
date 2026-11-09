mod common;

use common::*;
use serde_json::json;
use std::time::{Duration, Instant};

const ECHO: &str = "process.stdout.write('hi from child\\n');process.stdin.on('data',(d)=>{process.stdout.write('got:'+d);if(String(d).includes('q'))process.exit(3)})";
const SIZED: &str = "process.stdin.setRawMode(true);process.stdout.write('hi from child\\n');process.stdin.on('data',(d)=>{const s=String(d);process.stdout.write('got:'+s);if(s.includes('r'))process.stdout.write('size:'+process.stdout.getWindowSize().join('x')+';');if(s.includes('q'))process.exit(3)})";

fn spawn_pane(core: &mut Core, c: &mut Client, id: &str, cwd: &str, script: &str) {
    c.send(json!({ "t": "spawn", "preset": "shell", "cols": 100, "rows": 30, "cwd": cwd }));
    let req = core.next(|m| m["t"] == "resolve", 5_000);
    assert_eq!(req["msg"], json!({ "t": "spawn", "preset": "shell", "cols": 100, "rows": 30, "cwd": cwd }));
    core.resolved(&req["rid"], id, &node(), &["-e", script], cwd);
}

#[test]
fn handshake_refuses_wrong_host_origin_token_with_403() {
    let core = Core::start();
    let good = format!("ccm.pty.{TOKEN}");
    let host = format!("127.0.0.1:{}", core.port);
    assert!(handshake_status(core.port, &host, Some(ORIGIN), &good).contains(" 101 "));
    assert!(handshake_status(core.port, &host, Some(ORIGIN), "ccm.pty.nope").contains(" 403 "));
    assert!(handshake_status(core.port, &host, Some(ORIGIN), &format!("ccm.pty.{}", &TOKEN[1..])).contains(" 403 "));
    assert!(handshake_status(core.port, &host, Some("http://evil.test"), &good).contains(" 403 "));
    assert!(handshake_status(core.port, &host, None, &good).contains(" 403 "));
    assert!(handshake_status(core.port, &format!("localhost:{}", core.port), Some(ORIGIN), &good).contains(" 403 "));
    let c = Client::connect(core.port);
    drop(c);
}

#[test]
fn spawn_queues_input_until_resolved_then_echo_resize_and_exit_code() {
    let mut core = Core::start();
    let cwd = temp_dir("echo");
    let mut c = Client::connect(core.port);
    let id = "6f1c1d2e-0000-4000-8000-000000000001";
    c.send(json!({ "t": "spawn", "preset": "shell", "cols": 100, "rows": 30, "cwd": cwd }));
    let req = core.next(|m| m["t"] == "resolve", 5_000);
    c.send(json!({ "t": "resize", "cols": 90, "rows": 20 }));
    std::thread::sleep(Duration::from_millis(150));
    core.resolved(&req["rid"], id, &node(), &["-e", SIZED], &cwd);
    let ready = c.text("ready");
    assert_eq!(ready["id"], id);
    assert_eq!(ready["preset"], "shell");
    assert_eq!(ready["cwd"], cwd.as_str());
    assert_eq!(ready["replay"], false);
    assert!(ready.get("si").is_none());
    let pid = ready["pid"].as_u64().unwrap();
    assert!(pid > 0);
    let spawned = core.next(|m| m["t"] == "spawned" && m["id"] == id, 5_000);
    assert_eq!(spawned["pid"].as_u64(), Some(pid));
    c.out_has("hi from child", 10_000);
    c.send_bytes(b"r\r");
    c.out_has("size:90x20;", 10_000);
    c.send(json!({ "t": "resize", "cols": 120, "rows": 40 }));
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        std::thread::sleep(Duration::from_millis(150));
        c.send_bytes(b"r");
        c.pump();
        if c.out_str().contains("size:120x40;") {
            break;
        }
        assert!(Instant::now() < end, "resize not seen: {:?}", c.out_str());
    }
    c.send_bytes(b"q\r");
    let exit = c.text("exit");
    assert_eq!(exit["code"], 3);
    assert!(c.out_str().contains("got:q"));
    assert_eq!(core.next(|m| m["t"] == "exited" && m["id"] == id, 5_000), json!({ "t": "exited", "id": id, "code": 3 }));
    let pos_exit = c.texts.iter().position(|m| m["t"] == "exit").unwrap();
    assert_eq!(pos_exit, c.texts.len() - 1);

    let mut again = Client::connect(core.port);
    again.send(json!({ "t": "attach", "id": id }));
    let r = again.text("ready");
    assert_eq!(r["replay"], true);
    assert_eq!(again.text("exit")["code"], 3);
    c.until(5_000, |c| (c.close == Some(4000)).then_some(()));
}

#[test]
fn output_escapes_pass_through_untouched() {
    let mut core = Core::start();
    let cwd = temp_dir("osc");
    let mut c = Client::connect(core.port);
    let id = "6f1c1d2e-0000-4000-8000-0000000000e1";
    let script = "process.stdout.write('A\\x1b]633;E;ls -la;n0nce\\x07B\\x1b]7;file://host/tmp/x\\x07C\\u00e9\\u4e2d\\u{1F600}D\\n');setTimeout(()=>process.exit(0),300)";
    spawn_pane(&mut core, &mut c, id, &cwd, script);
    c.text("ready");
    c.text("exit");
    let out = c.out_str();
    assert!(out.contains("\x1b]633;E;ls -la;n0nce\x07"), "633;E lost: {out:?}");
    assert!(out.contains("\x1b]7;file://host/tmp/x\x07"), "OSC 7 lost: {out:?}");
    assert!(out.contains("C\u{e9}\u{4e2d}\u{1F600}D"), "utf-8 changed: {out:?}");
}

#[test]
fn input_bytes_reach_the_child_unchanged() {
    let mut core = Core::start();
    let cwd = temp_dir("input");
    let mut c = Client::connect(core.port);
    let id = "6f1c1d2e-0000-4000-8000-0000000000e2";
    let script = "process.stdin.setRawMode(true);process.stdout.write('raw\\n');let all='';process.stdin.on('data',(d)=>{all+=d.toString('hex');if(all.endsWith('7a7a'))process.stdout.write('hex:'+all+';')})";
    spawn_pane(&mut core, &mut c, id, &cwd, script);
    c.text("ready");
    c.out_has("raw", 10_000);
    let input = "ab\u{e9}\u{4e2d}\x1b[Azz".as_bytes();
    c.send_bytes(input);
    let hex: String = input.iter().map(|b| format!("{b:02x}")).collect();
    c.out_has(&format!("hex:{hex};"), 10_000);
    c.send(json!({ "t": "kill" }));
    core.next(|m| m["t"] == "removed" && m["id"] == id, 5_000);
}

#[test]
fn refused_resolve_error_kinds_then_spawn_again_and_unknown_attach_is_gone() {
    let mut core = Core::start();
    let cwd = temp_dir("refuse");
    let mut c = Client::connect(core.port);
    c.send(json!({ "t": "attach", "id": "nope" }));
    c.text("gone");
    for kind in ["preset", "cwd", "limit", "spawn"] {
        c.texts.clear();
        c.send(json!({ "t": "spawn", "preset": "shell", "cols": 80, "rows": 24 }));
        let req = core.next(|m| m["t"] == "resolve", 5_000);
        core.send(json!({ "t": "resolved", "rid": req["rid"], "ok": false, "message": kind }));
        assert_eq!(c.text("error"), json!({ "t": "error", "message": kind }));
    }
    c.texts.clear();
    c.send(json!({ "t": "spawn", "preset": "shell", "cols": 80, "rows": 24 }));
    let req = core.next(|m| m["t"] == "resolve", 5_000);
    let missing = format!("{cwd}-missing");
    core.resolved(&req["rid"], "6f1c1d2e-0000-4000-8000-0000000000b0", &node(), &["-e", "1"], &missing);
    assert_eq!(c.text("error"), json!({ "t": "error", "message": "spawn" }));
    core.next(|m| m["t"] == "removed" && m["id"] == "6f1c1d2e-0000-4000-8000-0000000000b0", 5_000);
    c.texts.clear();
    let id = "6f1c1d2e-0000-4000-8000-000000000002";
    spawn_pane(&mut core, &mut c, id, &cwd, ECHO);
    assert_eq!(c.text("ready")["id"], id);
    core.next(|m| m["t"] == "spawned" && m["id"] == id, 5_000);
    c.send(json!({ "t": "kill" }));
    core.next(|m| m["t"] == "removed" && m["id"] == id, 5_000);
}

#[test]
fn cwd_branch_wt_relay_census_attach_order_and_kill_by_id() {
    let mut core = Core::start();
    let cwd = temp_dir("relay");
    let mut c = Client::connect(core.port);
    let id = "6f1c1d2e-0000-4000-8000-000000000003";
    spawn_pane(&mut core, &mut c, id, &cwd, ECHO);
    c.text("ready");
    core.next(|m| m["t"] == "spawned" && m["id"] == id, 5_000);

    c.send(json!({ "t": "cwd", "cwd": cwd }));
    let ask = core.next(|m| m["t"] == "cwd", 5_000);
    assert_eq!(ask["id"], id);
    assert_eq!(ask["cwd"], cwd.as_str());
    let sub = format!("{cwd}/sub");
    core.send(json!({ "t": "cwd-ok", "rid": ask["rid"], "cwd": sub }));
    assert_eq!(c.text("cwd"), json!({ "t": "cwd", "cwd": sub }));
    c.send(json!({ "t": "cwd", "cwd": 42 }));
    c.until(5_000, |c| (c.texts.iter().filter(|m| m["t"] == "cwd").count() == 2).then_some(()));
    assert_eq!(c.texts.iter().filter(|m| m["t"] == "cwd").nth(1).unwrap(), &json!({ "t": "cwd", "cwd": null }));

    core.send(json!({ "t": "branch", "id": id, "branch": "main" }));
    assert_eq!(c.text("branch"), json!({ "t": "branch", "branch": "main" }));

    c.send(json!({ "t": "wt.list", "cwd": cwd }));
    let wt = core.next(|m| m["t"] == "wt", 5_000);
    assert_eq!(wt["msg"], json!({ "t": "wt.list", "cwd": cwd }));
    core.send(json!({ "t": "wt-reply", "rid": wt["rid"], "reply": { "t": "wt.list", "repo": null, "worktrees": [] } }));
    assert_eq!(c.text("wt.list"), json!({ "t": "wt.list", "repo": null, "worktrees": [] }));

    core.send(json!({ "t": "census" }));
    let census = core.next(|m| m["t"] == "census-reply", 5_000);
    let row = census["sessions"].as_array().unwrap().iter().find(|s| s["id"] == id).unwrap().clone();
    assert_eq!(row["preset"], "shell");
    assert_eq!(row["cwd"], cwd.as_str());
    assert_eq!(row["liveCwd"], sub.as_str());
    assert_eq!(row["alive"], true);

    let mut late = Client::connect(core.port);
    late.send(json!({ "t": "attach", "id": id }));
    late.until(5_000, |l| (l.texts.len() >= 3).then_some(()));
    let kinds: Vec<_> = late.texts.iter().map(|m| m["t"].as_str().unwrap().to_owned()).collect();
    assert_eq!(kinds, ["ready", "cwd", "branch"]);
    c.until(5_000, |c| (c.close == Some(4000)).then_some(()));

    let mut killer = Client::connect(core.port);
    killer.send(json!({ "t": "kill", "id": id }));
    assert_eq!(core.next(|m| m["t"] == "removed" && m["id"] == id, 5_000), json!({ "t": "removed", "id": id }));
    let code = late.text("exit")["code"].as_i64().unwrap();
    #[cfg(windows)]
    assert_eq!(code, 1);
    #[cfg(unix)]
    assert_eq!(code, 0);
    core.send(json!({ "t": "census" }));
    let after = core.next(|m| m["t"] == "census-reply", 5_000);
    assert!(!after["sessions"].as_array().unwrap().iter().any(|s| s["id"] == id));
    assert!(!core.seen(|m| m["t"] == "exited" && m["id"] == id));
}

const PARENT: &str = "const {spawn}=require('child_process');const g=spawn(process.execPath,['-e','setInterval(()=>{},1000)'],{detached:process.platform==='win32',stdio:'ignore'});g.unref();process.stdout.write('grandchild:'+g.pid+';\\n');setInterval(()=>{},1000)";

fn grandchild_pid(c: &mut Client) -> u64 {
    c.until(10_000, |c| {
        let s = c.out_str();
        let i = s.find("grandchild:")?;
        let rest = &s[i + 11..];
        let end = rest.find(';')?;
        rest[..end].parse::<u64>().ok()
    })
}

#[test]
fn kill_pane_kills_grandchild() {
    let mut core = Core::start();
    let cwd = temp_dir("tree");
    let mut c = Client::connect(core.port);
    let id = "6f1c1d2e-0000-4000-8000-0000000000c1";
    spawn_pane(&mut core, &mut c, id, &cwd, PARENT);
    let pid = c.text("ready")["pid"].as_u64().unwrap();
    let g = grandchild_pid(&mut c);
    assert!(pid_alive(g));
    c.send(json!({ "t": "kill" }));
    core.next(|m| m["t"] == "removed" && m["id"] == id, 5_000);
    let gone = wait_dead(g, 5_000);
    if !gone {
        kill_pid(g);
    }
    assert!(gone, "grandchild {g} survived the pane kill");
    assert!(wait_dead(pid, 5_000));
}

#[cfg(windows)]
#[test]
fn core_death_kills_panes_and_grandchildren() {
    let mut core = Core::start();
    let cwd = temp_dir("death");
    let mut c = Client::connect(core.port);
    let id = "6f1c1d2e-0000-4000-8000-0000000000c2";
    spawn_pane(&mut core, &mut c, id, &cwd, PARENT);
    let pid = c.text("ready")["pid"].as_u64().unwrap();
    let g = grandchild_pid(&mut c);
    core.child.kill().unwrap();
    let _ = core.child.wait();
    let gone = wait_dead(g, 5_000) && wait_dead(pid, 5_000);
    if !gone {
        kill_pid(g);
        kill_pid(pid);
    }
    assert!(gone, "pane {pid} or grandchild {g} survived core death");
}

#[test]
fn shutdown_kills_panes_refuses_pending_and_late_spawns_then_exits_zero() {
    let mut core = Core::start();
    let cwd = temp_dir("shutdown");
    let mut a = Client::connect(core.port);
    let live = "6f1c1d2e-0000-4000-8000-000000000005";
    spawn_pane(&mut core, &mut a, live, &cwd, PARENT);
    let pid = a.text("ready")["pid"].as_u64().unwrap();
    let g = grandchild_pid(&mut a);
    core.next(|m| m["t"] == "spawned" && m["id"] == live, 5_000);
    let mut b = Client::connect(core.port);
    b.send(json!({ "t": "spawn", "preset": "shell", "cols": 80, "rows": 24 }));
    let req = core.next(|m| m["t"] == "resolve", 5_000);
    core.send(json!({ "t": "shutdown" }));
    let late = "6f1c1d2e-0000-4000-8000-000000000006";
    core.resolved(&req["rid"], late, &node(), &["-e", "1"], &cwd);
    assert_eq!(core.next(|m| m["t"] == "removed" && m["id"] == late, 2_000), json!({ "t": "removed", "id": late }));
    assert_eq!(b.text("error"), json!({ "t": "error", "message": "spawn" }));
    assert_eq!(core.next(|m| m["t"] == "removed" && m["id"] == live, 2_000), json!({ "t": "removed", "id": live }));
    assert!(!core.seen(|m| m["t"] == "spawned"));
    assert_eq!(core.wait_exit(5_000).code(), Some(0));
    let gone = wait_dead(g, 5_000) && wait_dead(pid, 5_000);
    if !gone {
        kill_pid(g);
        kill_pid(pid);
    }
    assert!(gone);
}

#[test]
fn stdin_eof_shuts_down_and_kills_live_panes() {
    let mut core = Core::start();
    let cwd = temp_dir("eof");
    let mut c = Client::connect(core.port);
    let id = "6f1c1d2e-0000-4000-8000-000000000004";
    spawn_pane(&mut core, &mut c, id, &cwd, ECHO);
    let pid = c.text("ready")["pid"].as_u64().unwrap();
    core.next(|m| m["t"] == "spawned" && m["id"] == id, 5_000);
    core.close_stdin();
    assert_eq!(core.wait_exit(5_000).code(), Some(0));
    assert_eq!(core.next(|m| m["t"] == "removed" && m["id"] == id, 1_000), json!({ "t": "removed", "id": id }));
    assert!(wait_dead(pid, 5_000));
}

#[test]
fn kill_all_removes_every_session() {
    let mut core = Core::start();
    let cwd = temp_dir("killall");
    let ids = ["6f1c1d2e-0000-4000-8000-0000000000a1", "6f1c1d2e-0000-4000-8000-0000000000a2"];
    let mut clients = Vec::new();
    for id in ids {
        let mut c = Client::connect(core.port);
        spawn_pane(&mut core, &mut c, id, &cwd, ECHO);
        c.text("ready");
        clients.push(c);
    }
    core.send(json!({ "t": "kill-all" }));
    for id in ids {
        core.next(|m| m["t"] == "removed" && m["id"] == id, 5_000);
    }
    for c in clients.iter_mut() {
        c.text("exit");
    }
    core.send(json!({ "t": "census" }));
    let census = core.next(|m| m["t"] == "census-reply", 5_000);
    assert_eq!(census["sessions"], json!([]));
}

#[test]
fn unanswered_resolve_errors_after_15s_and_late_answer_is_removed() {
    let mut core = Core::start();
    let cwd = temp_dir("timeout");
    let mut c = Client::connect(core.port);
    c.send(json!({ "t": "spawn", "preset": "shell", "cols": 80, "rows": 24 }));
    let req = core.next(|m| m["t"] == "resolve", 5_000);
    let t0 = Instant::now();
    assert_eq!(c.until(20_000, |c| c.texts.iter().find(|m| m["t"] == "error").cloned()), json!({ "t": "error", "message": "spawn" }));
    assert!(t0.elapsed() >= Duration::from_millis(14_000));
    let id = "6f1c1d2e-0000-4000-8000-000000000007";
    core.resolved(&req["rid"], id, &node(), &["-e", "1"], &cwd);
    core.next(|m| m["t"] == "removed" && m["id"] == id, 5_000);
}

#[test]
fn input_queued_while_spawning_is_capped_at_4mb() {
    let mut core = Core::start();
    let mut c = Client::connect(core.port);
    c.send(json!({ "t": "spawn", "preset": "shell", "cols": 80, "rows": 24 }));
    let req = core.next(|m| m["t"] == "resolve", 5_000);
    let chunk = vec![0x78u8; 1_000_000];
    for _ in 0..5 {
        c.send_bytes(&chunk);
    }
    c.send(json!({ "t": "attach", "id": "nope" }));
    std::thread::sleep(Duration::from_millis(300));
    core.send(json!({ "t": "resolved", "rid": req["rid"], "ok": false, "message": "cwd" }));
    assert_eq!(c.text("error"), json!({ "t": "error", "message": "cwd" }));
    assert_eq!(c.text("gone"), json!({ "t": "gone" }));
    let end = Instant::now() + Duration::from_secs(5);
    while !core.stderr.lock().unwrap().contains("pty input dropped while spawning: 1 frames") {
        assert!(Instant::now() < end, "drop not logged");
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn logs_carry_no_token_env_or_terminal_content() {
    let mut core = Core::start();
    let cwd = temp_dir("logs");
    let mut c = Client::connect(core.port);
    let id = "6f1c1d2e-0000-4000-8000-0000000000d1";
    spawn_pane(&mut core, &mut c, id, &cwd, "process.stdout.write('SECRET-OUTPUT-42');process.exit(5)");
    c.text("exit");
    core.send(json!({ "t": "shutdown" }));
    core.wait_exit(5_000);
    std::thread::sleep(Duration::from_millis(100));
    let log = core.stderr.lock().unwrap().clone();
    assert!(!log.contains(TOKEN));
    assert!(!log.contains("SECRET-OUTPUT-42"));
    assert!(!log.contains(&"c".repeat(64)));
    assert!(log.contains("pty early exit: preset=shell code=5"));
    for line in log.lines() {
        assert!(line.len() < 300, "long log line: {line}");
    }
}

#[test]
fn socket_closed_while_resolving_gets_removed_and_instant_exit_still_reports() {
    let mut core = Core::start();
    let cwd = temp_dir("cancel");
    let mut c = Client::connect(core.port);
    c.send(json!({ "t": "spawn", "preset": "shell", "cols": 80, "rows": 24 }));
    let req = core.next(|m| m["t"] == "resolve", 5_000);
    c.ws.close(None).unwrap();
    c.until(5_000, |c| c.close.map(|_| ()));
    std::thread::sleep(Duration::from_millis(200));
    let id = "6f1c1d2e-0000-4000-8000-0000000000f1";
    core.resolved(&req["rid"], id, &node(), &["-e", "1"], &cwd);
    core.next(|m| m["t"] == "removed" && m["id"] == id, 5_000);
    assert!(!core.seen(|m| m["t"] == "spawned"));

    let mut d = Client::connect(core.port);
    let quick = "6f1c1d2e-0000-4000-8000-0000000000f2";
    spawn_pane(&mut core, &mut d, quick, &cwd, "process.stdout.write('bye');process.exit(7)");
    std::thread::sleep(Duration::from_millis(1_500));
    let kinds: Vec<String> = d.until(5_000, |d| d.texts.iter().any(|m| m["t"] == "exit").then(|| d.texts.iter().map(|m| m["t"].as_str().unwrap().to_owned()).collect()));
    assert_eq!(kinds, ["ready", "exit"]);
    assert_eq!(d.text("exit")["code"], 7);
    assert!(d.out_str().contains("bye"));
    assert_eq!(core.next(|m| m["t"] == "exited" && m["id"] == quick, 5_000)["code"], 7);
    d.send_bytes(b"ignored after exit");
    d.send(json!({ "t": "resize", "cols": 10, "rows": 10 }));
    d.send(json!({ "t": "kill" }));
    core.next(|m| m["t"] == "removed" && m["id"] == quick, 5_000);
}

#[test]
fn silent_connection_is_dropped_after_handshake_timeout() {
    use std::io::Read;
    let core = Core::start();
    let mut s = std::net::TcpStream::connect(("127.0.0.1", core.port)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let t0 = Instant::now();
    let mut buf = [0u8; 16];
    let n = s.read(&mut buf).unwrap_or(0);
    assert_eq!(n, 0);
    let waited = t0.elapsed();
    assert!(waited >= Duration::from_millis(4_500) && waited < Duration::from_secs(9), "dropped after {waited:?}");
}

#[cfg(unix)]
#[test]
fn shutdown_hangs_up_every_group_once_then_kills_survivors() {
    let mut core = Core::start();
    let cwd = temp_dir("hupall");
    let stubborn = "process.on('SIGHUP',()=>{});process.stdout.write('armed\n');setInterval(()=>{},1000)";
    let mut pids = Vec::new();
    let mut clients = Vec::new();
    for i in 0..4 {
        let mut c = Client::connect(core.port);
        let id = format!("6f1c1d2e-0000-4000-8000-0000000001{i:02}");
        spawn_pane(&mut core, &mut c, &id, &cwd, stubborn);
        pids.push(c.text("ready")["pid"].as_u64().unwrap());
        c.out_has("armed", 10_000);
        clients.push(c);
    }
    let t0 = Instant::now();
    core.send(json!({ "t": "shutdown" }));
    assert_eq!(core.wait_exit(5_000).code(), Some(0));
    assert!(t0.elapsed() < Duration::from_millis(900), "shutdown took {:?}", t0.elapsed());
    for pid in pids {
        let gone = wait_dead(pid, 3_000);
        if !gone {
            kill_pid(pid);
        }
        assert!(gone, "pane {pid} survived shutdown");
    }
}
