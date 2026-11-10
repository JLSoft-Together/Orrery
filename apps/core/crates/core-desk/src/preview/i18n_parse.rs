use regex::Regex;
use std::sync::OnceLock;

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub value: String,
    pub line: usize,
    pub fixed: bool,
}

#[derive(Default, Debug)]
pub struct Entries {
    list: Vec<(String, Entry)>,
}

impl Entries {
    pub fn set(&mut self, key: String, e: Entry) {
        match self.list.iter_mut().find(|(k, _)| *k == key) {
            Some((_, x)) => *x = e,
            None => self.list.push((key, e)),
        }
    }

    pub fn get(&self, key: &str) -> Option<&Entry> {
        self.list.iter().find(|(k, _)| k == key).map(|(_, e)| e)
    }

    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &Entry)> {
        self.list.iter().map(|(k, e)| (k, e))
    }
}

fn starts_at(c: &[char], i: usize, s: &str) -> bool {
    s.chars().enumerate().all(|(k, ch)| c.get(i + k) == Some(&ch))
}

fn is_word(ch: Option<&char>) -> bool {
    ch.map(|c| c.is_ascii_alphanumeric() || *c == '_').unwrap_or(false)
}

fn closing(c: &[char], from: usize, tag: &str) -> Option<(usize, usize)> {
    let pat = format!("</{tag}");
    let mut k = from;
    while k < c.len() {
        if starts_at(c, k, &pat) {
            let mut j = k + pat.chars().count();
            while j < c.len() && c[j].is_whitespace() {
                j += 1;
            }
            if c.get(j) == Some(&'>') {
                return Some((k, j + 1));
            }
        }
        k += 1;
    }
    None
}

struct Hit {
    start: usize,
    end: usize,
    attrs: String,
    body: Option<String>,
}

fn match_at(c: &[char], p: usize) -> Option<Hit> {
    if c[p] != '<' {
        return None;
    }
    for tag in ["string", "plurals", "string-array"] {
        let t = p + 1;
        if !starts_at(c, t, tag) {
            continue;
        }
        let a = t + tag.chars().count();
        if is_word(c.get(a)) {
            continue;
        }
        let mut e = a;
        while e < c.len() && c[e] != '>' {
            if c[e] == '/' && c.get(e + 1) == Some(&'>') {
                return Some(Hit { start: p, end: e + 2, attrs: c[a..e].iter().collect(), body: None });
            }
            e += 1;
        }
        if e >= c.len() {
            continue;
        }
        if let Some((k, end)) = closing(c, e + 1, tag) {
            return Some(Hit { start: p, end, attrs: c[a..e].iter().collect(), body: Some(c[e + 1..k].iter().collect()) });
        }
    }
    None
}

pub fn parse_android_strings(xml: &str) -> Entries {
    static COMMENT: OnceLock<Regex> = OnceLock::new();
    static NAME: OnceLock<Regex> = OnceLock::new();
    static FIXED: OnceLock<Regex> = OnceLock::new();
    let text = COMMENT.get_or_init(|| Regex::new(r"(?s)<!--.*?-->").expect("regex")).replace_all(xml, |m: &regex::Captures| m[0].chars().map(|ch| if ch == '\n' { '\n' } else { ' ' }).collect::<String>());
    let c: Vec<char> = text.chars().collect();
    let name_re = NAME.get_or_init(|| Regex::new(r#"\bname\s*=\s*"([^"]+)""#).expect("regex"));
    let fixed_re = FIXED.get_or_init(|| Regex::new(r#"\btranslatable\s*=\s*"false""#).expect("regex"));
    let mut out = Entries::default();
    let (mut last, mut line, mut p) = (0usize, 1usize, 0usize);
    while p < c.len() {
        let Some(hit) = match_at(&c, p) else {
            p += 1;
            continue;
        };
        p = hit.end.max(hit.start + 1);
        let Some(name) = name_re.captures(&hit.attrs).map(|m| m[1].to_string()) else { continue };
        line += c[last..hit.start].iter().filter(|ch| **ch == '\n').count();
        last = hit.start;
        let value = hit.body.unwrap_or_default().trim_matches(super::branches::js_space).to_string();
        out.set(name, Entry { value, line, fixed: fixed_re.is_match(&hit.attrs) });
    }
    out
}

struct Parser {
    s: Vec<char>,
    i: usize,
    line: usize,
    out: Entries,
}

impl Parser {
    fn ws(&mut self) {
        while self.i < self.s.len() {
            match self.s[self.i] {
                '\n' => self.line += 1,
                ' ' | '\t' | '\r' => {}
                _ => break,
            }
            self.i += 1;
        }
    }

    fn at(&self) -> Option<char> {
        self.s.get(self.i).copied()
    }

    fn string(&mut self) -> Option<String> {
        if self.at() != Some('"') {
            return None;
        }
        self.i += 1;
        let start = self.i;
        while self.i < self.s.len() && self.s[self.i] != '"' {
            if self.s[self.i] == '\\' {
                self.i += 1;
            } else if self.s[self.i] == '\n' {
                return None;
            }
            self.i += 1;
        }
        if self.i >= self.s.len() {
            return None;
        }
        let raw: String = self.s[start..self.i].iter().collect();
        self.i += 1;
        serde_json::from_str::<String>(&format!("\"{raw}\"")).ok()
    }

    fn value(&mut self, key: Option<&str>, key_line: usize) -> Option<()> {
        self.ws();
        let start = self.i;
        let c = self.at();
        if c == Some('{') {
            self.i += 1;
            self.ws();
            if self.at() == Some('}') {
                self.i += 1;
                return Some(());
            }
            loop {
                self.ws();
                let at = self.line;
                let k = self.string()?;
                self.ws();
                let colon = self.at();
                self.i += 1;
                if colon != Some(':') {
                    return None;
                }
                let child = key.map(|p| if p.is_empty() { k.clone() } else { format!("{p}.{k}") });
                self.value(child.as_deref(), at)?;
                self.ws();
                match self.at() {
                    Some(',') => self.i += 1,
                    Some('}') => {
                        self.i += 1;
                        return Some(());
                    }
                    _ => return None,
                }
            }
        }
        let leaf: String;
        if c == Some('[') {
            self.i += 1;
            self.ws();
            if self.at() == Some(']') {
                self.i += 1;
            } else {
                loop {
                    let l = self.line;
                    self.value(None, l)?;
                    self.ws();
                    if self.at() == Some(',') {
                        self.i += 1;
                        continue;
                    }
                    let close = self.at();
                    self.i += 1;
                    if close == Some(']') {
                        break;
                    }
                    return None;
                }
            }
            leaf = self.s[start..self.i.min(self.s.len())].iter().collect();
        } else if c == Some('"') {
            leaf = self.string()?;
        } else {
            static LIT: OnceLock<Regex> = OnceLock::new();
            let window: String = self.s[self.i..(self.i + 64).min(self.s.len())].iter().collect();
            let m = LIT.get_or_init(|| Regex::new(r"^(?:true|false|null|-?[0-9]+(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?)").expect("regex")).find(&window)?;
            leaf = m.as_str().to_string();
            self.i += leaf.chars().count();
        }
        if let Some(k) = key.filter(|k| !k.is_empty()) {
            self.out.set(k.to_string(), Entry { value: leaf, line: key_line, fixed: false });
        }
        Some(())
    }
}

pub fn parse_json_keys(text: &str) -> Option<Entries> {
    let src = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut p = Parser { s: src.chars().collect(), i: 0, line: 1, out: Entries::default() };
    p.ws();
    if p.at() != Some('{') {
        return None;
    }
    p.value(Some(""), 1)?;
    p.ws();
    if p.i == p.s.len() {
        Some(p.out)
    } else {
        None
    }
}

pub fn parse_arb(text: &str) -> Option<(Entries, Option<String>)> {
    let all = parse_json_keys(text)?;
    let mut entries = Entries::default();
    let mut locale = None;
    for (k, e) in all.iter() {
        if k == "@@locale" {
            locale = Some(e.value.clone());
        } else if !k.starts_with('@') {
            entries.set(k.clone(), e.clone());
        }
    }
    Some((entries, locale))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_resources() {
        let x = parse_android_strings("<resources>\n<!-- <string name=\"c\">x</string>\n-->\n<string name=\"a\">Hi</string>\n<string name=\"b\" translatable=\"false\"/>\n<plurals name=\"p\"><item>x</item></plurals>\n</resources>");
        assert_eq!(x.get("a").unwrap().line, 4);
        assert!(x.get("b").unwrap().fixed);
        assert_eq!(x.get("p").unwrap().value, "<item>x</item>");
        assert!(x.get("c").is_none());
        let j = parse_json_keys("{\n \"a\": {\"b\": \"x\\\"y\"},\n \"n\": 1.5, \"arr\": [1, {\"z\": 2}], \"e\": {}\n}").unwrap();
        assert_eq!(j.get("a.b").unwrap().value, "x\"y");
        assert_eq!(j.get("arr").unwrap().value, "[1, {\"z\": 2}]");
        assert_eq!(j.get("n").unwrap().line, 3);
        assert!(j.get("e").is_none());
        assert!(parse_json_keys("[1]").is_none());
        assert!(parse_json_keys("{\"a\": 1} x").is_none());
        let (e, l) = parse_arb("{\"@@locale\": \"vi\", \"a\": \"x\", \"@a\": {}}").unwrap();
        assert_eq!((e.len(), l.as_deref()), (1, Some("vi")));
    }
}
