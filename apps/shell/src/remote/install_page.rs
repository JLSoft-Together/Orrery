use std::fmt::Write as _;

pub const PAIR_SCHEME: &str = "orrery-remote://pair?q=";
const MAX_VERSION: usize = 32;

struct PageText {
    lang: &'static str,
    title: &'static str,
    version: &'static str,
    size: &'static str,
    download: &'static str,
    pair: &'static str,
    step1: &'static str,
    step2: &'static str,
}

const EN: PageText = PageText {
    lang: "en",
    title: "Install Orrery Remote",
    version: "Version",
    size: "Size",
    download: "Download APK",
    pair: "Open and pair",
    step1: "1. Download and install the app. Android may ask you to allow installs from this browser.",
    step2: "2. Come back to this page and tap Open and pair. Your computer asks you to allow this phone.",
};

const VI: PageText = PageText {
    lang: "vi",
    title: "Cài Orrery Remote",
    version: "Phiên bản",
    size: "Dung lượng",
    download: "Tải APK",
    pair: "Mở và ghép đôi",
    step1: "1. Tải và cài app. Android có thể hỏi cho phép cài từ trình duyệt này.",
    step2: "2. Quay lại trang này, bấm Mở và ghép đôi. Máy tính sẽ hỏi bạn cho phép điện thoại này.",
};

const STYLE: &str = "body{margin:0;font:16px/1.5 system-ui,-apple-system,'Segoe UI',Roboto,sans-serif;background:#f5f5f7;color:#1d1d1f}main{max-width:420px;margin:0 auto;padding:32px 20px}h1{font-size:24px;margin:0 0 4px}.meta{color:#6e6e73;margin:0 0 24px}.btn{display:block;text-align:center;padding:14px;border-radius:12px;background:#0a84ff;color:#fff;text-decoration:none;font-weight:600;margin:12px 0}.alt{background:#fff;color:#0a84ff;border:1px solid #d2d2d7}p{margin:8px 0}@media(prefers-color-scheme:dark){body{background:#000;color:#f5f5f7}.meta{color:#a1a1a6}.alt{background:#1c1c1e;border-color:#3a3a3c}}";

pub fn is_vi(head: &str) -> bool {
    head.split("\r\n")
        .skip(1)
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.trim().eq_ignore_ascii_case("accept-language").then_some(value)
        })
        .and_then(|value| value.split([',', ';']).next())
        .is_some_and(|tag| tag.trim().to_ascii_lowercase().starts_with("vi"))
}

fn clean_version(raw: &str) -> String {
    raw.chars().filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+')).take(MAX_VERSION).collect()
}

fn size_text(bytes: u64) -> String {
    let tenths = bytes.saturating_mul(10) / (1 << 20);
    format!("{}.{} MB", tenths / 10, tenths % 10)
}

pub fn render(vi: bool, version: &str, size: u64) -> String {
    let t = if vi { &VI } else { &EN };
    let version = clean_version(version);
    let mut meta = String::new();
    if !version.is_empty() {
        let _ = write!(meta, "{} {version}, ", t.version);
    }
    let _ = write!(meta, "{} {}", t.size, size_text(size));
    format!(
        "<!doctype html><html lang=\"{lang}\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><meta name=\"referrer\" content=\"no-referrer\"><title>{title}</title><style>{STYLE}</style></head><body><main><h1>Orrery Remote</h1><p class=\"meta\">{meta}</p><p>{step1}</p><a class=\"btn\" href=\"{file}\" download>{download}</a><p>{step2}</p><a class=\"btn alt\" href=\"{pair}\">{pair_text}</a></main></body></html>",
        lang = t.lang,
        title = t.title,
        step1 = t.step1,
        step2 = t.step2,
        download = t.download,
        pair_text = t.pair,
        file = super::install::FILE,
        pair = super::install::PAIR,
    )
}

pub fn pct(text: &str) -> String {
    let mut out = String::with_capacity(text.len() * 3);
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(char::from(b));
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

pub fn pair_url(qr: &str) -> String {
    format!("{PAIR_SCHEME}{}", pct(qr))
}
