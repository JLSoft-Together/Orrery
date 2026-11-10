use crate::run::android::jsre::{g, Js};
use std::collections::HashMap;

fn safe_char(code: Option<u32>, fallback: &str) -> String {
    code.filter(|c| *c > 0 && *c <= 0x10ffff).and_then(char::from_u32).map(String::from).unwrap_or_else(|| fallback.to_string())
}

pub fn decode_xml(s: &str) -> String {
    static ENT: Js = Js::new(r"&(#x[0-9a-fA-F]+|#\d+|[a-z]+);", "g");
    let re = ENT.re();
    let mut out = String::new();
    let mut last = 0;
    for m in re.find_iter(s) {
        let r = m.range();
        out.push_str(&s[last..r.start]);
        let whole = &s[r.clone()];
        let e = m.group(1).map(|x| &s[x]).unwrap_or("");
        let rep = if let Some(h) = e.strip_prefix("#x") {
            safe_char(u32::from_str_radix(h, 16).ok(), whole)
        } else if let Some(d) = e.strip_prefix('#') {
            safe_char(d.parse::<u32>().ok(), whole)
        } else {
            match e {
                "amp" => "&".into(),
                "lt" => "<".into(),
                "gt" => ">".into(),
                "quot" => "\"".into(),
                "apos" => "'".into(),
                _ => whole.to_string(),
            }
        };
        out.push_str(&rep);
        last = r.end;
    }
    out.push_str(&s[last..]);
    out
}

pub fn xml_attrs(body: &str) -> HashMap<String, String> {
    static ATTR: Js = Js::new(r#"([\w:.-]+)\s*=\s*(?:"([^"]*)"|'([^']*)')"#, "g");
    let mut out = HashMap::new();
    for m in ATTR.all(body) {
        let v = g(&m, 2).or_else(|| g(&m, 3)).unwrap_or("");
        out.insert(g(&m, 1).unwrap_or("").to_string(), decode_xml(v));
    }
    out
}
