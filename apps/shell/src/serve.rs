use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use core_serve::config::ServeConfig;

use crate::stdio::{take_stdin, ControlPipe};

pub const PORT_IN_USE_EXIT: i32 = 3;
const BOOT_TIMEOUT: Duration = Duration::from_secs(30);
const STOP_TIMEOUT: Duration = Duration::from_secs(4);
const IO_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug, PartialEq, Eq)]
pub enum ServeError {
    PortInUse,
    Exited(i32),
    Timeout,
    Io(String),
}

pub struct Serve {
    control: ControlPipe,
    thread: Option<JoinHandle<i32>>,
}

pub struct Reply {
    pub status: u16,
    pub body: String,
}

pub fn request(port: u16, method: &str, target: &str, key: Option<&str>, body: Option<&str>) -> Option<Reply> {
    let mut stream = TcpStream::connect_timeout(&([127, 0, 0, 1], port).into(), IO_TIMEOUT).ok()?;
    let _ = stream.set_read_timeout(Some(IO_TIMEOUT));
    let _ = stream.set_write_timeout(Some(IO_TIMEOUT));
    let mut head = format!("{method} {target} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n");
    if let Some(k) = key {
        head.push_str(&format!("x-ccm-app: {k}\r\n"));
    }
    if let Some(b) = body {
        head.push_str(&format!("content-type: application/json\r\ncontent-length: {}\r\n", b.len()));
    }
    head.push_str("\r\n");
    head.push_str(body.unwrap_or(""));
    stream.write_all(head.as_bytes()).ok()?;
    let mut raw = Vec::new();
    let _ = stream.read_to_end(&mut raw);
    let text = String::from_utf8_lossy(&raw).into_owned();
    let status = text.split(' ').nth(1)?.parse().ok()?;
    let body = text.split_once("\r\n\r\n").map_or(String::new(), |(_, b)| b.to_owned());
    Some(Reply { status, body })
}

pub fn healthy(port: u16) -> bool {
    request(port, "GET", "/health", None, None).is_some_and(|r| r.status == 200 && r.body.contains("\"ok\":true"))
}

impl Serve {
    pub fn start(cfg: ServeConfig) -> Result<Serve, ServeError> {
        let port = cfg.port;
        if healthy(port) {
            return Err(ServeError::PortInUse);
        }
        let control = take_stdin().map_err(|e| ServeError::Io(e.to_string()))?;
        let thread = std::thread::Builder::new().name("core-serve".into()).spawn(move || core_serve::run(cfg)).map_err(|e| ServeError::Io(e.to_string()))?;
        let mut serve = Serve { control, thread: Some(thread) };
        let deadline = Instant::now() + BOOT_TIMEOUT;
        while Instant::now() < deadline {
            if serve.thread.as_ref().is_some_and(JoinHandle::is_finished) {
                let code = serve.thread.take().map_or(1, |t| t.join().unwrap_or(1));
                return Err(if code == PORT_IN_USE_EXIT { ServeError::PortInUse } else { ServeError::Exited(code) });
            }
            if healthy(port) {
                return Ok(serve);
            }
            std::thread::sleep(Duration::from_millis(150));
        }
        serve.stop();
        Err(ServeError::Timeout)
    }

    pub fn stop(&mut self) {
        let Some(thread) = self.thread.take() else { return };
        self.control.send("{\"type\":\"shutdown\"}");
        let deadline = Instant::now() + STOP_TIMEOUT;
        while !thread.is_finished() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
        }
        if thread.is_finished() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_closed_port_is_not_healthy_and_gives_no_reply() {
        let port = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap().local_addr().unwrap().port();
        assert!(!healthy(port));
        assert!(request(port, "GET", "/", None, None).is_none());
    }

    #[test]
    fn reads_status_and_body_of_a_reply() {
        let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let port = listener.local_addr().unwrap().port();
        let server = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut buf = [0u8; 1024];
            let n = s.read(&mut buf).unwrap();
            let head = String::from_utf8_lossy(&buf[..n]).into_owned();
            s.write_all(b"HTTP/1.1 200 OK\r\ncontent-length: 21\r\n\r\n{\"ok\":true,\"seq\":0}\n\n").unwrap();
            head
        });
        assert!(healthy(port));
        let head = server.join().unwrap();
        assert!(head.starts_with("GET /health HTTP/1.1\r\n"));
        assert!(!head.contains("x-ccm-app"));
    }
}
