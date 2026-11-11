mod common;

use common::*;
use serde_json::json;
use std::time::{Duration, Instant};

const PANE: &str = "process.stdin.setRawMode(true);process.stdout.write('hi from child\\n');process.stdin.on('data',(d)=>{const s=String(d);if(s.includes('r'))process.stdout.write('size:'+process.stdout.getWindowSize().join('x')+';');if(s.includes('b')){let o='BEGIN';for(let i=0;i<3000;i++)o+='row'+i+' ';process.stdout.write(o+'END\\n')}if(s.includes('q'))process.exit(3)})";

const FLOOD: &str = "if(process.stdin.isTTY)process.stdin.setRawMode(true);const fs=require('fs');const p=process.argv[1];const TOTAL=Number(process.argv[2]);const line=(n)=>'L'+String(n).padStart(7,'0')+' '+'y'.repeat(110)+'\\n';let sent=0,n=0;const pump=()=>{let batch=0;while(batch<65536&&sent<TOTAL){const l=line(n++);sent+=l.length;batch+=l.length;process.stdout.write(l)}const wait=()=>{if(process.stdout.writableLength>0)return setTimeout(wait,20);if(sent<TOTAL)setImmediate(pump);else process.stdout.write('FLOOD-END '+n+'\\n',()=>fs.writeFileSync(p,'done '+sent))};wait()};process.stdout.write('flood-ready\\n');process.stdin.on('data',(d)=>{const s=String(d);if(s.includes('g'))pump();if(s.includes('q'))process.exit(0)})";

fn spawn_desk(core: &mut Core, c: &mut Client, id: &str, cwd: &str, cols: u16, args: &[&str]) {
    c.send(json!({ "t": "spawn", "preset": "shell", "cols": cols, "rows": 30, "cwd": cwd }));
    let req = core.next(|m| m["t"] == "resolve", 5_000);
    core.resolved(&req["rid"], id, &node(), args, cwd);
    assert_eq!(c.text("ready")["id"], id);
}

fn view(port: u16, id: &str, cols: u16, rows: u16) -> Client {
    let mut p = Client::connect(port);
    p.send(json!({ "t": "attach", "id": id, "mode": "view", "cols": cols, "rows": rows }));
    let ready = p.text("ready");
    assert_eq!(ready["id"], id);
    p
}

fn resized(c: &Client) -> Vec<(u64, u64)> {
    c.texts.iter().filter(|m| m["t"] == "resized").map(|m| (m["cols"].as_u64().unwrap(), m["rows"].as_u64().unwrap())).collect()
}

fn wait_size(c: &mut Client, size: &str) {
    c.send(json!({ "t": "claim" }));
    let end = Instant::now() + Duration::from_secs(10);
    loop {
        c.send_bytes(b"r");
        std::thread::sleep(Duration::from_millis(150));
        c.pump();
        if c.out_str().contains(size) {
            return;
        }
        assert!(Instant::now() < end, "{size} not seen: {:?}", c.out_str());
    }
}

fn between(out: &str) -> Option<String> {
    let a = out.find("BEGIN")?;
    let b = out[a..].find("END")?;
    Some(out[a..a + b].to_owned())
}

fn strip_csi(b: &[u8]) -> String {
    let s = String::from_utf8_lossy(b);
    let mut out = String::with_capacity(s.len());
    let mut it = s.chars().peekable();
    while let Some(ch) = it.next() {
        if ch != '\x1b' {
            out.push(ch);
            continue;
        }
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

#[test]
fn desktop_and_phone_see_the_same_bytes_and_typing_moves_the_size() {
    let mut core = Core::start();
    let cwd = temp_dir("view1");
    let mut desk = Client::connect(core.port);
    let id = "6f1c1d2e-0000-4000-8000-0000000000a1";
    spawn_desk(&mut core, &mut desk, id, &cwd, 100, &["-e", PANE]);
    desk.out_has("hi from child", 10_000);
    let mut phone = view(core.port, id, 50, 20);
    assert_eq!(phone.until(5_000, |p| (!resized(p).is_empty()).then(|| resized(p))), [(100, 30)]);
    phone.out_has("hi from child", 5_000);

    desk.send_bytes(b"b");
    desk.until(10_000, |c| between(&c.out_str()));
    phone.until(10_000, |c| between(&c.out_str()));
    let d = between(&desk.out_str()).unwrap();
    assert_eq!(between(&phone.out_str()).unwrap(), d, "both sockets carry the same stream");
    assert!(d.contains("row2999"));

    wait_size(&mut phone, "size:50x20;");
    assert_eq!(desk.until(5_000, |c| (!resized(c).is_empty()).then(|| resized(c))), [(50, 20)]);
    wait_size(&mut desk, "size:100x30;");
    assert_eq!(phone.until(5_000, |p| (resized(p).len() > 1).then(|| resized(p))), [(100, 30), (100, 30)]);

    let mut tablet = view(core.port, id, 60, 15);
    phone.output.clear();
    wait_size(&mut phone, "size:50x20;");
    tablet.until(5_000, |t| resized(t).contains(&(50, 20)).then_some(()));
    let before = resized(&tablet).len();
    drop(phone);
    let after = tablet.until(10_000, |t| (resized(t).len() > before).then(|| resized(t)));
    assert_eq!(after.last(), Some(&(100, 30)), "owner left: input and size go back to the desktop");
    desk.output.clear();
    wait_size(&mut desk, "size:100x30;");

    desk.pump();
    tablet.pump();
    assert_eq!(desk.close, None);
    assert_eq!(tablet.close, None);
    desk.send_bytes(b"q");
    assert_eq!(desk.text("exit")["code"], 3);
    assert_eq!(tablet.text("exit")["code"], 3);
}

#[test]
fn a_phone_that_stops_reading_is_closed_4001_alone_and_the_desktop_gets_everything() {
    let mut core = Core::start();
    let cwd = temp_dir("view2");
    let progress = std::path::PathBuf::from(&cwd).join("progress.txt");
    let _ = std::fs::remove_file(&progress);
    let total: u64 = 16 * 1024 * 1024;
    let mut desk = Client::connect(core.port);
    let id = "6f1c1d2e-0000-4000-8000-0000000000a2";
    spawn_desk(&mut core, &mut desk, id, &cwd, 200, &["-e", FLOOD, progress.to_str().unwrap(), &total.to_string()]);
    desk.out_has("flood-ready", 10_000);
    let mut phone = view(core.port, id, 50, 20);
    desk.send_bytes(b"g");
    let end = Instant::now() + Duration::from_secs(180);
    while !desk.out_str().contains("FLOOD-END") {
        desk.pump();
        assert!(Instant::now() < end, "flood stalled: {}", std::fs::read_to_string(&progress).unwrap_or_default());
    }
    std::thread::sleep(Duration::from_millis(300));
    desk.pump();
    let nums = line_numbers(&strip_csi(&desk.output));
    let expected = total.div_ceil(120);
    let missing: Vec<u64> = (0..expected).filter(|n| nums.binary_search(n).is_err()).take(10).collect();
    assert!(nums.windows(2).all(|w| w[0] < w[1]), "lines out of order");
    assert!(missing.is_empty(), "desktop lost lines {missing:?} of {expected}");
    assert_eq!(desk.close, None);
    phone.until(60_000, |p| p.close);
    assert_eq!(phone.close, Some(4001));
    assert!((phone.output.len() as u64) < total, "the phone was cut before the end");
    desk.send_bytes(b"q");
    desk.text("exit");
}

#[test]
fn a_view_after_exit_gets_ready_resized_replay_exit() {
    let mut core = Core::start();
    let cwd = temp_dir("view3");
    let mut desk = Client::connect(core.port);
    let id = "6f1c1d2e-0000-4000-8000-0000000000a3";
    spawn_desk(&mut core, &mut desk, id, &cwd, 100, &["-e", "process.stdout.write('bye-view');setTimeout(()=>process.exit(4),300)"]);
    assert_eq!(desk.text("exit")["code"], 4);
    drop(desk);
    std::thread::sleep(Duration::from_millis(200));
    let mut phone = Client::connect(core.port);
    phone.send(json!({ "t": "attach", "id": id, "mode": "view", "cols": 40, "rows": 10 }));
    let kinds: Vec<String> = phone.until(5_000, |p| p.texts.iter().any(|m| m["t"] == "exit").then(|| p.texts.iter().map(|m| m["t"].as_str().unwrap().to_owned()).collect()));
    assert_eq!(kinds, ["ready", "resized", "exit"]);
    assert_eq!(phone.texts[0]["replay"], true);
    assert_eq!(phone.texts[1], json!({ "t": "resized", "cols": 100, "rows": 30 }));
    assert!(phone.out_str().contains("bye-view"));
    assert_eq!(phone.close, None);
    phone.send(json!({ "t": "kill" }));
    core.next(|m| m["t"] == "removed" && m["id"] == id, 5_000);
}

#[test]
fn a_slow_desktop_still_paces_the_pane_while_the_claimed_phone_reads_fast() {
    let mut core = Core::start();
    let cwd = temp_dir("view4");
    let progress = std::path::PathBuf::from(&cwd).join("progress.txt");
    let _ = std::fs::remove_file(&progress);
    let total: u64 = 16 * 1024 * 1024;
    let mut desk = Client::connect(core.port);
    let id = "6f1c1d2e-0000-4000-8000-0000000000a4";
    spawn_desk(&mut core, &mut desk, id, &cwd, 200, &["-e", FLOOD, progress.to_str().unwrap(), &total.to_string()]);
    desk.out_has("flood-ready", 10_000);
    let mut phone = view(core.port, id, 200, 30);
    phone.send(json!({ "t": "claim" }));
    std::thread::sleep(Duration::from_millis(200));
    desk.send_bytes(b"g");
    let end = Instant::now() + Duration::from_secs(60);
    let mut last = 0;
    let mut quiet = Instant::now();
    loop {
        phone.pump();
        if phone.output.len() != last {
            last = phone.output.len();
            quiet = Instant::now();
        } else if last > 0 && quiet.elapsed() > Duration::from_millis(1500) {
            break;
        }
        assert!(Instant::now() < end, "phone never stalled");
        assert!(!phone.out_str().contains("FLOOD-END"), "the pane ran on while the desktop did not read");
    }
    assert!((last as u64) < total / 2, "pane not paced by the desktop: {last}");
    let end = Instant::now() + Duration::from_secs(180);
    while !desk.out_str().contains("FLOOD-END") {
        desk.pump();
        phone.pump();
        assert!(Instant::now() < end, "flood stalled after the desktop read again");
    }
    std::thread::sleep(Duration::from_millis(300));
    desk.pump();
    phone.pump();
    assert_eq!(desk.close, None, "the desktop is never closed 4001 for being slower than the phone");
    assert_eq!(phone.close, None);
    let nums = line_numbers(&strip_csi(&desk.output));
    let expected = total.div_ceil(120);
    let missing: Vec<u64> = (0..expected).filter(|n| nums.binary_search(n).is_err()).take(10).collect();
    assert!(missing.is_empty(), "desktop lost lines {missing:?} of {expected}");
    desk.send_bytes(b"q");
    desk.text("exit");
}
