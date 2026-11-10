pub fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

pub fn trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

pub fn collapse_spaces(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_run = false;
    for c in s.chars() {
        if is_js_space(c) {
            if !in_run {
                out.push(' ');
            }
            in_run = true;
        } else {
            out.push(c);
            in_run = false;
        }
    }
    out
}

pub fn slice_units(s: &str, max: usize) -> String {
    let mut units = 0;
    let mut out = String::new();
    for c in s.chars() {
        units += c.len_utf16();
        if units > max {
            break;
        }
        out.push(c);
    }
    out
}

pub fn js_round(x: f64) -> f64 {
    let r = x.floor();
    if x - r >= 0.5 {
        r + 1.0
    } else {
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_and_trims_like_javascript() {
        assert_eq!(js_round(2.5), 3.0);
        assert_eq!(js_round(-2.5), -2.0);
        assert_eq!(js_round(0.499_999_999_999_999_94), 0.0);
        assert_eq!(trim("\u{feff} a \u{a0}"), "a");
        assert_eq!(collapse_spaces("a \t\u{a0}b"), "a b");
        assert_eq!(slice_units("a\u{1f600}b", 2), "a");
        assert_eq!(slice_units("a\u{1f600}b", 3), "a\u{1f600}");
    }
}
