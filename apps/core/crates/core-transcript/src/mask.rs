use crate::jsstr::js_space;

fn is_word(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn fold(c: char) -> char {
    let mut it = c.to_lowercase();
    match (it.next(), it.next()) {
        (Some(l), None) => l,
        _ => c,
    }
}

fn same_ci(a: char, b: char) -> bool {
    a == b || fold(a) == fold(b)
}

fn replace_home(text: &str, form: &str, ci: bool) -> String {
    if form.is_empty() {
        return text.to_owned();
    }
    let t: Vec<char> = text.chars().collect();
    let f: Vec<char> = form.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < t.len() {
        let hit = i + f.len() <= t.len() && t[i..i + f.len()].iter().zip(&f).all(|(a, b)| if ci { same_ci(*a, *b) } else { a == b });
        if hit {
            out.push('~');
            i += f.len();
        } else {
            out.push(t[i]);
            i += 1;
        }
    }
    out
}

fn home_forms(home: &str, platform: &str) -> Vec<String> {
    if home.chars().count() <= 1 {
        return Vec::new();
    }
    if platform == "win32" {
        vec![home.replace('\\', "\\\\"), home.to_owned(), home.replace('\\', "/")]
    } else {
        vec![home.to_owned()]
    }
}

struct Token {
    prefix: &'static str,
    class: fn(char) -> bool,
    min: usize,
    exact: bool,
}

fn class_sk(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

fn class_alnum(c: char) -> bool {
    c.is_ascii_alphanumeric()
}

fn class_pat(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

fn class_akia(c: char) -> bool {
    c.is_ascii_digit() || c.is_ascii_uppercase()
}

fn class_xox(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-'
}

fn replace_token(text: &str, tok: &Token) -> String {
    let t: Vec<char> = text.chars().collect();
    let p: Vec<char> = tok.prefix.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < t.len() {
        let boundary = i == 0 || !is_word(t[i - 1]);
        if boundary && i + p.len() <= t.len() && t[i..i + p.len()] == p[..] {
            let mut j = i + p.len();
            let start = j;
            while j < t.len() && (tok.class)(t[j]) && (!tok.exact || j - start < tok.min) {
                j += 1;
            }
            let ok = j - start >= tok.min && (!tok.exact || t.get(j).is_none_or(|c| !is_word(*c)));
            if ok {
                out.push_str("[masked]");
                i = j;
                continue;
            }
        }
        out.push(t[i]);
        i += 1;
    }
    out
}

fn tokens() -> [Token; 8] {
    [
        Token { prefix: "sk-", class: class_sk, min: 8, exact: false },
        Token { prefix: "ghp_", class: class_alnum, min: 16, exact: false },
        Token { prefix: "gho_", class: class_alnum, min: 16, exact: false },
        Token { prefix: "ghu_", class: class_alnum, min: 16, exact: false },
        Token { prefix: "ghs_", class: class_alnum, min: 16, exact: false },
        Token { prefix: "ghr_", class: class_alnum, min: 16, exact: false },
        Token { prefix: "github_pat_", class: class_pat, min: 16, exact: false },
        Token { prefix: "AKIA", class: class_akia, min: 16, exact: true },
    ]
}

fn replace_xox(text: &str) -> String {
    let t: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < t.len() {
        let boundary = i == 0 || !is_word(t[i - 1]);
        if boundary && i + 4 <= t.len() && t[i] == 'x' && t[i + 1] == 'o' && t[i + 2] == 'x' && (t[i + 3] == 'b' || t[i + 3] == 'p') && t.get(i + 4) == Some(&'-') {
            let mut j = i + 5;
            while j < t.len() && class_xox(t[j]) {
                j += 1;
            }
            if j - (i + 5) >= 8 {
                out.push_str("[masked]");
                i = j;
                continue;
            }
        }
        out.push(t[i]);
        i += 1;
    }
    out
}

fn mask_common(text: &str, homes: &[String], ci: bool) -> String {
    let mut out = text.to_owned();
    for form in homes {
        out = replace_home(&out, form, ci);
    }
    for tok in tokens() {
        out = replace_token(&out, &tok);
    }
    replace_xox(&out)
}

fn long_runs(text: &str, class: fn(char) -> bool, min: usize, mut f: impl FnMut(&str) -> String) -> String {
    let t: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < t.len() {
        if class(t[i]) {
            let mut j = i;
            while j < t.len() && class(t[j]) {
                j += 1;
            }
            let run: String = t[i..j].iter().collect();
            if j - i >= min {
                out.push_str(&f(&run));
            } else {
                out.push_str(&run);
            }
            i = j;
        } else {
            out.push(t[i]);
            i += 1;
        }
    }
    out
}

fn class_blob(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '+' | '_' | '=')
}

fn class_wide(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=' | '_' | '.' | '-')
}

fn all_cases(s: &str) -> bool {
    s.chars().any(|c| c.is_ascii_digit()) && s.chars().any(|c| c.is_ascii_lowercase()) && s.chars().any(|c| c.is_ascii_uppercase())
}

fn keyword_len(t: &[char], i: usize) -> Option<usize> {
    let ci = |at: usize, w: &str| at + w.len() <= t.len() && t[at..at + w.len()].iter().zip(w.chars()).all(|(a, b)| a.is_ascii() && a.eq_ignore_ascii_case(&b));
    for w in ["password", "passwd", "secret", "token"] {
        if ci(i, w) {
            return Some(w.len());
        }
    }
    if ci(i, "api") {
        let mut j = i + 3;
        if t.get(j).is_some_and(|c| *c == '_' || *c == '-') && ci(j + 1, "key") {
            j += 1;
        }
        if ci(j, "key") {
            return Some(j + 3 - i);
        }
    }
    None
}

fn mask_key_values(text: &str) -> String {
    let t: Vec<char> = text.chars().collect();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < t.len() {
        if let Some(k) = keyword_len(&t, i) {
            let mut j = i + k;
            while j < t.len() && js_space(t[j]) {
                j += 1;
            }
            if t.get(j).is_some_and(|c| *c == ':' || *c == '=') {
                j += 1;
                while j < t.len() && js_space(t[j]) {
                    j += 1;
                }
                let start = j;
                while j < t.len() && !js_space(t[j]) {
                    j += 1;
                }
                if j - start >= 4 {
                    out.extend(&t[i..start]);
                    out.push_str("[masked]");
                    i = j;
                    continue;
                }
            }
        }
        out.push(t[i]);
        i += 1;
    }
    out
}

pub fn mask_secrets(text: &str, home: &str, platform: &str) -> String {
    let ci = platform == "win32";
    let mut out = mask_common(text, &home_forms(home, platform), ci);
    out = long_runs(&out, class_blob, 32, |_| "[masked]".to_owned());
    out = mask_key_values(&out);
    long_runs(&out, class_wide, 32, |run| if !run.starts_with('/') && all_cases(run) { "[masked]".to_owned() } else { run.to_owned() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_tokens_key_values_and_home() {
        let out = mask_secrets("use sk-ant-abcdef1234567890 and password=hunter22 in C:\\Users\\me\\proj", "C:\\Users\\me", "win32");
        assert!(!out.contains("sk-ant"));
        assert!(!out.contains("hunter22"));
        assert!(out.contains("~\\proj"));
        assert_eq!(mask_secrets("node vite --token=abcd1234secret", "/home/me", "linux"), "node vite --token=[masked]");
        assert_eq!(mask_secrets("api_key : abcd1 xx", "/h", "linux"), "api_key : [masked] xx");
        assert_eq!(mask_secrets("token=abc", "/h", "linux"), "token=abc");
    }

    #[test]
    fn masks_long_opaque_runs_but_keeps_paths() {
        let blob = "A".repeat(33);
        assert_eq!(mask_secrets(&blob, "", "linux"), "[masked]");
        let mixed = "abcDEF123abcDEF123-abcDEF123abcDEF123";
        assert_eq!(mask_secrets(mixed, "", "linux"), "[masked]");
        let path = "/usr/local/lib/abcDEF123abcDEF123-abcDEF123abcDEF123";
        assert_eq!(mask_secrets(path, "", "linux"), path);
        assert_eq!(mask_secrets("AKIAABCDEFGHIJKLMNOP end", "", "linux"), "[masked] end");
        assert_eq!(mask_secrets("xoxb-1234567890", "", "linux"), "[masked]");
        assert_eq!(mask_secrets("ghp_abcdefghijklmnop1234", "", "linux"), "[masked]");
    }

    #[test]
    fn windows_home_is_case_insensitive_and_slash_agnostic() {
        assert_eq!(mask_secrets("c:\\users\\ME\\x and C:/Users/me/y and C:\\\\Users\\\\me\\\\z", "C:\\Users\\me", "win32"), "~\\x and ~/y and ~\\\\z");
        assert_eq!(mask_secrets("/home/me/x", "/home/me", "linux"), "~/x");
        assert_eq!(mask_secrets("/x", "/", "linux"), "/x");
    }
}
