use crate::time::civil_from_days;
use serde_json::{Map, Value};
use std::cmp::Ordering;

pub fn js_space(c: char) -> bool {
    matches!(c, '\u{9}'..='\u{d}' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

pub fn js_trim(s: &str) -> &str {
    s.trim_matches(js_space)
}

pub fn word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

pub fn units(s: &str) -> usize {
    s.chars().map(char::len_utf16).sum()
}

pub fn head_end(s: &str, max: usize) -> usize {
    let mut n = 0;
    for (i, c) in s.char_indices() {
        n += c.len_utf16();
        if n > max {
            return i;
        }
    }
    s.len()
}

pub fn js_slice(s: &str, max: usize) -> &str {
    &s[..head_end(s, max)]
}

pub fn utf16_cmp(a: &str, b: &str) -> Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

pub fn later(a: &str, b: &str) -> bool {
    utf16_cmp(a, b) == Ordering::Greater
}

pub fn num(v: f64) -> Value {
    if v.fract() == 0.0 && v.abs() < 9_007_199_254_740_992.0 {
        if v >= 0.0 {
            return Value::from(v as u64);
        }
        return Value::from(v as i64);
    }
    serde_json::Number::from_f64(v).map_or(Value::Null, Value::Number)
}

pub fn iso_of_ms(ms: f64) -> Option<String> {
    let t = ms.trunc();
    if !t.is_finite() || t.abs() > 8.64e15 {
        return None;
    }
    let t = t as i64;
    let days = t.div_euclid(86_400_000);
    let rem = t.rem_euclid(86_400_000);
    let (y, m, d) = civil_from_days(days);
    let year = if (0..=9999).contains(&y) { format!("{y:04}") } else if y < 0 { format!("-{:06}", -y) } else { format!("+{y:06}") };
    Some(format!("{year}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z", rem / 3_600_000, rem / 60_000 % 60, rem / 1000 % 60, rem % 1000))
}

pub fn put(m: &mut Map<String, Value>, k: &str, v: impl Into<Value>) {
    m.insert(k.to_owned(), v.into());
}

pub fn put_opt(m: &mut Map<String, Value>, k: &str, v: &Option<String>) {
    if let Some(v) = v {
        m.insert(k.to_owned(), Value::from(v.as_str()));
    }
}

pub fn put_num(m: &mut Map<String, Value>, k: &str, v: Option<f64>) {
    if let Some(v) = v {
        m.insert(k.to_owned(), num(v));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slices_and_trims_like_js() {
        assert_eq!(js_trim("\u{feff} a \u{3000}"), "a");
        assert_eq!(js_slice("a😀b", 2), "a");
        assert_eq!(js_slice("a😀b", 3), "a😀");
        assert_eq!(units("a😀"), 3);
        assert!(later("\u{ffff}", "😀"));
    }

    #[test]
    fn formats_iso_like_date() {
        assert_eq!(iso_of_ms(1_800_000_000_000.0).unwrap(), "2027-01-15T08:00:00.000Z");
        assert_eq!(iso_of_ms(1.9).unwrap(), "1970-01-01T00:00:00.001Z");
        assert_eq!(iso_of_ms(-1.0).unwrap(), "1969-12-31T23:59:59.999Z");
        assert_eq!(iso_of_ms(8.64e15).unwrap(), "+275760-09-13T00:00:00.000Z");
        assert!(iso_of_ms(8.64e15 + 1.0).is_none());
    }

    #[test]
    fn numbers_stay_integers_when_whole() {
        assert_eq!(num(3.0).to_string(), "3");
        assert_eq!(num(2.5).to_string(), "2.5");
    }
}
