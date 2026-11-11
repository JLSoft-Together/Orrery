#[path = "../tests/common/mod.rs"]
mod common;

use std::io::Write;
use std::net::TcpListener;
use std::thread;
use std::time::Duration;

use common::Gateway;
use serde_json::json;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let port: u16 = args.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
    let exp_in: i64 = args.get(2).and_then(|p| p.parse().ok()).unwrap_or(300);
    let gateway = Gateway::new(exp_in);
    let listener = TcpListener::bind(("127.0.0.1", port)).expect("bind");
    gateway.serve(listener);
    thread::sleep(Duration::from_millis(50));
    let info = json!({
        "qr": gateway.qr(None),
        "port": *gateway.port.lock().unwrap(),
        "machine": gateway.machine_id,
    });
    println!("{info}");
    let _ = std::io::stdout().flush();
    loop {
        thread::sleep(Duration::from_secs(3600));
    }
}
