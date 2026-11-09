mod common;

use common::*;
use serde_json::json;
use std::time::{Duration, Instant};

struct Run {
    times: Vec<u128>,
    hung: usize,
    lost: usize,
    wrong: usize,
}

fn run(core: &mut Core, cwd: &str, tag: u16, n: usize, cmd: impl Fn(usize) -> (String, Vec<String>, String, i64)) -> Run {
    let mut r = Run { times: Vec::new(), hung: 0, lost: 0, wrong: 0 };
    for i in 0..n {
        let (file, args, tail, want) = cmd(i);
        let t0 = Instant::now();
        let mut c = Client::connect(core.port);
        c.send(json!({ "t": "spawn", "preset": "shell", "cols": 80, "rows": 24, "cwd": cwd }));
        let req = core.next(|m| m["t"] == "resolve", 5_000);
        let id = format!("6f1c1d2e-0000-4000-{tag:04x}-{i:012}");
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        core.resolved(&req["rid"], &id, &file, &args, cwd);
        let end = Instant::now() + Duration::from_secs(10);
        let exit = loop {
            c.pump();
            if let Some(e) = c.texts.iter().find(|m| m["t"] == "exit") {
                break Some(e["code"].as_i64().unwrap());
            }
            if Instant::now() > end || c.close.is_some() {
                break None;
            }
        };
        r.times.push(t0.elapsed().as_millis());
        match exit {
            None => r.hung += 1,
            Some(code) => {
                if code != want {
                    r.wrong += 1;
                }
                if !c.out_str().contains(&tail) {
                    r.lost += 1;
                }
            }
        }
        core.next(|m| m["t"] == "exited" && m["id"] == id.as_str(), 5_000);
    }
    r.times.sort();
    r
}

fn report(name: &str, r: &Run) {
    let n = r.times.len();
    eprintln!("spike {name}: n={n} hung={} lost_tail={} wrong_code={} p50={}ms p99={}ms max={}ms", r.hung, r.lost, r.wrong, r.times[n / 2], r.times[n * 99 / 100], r.times[n - 1]);
}

#[test]
fn fast_exit_200_times_keeps_code_and_last_frame() {
    let mut core = Core::start();
    let cwd = temp_dir("spike");
    let shell = run(&mut core, &cwd, 1, 200, |i| {
        let tail = format!("tail-{i}-end");
        #[cfg(windows)]
        let r = ("cmd".to_owned(), vec!["/c".to_owned(), format!("echo {tail}& exit 3")], tail, 3);
        #[cfg(unix)]
        let r = ("sh".to_owned(), vec!["-c".to_owned(), format!("printf {tail}; exit 3")], tail, 3);
        r
    });
    report("shell exit 3", &shell);
    let nodes = run(&mut core, &cwd, 2, 50, |i| {
        let tail = format!("node-{i}-end");
        (node(), vec!["-e".to_owned(), format!("process.stdout.write('{tail}');process.exit(0)")], tail, 0)
    });
    report("node write+exit", &nodes);
    assert_eq!((shell.hung, shell.lost, shell.wrong), (0, 0, 0));
    assert_eq!((nodes.hung, nodes.lost, nodes.wrong), (0, 0, 0));
    #[cfg(windows)]
    {
        let ntstatus = run(&mut core, &cwd, 3, 3, |_| (node(), vec!["-e".to_owned(), "process.exit(-1073741510)".to_owned()], String::new(), 3221225786));
        report("ntstatus", &ntstatus);
        assert_eq!((ntstatus.hung, ntstatus.wrong), (0, 0));
    }
}
