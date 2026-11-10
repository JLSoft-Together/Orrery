mod common;

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use common::*;

const BIN: &str = env!("CARGO_BIN_EXE_http-serve");

fn spawn(extra_env: &[(&str, String)], args: &[&str]) -> Child {
    let mut cmd = Command::new(BIN);
    cmd.args(args).stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd.env_remove("CCM_APP_KEY").env_remove("CCM_PORT").env_remove("CCM_WEB_DIST");
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    cmd.spawn().unwrap()
}

fn listening_port(child: &mut Child) -> u16 {
    let mut line = String::new();
    BufReader::new(child.stdout.as_mut().unwrap()).read_line(&mut line).unwrap();
    let value: serde_json::Value = serde_json::from_str(line.trim()).unwrap_or_else(|e| panic!("{line:?}: {e}"));
    assert_eq!(value["type"], "listening");
    value["port"].as_u64().unwrap() as u16
}

fn wait_exit(child: &mut Child) -> i32 {
    let started = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status.code().unwrap_or(-1);
        }
        assert!(started.elapsed() < Duration::from_secs(10), "process did not exit");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn env(dist: &std::path::Path, port: &str) -> Vec<(&'static str, String)> {
    vec![("CCM_PORT", port.to_owned()), ("CCM_WEB_DIST", dist.to_string_lossy().into_owned()), ("CCM_APP_KEY", "k3y".to_owned())]
}

#[tokio::test]
async fn the_binary_serves_then_exits_0_on_a_stdin_shutdown_line() {
    let dist = dist_with_index("<!doctype html>ok");
    let mut child = spawn(&env(&dist, "0"), &[]);
    let port = listening_port(&mut child);
    assert_eq!(get(port, "/health", &[]).await.text(), r#"{"ok":true,"seq":0}"#);
    assert_eq!(get(port, "/", &[]).await.status, 403);
    assert_eq!(get(port, "/", &[("x-ccm-app", "k3y")]).await.status, 200);
    let mut ws = ws_connect(port, &[("x-ccm-app", "k3y")]).await.unwrap();
    assert_eq!(next_json(&mut ws).await["events"][0]["type"], "snapshot");
    child.stdin.as_mut().unwrap().write_all(b"{\"type\":\"deviceUsed\"}\n{\"type\":\"shutdown\"}\n").unwrap();
    child.stdin.as_mut().unwrap().flush().unwrap();
    assert_eq!(wait_exit(&mut child), 0);
}

#[tokio::test]
async fn closing_stdin_stops_the_server_so_no_process_is_orphaned() {
    let dist = dist_with_index("ok");
    let mut child = spawn(&env(&dist, "0"), &[]);
    listening_port(&mut child);
    drop(child.stdin.take());
    assert_eq!(wait_exit(&mut child), 0);
}

#[tokio::test]
async fn a_taken_port_exits_with_code_3() {
    let dist = dist_with_index("ok");
    let (first, _) = serve(dist.clone(), |_| {}, core_http::Hooks::default()).await;
    let mut child = spawn(&env(&dist, &first.port().to_string()), &[]);
    assert_eq!(wait_exit(&mut child), core_http::PORT_IN_USE_EXIT);
    first.shutdown().await;
}

#[tokio::test]
async fn without_an_app_key_or_dev_the_binary_refuses_to_start() {
    let dist = dist_with_index("ok");
    let mut child = spawn(&[("CCM_PORT", "0".to_owned()), ("CCM_WEB_DIST", dist.to_string_lossy().into_owned())], &[]);
    assert_eq!(wait_exit(&mut child), 2);
}

#[tokio::test]
async fn dev_mode_needs_no_key() {
    let dist = dist_with_index("ok");
    let mut child = spawn(&[("CCM_PORT", "0".to_owned()), ("CCM_WEB_DIST", dist.to_string_lossy().into_owned())], &["--dev"]);
    let port = listening_port(&mut child);
    assert_eq!(get(port, "/", &[]).await.status, 200);
    drop(child.stdin.take());
    assert_eq!(wait_exit(&mut child), 0);
}
