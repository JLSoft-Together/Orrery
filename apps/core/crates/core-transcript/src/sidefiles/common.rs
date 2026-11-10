use crate::jsstr::iso_of_ms;
use crate::time::parse_iso;
use serde_json::Value;

pub fn uuid(b: &[u8]) -> bool {
    b.len() == 36
        && b.iter().enumerate().all(|(i, c)| match i {
            8 | 13 | 18 | 23 => *c == b'-',
            _ => c.is_ascii_hexdigit(),
        })
}

pub fn file_name(name: &str) -> bool {
    let b = name.as_bytes();
    b.len() == 41 && b[36..].eq_ignore_ascii_case(b".json") && uuid(&b[..36])
}

pub fn date(v: Option<&Value>) -> Option<String> {
    let s = v?.as_str()?;
    parse_iso(s).map(|_| s.to_owned())
}

pub fn iso(v: Option<&Value>) -> Option<String> {
    match v? {
        Value::Number(n) => {
            let n = n.as_f64()?;
            iso_of_ms(if n > 1e12 { n } else { n * 1000.0 })
        }
        Value::String(s) if s.encode_utf16().count() <= 64 => iso_of_ms(parse_iso(s)? as f64),
        _ => None,
    }
}

pub fn num(v: Option<&Value>) -> Option<f64> {
    v.and_then(Value::as_f64).filter(|n| n.is_finite())
}

pub fn pct(v: Option<&Value>) -> Option<f64> {
    num(v).filter(|n| (0.0..=100.0).contains(n))
}
