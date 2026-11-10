use crate::js;
use crate::release_check::xml::{decode_xml, xml_attrs};
use crate::run::android::jsre::{g, groups, Js};
use serde_json::{json, Value};

pub const PREF_TYPES: [&str; 6] = ["string", "int", "long", "float", "boolean", "set"];
const ENTRIES_MAX: usize = 5000;
const STRING_MAX: usize = 100_000;
const SET_MAX: usize = 1000;
const KEY_MAX: usize = 1024;

struct Located {
    key: String,
    ty: String,
    value: Value,
    start: usize,
    end: usize,
}

fn bad(s: &str) -> bool {
    s.chars().any(|c| matches!(c as u32, 0..=8 | 0xb | 0xc | 0xe..=0x1f | 0xfffe | 0xffff))
}

fn locate(xml: &str) -> Option<Vec<Located>> {
    static MAP: Js = Js::new(r"<map\b[^>]*>([\s\S]*)<\/map\s*>|<map\b[^>]*\/>", "");
    static COMMENT: Js = Js::new(r"<!--[\s\S]*?-->", "g");
    static ELEMENT: Js = Js::new(r"<(string|int|long|float|boolean|set|null)\b([^>]*?)(\/>|>([\s\S]*?)<\/\1\s*>)", "g");
    static INNER: Js = Js::new(r"<string>([\s\S]*?)<\/string\s*>", "g");
    let map = MAP.re().find(xml)?;
    let mg = groups(&map, xml);
    let Some(inner) = g(&mg, 1) else { return Some(Vec::new()) };
    let whole = &xml[map.range()];
    let base = map.start() + whole.find(inner).unwrap_or(0);
    let mut body = String::with_capacity(inner.len());
    let mut last = 0;
    for c in COMMENT.re().find_iter(inner) {
        body.push_str(&inner[last..c.start()]);
        body.push_str(&" ".repeat(c.end() - c.start()));
        last = c.end();
    }
    body.push_str(&inner[last..]);
    let mut out = Vec::new();
    for m in ELEMENT.re().find_iter(&body) {
        if out.len() >= ENTRIES_MAX {
            break;
        }
        let gs = groups(&m, &body);
        let ty = g(&gs, 1).unwrap_or("");
        if ty == "null" {
            continue;
        }
        let attrs = xml_attrs(g(&gs, 2).unwrap_or(""));
        let Some(key) = attrs.get("name") else { continue };
        let inner_text = g(&gs, 4).unwrap_or("");
        let value = match ty {
            "set" => Value::Array(INNER.all(inner_text).iter().map(|s| json!(decode_xml(g(s, 1).unwrap_or("")))).collect()),
            "string" => json!(decode_xml(inner_text)),
            _ => json!(attrs.get("value").cloned().unwrap_or_default()),
        };
        out.push(Located { key: key.clone(), ty: ty.to_string(), value, start: base + m.start(), end: base + m.end() });
    }
    Some(out)
}

pub fn parse_prefs(xml: &str) -> Option<Vec<Value>> {
    locate(xml).map(|l| l.into_iter().map(|e| json!({"key": e.key, "type": e.ty, "value": e.value})).collect())
}

fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

pub fn pref_element(key: &str, ty: &str, value: &Value) -> String {
    let name = esc(key);
    match ty {
        "string" => format!("<string name=\"{name}\">{}</string>", esc(value.as_str().unwrap_or(""))),
        "set" => format!("<set name=\"{name}\">{}</set>", value.as_array().map(|a| a.iter().map(|v| format!("<string>{}</string>", esc(v.as_str().unwrap_or("")))).collect::<String>()).unwrap_or_default()),
        _ => format!("<{ty} name=\"{name}\" value=\"{}\" />", esc(value.as_str().unwrap_or(""))),
    }
}

pub fn clean_pref_value(ty: &str, v: &Value) -> Option<Value> {
    static INT: Js = Js::new(r"^-?\d{1,19}$", "");
    static FLOAT: Js = Js::new(r"^-?(\d+(\.\d*)?|\.\d+)([eE][-+]?\d+)?$", "");
    if ty == "set" {
        let a = v.as_array().filter(|a| a.len() <= SET_MAX)?;
        return a.iter().all(|s| s.as_str().is_some_and(|s| js::len(s) <= STRING_MAX && !bad(s))).then(|| v.clone());
    }
    let raw = v.as_str()?;
    let s = if ty == "string" { raw } else { raw.trim() };
    match ty {
        "string" => (js::len(s) <= STRING_MAX && !bad(s)).then(|| json!(s)),
        "boolean" => (s == "true" || s == "false").then(|| json!(s)),
        "int" => {
            if !INT.test(s) {
                return None;
            }
            let n: i128 = s.parse().ok()?;
            ((-2147483648..=2147483647).contains(&n)).then(|| json!(n.to_string()))
        }
        "long" => {
            if !INT.test(s) {
                return None;
            }
            let n: i128 = s.parse().ok()?;
            (-(1i128 << 63)..(1i128 << 63)).contains(&n).then(|| json!(n.to_string()))
        }
        "float" => (FLOAT.test(s) && s.parse::<f64>().is_ok_and(f64::is_finite)).then(|| json!(s)),
        _ => None,
    }
}

pub struct PrefEdit {
    pub key: String,
    pub ty: String,
    pub before: Value,
    pub after: Value,
}

pub fn clean_pref_edit(v: &Value) -> Option<PrefEdit> {
    let e = v.as_object()?;
    let key = e.get("key")?.as_str().filter(|k| !k.is_empty() && js::len(k) <= KEY_MAX && !bad(k))?;
    let ty = e.get("type")?.as_str().filter(|t| PREF_TYPES.contains(t))?;
    let before = e.get("before")?;
    let before_ok = if ty == "set" { before.as_array().is_some_and(|a| a.iter().all(Value::is_string)) } else { before.is_string() };
    let after = clean_pref_value(ty, e.get("after")?)?;
    before_ok.then(|| PrefEdit { key: key.to_string(), ty: ty.to_string(), before: before.clone(), after })
}

pub fn apply_pref_edit(xml: &str, edit: &PrefEdit) -> Result<String, &'static str> {
    let entries = locate(xml).ok_or("invalid")?;
    let hit: Vec<&Located> = entries.iter().filter(|e| e.key == edit.key).collect();
    if hit.len() != 1 || hit[0].ty != edit.ty || hit[0].value != edit.before {
        return Err("changed");
    }
    let e = hit[0];
    Ok(format!("{}{}{}", &xml[..e.start], pref_element(&edit.key, &edit.ty, &edit.after), &xml[e.end..]))
}

pub fn pref_shown(v: &Value) -> String {
    match v.as_array() {
        Some(a) => format!("[{}]", a.iter().map(|x| x.as_str().unwrap_or("").to_string()).collect::<Vec<_>>().join(", ")),
        None => v.as_str().unwrap_or("").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &str = "<?xml version='1.0'?>\n<map>\n    <string name=\"a\">x &amp; y</string>\n    <int name=\"n\" value=\"3\" />\n    <!-- <int name=\"c\" value=\"9\" /> -->\n    <set name=\"s\"><string>p</string><string>q</string></set>\n    <null name=\"z\" />\n</map>\n";

    #[test]
    fn parses_and_edits_one_element() {
        let e = parse_prefs(XML).unwrap();
        assert_eq!(e.len(), 3);
        assert_eq!(e[0]["value"], json!("x & y"));
        assert_eq!(e[2]["value"], json!(["p", "q"]));
        let edit = clean_pref_edit(&json!({"key": "n", "type": "int", "before": "3", "after": " 07 "})).unwrap();
        let out = apply_pref_edit(XML, &edit).unwrap();
        assert!(out.contains("<int name=\"n\" value=\"7\" />") && out.contains("<null name=\"z\" />") && out.contains("<!-- <int"));
        assert_eq!(apply_pref_edit(XML.replace("\"3\"", "\"4\"").as_str(), &edit).unwrap_err(), "changed");
        assert!(clean_pref_edit(&json!({"key": "n", "type": "int", "before": "3", "after": "x"})).is_none());
        assert_eq!(clean_pref_value("long", &json!("9223372036854775808")), None);
    }
}
