use serde_json::{Map, Value};

pub fn int_of(v: &Value) -> Option<i64> {
    if let Some(i) = v.as_i64() {
        return (i.unsigned_abs() <= 9_007_199_254_740_991).then_some(i);
    }
    let f = v.as_f64()?;
    (f.is_finite() && f.fract() == 0.0 && f.abs() <= 9_007_199_254_740_991.0).then_some(f as i64)
}

pub fn num_of(v: &Value) -> Option<f64> {
    v.as_f64().filter(|f| f.is_finite())
}

pub fn collapse(s: &str, hit: impl Fn(char) -> bool, with: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut run = false;
    for c in s.chars() {
        if hit(c) {
            if !run {
                out.push_str(with);
            }
            run = true;
        } else {
            out.push(c);
            run = false;
        }
    }
    out
}

pub fn is_ctl(c: char) -> bool {
    (c as u32) < 0x20 || c as u32 == 0x7f
}

pub fn is_ctl_c1(c: char) -> bool {
    let n = c as u32;
    n < 0x20 || (0x7f..=0x9f).contains(&n)
}

pub fn is_bidi(c: char) -> bool {
    matches!(c as u32, 0x202a..=0x202e | 0x2066..=0x2069)
}

pub fn trim(s: &str) -> &str {
    core_transcript::jsstr::js_trim(s)
}

pub fn slice(s: &str, max: usize) -> String {
    crate::js::slice(s, max)
}

pub fn tail(s: &str, max: usize) -> String {
    crate::claude::tail(s, max).to_string()
}

pub fn len(s: &str) -> usize {
    crate::js::len(s)
}

pub fn chars(s: &str) -> usize {
    s.chars().count()
}

pub fn has_any(s: &str, set: &[char]) -> bool {
    s.chars().any(|c| set.contains(&c))
}

pub fn bad_path(s: &str) -> bool {
    has_any(s, &['\0', '\r', '\n'])
}

pub fn digits(s: &str, lo: usize, hi: usize) -> bool {
    (lo..=hi).contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit())
}

pub fn obj(v: &Value) -> Option<&Map<String, Value>> {
    v.as_object()
}

pub fn get<'a>(v: &'a Value, k: &str) -> &'a Value {
    v.get(k).unwrap_or(&Value::Null)
}

pub fn s<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::as_str)
}

pub fn index_key(k: &str) -> bool {
    !k.is_empty() && k.len() <= 10 && (k.len() == 1 || !k.starts_with('0')) && k.bytes().all(|b| b.is_ascii_digit()) && k.parse::<u64>().map(|n| n < 4_294_967_295).unwrap_or(false)
}

pub fn js_insert<T>(list: &mut Vec<(String, T)>, key: &str, v: T) {
    if let Some(slot) = list.iter_mut().find(|(k, _)| k == key) {
        slot.1 = v;
        return;
    }
    if index_key(key) {
        let n: u64 = key.parse().unwrap_or(0);
        let at = list.iter().position(|(k, _)| !index_key(k) || k.parse::<u64>().unwrap_or(0) > n).unwrap_or(list.len());
        list.insert(at, (key.to_string(), v));
    } else {
        list.push((key.to_string(), v));
    }
}

pub fn js_get<'a, T>(list: &'a [(String, T)], key: &str) -> Option<&'a T> {
    list.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

pub fn js_remove<T>(list: &mut Vec<(String, T)>, key: &str) -> Option<T> {
    let at = list.iter().position(|(k, _)| k == key)?;
    Some(list.remove(at).1)
}

pub fn js_sort_keys<T>(list: &mut Vec<(String, T)>) {
    let mut idx: Vec<(String, T)> = Vec::new();
    let mut rest: Vec<(String, T)> = Vec::new();
    for e in list.drain(..) {
        if index_key(&e.0) {
            idx.push(e);
        } else {
            rest.push(e);
        }
    }
    idx.sort_by_key(|(k, _)| k.parse::<u64>().unwrap_or(0));
    list.extend(idx);
    list.extend(rest);
}

pub fn locale_compare(a: &str, b: &str) -> std::cmp::Ordering {
    let fold = |s: &str| s.to_lowercase();
    fold(a).cmp(&fold(b)).then_with(|| core_transcript::jsstr::utf16_cmp(b, a))
}

pub fn to_fixed1(f: f64) -> String {
    crate::util::to_fixed1(f)
}

pub fn round(f: f64) -> i64 {
    (f + 0.5).floor() as i64
}
