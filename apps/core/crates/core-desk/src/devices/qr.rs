use super::adb::Adb;
use super::parse::*;
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

const WIFI_PORT: u16 = 5555;

pub fn qr_payload(name: &str, password: &str) -> String {
    format!("WIFI:T:ADB;S:{name};P:{password};;")
}

pub fn new_qr_secret() -> (String, String) {
    let mut b = [0u8; 8];
    let _ = getrandom::fill(&mut b);
    let name = format!("orrery-{}", b[..4].iter().map(|x| format!("{x:02x}")).collect::<String>());
    let mut n = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
    while n >= 4_294_000_000 {
        let mut c = [0u8; 4];
        let _ = getrandom::fill(&mut c);
        n = u32::from_le_bytes(c);
    }
    (name, format!("{:06}", n % 1_000_000))
}

pub type OnStatus = Arc<dyn Fn(&str, Option<&str>) + Send + Sync>;
pub type LastPort = Arc<dyn Fn(&str) -> Option<u64> + Send + Sync>;

pub struct QrTiming {
    pub pair_wait_ms: u64,
    pub connect_wait_ms: u64,
    pub poll_ms: u64,
}

pub struct QrOptions {
    pub adb: Adb,
    pub name: String,
    pub password: String,
    pub abort: Arc<AtomicBool>,
    pub on_status: OnStatus,
    pub last_port_for: LastPort,
    pub timing: QrTiming,
}

fn sleep(ms: u64, abort: &AtomicBool) {
    let end = Instant::now() + Duration::from_millis(ms);
    while Instant::now() < end && !abort.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_millis(25).min(end.saturating_duration_since(Instant::now())));
    }
}

fn detail_of(v: &Value) -> Option<String> {
    if is_ok(v) {
        None
    } else {
        v.get("detail").and_then(Value::as_str).map(String::from)
    }
}

pub fn run_qr_pairing(o: QrOptions) -> Value {
    let end = |status: &str, result: Value| -> Value {
        let d = detail_of(&result);
        (o.on_status)(status, d.as_deref());
        result
    };
    let cancelled = || end("cancelled", err("cancelled"));
    (o.on_status)("waiting", None);
    let pair_deadline = Instant::now() + Duration::from_millis(o.timing.pair_wait_ms);
    let (host, port);
    loop {
        if o.abort.load(Ordering::SeqCst) {
            return cancelled();
        }
        let svc = o.adb.mdns_services().into_iter().find(|s| s.kind == "pairing" && s.name == o.name);
        if o.abort.load(Ordering::SeqCst) {
            return cancelled();
        }
        if let Some(s) = svc {
            host = s.host;
            port = s.port;
            break;
        }
        if Instant::now() >= pair_deadline {
            return end("timeout", err("timeout"));
        }
        sleep(o.timing.poll_ms, &o.abort);
    }
    (o.on_status)("pairing", None);
    let paired = o.adb.pair(&format!("{host}:{port}"), &o.password);
    if o.abort.load(Ordering::SeqCst) {
        return cancelled();
    }
    if !is_ok(&paired) {
        return end("failed", paired);
    }
    (o.on_status)("connecting", None);
    let host_key = address_key(&host, 0);
    let connect_deadline = Instant::now() + Duration::from_millis(o.timing.connect_wait_ms);
    let mut targets: Vec<String> = Vec::new();
    loop {
        if o.abort.load(Ordering::SeqCst) {
            return cancelled();
        }
        let svc = o.adb.mdns_services().into_iter().find(|s| s.kind == "connect" && address_key(&s.host, 0) == host_key);
        if o.abort.load(Ordering::SeqCst) {
            return cancelled();
        }
        if let Some(s) = svc {
            targets.push(format!("{}:{}", s.host, s.port));
            break;
        }
        if Instant::now() >= connect_deadline {
            break;
        }
        sleep(o.timing.poll_ms, &o.abort);
    }
    if targets.is_empty() {
        if let Some(r) = (o.last_port_for)(&host) {
            targets.push(format!("{host}:{r}"));
        }
        targets.push(format!("{host}:{WIFI_PORT}"));
    }
    let mut last = err("connect-failed");
    for t in targets {
        last = o.adb.connect(&t);
        if o.abort.load(Ordering::SeqCst) {
            return cancelled();
        }
        if is_ok(&last) {
            return end("connected", last);
        }
    }
    end("failed", last)
}

pub fn status_event(id: &str, status: &str, detail: Option<&str>) -> Value {
    match detail.filter(|d| !d.is_empty()) {
        Some(d) => json!({"type": "qr", "id": id, "status": status, "detail": d}),
        None => json!({"type": "qr", "id": id, "status": status}),
    }
}
