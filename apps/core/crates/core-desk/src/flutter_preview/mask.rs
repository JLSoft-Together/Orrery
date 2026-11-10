enum Frame {
    Code(i32),
    Str { q: Vec<u8>, raw: bool },
}

pub fn sub(s: &str, from: usize, to: usize) -> &str {
    let mut to = to.min(s.len());
    let mut from = from.min(to);
    while !s.is_char_boundary(to) {
        to -= 1;
    }
    while !s.is_char_boundary(from) {
        from += 1;
    }
    &s[from..to.max(from)]
}

fn is_word(c: u8) -> bool {
    c.is_ascii_alphanumeric() || c == b'_' || c == b'$'
}

fn blank(out: &mut [u8], from: usize, to: usize) {
    for k in from..to.min(out.len()) {
        if out[k] != b'\n' && out[k] != b'\r' {
            out[k] = b' ';
        }
    }
}

pub fn mask_dart(src: &str) -> String {
    let b = src.as_bytes();
    let n = b.len();
    let mut out = b.to_vec();
    let mut stack = vec![Frame::Code(0)];
    let mut str_start = 0usize;
    let mut i = 0usize;
    while i < n {
        let c = b[i];
        if matches!(stack.last(), Some(Frame::Code(_))) {
            if c == b'/' && b.get(i + 1) == Some(&b'/') {
                let end = b[i..].iter().position(|x| *x == b'\n').map_or(n, |p| i + p);
                if stack.len() == 1 {
                    blank(&mut out, i, end);
                }
                i = end;
                continue;
            }
            if c == b'/' && b.get(i + 1) == Some(&b'*') {
                let mut depth = 0;
                let mut j = i;
                while j < n {
                    if b[j] == b'/' && b.get(j + 1) == Some(&b'*') {
                        depth += 1;
                        j += 2;
                    } else if b[j] == b'*' && b.get(j + 1) == Some(&b'/') {
                        depth -= 1;
                        j += 2;
                        if depth == 0 {
                            break;
                        }
                    } else {
                        j += 1;
                    }
                }
                if stack.len() == 1 {
                    blank(&mut out, i, j);
                }
                i = j;
                continue;
            }
            if c == b'"' || c == b'\'' {
                let raw = i > 0 && b[i - 1] == b'r' && !(i >= 2 && is_word(b[i - 2]));
                let q: Vec<u8> = if b[i..].starts_with(&[c, c, c]) { vec![c; 3] } else { vec![c] };
                if stack.len() == 1 {
                    str_start = if raw { i - 1 } else { i };
                }
                i += q.len();
                stack.push(Frame::Str { q, raw });
                continue;
            }
            if c == b'{' {
                if let Some(Frame::Code(d)) = stack.last_mut() {
                    *d += 1;
                }
            } else if c == b'}' {
                let at_zero = matches!(stack.last(), Some(Frame::Code(0)));
                if at_zero && stack.len() > 1 {
                    stack.pop();
                    i += 1;
                    continue;
                }
                if let Some(Frame::Code(d)) = stack.last_mut() {
                    *d -= 1;
                }
            }
            i += 1;
            continue;
        }
        let Some(Frame::Str { q, raw }) = stack.last() else { break };
        let (q, raw) = (q.clone(), *raw);
        if !raw && c == b'\\' {
            i += 2;
            continue;
        }
        let closes = b[i..].starts_with(&q);
        let unterminated = q.len() == 1 && c == b'\n';
        if closes || unterminated {
            stack.pop();
            if closes {
                i += q.len();
            }
            if stack.len() == 1 {
                blank(&mut out, str_start, i);
            }
            if unterminated {
                i += 1;
            }
            continue;
        }
        if !raw && c == b'$' && b.get(i + 1) == Some(&b'{') {
            stack.push(Frame::Code(0));
            i += 2;
            continue;
        }
        i += 1;
    }
    if stack.len() > 1 {
        blank(&mut out, str_start, n);
    }
    String::from_utf8(out).unwrap_or_else(|e| e.into_bytes().iter().map(|x| if x.is_ascii() { *x as char } else { ' ' }).collect())
}

pub fn line_at(text: &str, pos: usize) -> u32 {
    1 + text.as_bytes()[..pos.min(text.len())].iter().filter(|c| **c == b'\n').count() as u32
}

pub fn match_close(text: &str, open: usize) -> Option<usize> {
    let b = text.as_bytes();
    let mut stack: Vec<u8> = Vec::new();
    for (k, c) in b.iter().enumerate().skip(open) {
        match c {
            b'(' => stack.push(b')'),
            b'[' => stack.push(b']'),
            b'{' => stack.push(b'}'),
            b')' | b']' | b'}' => {
                if stack.pop() != Some(*c) {
                    return None;
                }
                if stack.is_empty() {
                    return Some(k + 1);
                }
            }
            _ => {}
        }
    }
    None
}

pub fn top_ranges(text: &str) -> Vec<(usize, usize)> {
    let mut ranges = Vec::new();
    let mut depth = 0i32;
    let mut from = 0usize;
    for (k, c) in text.bytes().enumerate() {
        match c {
            b'(' | b'[' | b'{' | b'<' => depth += 1,
            b')' | b']' | b'}' | b'>' => depth -= 1,
            b',' if depth == 0 => {
                ranges.push((from, k));
                from = k + 1;
            }
            _ => {}
        }
    }
    ranges.push((from, text.len()));
    ranges
}

pub fn split_top(text: &str) -> Vec<String> {
    top_ranges(text).into_iter().map(|(a, b)| sub(text, a, b).trim().to_string()).filter(|s| !s.is_empty()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_comments_and_strings_keeping_offsets() {
        let src = "a // x\nb 'q' /* c */ r'\\' \"${'}'}\" d";
        let m = mask_dart(src);
        assert_eq!(m.len(), src.len());
        assert!(m.starts_with("a     \nb"));
        assert!(m.trim_end().ends_with('d'));
        assert!(!m.contains('q') && !m.contains('c'));
    }

    #[test]
    fn keeps_newlines_in_multiline_strings() {
        let m = mask_dart("x '''a\nb''' y");
        assert_eq!(m, "x     \n     y");
    }

    #[test]
    fn matches_brackets() {
        assert_eq!(match_close("(a[b]{c})x", 0), Some(9));
        assert_eq!(match_close("(a]", 0), None);
        assert_eq!(match_close("(a", 0), None);
    }

    #[test]
    fn splits_top_level_commas() {
        assert_eq!(split_top("a, Map<A, B> b, (c, d)"), vec!["a", "Map<A, B> b", "(c, d)"]);
    }

    #[test]
    fn counts_lines() {
        assert_eq!(line_at("a\nb\nc", 4), 3);
    }
}
