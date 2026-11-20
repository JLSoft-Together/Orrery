use vt100::Parser;

pub const SCROLLBACK: usize = 2000;
pub const SNAP_MAX: usize = 512 * 1024;
pub const CAP: &str = "pty.snapshot";

pub struct Vt {
    parser: Parser,
}

fn escaped_len(line: &[u8]) -> usize {
    line.len() + 5 * line.iter().filter(|b| **b < 0x20 || **b == b'"' || **b == b'\\').count() + 6
}

impl Vt {
    pub fn new(cols: u16, rows: u16) -> Vt {
        Vt { parser: Parser::new(rows.max(1), cols.max(1), SCROLLBACK) }
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
    }

    pub fn size(&self) -> (u16, u16) {
        let (rows, cols) = self.parser.screen().size();
        (cols, rows)
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        let (cols, rows) = (cols.max(1), rows.max(1));
        let screen = self.parser.screen();
        let (old_rows, _) = screen.size();
        let (row, col) = screen.cursor_position();
        if rows < old_rows && row >= rows && !screen.alternate_screen() {
            let lift = row + 1 - rows;
            let mut seq = format!("\x18\x1b[{old_rows};1H").into_bytes();
            seq.extend(std::iter::repeat_n(b'\n', usize::from(lift)));
            seq.extend(format!("\x1b[{};{}H", row + 1 - lift, col + 1).into_bytes());
            self.parser.process(&seq);
        }
        if (rows, cols) != self.parser.screen().size() {
            self.parser.screen_mut().set_size(rows, cols);
        }
    }

    fn lines(&mut self) -> (Vec<Vec<u8>>, usize) {
        let rows = usize::from(self.parser.screen().size().0);
        self.parser.screen_mut().set_scrollback(usize::MAX);
        let total = self.parser.screen().scrollback();
        let mut lines = Vec::with_capacity(total + rows);
        let mut offset = total;
        while offset > 0 {
            self.parser.screen_mut().set_scrollback(offset);
            let take = offset.min(rows);
            lines.extend(self.parser.screen().rows_formatted(0, u16::MAX).take(take));
            offset -= take;
        }
        self.parser.screen_mut().set_scrollback(0);
        lines.extend(self.parser.screen().rows_formatted(0, u16::MAX));
        (lines, total)
    }

    pub fn snapshot(&mut self) -> String {
        if self.parser.screen().alternate_screen() {
            let mut out = b"\x1b[?1049h".to_vec();
            out.extend(self.parser.screen().state_formatted());
            return text(out);
        }
        let (lines, history) = self.lines();
        let mut budget: usize = lines.iter().map(|l| escaped_len(l)).sum();
        let mut skip = 0;
        while skip < history && budget > SNAP_MAX {
            budget -= escaped_len(&lines[skip]);
            skip += 1;
        }
        let mut out = Vec::with_capacity(budget);
        for (i, line) in lines[skip..].iter().enumerate() {
            if i > 0 {
                out.extend_from_slice(b"\x1b[m\r\n");
            }
            out.extend_from_slice(b"\x1b[m");
            out.extend_from_slice(line);
        }
        let screen = self.parser.screen();
        let (_, cols) = screen.size();
        let (row, col) = screen.cursor_position();
        out.extend(format!("\x1b[m\x1b[{};{}H", row + 1, col.min(cols.saturating_sub(1)) + 1).into_bytes());
        out.extend(screen.attributes_formatted());
        out.extend(screen.input_mode_formatted());
        if screen.hide_cursor() {
            out.extend_from_slice(b"\x1b[?25l");
        }
        text(out)
    }
}

fn text(bytes: Vec<u8>) -> String {
    String::from_utf8(bytes).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())
}
