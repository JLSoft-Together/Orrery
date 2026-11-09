use std::io::{ErrorKind, Read};

pub const LINE_MAX: usize = 1024 * 1024;
const CHUNK: usize = 64 * 1024;

pub struct Lines<R> {
    reader: R,
    buf: Vec<u8>,
    scanned: usize,
    skipping: bool,
    eof: bool,
}

fn blank(line: &[u8]) -> bool {
    line.iter().all(u8::is_ascii_whitespace)
}

impl<R: Read> Lines<R> {
    pub fn new(reader: R) -> Lines<R> {
        Lines { reader, buf: Vec::new(), scanned: 0, skipping: false, eof: false }
    }

    fn take_line(&mut self, end: usize) -> Option<Vec<u8>> {
        let cut = (end + 1).min(self.buf.len());
        let mut line: Vec<u8> = self.buf.drain(..cut).collect();
        if line.len() > end {
            line.truncate(end);
        }
        self.scanned = 0;
        if std::mem::take(&mut self.skipping) || line.len() > LINE_MAX {
            return None;
        }
        if line.last() == Some(&b'\r') {
            line.pop();
        }
        (!blank(&line)).then_some(line)
    }

    pub fn next_line(&mut self) -> Option<Vec<u8>> {
        loop {
            if let Some(i) = self.buf[self.scanned..].iter().position(|&b| b == b'\n') {
                let end = self.scanned + i;
                match self.take_line(end) {
                    Some(line) => return Some(line),
                    None => continue,
                }
            }
            self.scanned = self.buf.len();
            if self.buf.len() > LINE_MAX {
                self.buf.clear();
                self.scanned = 0;
                self.skipping = true;
            }
            if self.eof {
                if self.buf.is_empty() {
                    return None;
                }
                let end = self.buf.len();
                return self.take_line(end).or_else(|| self.next_line());
            }
            let mut chunk = [0u8; CHUNK];
            match self.reader.read(&mut chunk) {
                Ok(0) => self.eof = true,
                Ok(n) => self.buf.extend_from_slice(&chunk[..n]),
                Err(e) if e.kind() == ErrorKind::Interrupted => {}
                Err(_) => self.eof = true,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(input: &[u8]) -> Vec<String> {
        let mut l = Lines::new(input);
        let mut out = Vec::new();
        while let Some(line) = l.next_line() {
            out.push(String::from_utf8(line).unwrap());
        }
        out
    }

    #[test]
    fn splits_lines_strips_cr_and_skips_blank() {
        assert_eq!(all(b"a\r\n\n  \nb\nlast"), vec!["a", "b", "last"]);
        assert!(all(b"").is_empty());
    }

    #[test]
    fn drops_lines_over_one_mib_and_keeps_the_next() {
        let mut input = vec![b'x'; LINE_MAX + 10];
        input.extend_from_slice(b"\nok\n");
        input.extend(vec![b'y'; 3 * LINE_MAX]);
        input.extend_from_slice(b"\nok2\n");
        input.extend(vec![b'z'; LINE_MAX + 1]);
        assert_eq!(all(&input), vec!["ok", "ok2"]);
        let mut exact = vec![b'e'; LINE_MAX];
        exact.push(b'\n');
        assert_eq!(all(&exact).len(), 1);
    }
}
