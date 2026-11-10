use serde_json::{Map, Value};

pub fn len(s: &str) -> usize {
    s.encode_utf16().count()
}

pub fn slice(s: &str, max: usize) -> String {
    let mut units = 0;
    let mut end = 0;
    for (i, c) in s.char_indices() {
        let n = c.len_utf16();
        if units + n > max {
            return s[..i].to_string();
        }
        units += n;
        end = i + c.len_utf8();
    }
    s[..end].to_string()
}

fn index_key(k: &str) -> Option<u64> {
    if k.is_empty() || k.len() > 10 || (k.len() > 1 && k.starts_with('0')) || !k.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    k.parse::<u64>().ok().filter(|n| *n < 4_294_967_295)
}

pub fn keys(map: &Map<String, Value>) -> Vec<&String> {
    let mut idx: Vec<(u64, &String)> = Vec::new();
    let mut rest: Vec<&String> = Vec::new();
    for k in map.keys() {
        match index_key(k) {
            Some(n) => idx.push((n, k)),
            None => rest.push(k),
        }
    }
    idx.sort_by_key(|(n, _)| *n);
    idx.into_iter().map(|(_, k)| k).chain(rest).collect()
}

pub fn entries(map: &Map<String, Value>) -> Vec<(&String, &Value)> {
    keys(map).into_iter().map(|k| (k, &map[k])).collect()
}

pub fn number(f: f64) -> String {
    if f.is_nan() || f.is_infinite() {
        return "null".into();
    }
    if f == 0.0 {
        return "0".into();
    }
    let sci = format!("{:e}", f);
    let (mant, exp) = sci.split_once('e').unwrap_or((&sci, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let neg = mant.starts_with('-');
    let digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
    let k = digits.len() as i32;
    let n = exp + 1;
    let body = if k <= n && n <= 21 {
        format!("{}{}", digits, "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{}", "0".repeat((-n) as usize), digits)
    } else {
        let e = n - 1;
        let sign = if e < 0 { '-' } else { '+' };
        if k == 1 {
            format!("{}e{}{}", digits, sign, e.abs())
        } else {
            format!("{}.{}e{}{}", &digits[..1], &digits[1..], sign, e.abs())
        }
    };
    if neg {
        format!("-{body}")
    } else {
        body
    }
}

fn num_text(n: &serde_json::Number) -> String {
    if let Some(i) = n.as_i64() {
        if i.unsigned_abs() <= (1u64 << 53) {
            return i.to_string();
        }
    }
    if let Some(u) = n.as_u64() {
        if u <= (1u64 << 53) {
            return u.to_string();
        }
    }
    number(n.as_f64().unwrap_or(f64::NAN))
}

fn quote(s: &str, out: &mut String) {
    out.push_str(&Value::String(s.to_string()).to_string());
}

fn write(v: &Value, indent: &str, level: usize, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&num_text(n)),
        Value::String(s) => quote(s, out),
        Value::Array(a) => {
            if a.is_empty() {
                out.push_str("[]");
                return;
            }
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                newline(indent, level + 1, out);
                write(x, indent, level + 1, out);
            }
            newline(indent, level, out);
            out.push(']');
        }
        Value::Object(m) => {
            if m.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push('{');
            for (i, (k, x)) in entries(m).into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                newline(indent, level + 1, out);
                quote(k, out);
                out.push(':');
                if !indent.is_empty() {
                    out.push(' ');
                }
                write(x, indent, level + 1, out);
            }
            newline(indent, level, out);
            out.push('}');
        }
    }
}

fn newline(indent: &str, level: usize, out: &mut String) {
    if indent.is_empty() {
        return;
    }
    out.push('\n');
    for _ in 0..level {
        out.push_str(indent);
    }
}

pub fn stringify(v: &Value) -> String {
    let mut out = String::new();
    write(v, "", 0, &mut out);
    out
}

pub fn stringify_indent(v: &Value, indent: &str) -> String {
    let mut out = String::new();
    write(v, indent, 0, &mut out);
    out
}

pub fn parse(text: &str) -> Option<Value> {
    serde_json::from_str(text.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn numbers_print_like_js() {
        assert_eq!(number(1.0), "1");
        assert_eq!(number(1.5), "1.5");
        assert_eq!(number(-0.000001), "-0.000001");
        assert_eq!(number(1e-7), "1e-7");
        assert_eq!(number(1e21), "1e+21");
        assert_eq!(number(123456789012345680000.0), "123456789012345680000");
        assert_eq!(number(1.25e-10), "1.25e-10");
        assert_eq!(stringify(&parse("12345678901234567890").unwrap()), "12345678901234567000");
    }

    #[test]
    fn index_keys_come_first() {
        let v = parse(r#"{"b":1,"2":2,"1":3,"01":4}"#).unwrap();
        assert_eq!(stringify(&v), r#"{"1":3,"2":2,"b":1,"01":4}"#);
        assert_eq!(stringify_indent(&json!({"a":[1,{}],"b":[]}), " "), "{\n \"a\": [\n  1,\n  {}\n ],\n \"b\": []\n}");
    }

    #[test]
    fn utf16_lengths() {
        assert_eq!(len("a😀"), 3);
        assert_eq!(slice("a😀b", 2), "a");
        assert_eq!(slice("a😀b", 3), "a😀");
    }
}
