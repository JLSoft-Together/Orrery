use core_http::control::{spawn_stdin, write_line, ControlIn};
use core_http::{start, Config, Hooks, StartError, StoreHandle, PORT_IN_USE_EXIT};
use core_store::{Store, StoreOptions};
use serde_json::json;

fn main() {
    std::process::exit(run());
}

fn run() -> i32 {
    let dev = std::env::args().any(|a| a == "--dev");
    let app_key = std::env::var("CCM_APP_KEY").ok().filter(|k| !k.is_empty());
    if app_key.is_none() && !dev {
        eprintln!("[http-serve] needs CCM_APP_KEY or --dev");
        return 2;
    }
    let port = std::env::var("CCM_PORT").ok().and_then(|p| p.parse::<u16>().ok()).unwrap_or(4317);
    let web_dist = std::env::var("CCM_WEB_DIST").unwrap_or_else(|_| "apps/web/dist".into());
    let mut config = Config::new(port, web_dist);
    config.app_key = if dev { None } else { app_key };
    if dev {
        config.dev_origin_ports = vec![5173];
    }
    let runtime = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[http-serve] runtime: {e}");
            return 1;
        }
    };
    runtime.block_on(async move {
        let store = StoreHandle::new(Store::new(StoreOptions { activity_limit: 1000, batch_ms: 50 }));
        let server = match start(config, store, Hooks::default()).await {
            Ok(s) => s,
            Err(StartError::PortInUse(e)) => {
                eprintln!("[http-serve] port {port} in use: {e}");
                return PORT_IN_USE_EXIT;
            }
            Err(e) => {
                eprintln!("[http-serve] {e}");
                return 1;
            }
        };
        write_line(&json!({ "type": "listening", "port": server.port() }));
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        spawn_stdin(tx);
        while let Some(message) = rx.recv().await {
            if matches!(message, ControlIn::Shutdown | ControlIn::Eof) {
                break;
            }
        }
        server.shutdown().await;
        0
    })
}
