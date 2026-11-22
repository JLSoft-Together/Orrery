use std::time::Instant;

pub const OSC_MAX: usize = 16 * 1024;
pub const COMMAND_MAX: usize = 2000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Mark {
    PromptStart,
    CommandStart,
    Executed,
    Finished(Option<i64>),
    CommandLine { command: String, nonce: Option<String> },
    Cwd(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Ground,
    Esc,
    Body,
    BodyEsc,
    Skip,
    SkipEsc,
}

#[derive(Debug)]
pub struct OscScanner {
    state: State,
    buf: Vec<u8>,
}

impl Default for OscScanner {
    fn default() -> Self {
        OscScanner { state: State::Ground, buf: Vec::new() }
    }
}

impl OscScanner {
    pub fn push(&mut self, chunk: &[u8]) -> Vec<Mark> {
        let mut out = Vec::new();
        let mut i = 0;
        while i < chunk.len() {
            if self.state == State::Ground {
                match chunk[i..].iter().position(|b| *b == 0x1b) {
                    Some(at) => {
                        i += at + 1;
                        self.state = State::Esc;
                    }
                    None => break,
                }
                continue;
            }
            let b = chunk[i];
            i += 1;
            self.state = match self.state {
                State::Esc if b == b']' => {
                    self.buf.clear();
                    State::Body
                }
                State::Esc if b == 0x1b => State::Esc,
                State::Esc => State::Ground,
                State::Body | State::Skip if b == 0x07 => {
                    self.finish(&mut out);
                    State::Ground
                }
                State::Body => match b {
                    0x1b => State::BodyEsc,
                    _ if self.buf.len() >= OSC_MAX || !self.wanted(b) => {
                        self.buf.clear();
                        State::Skip
                    }
                    _ => {
                        self.buf.push(b);
                        State::Body
                    }
                },
                State::Skip if b == 0x1b => State::SkipEsc,
                State::Skip => State::Skip,
                State::BodyEsc | State::SkipEsc if b == b'\\' => {
                    self.finish(&mut out);
                    State::Ground
                }
                State::BodyEsc | State::SkipEsc if b == b']' => {
                    self.buf.clear();
                    State::Body
                }
                State::BodyEsc | State::SkipEsc => {
                    self.buf.clear();
                    State::Ground
                }
                State::Ground => State::Ground,
            };
        }
        out
    }

    fn wanted(&self, b: u8) -> bool {
        let n = self.buf.len();
        if n >= 4 {
            return true;
        }
        let ok = |p: &[u8]| p[..n] == self.buf[..] && p[n] == b;
        ok(b"633;") || ok(b"133;")
    }

    fn finish(&mut self, out: &mut Vec<Mark>) {
        let body = std::mem::take(&mut self.buf);
        if let Some(m) = std::str::from_utf8(&body).ok().and_then(parse_osc) {
            out.push(m);
        }
    }
}

pub fn parse_osc(body: &str) -> Option<Mark> {
    let (vscode, rest) = match body.split_at_checked(4)? {
        ("633;", r) => (true, r),
        ("133;", r) => (false, r),
        _ => return None,
    };
    let (code, arg) = match rest.split_once(';') {
        Some((c, a)) => (c, Some(a)),
        None => (rest, None),
    };
    match code {
        "A" => Some(Mark::PromptStart),
        "B" => Some(Mark::CommandStart),
        "C" => Some(Mark::Executed),
        "D" => Some(Mark::Finished(arg.and_then(exit_code))),
        "E" if vscode => {
            let raw = arg.unwrap_or("");
            let (cmd, nonce) = match raw.split_once(';') {
                Some((c, n)) => (c, Some(n.to_owned())),
                None => (raw, None),
            };
            Some(Mark::CommandLine { command: unescape(cmd).chars().take(COMMAND_MAX).collect(), nonce })
        }
        "P" if vscode => arg?.strip_prefix("Cwd=").map(|v| Mark::Cwd(unescape(v))),
        _ => None,
    }
}

fn exit_code(arg: &str) -> Option<i64> {
    let first = arg.split(';').next()?;
    let digits = first.strip_prefix('-').unwrap_or(first);
    if digits.is_empty() || digits.len() > 10 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    first.parse().ok()
}

pub fn unescape(text: &str) -> String {
    let b = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\' {
            if b.get(i + 1) == Some(&b'\\') {
                out.push(b'\\');
                i += 2;
                continue;
            }
            if b.get(i + 1) == Some(&b'x') {
                if let Some(v) = b.get(i + 2..i + 4).and_then(|h| std::str::from_utf8(h).ok()).and_then(|h| u8::from_str_radix(h, 16).ok()) {
                    out.push(v);
                    i += 4;
                    continue;
                }
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CmdEvent {
    Started { command: String },
    Finished { command: String, code: i64, ms: u64, cwd: Option<String> },
}

#[derive(Debug)]
pub struct CommandClock {
    nonce: String,
    command: String,
    started: Option<Instant>,
    cwd: Option<String>,
}

impl CommandClock {
    pub fn new(nonce: &str) -> Self {
        CommandClock { nonce: nonce.to_owned(), command: String::new(), started: None, cwd: None }
    }

    pub fn on_mark(&mut self, mark: Mark, now: Instant) -> Option<CmdEvent> {
        match mark {
            Mark::PromptStart => {
                self.started = None;
                self.command.clear();
                None
            }
            Mark::CommandStart => None,
            Mark::CommandLine { command, nonce } => {
                if nonce.as_deref() == Some(self.nonce.as_str()) {
                    self.command = command.trim().to_owned();
                }
                None
            }
            Mark::Cwd(dir) => {
                self.cwd = Some(dir).filter(|d| !d.is_empty());
                None
            }
            Mark::Executed => {
                self.started = Some(now);
                Some(CmdEvent::Started { command: self.command.clone() })
            }
            Mark::Finished(code) => {
                let started = self.started.take();
                let command = std::mem::take(&mut self.command);
                let (start, code) = (started?, code?);
                let ms = u64::try_from(now.saturating_duration_since(start).as_millis()).unwrap_or(u64::MAX);
                Some(CmdEvent::Finished { command, code, ms, cwd: self.cwd.clone() })
            }
        }
    }
}

#[derive(Debug)]
pub struct Marks {
    scanner: OscScanner,
    clock: CommandClock,
}

impl Marks {
    pub fn new(nonce: &str) -> Self {
        Marks { scanner: OscScanner::default(), clock: CommandClock::new(nonce) }
    }

    pub fn feed(&mut self, chunk: &[u8], now: Instant) -> Vec<CmdEvent> {
        let marks = self.scanner.push(chunk);
        marks.into_iter().filter_map(|m| self.clock.on_mark(m, now)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn scan(chunks: &[&[u8]]) -> Vec<Mark> {
        let mut s = OscScanner::default();
        chunks.iter().flat_map(|c| s.push(c)).collect()
    }

    #[test]
    fn whole_marks_are_read_with_bel_and_st() {
        let out = scan(&[b"x\x1b]633;A\x07$ \x1b]633;B\x1b\\ls\r\n\x1b]633;C\x07out\x1b]633;D;0\x07"]);
        assert_eq!(out, vec![Mark::PromptStart, Mark::CommandStart, Mark::Executed, Mark::Finished(Some(0))]);
    }

    #[test]
    fn a_mark_cut_anywhere_between_chunks_still_parses() {
        let whole: &[u8] = b"ab\x1b]633;E;npm run build;n0nce\x07\x1b]633;C\x07\x1b]633;D;2\x1b\\";
        let want = vec![Mark::CommandLine { command: "npm run build".into(), nonce: Some("n0nce".into()) }, Mark::Executed, Mark::Finished(Some(2))];
        for cut in 0..whole.len() {
            for cut2 in cut..whole.len() {
                assert_eq!(scan(&[&whole[..cut], &whole[cut..cut2], &whole[cut2..]]), want, "cut at {cut}/{cut2}");
            }
        }
    }

    #[test]
    fn byte_by_byte_feeding_works() {
        let whole: &[u8] = b"\x1b]133;D;137\x07\x1b]633;P;Cwd=C:\\x5cwork\\x3bapi\x07";
        let mut s = OscScanner::default();
        let out: Vec<Mark> = whole.iter().flat_map(|b| s.push(std::slice::from_ref(b))).collect();
        assert_eq!(out, vec![Mark::Finished(Some(137)), Mark::Cwd("C:\\work;api".into())]);
    }

    #[test]
    fn other_osc_and_csi_are_ignored_and_do_not_leak() {
        let out = scan(&[b"\x1b]0;title\x07\x1b[31mred\x1b]7;file://h/tmp\x1b\\\x1b]633;C\x07"]);
        assert_eq!(out, vec![Mark::Executed]);
    }

    #[test]
    fn exit_codes_are_numbers_or_none() {
        assert_eq!(parse_osc("633;D"), Some(Mark::Finished(None)));
        assert_eq!(parse_osc("633;D;-1"), Some(Mark::Finished(Some(-1))));
        assert_eq!(parse_osc("133;D;1;aid=9"), Some(Mark::Finished(Some(1))));
        assert_eq!(parse_osc("633;D;x"), Some(Mark::Finished(None)));
        assert_eq!(parse_osc("633;D;99999999999"), Some(Mark::Finished(None)));
        assert_eq!(parse_osc("133;E;ls"), None);
        assert_eq!(parse_osc("633;Q"), None);
    }

    #[test]
    fn values_are_unescaped_like_vscode() {
        assert_eq!(unescape("a\\x3bb\\x0ac\\\\d\\xZZ"), "a;b\nc\\d\\xZZ");
        assert_eq!(parse_osc("633;P;Cwd=/home/u/my\\x20dir"), Some(Mark::Cwd("/home/u/my dir".into())));
        assert_eq!(parse_osc("633;P;Other=1"), None);
    }

    #[test]
    fn an_oversized_osc_is_dropped_and_the_next_one_still_parses() {
        let mut big = b"\x1b]633;E;".to_vec();
        big.extend(std::iter::repeat_n(b'a', OSC_MAX + 10));
        big.extend_from_slice(b"\x07\x1b]633;C\x07");
        assert_eq!(scan(&[&big]), vec![Mark::Executed]);
    }

    #[test]
    fn the_clock_reports_start_and_end_with_exit_code_time_and_cwd() {
        let t0 = Instant::now();
        let mut m = Marks::new("n0nce");
        assert!(m.feed(b"\x1b]633;P;Cwd=/w\x07\x1b]633;A\x07$ \x1b]633;B\x07", t0).is_empty());
        let started = m.feed(b"\x1b]633;E;sleep 12;n0nce\x07\x1b]633;C\x07", t0);
        assert_eq!(started, vec![CmdEvent::Started { command: "sleep 12".into() }]);
        let done = m.feed(b"\x1b]633;D;0\x07", t0 + Duration::from_millis(12_345));
        assert_eq!(done, vec![CmdEvent::Finished { command: "sleep 12".into(), code: 0, ms: 12_345, cwd: Some("/w".into()) }]);
    }

    #[test]
    fn a_command_line_without_the_nonce_is_not_trusted() {
        let t0 = Instant::now();
        let mut m = Marks::new("n0nce");
        m.feed(b"\x1b]633;A\x07\x1b]633;E;rm -rf;evil\x07\x1b]633;C\x07", t0);
        let done = m.feed(b"\x1b]633;D;1\x07", t0);
        assert_eq!(done, vec![CmdEvent::Finished { command: String::new(), code: 1, ms: 0, cwd: None }]);
    }

    #[test]
    fn nothing_ran_means_no_finished_event() {
        let t0 = Instant::now();
        let mut m = Marks::new("n");
        assert!(m.feed(b"\x1b]633;A\x07\x1b]633;D\x07\x1b]633;A\x07\x1b]633;D;0\x07", t0).is_empty());
        m.feed(b"\x1b]633;C\x07", t0);
        assert!(m.feed(b"\x1b]633;D\x07", t0).is_empty());
    }
}
