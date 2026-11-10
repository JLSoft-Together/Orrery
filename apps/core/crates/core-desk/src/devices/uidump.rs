use crate::release_check::xml::xml_attrs;
use crate::run::android::jsre::{g, Js};
use serde_json::{json, Value};

pub const UI_NODES_MAX: usize = 5000;
const TEXT_MAX: usize = 500;
const PNG_SIG: [u8; 8] = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

fn parse_bounds(s: Option<&String>) -> Option<[i64; 4]> {
    static RE: Js = Js::new(r"^\[(-?\d+),(-?\d+)\]\[(-?\d+),(-?\d+)\]$", "");
    let m = RE.exec(s?)?;
    let n = |i: usize| g(&m, i).and_then(|v| v.parse::<i64>().ok());
    Some([n(1)?, n(2)?, n(3)?, n(4)?])
}

fn clip(s: Option<&String>) -> String {
    let cleaned: String = s.map(String::as_str).unwrap_or("").chars().map(|c| if matches!(c as u32, 0..=8 | 0x0b..=0x1f | 0x7f) { ' ' } else { c }).collect();
    crate::js::slice(&cleaned, TEXT_MAX)
}

pub fn parse_ui_dump(xml: &str) -> Option<(Vec<Value>, Value)> {
    static HEAD: Js = Js::new(r"<hierarchy\b([^>]*)>", "");
    static NODE: Js = Js::new(r"<(\/?)node\b([^>]*?)(\/?)>", "g");
    let head = HEAD.exec(xml)?;
    let rotation = xml_attrs(g(&head, 1).unwrap_or("")).get("rotation").map(|r| r.trim().parse::<f64>().ok().filter(|f| f.is_finite()).unwrap_or(0.0)).unwrap_or(0.0);
    let mut nodes: Vec<Value> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();
    for m in NODE.all(xml) {
        if !g(&m, 1).unwrap_or("").is_empty() {
            stack.pop();
            continue;
        }
        let a = xml_attrs(g(&m, 2).unwrap_or(""));
        let bounds = parse_bounds(a.get("bounds")).unwrap_or([0, 0, 0, 0]);
        let index = nodes.len();
        if index >= UI_NODES_MAX {
            break;
        }
        let secret = a.get("password").map(String::as_str) == Some("true");
        nodes.push(json!({
            "parent": stack.last().map(|p| *p as i64).unwrap_or(-1),
            "depth": stack.len(),
            "cls": clip(a.get("class")),
            "resId": clip(a.get("resource-id")),
            "text": if secret { String::new() } else { clip(a.get("text")) },
            "desc": clip(a.get("content-desc")),
            "pkg": clip(a.get("package")),
            "bounds": bounds,
            "clickable": a.get("clickable").map(String::as_str) == Some("true"),
            "focusable": a.get("focusable").map(String::as_str) == Some("true"),
        }));
        if g(&m, 3).unwrap_or("").is_empty() {
            stack.push(index);
        }
    }
    Some((nodes, super::history::num(rotation)))
}

pub fn png_size(buf: &[u8]) -> Option<(u32, u32)> {
    if buf.len() < 24 || buf[..8] != PNG_SIG {
        return None;
    }
    let w = u32::from_be_bytes([buf[16], buf[17], buf[18], buf[19]]);
    let h = u32::from_be_bytes([buf[20], buf[21], buf[22], buf[23]]);
    (w > 0 && h > 0).then_some((w, h))
}

pub fn find_png_start(buf: &[u8]) -> Option<usize> {
    let limit = buf.len().min(4096);
    (0..).take_while(|i| i + 8 <= limit).find(|i| buf[*i..*i + 8] == PNG_SIG)
}
