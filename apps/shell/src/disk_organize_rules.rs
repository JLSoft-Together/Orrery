use serde_json::{json, Map, Value};

pub const KINDS: [&str; 8] = ["image", "video", "audio", "doc", "archive", "installer", "code", "other"];
pub const MODES: [&str; 2] = ["type", "month"];
pub const MAX_RULES: usize = 30;
pub const STABLE_MS: u64 = 120_000;
pub const PARTIAL_EXT: [&str; 8] = ["crdownload", "part", "partial", "tmp", "download", "opdownload", "!ut", "filepart"];

const EXT: [(&str, &[&str]); 7] = [
    ("image", &["jpg", "jpeg", "png", "gif", "webp", "bmp", "tif", "tiff", "heic", "heif", "svg", "ico", "avif", "raw", "cr2", "nef", "arw", "dng", "psd"]),
    ("video", &["mp4", "mkv", "mov", "avi", "wmv", "flv", "webm", "m4v", "mpg", "mpeg", "3gp", "mts", "m2ts"]),
    ("audio", &["mp3", "wav", "flac", "aac", "ogg", "oga", "m4a", "wma", "opus", "aiff", "aif", "mid", "midi"]),
    ("doc", &["pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx", "odt", "ods", "odp", "txt", "rtf", "md", "csv", "epub", "pages", "numbers", "key", "xps"]),
    ("archive", &["zip", "rar", "7z", "tar", "gz", "tgz", "bz2", "xz", "zst", "iso", "cab", "lz", "lzma"]),
    ("installer", &["exe", "msi", "msix", "msixbundle", "appx", "appxbundle", "dmg", "pkg", "apk", "aab", "xapk", "apks", "deb", "rpm", "appimage"]),
    (
        "code",
        &[
            "js", "mjs", "cjs", "ts", "tsx", "jsx", "json", "py", "java", "kt", "kts", "dart", "html", "htm", "css", "scss", "xml", "yml", "yaml", "toml", "sh", "ps1", "bat", "cmd", "c", "cpp", "h", "hpp", "cs", "go", "rs", "rb", "php",
            "sql", "ipynb", "vue", "swift", "gradle", "lua",
        ],
    ),
];

const FOLDERS_EN: [&str; 8] = ["Images", "Videos", "Music", "Documents", "Archives", "Installers", "Code", "Other"];
const FOLDERS_VI: [&str; 8] = ["Ảnh", "Video", "Nhạc", "Tài liệu", "File nén", "Cài đặt", "Code", "Khác"];
const HIDDEN_NAMES: [&str; 6] = ["desktop.ini", "thumbs.db", "ehthumbs.db", ".ds_store", "icon\r", ".localized"];
const MB: f64 = 1024.0 * 1024.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub kind: String,
    pub value: String,
    pub folder: String,
}

impl Rule {
    pub fn to_json(&self) -> Value {
        json!({ "kind": self.kind, "value": self.value, "folder": self.folder })
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Patch {
    pub source: Option<String>,
    pub mode: Option<String>,
    pub kinds: Option<Vec<String>>,
    pub rules: Option<Vec<Rule>>,
    pub auto: Option<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Classified {
    pub folder: String,
    pub kind: &'static str,
    pub rule: Option<usize>,
}

pub fn ext_of(name: &str) -> String {
    match name.rfind('.') {
        Some(i) if i > 0 => name[i + 1..].to_lowercase(),
        _ => String::new(),
    }
}

pub fn kind_of(name: &str) -> &'static str {
    let e = ext_of(name);
    EXT.iter().find(|(_, list)| list.contains(&e.as_str())).map_or("other", |(k, _)| k)
}

fn kind_index(kind: &str) -> usize {
    KINDS.iter().position(|k| *k == kind).unwrap_or(7)
}

pub fn folder_name(locale: &str, kind: &str) -> &'static str {
    let list = if locale.to_lowercase().starts_with("vi") { FOLDERS_VI } else { FOLDERS_EN };
    list[kind_index(kind)]
}

pub fn is_hidden_name(name: &str) -> bool {
    name.starts_with('.') || name.starts_with("~$") || HIDDEN_NAMES.contains(&name.to_lowercase().as_str())
}

pub fn is_partial_name(name: &str) -> bool {
    PARTIAL_EXT.contains(&ext_of(name).as_str())
}

pub fn month_of(ms: f64) -> String {
    let l = core_desk::telegram::clock::local(ms.floor() as i64);
    format!("{}-{:02}", l.y, l.mo)
}

fn reserved(s: &str) -> bool {
    let lower = s.to_lowercase();
    let stem = lower.split('.').next().unwrap_or("");
    if matches!(stem, "con" | "prn" | "aux" | "nul") {
        return true;
    }
    (stem.starts_with("com") || stem.starts_with("lpt")) && stem.len() == 4 && stem.as_bytes()[3].is_ascii_digit()
}

fn bad_folder(s: &str) -> bool {
    s.chars().any(|c| matches!(c, '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*') || (c as u32) < 0x20) || s.ends_with('.') || s.ends_with(' ')
}

pub fn clean_folder(v: &Value) -> Option<String> {
    let s = v.as_str()?.trim();
    if s.is_empty() || s.encode_utf16().count() > 64 || s == "." || s == ".." || bad_folder(s) || reserved(s) {
        return None;
    }
    Some(s.to_owned())
}

pub fn clean_rule(v: &Value) -> Option<Rule> {
    let o = v.as_object()?;
    let kind = o.get("kind")?.as_str().filter(|k| matches!(*k, "ext" | "name" | "size"))?;
    let folder = clean_folder(o.get("folder")?)?;
    let raw = o.get("value")?.as_str()?.trim();
    match kind {
        "ext" => {
            let e = raw.strip_prefix('*').unwrap_or(raw);
            let e = e.strip_prefix('.').unwrap_or(e).to_lowercase();
            let ok = (1..=16).contains(&e.len()) && e.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'!' | b'-'));
            ok.then(|| Rule { kind: "ext".into(), value: e, folder })
        }
        "name" => (!raw.is_empty() && raw.encode_utf16().count() <= 64).then(|| Rule { kind: "name".into(), value: raw.to_owned(), folder }),
        _ => {
            let mb: f64 = raw.parse().ok()?;
            (mb.is_finite() && mb > 0.0 && mb <= 10_000_000.0).then(|| Rule { kind: "size".into(), value: js_number(mb), folder })
        }
    }
}

pub fn js_number(n: f64) -> String {
    if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

pub fn clean_kinds(v: &Value) -> Option<Vec<String>> {
    let a = v.as_array()?;
    Some(KINDS.iter().filter(|k| a.iter().any(|x| x.as_str() == Some(**k))).map(|k| (*k).to_owned()).collect())
}

pub fn clean_patch(v: &Value) -> Patch {
    let empty = Map::new();
    let o = v.as_object().unwrap_or(&empty);
    Patch {
        source: o.get("source").and_then(Value::as_str).map(str::to_owned),
        mode: o.get("mode").and_then(Value::as_str).filter(|m| MODES.contains(m)).map(str::to_owned),
        kinds: o.get("kinds").and_then(clean_kinds),
        rules: o.get("rules").and_then(Value::as_array).map(|a| a.iter().filter_map(clean_rule).take(MAX_RULES).collect()),
        auto: o.get("auto").and_then(Value::as_bool),
    }
}

pub fn classify(name: &str, size: u64, mtime: f64, mode: &str, kinds: &[String], rules: &[Rule], locale: &str) -> Option<Classified> {
    let kind = kind_of(name);
    let ext = ext_of(name);
    let lower = name.to_lowercase();
    for (i, r) in rules.iter().enumerate() {
        let hit = match r.kind.as_str() {
            "ext" => ext == r.value,
            "name" => lower.contains(&r.value.to_lowercase()),
            _ => r.value.parse::<f64>().is_ok_and(|mb| size as f64 > mb * MB),
        };
        if hit {
            return Some(Classified { folder: r.folder.clone(), kind, rule: Some(i) });
        }
    }
    if !kinds.iter().any(|k| k == kind) {
        return None;
    }
    let folder = if mode == "month" { month_of(mtime) } else { folder_name(locale, kind).to_owned() };
    Some(Classified { folder, kind, rule: None })
}

pub fn with_suffix(name: &str, n: usize) -> String {
    if n <= 1 {
        return name.to_owned();
    }
    match name.rfind('.') {
        Some(i) if i > 0 => format!("{} ({n}){}", &name[..i], &name[i..]),
        _ => format!("{name} ({n})"),
    }
}
