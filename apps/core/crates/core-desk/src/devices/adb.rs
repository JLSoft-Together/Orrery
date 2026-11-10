use super::parse::*;
use super::uidump::{find_png_start, parse_ui_dump, png_size};
use crate::js;
use crate::run::android::jsre::Js;
use crate::run::collect::{collect, CollectOpts, Collected};
use crate::run::proc::{kill_pid_only, spawn, SpawnSpec};
use crate::run::types::Env;
use crate::HostLink;
use base64::Engine;
use serde_json::{json, Value};
use std::io::Read;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const UI_DUMP_PATH: &str = "/sdcard/orrery-ui.xml";
const CAPTURE_MAX: usize = 64 * 1024 * 1024;
const DETAIL_MAX: usize = 300;

#[derive(Clone, Copy)]
pub struct Timing {
    pub connect_ms: u64,
    pub pair_ms: u64,
    pub cmd_ms: u64,
    pub capture_ms: u64,
    pub dump_ms: u64,
}

pub const TIMING: Timing = Timing { connect_ms: 15_000, pair_ms: 20_000, cmd_ms: 10_000, capture_ms: 15_000, dump_ms: 20_000 };

pub struct Bytes {
    pub out: Vec<u8>,
    pub err: String,
    pub timed_out: bool,
}

#[derive(Clone)]
pub struct Adb {
    pub path: Arc<Mutex<Option<String>>>,
    pub home: String,
    pub env: Env,
    pub host: HostLink,
    pub t: Timing,
}

pub fn fail(code: &str, text: &str) -> Value {
    static WS: Js = Js::new(r"\s+", "g");
    let collapsed = WS.replace_all(text, " ");
    let detail = js::slice(collapsed.trim(), DETAIL_MAX);
    err_detail(code, &detail)
}

impl Adb {
    fn current(&self) -> Option<String> {
        self.path.lock().unwrap().clone()
    }

    pub fn used(&self, serial: &str) {
        let _ = self.host.call("devices.use", json!({"serial": serial}));
    }

    pub fn run(&self, args: &[&str], timeout_ms: u64) -> Option<Collected> {
        if args.first() == Some(&"-s") {
            if let Some(s) = args.get(1) {
                self.used(s);
            }
        }
        let adb = self.current()?;
        let list: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        Some(collect(&adb, &list, &self.home, &self.env, &CollectOpts { timeout_ms: Some(timeout_ms), kill: Some(kill_pid_only), ..Default::default() }))
    }

    fn result(&self, args: &[&str], timeout_ms: u64, spawn_failed: &str, parse: impl Fn(&Collected) -> Value) -> Value {
        let Some(r) = self.run(args, timeout_ms) else { return err("adb-missing") };
        if r.timed_out {
            return err("timeout");
        }
        if r.code.is_none() {
            return err(spawn_failed);
        }
        parse(&r)
    }

    pub fn connect(&self, target: &str) -> Value {
        self.result(&["connect", target], self.t.connect_ms, "connect-failed", |r| parse_connect(&r.out))
    }

    pub fn pair(&self, host_port: &str, code: &str) -> Value {
        self.result(&["pair", host_port, code], self.t.pair_ms, "pair-failed", |r| parse_pair(&r.out))
    }

    pub fn disconnect(&self, id: &str) -> Value {
        self.result(&["disconnect", id], self.t.cmd_ms, "not-connected", |r| parse_disconnect(&r.out))
    }

    pub fn tcpip(&self, id: &str, port: u16) -> Value {
        let p = port.to_string();
        self.result(&["-s", id, "tcpip", &p], self.t.cmd_ms, "tcpip-failed", |r| {
            if r.code == Some(0) && !r.out.to_lowercase().contains("error") {
                return ok();
            }
            err_detail("tcpip-failed", r.out.trim())
        })
    }

    pub fn wlan_ip(&self, id: &str) -> Option<String> {
        let r = self.run(&["-s", id, "shell", "ip", "-f", "inet", "addr", "show", "wlan0"], self.t.cmd_ms)?;
        if r.code == Some(0) {
            parse_wlan_ip(&r.out)
        } else {
            None
        }
    }

    pub fn mdns_check(&self) -> bool {
        self.run(&["mdns", "check"], self.t.cmd_ms).is_some_and(|r| r.code == Some(0) && !r.out.to_lowercase().contains("unavailable"))
    }

    pub fn mdns_services(&self) -> Vec<Mdns> {
        match self.run(&["mdns", "services"], self.t.cmd_ms) {
            Some(r) if r.code == Some(0) => parse_mdns_services(&r.out),
            _ => Vec::new(),
        }
    }

    pub fn list(&self) -> Option<Vec<Row>> {
        let r = self.run(&["devices", "-l"], self.t.cmd_ms)?;
        (!r.timed_out && r.code == Some(0)).then(|| parse_adb_device_rows(&r.out))
    }

    pub fn shell(&self, id: &str, args: &[String], timeout_ms: Option<u64>) -> Option<Collected> {
        let mut list: Vec<&str> = vec!["-s", id, "shell"];
        list.extend(args.iter().map(String::as_str));
        self.run(&list, timeout_ms.unwrap_or(self.t.cmd_ms))
    }

    fn collect_bytes(&self, args: &[&str], timeout_ms: u64, max: usize) -> Option<Bytes> {
        let adb = self.current()?;
        let spec = SpawnSpec { file: adb, args: args.iter().map(|a| a.to_string()).collect(), cwd: self.home.clone(), ..Default::default() };
        let Ok(mut child) = spawn(&spec, &self.env) else { return Some(Bytes { out: Vec::new(), err: String::new(), timed_out: false }) };
        let pid = child.id();
        let out_buf = Arc::new(Mutex::new(Vec::<u8>::new()));
        let err_buf = Arc::new(Mutex::new(String::new()));
        let (tx, rx) = mpsc::channel::<()>();
        if let Some(mut so) = child.stdout.take() {
            let (buf, tx) = (out_buf.clone(), tx.clone());
            std::thread::spawn(move || {
                let mut chunk = [0u8; 32 * 1024];
                while let Ok(n) = so.read(&mut chunk) {
                    if n == 0 {
                        break;
                    }
                    let mut b = buf.lock().unwrap();
                    b.extend_from_slice(&chunk[..n]);
                    if b.len() > max {
                        kill_pid_only(pid);
                        break;
                    }
                }
                let _ = tx.send(());
            });
        }
        if let Some(mut se) = child.stderr.take() {
            let (buf, tx) = (err_buf.clone(), tx.clone());
            std::thread::spawn(move || {
                let mut bytes = Vec::new();
                let _ = se.read_to_end(&mut bytes);
                *buf.lock().unwrap() = String::from_utf8_lossy(&bytes).to_string();
                let _ = tx.send(());
            });
        }
        drop(tx);
        let (done_tx, done_rx) = mpsc::channel::<()>();
        std::thread::spawn(move || {
            let _ = child.wait();
            let _ = done_tx.send(());
        });
        let timed_out = done_rx.recv_timeout(Duration::from_millis(timeout_ms)).is_err();
        if timed_out {
            kill_pid_only(pid);
        } else {
            while rx.recv_timeout(Duration::from_millis(1500)).is_ok() {}
        }
        let mut out = out_buf.lock().unwrap().clone();
        out.truncate(if out.len() > max { 0 } else { out.len() });
        let err = err_buf.lock().unwrap().clone();
        let tail: Vec<char> = err.chars().collect();
        Some(Bytes { out, err: tail[tail.len().saturating_sub(4096)..].iter().collect(), timed_out })
    }

    pub fn screencap(&self, id: &str) -> Value {
        static SECURE: Js = Js::new(r"secure", "i");
        if self.current().is_none() {
            return fail("adb-missing", "");
        }
        self.used(id);
        let Some(r) = self.collect_bytes(&["-s", id, "exec-out", "screencap", "-p"], self.t.capture_ms, CAPTURE_MAX) else { return fail("adb-missing", "") };
        if r.timed_out {
            return fail("timeout", "");
        }
        let png = find_png_start(&r.out).map(|s| &r.out[s..]);
        let size = png.and_then(png_size);
        match (png, size) {
            (Some(png), Some((width, height))) => json!({"ok": true, "png": format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(png)), "width": width, "height": height}),
            _ => {
                let text = format!("{} {}", String::from_utf8_lossy(&r.out[..r.out.len().min(2048)]), r.err);
                fail(if SECURE.test(&text) { "secure" } else { "capture-failed" }, &text)
            }
        }
    }

    pub fn ui_dump(&self, id: &str) -> Value {
        static SECURE: Js = Js::new(r"secure", "i");
        static DUMPED: Js = Js::new(r"dumped to", "i");
        let Some(dumped) = self.run(&["-s", id, "shell", "uiautomator", "dump", UI_DUMP_PATH], self.t.dump_ms) else { return fail("adb-missing", "") };
        if dumped.timed_out {
            return fail("timeout", "");
        }
        if dumped.code != Some(0) || !DUMPED.test(&dumped.out) {
            return fail(if SECURE.test(&dumped.out) { "secure" } else { "dump-failed" }, &dumped.out);
        }
        let result = {
            let read = self.run(&["-s", id, "exec-out", "cat", UI_DUMP_PATH], self.t.cmd_ms);
            match read {
                Some(r) if !r.timed_out && r.code == Some(0) => match parse_ui_dump(&r.out) {
                    Some((nodes, rotation)) => json!({"ok": true, "nodes": nodes, "rotation": rotation}),
                    None => fail("dump-failed", &js::slice(&r.out, DETAIL_MAX)),
                },
                other => fail("dump-failed", &other.map(|r| r.out).unwrap_or_default()),
            }
        };
        self.run(&["-s", id, "shell", "rm", "-f", UI_DUMP_PATH], self.t.cmd_ms);
        result
    }
}
