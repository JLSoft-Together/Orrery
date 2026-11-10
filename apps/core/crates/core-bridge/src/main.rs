use std::io::{Read, Write};

const RAW_MAX: usize = 1_000_000;

fn read_stdin(cap: Option<usize>) -> Vec<u8> {
    let mut raw = Vec::new();
    let mut chunk = vec![0u8; 65_536];
    let mut stdin = std::io::stdin().lock();
    loop {
        match stdin.read(&mut chunk) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                if cap.is_none_or(|c| raw.len() < c) {
                    raw.extend_from_slice(&chunk[..n]);
                }
            }
        }
    }
    raw
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("hook-bridge") => {
            let raw = read_stdin(Some(RAW_MAX));
            core_bridge::hook::run(&raw);
        }
        Some("statusline-bridge") => {
            let raw = read_stdin(None);
            let line = core_bridge::statusline::run(&args, &raw);
            if !line.is_empty() {
                let mut out = std::io::stdout().lock();
                let _ = out.write_all(line.as_bytes());
                let _ = out.flush();
            }
        }
        _ => {
            eprintln!("usage: orrery-bridge statusline-bridge [--chain-b64 <base64>] | hook-bridge");
            std::process::exit(2);
        }
    }
}
