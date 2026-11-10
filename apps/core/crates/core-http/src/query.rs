fn hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

pub fn percent_decode(input: &str, plus_is_space: bool) -> Option<String> {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                let hi = hex(*bytes.get(i + 1)?)?;
                let lo = hex(*bytes.get(i + 2)?)?;
                out.push(hi * 16 + lo);
                i += 3;
            }
            b'+' if plus_is_space => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

fn percent_decode_lossy(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() && hex(bytes[i + 1]).is_some() && hex(bytes[i + 2]).is_some() => {
                out.push(hex(bytes[i + 1]).unwrap_or(0) * 16 + hex(bytes[i + 2]).unwrap_or(0));
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub struct Params(Vec<(String, String)>);

impl Params {
    pub fn parse(query: &str) -> Params {
        Params(
            query
                .split('&')
                .filter(|p| !p.is_empty())
                .map(|p| {
                    let (k, v) = p.split_once('=').unwrap_or((p, ""));
                    (percent_decode_lossy(k), percent_decode_lossy(v))
                })
                .collect(),
        )
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.0.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }
}

pub fn js_number(s: &str) -> Option<f64> {
    let t = crate::js::trim(s);
    if t.is_empty() {
        return Some(0.0);
    }
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        return u64::from_str_radix(hex, 16).ok().map(|n| n as f64);
    }
    let body = t.strip_prefix(['+', '-']).unwrap_or(t);
    let numeric = body.bytes().all(|b| b.is_ascii_digit() || matches!(b, b'.' | b'e' | b'E' | b'+' | b'-')) && body.bytes().any(|b| b.is_ascii_digit());
    if !numeric {
        return None;
    }
    t.parse::<f64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_percent_sequences_strictly() {
        assert_eq!(percent_decode("/a%20b", false).as_deref(), Some("/a b"));
        assert_eq!(percent_decode("/%E0%A4%A", false), None);
        assert_eq!(percent_decode("/%E0%A4", false), None);
        assert_eq!(percent_decode("%zz", false), None);
        assert_eq!(percent_decode("a+b", false).as_deref(), Some("a+b"));
        assert_eq!(percent_decode("a+b", true).as_deref(), Some("a b"));
        assert_eq!(percent_decode("d%E1%BB%B1", false).as_deref(), Some("d\u{1ef1}"));
    }

    #[test]
    fn query_params_follow_url_search_params() {
        let p = Params::parse("t=12%3A3&m=5&k=a+b%2Bc&empty=&flag");
        assert_eq!(p.get("t"), Some("12:3"));
        assert_eq!(p.get("m"), Some("5"));
        assert_eq!(p.get("k"), Some("a b+c"));
        assert_eq!(p.get("empty"), Some(""));
        assert_eq!(p.get("flag"), Some(""));
        assert_eq!(p.get("none"), None);
        assert_eq!(Params::parse("t=%zz").get("t"), Some("%zz"));
    }

    #[test]
    fn numbers_like_js_number() {
        assert_eq!(js_number("5"), Some(5.0));
        assert_eq!(js_number(" 5 "), Some(5.0));
        assert_eq!(js_number(""), Some(0.0));
        assert_eq!(js_number("5.0"), Some(5.0));
        assert_eq!(js_number("1e2"), Some(100.0));
        assert_eq!(js_number("0x10"), Some(16.0));
        assert_eq!(js_number("abc"), None);
        assert_eq!(js_number("5x"), None);
        assert_eq!(js_number("inf"), None);
        assert_eq!(js_number("NaN"), None);
        assert_eq!(js_number("-3"), Some(-3.0));
    }
}
