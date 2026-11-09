use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

const INIT: &str = r#"{"t":"init","token":"t0ken","origins":["http://127.0.0.1:4317"],"appVersion":"0.0.0-test"}"#;

fn spawn() -> Child {
    Command::new(env!("CARGO_BIN_EXE_orrery-core"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

fn wait_exit(child: &mut Child) -> ExitStatus {
    let start = Instant::now();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return status;
        }
        if start.elapsed() > Duration::from_secs(10) {
            let _ = child.kill();
            panic!("orrery-core did not exit");
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn read_port(child: &mut Child) -> u16 {
    let mut line = String::new();
    BufReader::new(child.stdout.as_mut().unwrap()).read_line(&mut line).unwrap();
    let v: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(v["t"], "listening");
    let port = v["port"].as_u64().unwrap();
    assert!(port > 0 && port < 65536);
    port as u16
}

#[test]
fn init_then_listening_then_stdin_eof_exits_zero() {
    let mut child = spawn();
    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, "{INIT}").unwrap();
    let port = read_port(&mut child);
    TcpStream::connect(("127.0.0.1", port)).unwrap();
    drop(stdin);
    assert!(wait_exit(&mut child).success());
}

#[test]
fn shutdown_message_exits_zero() {
    let mut child = spawn();
    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, "{INIT}").unwrap();
    read_port(&mut child);
    writeln!(stdin, r#"{{"t":"shutdown"}}"#).unwrap();
    stdin.flush().unwrap();
    assert!(wait_exit(&mut child).success());
}

#[test]
fn lines_before_init_are_ignored_then_init_still_listens() {
    let mut child = spawn();
    let mut stdin = child.stdin.take().unwrap();
    writeln!(stdin, r#"{{"t":"census"}}"#).unwrap();
    writeln!(stdin, "not json").unwrap();
    stdin.flush().unwrap();
    std::thread::sleep(Duration::from_millis(200));
    assert!(child.try_wait().unwrap().is_none());
    writeln!(stdin, "{INIT}").unwrap();
    let port = read_port(&mut child);
    TcpStream::connect(("127.0.0.1", port)).unwrap();
    drop(stdin);
    assert!(wait_exit(&mut child).success());
}

#[test]
fn stdin_closed_before_init_exits_zero() {
    let mut child = spawn();
    drop(child.stdin.take());
    assert!(wait_exit(&mut child).success());
}
