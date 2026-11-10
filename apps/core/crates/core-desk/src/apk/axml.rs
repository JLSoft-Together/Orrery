use serde_json::{json, Value};
use std::collections::HashMap;

pub struct XmlNode {
    pub name: String,
    pub attrs: HashMap<String, String>,
    pub children: Vec<XmlNode>,
}

const MAX_NODES: usize = 20_000;
const MAX_DEPTH: usize = 64;
const MAX_PERMISSIONS: usize = 300;
const NO_INDEX: u32 = 0xffff_ffff;

fn attr_id(id: u32) -> Option<&'static str> {
    Some(match id {
        0x0101_0003 => "name",
        0x0101_000f => "debuggable",
        0x0101_020c => "minSdkVersion",
        0x0101_0270 => "targetSdkVersion",
        0x0101_021b => "versionCode",
        0x0101_021c => "versionName",
        0x0101_0572 => "compileSdkVersion",
        _ => return None,
    })
}

fn u16_at(b: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(at..at + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(at..at + 4)?.try_into().ok()?))
}

fn clamp(b: &[u8], from: usize, to: usize) -> &[u8] {
    let to = to.min(b.len());
    let from = from.min(to);
    &b[from..to]
}

fn read_pool(buf: &[u8], at: usize) -> Option<Vec<String>> {
    let header = u16_at(buf, at + 2)? as usize;
    let count = u32_at(buf, at + 8)? as usize;
    let utf8 = u32_at(buf, at + 16)? & 0x100 != 0;
    let start = at + u32_at(buf, at + 20)? as usize;
    if count > 500_000 {
        return None;
    }
    let mut out = Vec::new();
    for i in 0..count {
        let mut p = start + u32_at(buf, at + header + i * 4)? as usize;
        if utf8 {
            p += if *buf.get(p)? & 0x80 != 0 { 2 } else { 1 };
            let mut len = *buf.get(p)? as usize;
            if len & 0x80 != 0 {
                len = ((len & 0x7f) << 8) | *buf.get(p + 1)? as usize;
                p += 2;
            } else {
                p += 1;
            }
            out.push(String::from_utf8_lossy(clamp(buf, p, p + len)).to_string());
        } else {
            let mut len = u16_at(buf, p)? as usize;
            p += 2;
            if len & 0x8000 != 0 {
                len = ((len & 0x7fff) << 16) | u16_at(buf, p)? as usize;
                p += 2;
            }
            let units: Vec<u16> = clamp(buf, p, p + len * 2).as_chunks::<2>().0.iter().map(|c| u16::from_le_bytes(*c)).collect();
            out.push(String::from_utf16_lossy(&units));
        }
    }
    Some(out)
}

fn typed_value(kind: u8, data: u32, strings: &[String]) -> String {
    match kind {
        0x03 => strings.get(data as usize).cloned().unwrap_or_default(),
        0x10 => (data as i32).to_string(),
        0x11 => format!("0x{data:x}"),
        0x12 => (if data != 0 { "true" } else { "false" }).to_string(),
        0x01 => format!("@0x{data:08x}"),
        _ => data.to_string(),
    }
}

fn parse_inner(buf: &[u8]) -> Option<Option<XmlNode>> {
    if buf.len() < 8 || u16_at(buf, 0)? != 0x0003 {
        return Some(None);
    }
    let mut strings: Vec<String> = Vec::new();
    let mut res_ids: Vec<u32> = Vec::new();
    let mut stack: Vec<XmlNode> = vec![XmlNode { name: "#document".into(), attrs: HashMap::new(), children: Vec::new() }];
    let mut nodes = 0usize;
    let mut at = u16_at(buf, 2)? as usize;
    let end = buf.len().min(u32_at(buf, 4)? as usize);
    while at + 8 <= end {
        let kind = u16_at(buf, at)?;
        let size = u32_at(buf, at + 4)? as usize;
        if size < 8 || at + size > end {
            return Some(None);
        }
        match kind {
            0x0001 => strings = read_pool(buf, at)?,
            0x0180 => {
                let hs = u16_at(buf, at + 2)? as usize;
                res_ids.clear();
                let mut p = at + hs;
                while p + 4 <= at + size {
                    res_ids.push(u32_at(buf, p)?);
                    p += 4;
                }
            }
            0x0102 => {
                nodes += 1;
                if nodes > MAX_NODES || stack.len() > MAX_DEPTH {
                    return Some(None);
                }
                let ext = at + u16_at(buf, at + 2)? as usize;
                let name = strings.get(u32_at(buf, ext + 4)? as usize).cloned().unwrap_or_default();
                let attr_start = u16_at(buf, ext + 8)? as usize;
                let attr_size = u16_at(buf, ext + 10)? as usize;
                let attr_count = u16_at(buf, ext + 12)? as usize;
                let mut attrs = HashMap::new();
                for i in 0..attr_count {
                    let a = ext + attr_start + i * attr_size;
                    let name_idx = u32_at(buf, a + 4)?;
                    let mut attr_name = strings.get(name_idx as usize).cloned().unwrap_or_default();
                    if attr_name.is_empty() || attr_name.starts_with(|c: char| c.is_ascii_digit()) {
                        if let Some(n) = res_ids.get(name_idx as usize).and_then(|id| attr_id(*id)) {
                            attr_name = n.to_string();
                        }
                    }
                    let raw = u32_at(buf, a + 8)?;
                    let value = if raw != NO_INDEX { strings.get(raw as usize).cloned().unwrap_or_default() } else { typed_value(*buf.get(a + 15)?, u32_at(buf, a + 16)?, &strings) };
                    attrs.insert(attr_name, value);
                }
                stack.push(XmlNode { name, attrs, children: Vec::new() });
            }
            0x0103 if stack.len() > 1 => {
                let done = stack.pop()?;
                stack.last_mut()?.children.push(done);
            }
            _ => {}
        }
        at += size;
    }
    while stack.len() > 1 {
        let done = stack.pop()?;
        stack.last_mut()?.children.push(done);
    }
    Some(stack.pop()?.children.into_iter().next())
}

pub fn parse_axml(buf: &[u8]) -> Option<XmlNode> {
    parse_inner(buf).flatten()
}

struct Fields<'a> {
    buf: &'a [u8],
    at: usize,
}

enum PbValue<'a> {
    Num(u64),
    Bytes(&'a [u8]),
    Skip,
}

impl<'a> Fields<'a> {
    fn varint(&mut self) -> Option<u64> {
        let mut v = 0f64;
        let mut mul = 1f64;
        for _ in 0..10 {
            let b = *self.buf.get(self.at)?;
            self.at += 1;
            v += (b & 0x7f) as f64 * mul;
            if b & 0x80 == 0 {
                return Some(v as u64);
            }
            mul *= 128.0;
        }
        None
    }

    fn next_field(&mut self) -> Option<Option<(u64, PbValue<'a>)>> {
        if self.at >= self.buf.len() {
            return Some(None);
        }
        let key = self.varint()?;
        let no = key / 8;
        match key & 7 {
            0 => Some(Some((no, PbValue::Num(self.varint()?)))),
            2 => {
                let len = self.varint()? as usize;
                if self.at + len > self.buf.len() {
                    return None;
                }
                let v = &self.buf[self.at..self.at + len];
                self.at += len;
                Some(Some((no, PbValue::Bytes(v))))
            }
            1 => {
                self.at += 8;
                Some(Some((no, PbValue::Skip)))
            }
            5 => {
                self.at += 4;
                Some(Some((no, PbValue::Skip)))
            }
            _ => None,
        }
    }
}

fn pb_element(buf: &[u8], depth: usize, count: &mut usize) -> Option<XmlNode> {
    *count += 1;
    if depth > MAX_DEPTH || *count > MAX_NODES {
        return None;
    }
    let mut node = XmlNode { name: String::new(), attrs: HashMap::new(), children: Vec::new() };
    let mut f = Fields { buf, at: 0 };
    while let Some((no, v)) = f.next_field()? {
        let PbValue::Bytes(bytes) = v else { continue };
        if no == 3 {
            node.name = String::from_utf8_lossy(bytes).to_string();
        } else if no == 4 {
            let (mut name, mut value, mut res_id) = (String::new(), String::new(), 0u64);
            let mut a = Fields { buf: bytes, at: 0 };
            while let Some((ano, av)) = a.next_field()? {
                match (ano, av) {
                    (2, PbValue::Bytes(b)) => name = String::from_utf8_lossy(b).to_string(),
                    (3, PbValue::Bytes(b)) => value = String::from_utf8_lossy(b).to_string(),
                    (5, PbValue::Num(n)) => res_id = n,
                    _ => {}
                }
            }
            if name.is_empty() {
                name = u32::try_from(res_id).ok().and_then(attr_id).unwrap_or("").to_string();
            }
            node.attrs.insert(name, value);
        } else if no == 5 {
            let mut c = Fields { buf: bytes, at: 0 };
            while let Some((cno, cv)) = c.next_field()? {
                if let (1, PbValue::Bytes(b)) = (cno, cv) {
                    node.children.push(pb_element(b, depth + 1, count)?);
                }
            }
        }
    }
    Some(node)
}

pub fn parse_proto_xml(buf: &[u8]) -> Option<XmlNode> {
    let mut f = Fields { buf, at: 0 };
    while let Some((no, v)) = f.next_field()? {
        if let (1, PbValue::Bytes(b)) = (no, v) {
            return pb_element(b, 0, &mut 0);
        }
    }
    None
}

fn attr(n: &XmlNode, name: &str) -> Option<String> {
    n.attrs.get(&format!("android:{name}")).or_else(|| n.attrs.get(name)).cloned()
}

fn opt(v: Option<String>) -> Value {
    v.map(Value::String).unwrap_or(Value::Null)
}

pub fn manifest_summary(root: Option<&XmlNode>) -> Value {
    let Some(root) = root.filter(|r| r.name == "manifest") else { return Value::Null };
    let sdk = root.children.iter().find(|c| c.name == "uses-sdk");
    let app = root.children.iter().find(|c| c.name == "application");
    let mut permissions: Vec<String> = Vec::new();
    for c in root.children.iter().filter(|c| matches!(c.name.as_str(), "uses-permission" | "uses-permission-sdk-23" | "uses-permission-sdk-m")) {
        if let Some(n) = attr(c, "name").filter(|n| !n.is_empty()) {
            if !permissions.contains(&n) {
                permissions.push(n);
            }
        }
    }
    permissions.sort();
    permissions.truncate(MAX_PERMISSIONS);
    json!({
        "pkg": opt(root.attrs.get("package").cloned()),
        "versionCode": opt(attr(root, "versionCode")),
        "versionName": opt(attr(root, "versionName")),
        "minSdk": opt(sdk.and_then(|s| attr(s, "minSdkVersion"))),
        "targetSdk": opt(sdk.and_then(|s| attr(s, "targetSdkVersion"))),
        "compileSdk": opt(attr(root, "compileSdkVersion")),
        "debuggable": app.is_some_and(|a| attr(a, "debuggable").as_deref() == Some("true")),
        "permissions": permissions,
    })
}
