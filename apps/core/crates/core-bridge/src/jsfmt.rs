use serde_json::Value;

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

fn write(v: &Value, out: &mut String) {
    match v {
        Value::Number(n) => match (n.as_i64(), n.as_u64()) {
            (Some(i), _) if i.unsigned_abs() <= 1 << 53 => out.push_str(&i.to_string()),
            (_, Some(u)) if u <= 1 << 53 => out.push_str(&u.to_string()),
            _ => out.push_str(&number(n.as_f64().unwrap_or(f64::NAN))),
        },
        Value::Array(a) => {
            out.push('[');
            for (i, x) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write(x, out);
            }
            out.push(']');
        }
        Value::Object(m) => {
            out.push('{');
            for (i, (k, x)) in m.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&Value::String(k.clone()).to_string());
                out.push(':');
                write(x, out);
            }
            out.push('}');
        }
        other => out.push_str(&other.to_string()),
    }
}

pub fn stringify(v: &Value) -> String {
    let mut out = String::new();
    write(v, &mut out);
    out
}

pub fn num(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64).filter(|f| f.is_finite())
}

pub fn now_ms() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as f64).unwrap_or(0.0)
}

fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

pub fn iso_string(ms: f64) -> Option<String> {
    if !ms.is_finite() || ms.abs() > 8.64e15 {
        return None;
    }
    let ms = ms.trunc() as i64;
    let days = ms.div_euclid(86_400_000);
    let rest = ms.rem_euclid(86_400_000);
    let (y, m, d) = civil(days);
    let year = if (0..=9999).contains(&y) { format!("{y:04}") } else { format!("{}{:06}", if y < 0 { '-' } else { '+' }, y.abs()) };
    Some(format!("{}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z", year, m, d, rest / 3_600_000, rest / 60_000 % 60, rest / 1000 % 60, rest % 1000))
}

pub fn round(x: f64) -> f64 {
    let f = x.floor();
    if x - f >= 0.5 {
        f + 1.0
    } else {
        f
    }
}

pub fn to_fixed(v: f64, digits: usize) -> String {
    if !v.is_finite() {
        return number(v);
    }
    if v.abs() >= 1e21 {
        return number(v);
    }
    let exact = format!("{:.1100}", v.abs());
    let (int, frac) = exact.split_once('.').unwrap_or((&exact, ""));
    let keep = &frac[..digits];
    let rest = &frac[digits..];
    let up = rest.as_bytes().first().is_some_and(|b| *b >= b'5');
    let mut num: Vec<u8> = format!("{int}{keep}").into_bytes();
    if up {
        let mut i = num.len();
        loop {
            if i == 0 {
                num.insert(0, b'1');
                break;
            }
            i -= 1;
            if num[i] == b'9' {
                num[i] = b'0';
            } else {
                num[i] += 1;
                break;
            }
        }
    }
    let s = String::from_utf8(num).unwrap_or_default();
    let split = s.len() - digits;
    let body = if digits == 0 { s.clone() } else { format!("{}.{}", &s[..split], &s[split..]) };
    if v < 0.0 {
        format!("-{body}")
    } else {
        body
    }
}

pub fn parse_int(s: &str) -> Option<i64> {
    let t = s.trim_start_matches(crate::is_js_space);
    let (neg, rest) = match t.as_bytes().first() {
        Some(b'-') => (true, &t[1..]),
        Some(b'+') => (false, &t[1..]),
        _ => (false, t),
    };
    let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let n: i64 = digits.parse().unwrap_or(i64::MAX);
    Some(if neg { -n } else { n })
}

pub fn b64_decode(s: &str) -> Vec<u8> {
    let mut out = Vec::new();
    let mut acc: u32 = 0;
    let mut bits = 0;
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            b'=' => break,
            _ => continue,
        };
        acc = (acc << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_like_js() {
        assert_eq!(number(100.0), "100");
        assert_eq!(number(12.5), "12.5");
        assert_eq!(to_fixed(0.125, 2), "0.13");
        assert_eq!(to_fixed(1.005, 2), "1.00");
        assert_eq!(to_fixed(-0.001, 2), "-0.00");
        assert_eq!(to_fixed(2.0, 2), "2.00");
        assert_eq!(to_fixed(9.999, 2), "10.00");
        assert_eq!(round(2.5), 3.0);
        assert_eq!(round(-2.5), -2.0);
        assert_eq!(iso_string(0.0).unwrap(), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso_string(1.5).unwrap(), "1970-01-01T00:00:00.001Z");
        assert_eq!(iso_string(253402300800000.0).unwrap(), "+010000-01-01T00:00:00.000Z");
        assert!(iso_string(9e15).is_none());
        assert_eq!(parse_int(" 200000abc"), Some(200000));
        assert_eq!(parse_int("x"), None);
        assert_eq!(String::from_utf8(b64_decode("bm9kZSAieCI=")).unwrap(), "node \"x\"");
    }
}
