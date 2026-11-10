#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Bold,
    Italic,
    Underline,
    Strike,
    Code,
    Pre,
    Url,
    Quote,
    Spoiler,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entity {
    pub kind: Kind,
    pub offset: i32,
    pub length: i32,
    pub url: Option<String>,
    pub collapsed: bool,
}

fn kind_of(tag: &str) -> Option<Kind> {
    Some(match tag {
        "b" | "strong" => Kind::Bold,
        "i" | "em" => Kind::Italic,
        "u" | "ins" => Kind::Underline,
        "s" | "strike" | "del" => Kind::Strike,
        "code" => Kind::Code,
        "pre" => Kind::Pre,
        "a" => Kind::Url,
        "blockquote" => Kind::Quote,
        "tg-spoiler" => Kind::Spoiler,
        _ => return None,
    })
}

fn attr(inner: &str, name: &str) -> Option<String> {
    let at = inner.find(&format!("{name}="))?;
    let rest = &inner[at + name.len() + 1..];
    let quote = rest.chars().next()?;
    if quote == '"' || quote == '\'' {
        let body = &rest[1..];
        let end = body.find(quote)?;
        return Some(unescape(&body[..end]));
    }
    Some(unescape(rest.split_whitespace().next().unwrap_or("")))
}

fn entity_char(name: &str) -> Option<char> {
    match name {
        "amp" => Some('&'),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "nbsp" => Some('\u{a0}'),
        _ => {
            let num = name.strip_prefix('#')?;
            let code = match num.strip_prefix('x').or_else(|| num.strip_prefix('X')) {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => num.parse::<u32>().ok()?,
            };
            char::from_u32(code)
        }
    }
}

fn unescape(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(at) = rest.find('&') {
        out.push_str(&rest[..at]);
        let after = &rest[at + 1..];
        match after.find(';').filter(|n| *n <= 10).and_then(|n| entity_char(&after[..n]).map(|c| (c, n))) {
            Some((c, n)) => {
                out.push(c);
                rest = &after[n + 1..];
            }
            None => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

pub fn parse(html: &str) -> (String, Vec<Entity>) {
    let mut text = String::new();
    let mut pos: i32 = 0;
    let mut open: Vec<(String, Kind, i32, Option<String>, bool)> = Vec::new();
    let mut out: Vec<Entity> = Vec::new();
    let mut rest = html;
    loop {
        let next = rest.find(['<', '&']);
        let Some(at) = next else {
            text.push_str(rest);
            break;
        };
        let plain = &rest[..at];
        pos += plain.encode_utf16().count() as i32;
        text.push_str(plain);
        let after = &rest[at + 1..];
        if rest.as_bytes()[at] == b'&' {
            match after.find(';').filter(|n| *n <= 10).and_then(|n| entity_char(&after[..n]).map(|c| (c, n))) {
                Some((c, n)) => {
                    text.push(c);
                    pos += c.len_utf16() as i32;
                    rest = &after[n + 1..];
                }
                None => {
                    text.push('&');
                    pos += 1;
                    rest = after;
                }
            }
            continue;
        }
        let Some(end) = after.find('>') else {
            text.push('<');
            pos += 1;
            rest = after;
            continue;
        };
        let inner = after[..end].trim();
        rest = &after[end + 1..];
        if let Some(name) = inner.strip_prefix('/') {
            let name = name.trim().to_ascii_lowercase();
            if let Some(i) = open.iter().rposition(|o| o.0 == name) {
                let (_, kind, start, url, collapsed) = open.remove(i);
                if pos > start {
                    out.push(Entity { kind, offset: start, length: pos - start, url, collapsed });
                }
            }
            continue;
        }
        let name = inner.split(|c: char| c.is_whitespace() || c == '/').next().unwrap_or("").to_ascii_lowercase();
        let Some(kind) = kind_of(&name) else { continue };
        let url = if kind == Kind::Url { attr(inner, "href") } else { None };
        if kind == Kind::Url && url.is_none() {
            continue;
        }
        let collapsed = kind == Kind::Quote && inner.split_whitespace().any(|w| w.eq_ignore_ascii_case("expandable"));
        open.push((name, kind, pos, url, collapsed));
    }
    out.sort_by_key(|e| e.offset);
    (text, out)
}
