use crate::extract::extract_signals;
use crate::signal::Signal;
use serde::Deserialize;
use serde_json::Value;

const DEEP_STACK_BASE: usize = 16 * 1024 * 1024;
const DEEP_STACK_PER_LEVEL: usize = 2048;
const DEEP_STACK_MAX: usize = 1024 * 1024 * 1024;

pub enum Parsed {
    Value(Value),
    Deep(String),
    Bad,
}

fn hex4(b: &[u8]) -> Option<u16> {
    let s = std::str::from_utf8(b.get(..4)?).ok()?;
    u16::from_str_radix(s, 16).ok()
}

fn number_end(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && matches!(b[i], b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E') {
        i += 1;
    }
    i
}

pub fn sanitize(line: &str) -> Option<String> {
    let b = line.as_bytes();
    let mut out = String::with_capacity(line.len());
    let mut changed = false;
    let mut in_str = false;
    let mut last = 0;
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'"' => {
                in_str = !in_str;
                i += 1;
            }
            b'\\' if in_str => {
                if b.get(i + 1) != Some(&b'u') {
                    i += 2;
                    continue;
                }
                let Some(u) = hex4(&b[(i + 2).min(b.len())..]) else {
                    i += 2;
                    continue;
                };
                if (0xd800..0xdc00).contains(&u) && b.get(i + 6) == Some(&b'\\') && b.get(i + 7) == Some(&b'u') {
                    if let Some(lo) = hex4(&b[(i + 8).min(b.len())..]) {
                        if (0xdc00..0xe000).contains(&lo) {
                            i += 12;
                            continue;
                        }
                    }
                }
                if (0xd800..0xe000).contains(&u) {
                    out.push_str(&line[last..i]);
                    out.push_str("\\ufffd");
                    last = i + 6;
                    changed = true;
                }
                i += 6;
            }
            b'-' | b'0'..=b'9' if !in_str => {
                let end = number_end(b, i);
                if line[i..end].parse::<f64>().is_ok_and(|v| v.is_infinite()) {
                    out.push_str(&line[last..i]);
                    out.push_str("-1");
                    last = end;
                    changed = true;
                }
                i = end;
            }
            _ => i += 1,
        }
    }
    if !changed {
        return None;
    }
    out.push_str(&line[last..]);
    Some(out)
}

fn too_deep(e: &serde_json::Error) -> bool {
    e.to_string().contains("recursion limit")
}

fn attempt(line: &str) -> Result<Value, bool> {
    serde_json::from_str::<Value>(line).map_err(|e| too_deep(&e))
}

pub fn parse_line(line: &str) -> Parsed {
    let fixed = match attempt(line) {
        Ok(v) => return Parsed::Value(v),
        Err(true) => return Parsed::Deep(sanitize(line).unwrap_or_else(|| line.to_owned())),
        Err(false) => match sanitize(line) {
            Some(f) => f,
            None => return Parsed::Bad,
        },
    };
    match attempt(&fixed) {
        Ok(v) => Parsed::Value(v),
        Err(true) => Parsed::Deep(fixed),
        Err(false) => Parsed::Bad,
    }
}

fn depth_bound(line: &str) -> usize {
    let mut n = 0;
    let mut in_str = false;
    let mut esc = false;
    for &c in line.as_bytes() {
        if in_str {
            if esc {
                esc = false;
            } else if c == b'\\' {
                esc = true;
            } else if c == b'"' {
                in_str = false;
            }
        } else if c == b'"' {
            in_str = true;
        } else if c == b'[' || c == b'{' {
            n += 1;
        }
    }
    n
}

fn parse_unbounded(line: &str) -> Option<Value> {
    let mut de = serde_json::Deserializer::from_str(line);
    de.disable_recursion_limit();
    let v = Value::deserialize(&mut de).ok()?;
    de.end().ok()?;
    Some(v)
}

pub fn deep_signals(line: String) -> Vec<Signal> {
    let stack = DEEP_STACK_BASE.saturating_add(depth_bound(&line).saturating_mul(DEEP_STACK_PER_LEVEL));
    if stack > DEEP_STACK_MAX {
        return Vec::new();
    }
    let worker = std::thread::Builder::new().name("deep-json".into()).stack_size(stack).spawn(move || {
        let v = parse_unbounded(&line);
        let out = v.as_ref().map(extract_signals).unwrap_or_default();
        drop(v);
        out
    });
    worker.ok().and_then(|h| h.join().ok()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(line: &str) -> Value {
        match parse_line(line) {
            Parsed::Value(v) => v,
            _ => panic!("not a value"),
        }
    }

    #[test]
    fn replaces_only_lone_surrogates() {
        let v = value(r#"{"a":"x\ud800y","b":"😀","c":"\\ud800"}"#);
        assert_eq!(v["a"], "x\u{fffd}y");
        assert_eq!(v["b"], "😀");
        assert_eq!(v["c"], "\\ud800");
        assert!(matches!(parse_line("{broken"), Parsed::Bad));
    }

    #[test]
    fn out_of_range_numbers_read_like_infinity() {
        let v = value(r#"{"a":1e400,"b":-1E+999,"c":"1e400","d":[2,1e-400]}"#);
        assert_eq!(v["a"], -1);
        assert_eq!(v["b"], -1);
        assert_eq!(v["c"], "1e400");
        assert_eq!(v["d"][0], 2);
    }

    #[test]
    fn deep_lines_still_give_signals() {
        let nest = format!("{}{}", "[".repeat(5000), "]".repeat(5000));
        let line = format!(r#"{{"type":"user","timestamp":"2026-10-07T10:00:00.000Z","x":{nest},"message":{{"content":"hi"}}}}"#);
        let Parsed::Deep(text) = parse_line(&line) else { panic!("expected deep") };
        let kinds: Vec<&str> = deep_signals(text).iter().map(Signal::kind).collect();
        assert_eq!(kinds, vec!["seen", "prompt"]);
    }
}
