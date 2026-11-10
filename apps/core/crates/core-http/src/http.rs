use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

pub const HEAD_MAX: usize = 16 * 1024;
pub const HEAD_TIMEOUT: Duration = Duration::from_secs(10);
pub const BODY_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug)]
pub struct Request {
    pub method: String,
    pub target: String,
    headers: Vec<(String, String)>,
    pub rest: Vec<u8>,
}

impl Request {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    pub fn headers_all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> {
        self.headers.iter().filter(move |(k, _)| k == name).map(|(_, v)| v.as_str())
    }

    pub fn path(&self) -> &str {
        let end = self.target.find(['?', '#']).unwrap_or(self.target.len());
        &self.target[..end]
    }

    pub fn query(&self) -> &str {
        let no_fragment = self.target.split('#').next().unwrap_or("");
        no_fragment.split_once('?').map_or("", |(_, q)| q)
    }

    pub fn wants_upgrade(&self) -> bool {
        let connection = self.headers_all("connection").any(|v| v.split(',').any(|t| t.trim().eq_ignore_ascii_case("upgrade")));
        connection && self.header("upgrade").is_some()
    }
}

pub enum HeadError {
    Closed,
    TooLarge,
    Bad,
}

pub async fn read_head<S: AsyncRead + Unpin>(stream: &mut S) -> Result<Request, HeadError> {
    let mut buf: Vec<u8> = Vec::with_capacity(2048);
    let mut chunk = [0u8; 4096];
    loop {
        let n = match tokio::time::timeout(HEAD_TIMEOUT, stream.read(&mut chunk)).await {
            Ok(Ok(0)) | Ok(Err(_)) | Err(_) => return Err(HeadError::Closed),
            Ok(Ok(n)) => n,
        };
        buf.extend_from_slice(&chunk[..n]);
        let mut headers = [httparse::EMPTY_HEADER; 64];
        let mut parsed = httparse::Request::new(&mut headers);
        match parsed.parse(&buf) {
            Ok(httparse::Status::Complete(used)) => {
                let method = parsed.method.unwrap_or("").to_owned();
                let target = parsed.path.unwrap_or("").to_owned();
                let headers = parsed.headers.iter().map(|h| (h.name.to_ascii_lowercase(), String::from_utf8_lossy(h.value).trim().to_owned())).collect();
                let rest = buf[used..].to_vec();
                return Ok(Request { method, target, headers, rest });
            }
            Ok(httparse::Status::Partial) => {
                if buf.len() > HEAD_MAX {
                    return Err(HeadError::TooLarge);
                }
            }
            Err(_) => return Err(HeadError::Bad),
        }
    }
}

pub enum Body {
    Empty,
    Bytes(Vec<u8>),
    File(tokio::fs::File, u64),
}

pub struct Response {
    pub status: u16,
    pub headers: Vec<(&'static str, String)>,
    pub body: Body,
}

impl Response {
    pub fn empty(status: u16) -> Response {
        Response { status, headers: Vec::new(), body: Body::Empty }
    }

    pub fn text(status: u16, content_type: &str, text: impl Into<String>) -> Response {
        Response { status, headers: vec![("content-type", content_type.to_owned())], body: Body::Bytes(text.into().into_bytes()) }
    }

    pub fn json(status: u16, json: String) -> Response {
        Response { status, headers: vec![("content-type", "application/json".to_owned())], body: Body::Bytes(json.into_bytes()) }
    }

    pub fn header(mut self, name: &'static str, value: &str) -> Response {
        self.headers.push((name, value.to_owned()));
        self
    }
}

fn reason(status: u16) -> &'static str {
    match status {
        101 => "Switching Protocols",
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        _ => "OK",
    }
}

pub fn head_text(status: u16, headers: &[(&'static str, String)], length: Option<u64>) -> String {
    let mut head = format!("HTTP/1.1 {status} {}\r\n", reason(status));
    for (k, v) in headers {
        head.push_str(k);
        head.push_str(": ");
        head.push_str(v);
        head.push_str("\r\n");
    }
    if let Some(n) = length {
        head.push_str(&format!("content-length: {n}\r\n"));
    }
    head.push_str("\r\n");
    head
}

pub async fn write_response<S: AsyncWrite + Unpin>(stream: &mut S, response: Response, head_only: bool) -> std::io::Result<()> {
    let length = match &response.body {
        Body::Empty if response.status == 204 => None,
        Body::Empty => Some(0),
        Body::Bytes(b) => Some(b.len() as u64),
        Body::File(_, n) => Some(*n),
    };
    let mut headers = response.headers.clone();
    headers.push(("connection", "close".to_owned()));
    stream.write_all(head_text(response.status, &headers, length).as_bytes()).await?;
    if !head_only {
        match response.body {
            Body::Empty => {}
            Body::Bytes(b) => stream.write_all(&b).await?,
            Body::File(mut f, n) => {
                let mut limited = (&mut f).take(n);
                tokio::io::copy(&mut limited, stream).await?;
            }
        }
    }
    stream.flush().await?;
    let _ = stream.shutdown().await;
    Ok(())
}

pub async fn read_body<S: AsyncRead + Unpin>(stream: &mut S, request: &Request, limit: usize) -> Option<Vec<u8>> {
    let wanted = match request.header("content-length") {
        None => return Some(Vec::new()),
        Some(v) => v.parse::<usize>().ok()?,
    };
    if wanted > limit {
        return None;
    }
    let mut body = request.rest.clone();
    body.truncate(wanted);
    let mut chunk = [0u8; 2048];
    while body.len() < wanted {
        let n = match tokio::time::timeout(BODY_TIMEOUT, stream.read(&mut chunk)).await {
            Ok(Ok(0)) | Ok(Err(_)) | Err(_) => return None,
            Ok(Ok(n)) => n,
        };
        let take = n.min(wanted - body.len());
        body.extend_from_slice(&chunk[..take]);
    }
    Some(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn parse(raw: &str) -> Result<Request, HeadError> {
        let mut input = raw.as_bytes();
        read_head(&mut input).await
    }

    #[tokio::test]
    async fn parses_head_path_and_query() {
        let Ok(r) = parse("GET /snooze?t=a&m=5#frag HTTP/1.1\r\nHost: 127.0.0.1:1\r\nX-CCM-App: k\r\n\r\nrest").await else { panic!("parse") };
        assert_eq!(r.method, "GET");
        assert_eq!(r.path(), "/snooze");
        assert_eq!(r.query(), "t=a&m=5");
        assert_eq!(r.header("x-ccm-app"), Some("k"));
        assert_eq!(r.header("host"), Some("127.0.0.1:1"));
        assert_eq!(r.rest, b"rest");
        assert!(!r.wants_upgrade());
    }

    #[tokio::test]
    async fn detects_an_upgrade_and_rejects_garbage() {
        let Ok(r) = parse("GET /ws HTTP/1.1\r\nConnection: keep-alive, Upgrade\r\nUpgrade: websocket\r\n\r\n").await else { panic!("parse") };
        assert!(r.wants_upgrade());
        assert!(matches!(parse("\u{1}\u{2} nonsense\r\n\r\n").await, Err(HeadError::Bad)));
        assert!(matches!(parse("GET / HTTP/1.1\r\nHost: x").await, Err(HeadError::Closed)));
        let big = format!("GET / HTTP/1.1\r\nX: {}\r\n", "a".repeat(HEAD_MAX + 100));
        assert!(matches!(parse(&big).await, Err(HeadError::TooLarge)));
    }

    #[test]
    fn writes_a_head_with_length_and_close() {
        let head = head_text(404, &[("content-type", "text/plain".to_owned())], Some(3));
        assert_eq!(head, "HTTP/1.1 404 Not Found\r\ncontent-type: text/plain\r\ncontent-length: 3\r\n\r\n");
    }
}
