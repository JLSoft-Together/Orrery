use super::xml::xml_attrs;
use crate::js;
use crate::libs::versions::compare_lib_versions;
use crate::run::android::jsre::{g, Js};
use regress::Regex;
use serde_json::{json, Map, Value};
use std::cmp::Ordering;

pub const PLAY_TARGET_SDK: i64 = 36;
pub const PLAY_TARGET_SDK_SINCE: &str = "2026-08-31";

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    C,
    Xml,
    Yaml,
}

fn blank(s: &str) -> String {
    s.chars().map(|c| if c == '\r' || c == '\n' { c } else { ' ' }).collect()
}

pub fn mask_comments(text: &str, lang: Lang) -> String {
    match lang {
        Lang::Xml => {
            static RE: Js = Js::new(r"<!--[\s\S]*?(-->|$)", "g");
            replace_with(RE.re(), text, |whole, _| blank(whole))
        }
        Lang::Yaml => {
            static RE: Js = Js::new(r"(^|[ \t])#.*$", "gm");
            replace_with(RE.re(), text, |whole, lead| format!("{lead}{}", blank(&whole[lead.len()..])))
        }
        Lang::C => mask_c(text),
    }
}

fn replace_with(re: &Regex, text: &str, f: impl Fn(&str, &str) -> String) -> String {
    let mut out = String::new();
    let mut last = 0;
    for m in re.find_iter(text) {
        let r = m.range();
        out.push_str(&text[last..r.start]);
        let lead = m.group(1).map(|x| &text[x]).unwrap_or("");
        let lead = if lead == "-->" { "" } else { lead };
        out.push_str(&f(&text[r.clone()], lead));
        last = r.end;
    }
    out.push_str(&text[last..]);
    out
}

fn mask_c(text: &str) -> String {
    let b = text.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    let blank_bytes = |out: &mut Vec<u8>, from: usize, to: usize| out.extend(blank(&text[from..to]).bytes());
    while i < b.len() {
        let c = b[i];
        if b[i..].starts_with(b"//") {
            let stop = text[i..].find('\n').map(|p| i + p).unwrap_or(b.len());
            blank_bytes(&mut out, i, stop);
            i = stop;
        } else if b[i..].starts_with(b"/*") {
            let stop = text[i + 2..].find("*/").map(|p| i + 2 + p + 2).unwrap_or(b.len());
            blank_bytes(&mut out, i, stop);
            i = stop;
        } else if c == b'"' || c == b'\'' {
            let triple = b[i..].starts_with(&[c, c, c]);
            let qlen = if triple { 3 } else { 1 };
            let mut j = i + qlen;
            while j < b.len() && !b[j..].starts_with(&b[i..i + qlen]) {
                if !triple && b[j] == b'\n' {
                    break;
                }
                j += if b[j] == b'\\' { 2 } else { 1 };
            }
            let stop = b.len().min(j + if j < b.len() && b[j..].starts_with(&b[i..i + qlen]) { qlen } else { 0 });
            out.extend_from_slice(&b[i..stop]);
            i = stop;
        } else {
            let ch_len = text[i..].chars().next().map(char::len_utf8).unwrap_or(1);
            out.extend_from_slice(&b[i..i + ch_len]);
            i += ch_len;
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

pub fn line_at(text: &str, index: usize) -> usize {
    1 + text.as_bytes()[..index.min(text.len())].iter().filter(|b| **b == b'\n').count()
}

fn block_end(text: &str, open: usize) -> usize {
    let mut depth = 0i32;
    for (i, c) in text.as_bytes().iter().enumerate().skip(open) {
        if *c == b'{' {
            depth += 1;
        } else if *c == b'}' {
            depth -= 1;
            if depth == 0 {
                return i;
            }
        }
    }
    text.len()
}

#[derive(Clone, Copy)]
pub struct Range {
    pub at: usize,
    pub start: usize,
    pub end: usize,
}

fn find_block(text: &str, head: &Js, within: Option<(usize, usize)>) -> Option<Range> {
    let (ws, we) = within.unwrap_or((0, text.len()));
    for m in head.re().find_from(text, ws) {
        let r = m.range();
        if r.start >= we {
            break;
        }
        let open = r.end - 1;
        if text.as_bytes().get(open) != Some(&b'{') {
            continue;
        }
        return Some(Range { at: r.start, start: open + 1, end: block_end(text, open).min(we) });
    }
    None
}

#[derive(Clone)]
pub struct GradleValue<T> {
    pub value: Option<T>,
    pub raw: String,
    pub line: usize,
}

#[derive(Clone)]
pub struct Signing {
    pub v: GradleValue<String>,
    pub name: Option<String>,
}

#[derive(Clone)]
pub struct GradleRelease {
    pub line: usize,
    pub minify: Option<GradleValue<bool>>,
    pub shrink: Option<GradleValue<bool>>,
    pub debuggable: Option<GradleValue<bool>>,
    pub signing: Option<Signing>,
}

#[derive(Clone)]
pub struct GradleApp {
    pub version_code: Option<GradleValue<i64>>,
    pub version_name: Option<GradleValue<String>>,
    pub min_sdk: Option<GradleValue<i64>>,
    pub target_sdk: Option<GradleValue<i64>>,
    pub compile_sdk: Option<GradleValue<i64>>,
    pub release: Option<GradleRelease>,
    pub anchor_line: usize,
}

const RAW_MAX: usize = 80;

fn tidy(raw: &str) -> String {
    let mut s = raw.trim().to_string();
    while s.ends_with('}') {
        s.pop();
        s = s.trim_end().to_string();
    }
    if s.ends_with(';') || s.ends_with(',') {
        s.pop();
    }
    let mut s = s.trim().to_string();
    while s.ends_with(')') && s.matches(')').count() > s.matches('(').count() {
        s.pop();
        s = s.trim().to_string();
    }
    if js::len(&s) > RAW_MAX {
        format!("{}…", js::slice(&s, RAW_MAX))
    } else {
        s
    }
}

fn pick<T>(text: &str, re: &Regex, start: usize, end: usize, parse: impl Fn(&str) -> Option<T>) -> Option<GradleValue<T>> {
    let m = re.find_from(text, start).next()?;
    let r = m.range();
    if r.start >= end {
        return None;
    }
    let raw = tidy(&text[m.group(1)?]);
    Some(GradleValue { value: parse(&raw), line: line_at(text, r.start), raw })
}

fn int(raw: &str) -> Option<i64> {
    (!raw.is_empty() && raw.len() <= 10 && raw.bytes().all(|b| b.is_ascii_digit())).then(|| raw.parse().ok()).flatten()
}

pub fn sdk_value(raw: &str) -> Option<i64> {
    static ANDROID: Js = Js::new(r#"^["']android-(\d+)["']$"#, "");
    static RELEASE: Js = Js::new(r"release\(\s*(\d+)", "");
    int(raw)
        .or_else(|| ANDROID.exec(raw).and_then(|m| g(&m, 1).and_then(int)))
        .or_else(|| RELEASE.exec(raw).and_then(|m| g(&m, 1).and_then(int)))
}

fn boolean(raw: &str) -> Option<bool> {
    match raw {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn string_lit(raw: &str) -> Option<String> {
    static RE: Js = Js::new(r#"^["']([^"'\r\n]*)["']$"#, "");
    RE.exec(raw).and_then(|m| g(&m, 1).map(String::from))
}

const VALUE: &str = r"\s*(?:=\s*|\(\s*)?([^\r\n;]+)";

fn sdk_re(name: &str) -> Regex {
    Regex::new(&format!(r"\b{name}(?:Version)?\b{VALUE}")).expect("regex")
}

fn key_re(pattern: &str) -> Regex {
    Regex::new(&format!(r"\b(?:{pattern})\b{VALUE}")).expect("regex")
}

pub fn parse_gradle_app(source: &str) -> GradleApp {
    static ANDROID: Js = Js::new(r"\bandroid\s*\{", "");
    static DEFAULT_CONFIG: Js = Js::new(r"\bdefaultConfig\s*\{", "");
    static COMPILE_BLOCK: Js = Js::new(r"\bcompileSdk\s*\{", "");
    static TYPES: Js = Js::new(r"\bbuildTypes\s*\{", "");
    static RELEASE_HEAD: Js = Js::new(r#"(?:\brelease|getByName\(\s*["']release["']\s*\)|named\(\s*["']release["']\s*\)|maybeCreate\(\s*["']release["']\s*\))\s*\{"#, "");
    static RELEASE_INNER: Js = Js::new(r"release\(\s*(\d+)\s*\)", "");
    static SIGN_GET: Js = Js::new(r#"signingConfigs\.getByName\(\s*["'](\w+)["']\s*\)"#, "");
    static SIGN_IDX: Js = Js::new(r#"signingConfigs\s*\[\s*["'](\w+)["']\s*\]"#, "");
    static SIGN_DOT: Js = Js::new(r"signingConfigs\.(?!getByName|named)(\w+)", "");
    let text = mask_comments(source, Lang::C);
    let text = text.as_str();
    let android = find_block(text, &ANDROID, None);
    let scope = android.map(|b| (b.start, b.end)).unwrap_or((0, text.len()));
    let dc = find_block(text, &DEFAULT_CONFIG, Some(scope));
    let in_dc = dc.map(|b| (b.start, b.end)).unwrap_or(scope);
    let sdk_in = |name: &str| {
        let re = sdk_re(name);
        pick(text, &re, in_dc.0, in_dc.1, sdk_value).or_else(|| if dc.is_some() { pick(text, &re, scope.0, scope.1, sdk_value) } else { None })
    };
    let compile_re = Regex::new(r"\bcompileSdk(?:Version)?\b\s*(?:=\s*|\(\s*)?([^\r\n;]+)").expect("regex");
    let mut compile_sdk = pick(text, &compile_re, scope.0, scope.1, sdk_value);
    if let Some(c) = compile_sdk.as_mut() {
        if c.raw.starts_with('{') {
            let block = find_block(text, &COMPILE_BLOCK, Some(scope));
            let inner = block.and_then(|b| RELEASE_INNER.exec(&text[b.start..b.end]));
            let n = inner.as_ref().and_then(|m| g(m, 1)).and_then(|s| s.parse::<i64>().ok());
            if let Some(n) = n {
                c.raw = format!("release({n})");
            }
            c.value = n;
        }
    }
    let types = find_block(text, &TYPES, Some(scope));
    let rel = types.and_then(|t| find_block(text, &RELEASE_HEAD, Some((t.start, t.end))));
    let release = rel.map(|r| {
        let signing_re = Regex::new(r"\bsigningConfig\b\s*(?:=\s*)?([^\r\n;]+)").expect("regex");
        let signing = pick(text, &signing_re, r.start, r.end, |raw| Some(raw.to_string()));
        let name = signing.as_ref().and_then(|s| {
            [&SIGN_GET, &SIGN_IDX, &SIGN_DOT].iter().find_map(|re| re.exec(&s.raw).and_then(|m| g(&m, 1).map(String::from)))
        });
        let flag = |p: &str| pick(text, &key_re(p), r.start, r.end, boolean);
        GradleRelease {
            line: line_at(text, r.at),
            minify: flag("isMinifyEnabled|minifyEnabled"),
            shrink: flag("isShrinkResources|shrinkResources"),
            debuggable: flag("isDebuggable|debuggable"),
            signing: signing.map(|v| Signing { v, name }),
        }
    });
    GradleApp {
        version_code: pick(text, &key_re("versionCode"), in_dc.0, in_dc.1, int),
        version_name: pick(text, &key_re("versionName"), in_dc.0, in_dc.1, string_lit),
        min_sdk: sdk_in("minSdk"),
        target_sdk: sdk_in("targetSdk"),
        compile_sdk,
        release,
        anchor_line: types.map(|t| line_at(text, t.at)).or_else(|| android.map(|a| line_at(text, a.at))).unwrap_or(1),
    }
}

pub struct PubspecVersion {
    pub name: Option<String>,
    pub code: Option<i64>,
    pub raw: String,
    pub line: usize,
}

pub fn parse_pubspec_version(source: &str) -> Option<PubspecVersion> {
    static RE: Js = Js::new(r#"^version:[ \t]*["']?([^"'\r\n]+?)["']?[ \t]*$"#, "m");
    let text = mask_comments(source, Lang::Yaml);
    let m = RE.re().find(&text)?;
    let raw = text[m.group(1)?].trim().to_string();
    let mut parts = raw.split('+');
    let name = parts.next().map(str::trim).filter(|s| !s.is_empty()).map(String::from);
    let code = parts.next().and_then(|b| int(b.trim()));
    Some(PubspecVersion { name, code, line: line_at(&text, m.range().start), raw })
}

pub fn pubspec_name(source: &str) -> Option<String> {
    static RE: Js = Js::new(r#"^name:[ \t]*["']?([\w-]+)"#, "m");
    RE.exec(&mask_comments(source, Lang::Yaml)).and_then(|m| g(&m, 1).map(String::from))
}

type Params = Vec<(&'static str, Value)>;

fn make(rule: &str, section: &str, status: &str, detail: &str, file: Option<&str>, line: Option<usize>, params: Params) -> Value {
    let mut o = Map::new();
    o.insert("rule".into(), json!(rule));
    o.insert("section".into(), json!(section));
    o.insert("status".into(), json!(status));
    o.insert("detail".into(), json!(detail));
    if !params.is_empty() {
        o.insert("params".into(), Value::Object(params.into_iter().map(|(k, v)| (k.to_string(), v)).collect()));
    }
    o.insert("file".into(), file.map(|f| json!(f)).unwrap_or(Value::Null));
    o.insert("line".into(), line.map(|l| json!(l)).unwrap_or(Value::Null));
    Value::Object(o)
}

pub struct VersionInput {
    pub code: Option<i64>,
    pub code_raw: Option<String>,
    pub name: Option<String>,
    pub file: Option<String>,
    pub code_line: Option<usize>,
    pub name_line: Option<usize>,
}

pub struct Released {
    pub code: Option<i64>,
    pub name: Option<String>,
}

pub fn version_findings(v: &VersionInput, released: Option<&Released>) -> Vec<Value> {
    let mut out = Vec::new();
    let file = v.file.as_deref();
    match (v.code, &v.code_raw) {
        (Some(code), _) => match released.and_then(|r| r.code) {
            None => out.push(make("version-code", "version", "ok", "first", file, v.code_line, vec![("code", json!(code))])),
            Some(prev) if code > prev => out.push(make("version-code", "version", "ok", "raised", file, v.code_line, vec![("code", json!(code)), ("prev", json!(prev))])),
            Some(prev) => out.push(make("version-code", "version", "fail", "notRaised", file, v.code_line, vec![("code", json!(code)), ("prev", json!(prev))])),
        },
        (None, Some(raw)) if !raw.is_empty() => out.push(make("version-code", "version", "unknown", "expr", file, v.code_line, vec![("raw", json!(raw))])),
        _ => out.push(make("version-code", "version", "fail", "missing", file, v.code_line, vec![])),
    }
    let prev_name = released.and_then(|r| r.name.as_deref()).filter(|n| !n.is_empty());
    match (&v.name, prev_name) {
        (None, _) => out.push(make("version-name", "version", "warn", "missing", file, v.name_line, vec![])),
        (Some(n), Some(p)) if n == p => out.push(make("version-name", "version", "warn", "same", file, v.name_line, vec![("name", json!(n))])),
        (Some(n), Some(p)) if compare_lib_versions(n, p) == Ordering::Less => out.push(make("version-name", "version", "warn", "lower", file, v.name_line, vec![("name", json!(n)), ("prev", json!(p))])),
        (Some(n), _) => out.push(make("version-name", "version", "ok", "value", file, v.name_line, vec![("name", json!(n))])),
    }
    out
}

pub fn build_findings(g: &GradleApp, file: &str, flutter: bool) -> Vec<Value> {
    let r = g.release.as_ref();
    let at = r.map(|r| r.line).unwrap_or(g.anchor_line);
    let f = Some(file);
    let toggle = |rule: &str, v: Option<&GradleValue<bool>>| -> Value {
        match v {
            None if flutter => make(rule, "build", "ok", "flutterDefault", f, Some(at), vec![]),
            None => make(rule, "build", "warn", "missing", f, Some(at), vec![]),
            Some(v) => match v.value {
                Some(true) => make(rule, "build", "ok", "on", f, Some(v.line), vec![]),
                Some(false) => make(rule, "build", "warn", "off", f, Some(v.line), vec![]),
                None => make(rule, "build", "unknown", "expr", f, Some(v.line), vec![("raw", json!(v.raw))]),
            },
        }
    };
    let mut out = vec![toggle("minify", r.and_then(|r| r.minify.as_ref())), toggle("shrink", r.and_then(|r| r.shrink.as_ref()))];
    match r.and_then(|r| r.debuggable.as_ref()) {
        Some(d) if d.value == Some(true) => out.push(make("debuggable", "build", "fail", "on", f, Some(d.line), vec![])),
        Some(d) if d.value.is_none() => out.push(make("debuggable", "build", "unknown", "expr", f, Some(d.line), vec![("raw", json!(d.raw))])),
        d => out.push(make("debuggable", "build", "ok", "off", f, Some(d.map(|d| d.line).unwrap_or(at)), vec![])),
    }
    match r.and_then(|r| r.signing.as_ref()) {
        None => out.push(make("signing", "build", "warn", "none", f, Some(at), vec![])),
        Some(s) if s.v.raw == "null" => out.push(make("signing", "build", "warn", "none", f, Some(s.v.line), vec![])),
        Some(s) if s.name.as_deref() == Some("debug") => out.push(make("signing", "build", "warn", "debugKey", f, Some(s.v.line), vec![])),
        Some(s) => match &s.name {
            Some(n) if !n.is_empty() => out.push(make("signing", "build", "ok", "named", f, Some(s.v.line), vec![("name", json!(n))])),
            _ => out.push(make("signing", "build", "unknown", "expr", f, Some(s.v.line), vec![("raw", json!(s.v.raw))])),
        },
    }
    out
}

pub fn sdk_findings(g: &GradleApp, file: &str, flutter: bool) -> Vec<Value> {
    let mut out = Vec::new();
    let f = Some(file);
    let need = || -> Params { vec![("need", json!(PLAY_TARGET_SDK)), ("since", json!(PLAY_TARGET_SDK_SINCE))] };
    let with = |mut p: Params, extra: Params| -> Params {
        p.extend(extra);
        p
    };
    let t = g.target_sdk.as_ref();
    match t {
        None => out.push(make("target-sdk", "sdk", "fail", "missing", f, Some(g.anchor_line), need())),
        Some(t) => match t.value {
            None => {
                let detail = if flutter && t.raw.contains("flutter.") { "flutter" } else { "expr" };
                out.push(make("target-sdk", "sdk", "unknown", detail, f, Some(t.line), with(need(), vec![("raw", json!(t.raw))])));
            }
            Some(n) if n < PLAY_TARGET_SDK => out.push(make("target-sdk", "sdk", "fail", "low", f, Some(t.line), with(need(), vec![("n", json!(n))]))),
            Some(n) => out.push(make("target-sdk", "sdk", "ok", "meets", f, Some(t.line), with(need(), vec![("n", json!(n))]))),
        },
    }
    if let Some(m) = &g.min_sdk {
        out.push(match m.value {
            None => make("min-sdk", "sdk", "unknown", "expr", f, Some(m.line), vec![("raw", json!(m.raw))]),
            Some(n) => make("min-sdk", "sdk", "ok", "value", f, Some(m.line), vec![("n", json!(n))]),
        });
    }
    if let Some(c) = &g.compile_sdk {
        let target = t.and_then(|t| t.value);
        match (c.value, target) {
            (Some(cv), Some(tv)) if cv < tv => out.push(make("compile-sdk", "sdk", "warn", "belowTarget", f, Some(c.line), vec![("n", json!(cv)), ("target", json!(tv))])),
            (None, _) => out.push(make("compile-sdk", "sdk", "unknown", "expr", f, Some(c.line), vec![("raw", json!(c.raw))])),
            (Some(cv), _) => out.push(make("compile-sdk", "sdk", "ok", "value", f, Some(c.line), vec![("n", json!(cv))])),
        }
    }
    out
}

pub const SENSITIVE_PERMISSIONS: [&str; 10] = [
    "READ_SMS",
    "RECEIVE_SMS",
    "SEND_SMS",
    "READ_CALL_LOG",
    "WRITE_CALL_LOG",
    "PROCESS_OUTGOING_CALLS",
    "QUERY_ALL_PACKAGES",
    "MANAGE_EXTERNAL_STORAGE",
    "ACCESS_BACKGROUND_LOCATION",
    "REQUEST_INSTALL_PACKAGES",
];

fn attr<'a>(a: &'a std::collections::HashMap<String, String>, name: &str) -> Option<&'a String> {
    a.get(&format!("android:{name}")).or_else(|| a.get(name))
}

pub fn manifest_findings(source: &str, file: &str, main: bool) -> Vec<Value> {
    static APP: Js = Js::new(r"<application\b([^>]*)>?", "");
    static COMPONENTS: Js = Js::new(r"<(activity|activity-alias|service|receiver|provider)\b([^>]*?)(\/>|>([\s\S]*?)<\/\1\s*>)", "g");
    static FILTER: Js = Js::new(r"<intent-filter\b", "");
    static PERM: Js = Js::new(r"<uses-permission(?:-sdk-23)?\b([^>]*)>", "g");
    static ANY_PERM: Js = Js::new(r"<uses-permission\b", "");
    let text = mask_comments(source, Lang::Xml);
    let text = text.as_str();
    let f = Some(file);
    let mut out = Vec::new();
    let app = APP.re().find(text);
    let line_of_attr = |name: &str| {
        let m = app.as_ref().expect("app");
        let r = m.range();
        let i = text[r.clone()].find(name).unwrap_or(0);
        line_at(text, r.start + i)
    };
    let app_attrs = app.as_ref().and_then(|m| m.group(1)).map(|r| xml_attrs(&text[r])).unwrap_or_default();
    let app_line = app.as_ref().map(|m| line_at(text, m.range().start)).unwrap_or(1);
    if attr(&app_attrs, "usesCleartextTraffic").map(String::as_str) == Some("true") {
        out.push(make("cleartext", "manifest", "warn", "on", f, Some(line_of_attr("usesCleartextTraffic")), vec![]));
    } else if main {
        out.push(make("cleartext", "manifest", "ok", "off", f, Some(app_line), vec![]));
    }
    if attr(&app_attrs, "debuggable").map(String::as_str) == Some("true") {
        out.push(make("manifest-debuggable", "manifest", "fail", "on", f, Some(line_of_attr("debuggable")), vec![]));
    } else if main {
        out.push(make("manifest-debuggable", "manifest", "ok", "off", f, Some(app_line), vec![]));
    }
    let mut exported = 0;
    for m in COMPONENTS.re().find_iter(text) {
        let grp = |i: usize| m.group(i).map(|r| &text[r]);
        let a = xml_attrs(grp(2).unwrap_or(""));
        if attr(&a, "exported").map(String::as_str) != Some("true") || FILTER.test(grp(4).unwrap_or("")) {
            continue;
        }
        exported += 1;
        let tag = grp(1).unwrap_or("");
        out.push(make("exported", "manifest", "warn", "noFilter", f, Some(line_at(text, m.range().start)), vec![("name", json!(attr(&a, "name").map(String::as_str).unwrap_or(tag))), ("tag", json!(tag))]));
    }
    if exported == 0 && main {
        out.push(make("exported", "manifest", "ok", "none", f, Some(app_line), vec![]));
    }
    let mut sensitive = 0;
    for m in PERM.re().find_iter(text) {
        let a = xml_attrs(m.group(1).map(|r| &text[r]).unwrap_or(""));
        let name = attr(&a, "name").cloned().unwrap_or_default();
        let short = name.strip_prefix("android.permission.").unwrap_or(&name).to_string();
        if a.get("tools:node").map(String::as_str) == Some("remove") || !SENSITIVE_PERMISSIONS.contains(&short.as_str()) || !name.starts_with("android.permission.") {
            continue;
        }
        sensitive += 1;
        out.push(make("permission", "manifest", "warn", "declare", f, Some(line_at(text, m.range().start)), vec![("name", json!(short))]));
    }
    if sensitive == 0 && main {
        let idx = ANY_PERM.re().find(text).map(|m| m.range().start).or_else(|| app.as_ref().map(|m| m.range().start)).unwrap_or(0);
        out.push(make("permission", "manifest", "ok", "none", f, Some(line_at(text, idx)), vec![]));
    }
    out
}

const GUARD_LINES: usize = 6;

pub fn lang_of(file: &str) -> Option<Lang> {
    let f = file.to_lowercase();
    let ends = |exts: &[&str]| exts.iter().any(|e| f.ends_with(&format!(".{e}")));
    if ends(&["kt", "kts", "java", "dart", "gradle", "swift", "m", "js", "ts", "json"]) {
        Some(Lang::C)
    } else if ends(&["xml", "plist"]) {
        Some(Lang::Xml)
    } else if ends(&["yaml", "yml", "properties"]) {
        Some(Lang::Yaml)
    } else {
        None
    }
}

pub fn admob_findings(source: &str, file: &str, debug_path: bool) -> Vec<Value> {
    static TEST: Js = Js::new(r"ca-app-pub-3940256099942544[~/]\d{6,12}", "g");
    static GUARD: Js = Js::new(r"\b(kDebugMode|kReleaseMode|kProfileMode|BuildConfig\.DEBUG|isDebug\w*|DEBUG)\b|\bassert\s*\(", "");
    let Some(lang) = lang_of(file) else { return Vec::new() };
    if !source.contains("ca-app-pub-3940256099942544") {
        return Vec::new();
    }
    let text = mask_comments(source, lang);
    let lines: Vec<&str> = text.split('\n').collect();
    let mut out = Vec::new();
    for m in TEST.re().find_iter(&text) {
        let r = m.range();
        let id = &text[r.clone()];
        let line = line_at(&text, r.start);
        let params: Params = vec![("id", json!(id)), ("kind", json!(if id.contains('~') { "app" } else { "unit" }))];
        let from = (line - 1).saturating_sub(GUARD_LINES);
        let guarded = lines[from.min(lines.len())..line.min(lines.len())].iter().any(|l| GUARD.test(l));
        let detail = if debug_path {
            ("warn", "debugPath")
        } else if guarded {
            ("warn", "guarded")
        } else {
            ("fail", "release")
        };
        out.push(make("admob-test", "ads", detail.0, detail.1, Some(file), Some(line), params));
    }
    out
}

#[derive(Clone)]
pub struct Located {
    pub file: String,
    pub line: usize,
}

pub struct ExtrasFacts {
    pub ads: Option<Located>,
    pub consent: bool,
    pub billing: Option<(Located, bool)>,
    pub acknowledge: bool,
    pub google_services_json: Option<String>,
    pub gms_plugin: Option<Located>,
}

pub fn extras_findings(f: &ExtrasFacts) -> Vec<Value> {
    let mut out = Vec::new();
    if let Some(ads) = &f.ads {
        let detail = if f.consent { ("ok", "found") } else { ("warn", "missing") };
        out.push(make("ump", "extras", detail.0, detail.1, Some(&ads.file), Some(ads.line), vec![]));
    }
    if let Some((b, revenue_cat)) = &f.billing {
        let (status, detail) = if *revenue_cat { ("ok", "revenueCat") } else if f.acknowledge { ("ok", "found") } else { ("warn", "missing") };
        out.push(make("billing-ack", "extras", status, detail, Some(&b.file), Some(b.line), vec![]));
    }
    match (&f.google_services_json, &f.gms_plugin) {
        (Some(json), None) => out.push(make("firebase", "extras", "warn", "noPlugin", Some(json), Some(1), vec![])),
        (None, Some(p)) => out.push(make("firebase", "extras", "warn", "noJson", Some(&p.file), Some(p.line), vec![])),
        (Some(_), Some(p)) => out.push(make("firebase", "extras", "ok", "paired", Some(&p.file), Some(p.line), vec![])),
        (None, None) => {}
    }
    out
}

pub fn locate(source: &str, file: &str, lang: Lang, re: &Js) -> Option<Located> {
    let text = mask_comments(source, lang);
    let m = re.re().find(&text)?;
    Some(Located { file: file.to_string(), line: line_at(&text, m.range().start) })
}

pub static ADS_GRADLE: Js = Js::new(r"play-services-ads(?:-lite)?(?![\w-])", "");
pub static ADS_PUBSPEC: Js = Js::new(r"^[ \t]+google_mobile_ads[ \t]*:", "m");
pub static BILLING_GRADLE: Js = Js::new(r"com\.android\.billingclient|com\.revenuecat\.purchases", "");
pub static BILLING_PUBSPEC: Js = Js::new(r"^[ \t]+(in_app_purchase|purchases_flutter|flutter_inapp_purchase)[ \t]*:", "m");
pub static CONSENT_SOURCE: Js = Js::new(r"\b(UserMessagingPlatform|ConsentInformation|ConsentRequestParameters|ConsentForm)\b", "");
pub static ACK_SOURCE: Js = Js::new(r"\b(acknowledgePurchase|completePurchase|consumeAsync|consumePurchase|finishTransaction|finishPurchase)\b", "");
pub static GMS_PLUGIN: Js = Js::new(r#"com\.google\.gms\.google-services|plugins\.(?:google[.-]?services|gms[\w.-]*)\b|google-services["']"#, "");

pub fn format_sha256(hex: &str) -> Option<String> {
    let h: String = hex.chars().filter(|c| c.is_ascii_hexdigit()).collect::<String>().to_uppercase();
    (h.len() == 64).then(|| h.as_bytes().chunks(2).map(|c| String::from_utf8_lossy(c).to_string()).collect::<Vec<_>>().join(":"))
}

pub struct Signer {
    pub sha256: String,
    pub subject: Option<String>,
}

pub fn parse_apksigner(out: &str) -> Option<Signer> {
    static SHA: Js = Js::new(r"certificate SHA-256 digest:\s*([0-9a-fA-F]{64})", "");
    static DN: Js = Js::new(r"certificate DN:\s*(.+)", "");
    let sha = SHA.exec(out).and_then(|m| g(&m, 1).map(String::from))?;
    let sha256 = format_sha256(&sha)?;
    Some(Signer { sha256, subject: DN.exec(out).and_then(|m| g(&m, 1).map(|s| s.trim().to_string())) })
}

pub fn parse_keytool(out: &str) -> Option<Signer> {
    static SHA: Js = Js::new(r"SHA-?256:\s*((?:[0-9A-Fa-f]{2}:){31}[0-9A-Fa-f]{2})", "");
    static OWNER: Js = Js::new(r"(?:Owner|Chủ sở hữu):\s*(.+)", "");
    let sha = SHA.exec(out).and_then(|m| g(&m, 1).map(String::from))?;
    let sha256 = format_sha256(&sha)?;
    Some(Signer { sha256, subject: OWNER.exec(out).and_then(|m| g(&m, 1).map(|s| s.trim().to_string())) })
}

pub fn is_debug_subject(subject: Option<&str>) -> bool {
    static RE: Js = Js::new(r"CN=Android Debug\b", "i");
    subject.is_some_and(|s| RE.test(s))
}

pub fn cert_finding(sha256: Option<&str>, remembered: Option<&str>, debug_key: bool, file: Option<&str>) -> Value {
    let short = |s: &str| {
        let chars: Vec<char> = s.encode_utf16().map(|u| char::from_u32(u as u32).unwrap_or('?')).collect();
        let head: String = chars.iter().take(11).collect();
        let tail: String = chars[chars.len().saturating_sub(5)..].iter().collect();
        format!("{head}…{tail}")
    };
    match sha256 {
        None => match remembered {
            Some(r) => make("cert", "signing", "unknown", "remembered", None, None, vec![("sha", json!(short(r)))]),
            None => make("cert", "signing", "unknown", "notVerified", None, None, vec![]),
        },
        Some(s) if debug_key => make("cert", "signing", "fail", "debugKey", file, None, vec![("sha", json!(short(s)))]),
        Some(s) => match remembered {
            None => make("cert", "signing", "unknown", "notRemembered", file, None, vec![("sha", json!(short(s)))]),
            Some(r) if r == s => make("cert", "signing", "ok", "match", file, None, vec![("sha", json!(short(s)))]),
            Some(r) => make("cert", "signing", "fail", "mismatch", file, None, vec![("sha", json!(short(s))), ("prev", json!(short(r)))]),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradle_and_masking() {
        let g = parse_gradle_app("android {\n compileSdk = 35\n defaultConfig {\n  versionCode = 7 // seven\n  versionName \"1.2\"\n  targetSdk 34\n }\n buildTypes {\n  release {\n   isMinifyEnabled = true\n   signingConfig = signingConfigs.getByName(\"upload\")\n  }\n }\n}\n");
        assert_eq!(g.version_code.as_ref().and_then(|v| v.value), Some(7));
        assert_eq!(g.version_name.as_ref().and_then(|v| v.value.clone()).as_deref(), Some("1.2"));
        assert_eq!(g.target_sdk.as_ref().and_then(|v| v.value), Some(34));
        let r = g.release.as_ref().unwrap();
        assert_eq!(r.minify.as_ref().and_then(|v| v.value), Some(true));
        assert_eq!(r.signing.as_ref().and_then(|s| s.name.clone()).as_deref(), Some("upload"));
        assert_eq!(mask_comments("a // b\n'//x' /* c */", Lang::C), "a     \n'//x'        ");
        assert_eq!(line_at("a\nb\nc", 4), 3);
        assert_eq!(format_sha256(&"ab".repeat(32)).map(|s| s.len()), Some(95));
        assert!(cert_finding(Some("AA"), Some("AA"), false, None)["status"] == "ok");
    }
}
