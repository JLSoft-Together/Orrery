use std::cmp::Ordering;

const PUNCT: &str = "\t\n\u{b}\u{c}\r _-,;:!?.'\"()[]{}@*/\\&#%`^+<=>|~$";

fn primary(c: char) -> (u32, u32) {
    if let Some(i) = PUNCT.find(c) {
        return (0, i as u32);
    }
    if c.is_ascii_digit() {
        return (1, c as u32);
    }
    if c.is_ascii_alphabetic() {
        return (2, c.to_ascii_lowercase() as u32);
    }
    let lower = c.to_lowercase().next().unwrap_or(c);
    (3, lower as u32)
}

fn tertiary(c: char) -> u32 {
    u32::from(c.is_uppercase())
}

pub fn locale_cmp(a: &str, b: &str) -> Ordering {
    let pa = a.chars().map(primary);
    let pb = b.chars().map(primary);
    pa.cmp(pb)
        .then_with(|| a.chars().map(tertiary).cmp(b.chars().map(tertiary)))
        .then_with(|| a.cmp(b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orders_like_icu_root_for_ascii_names() {
        let mut v = vec!["Read", "bash", "Bash", "_x", "a-b", "a_b", "Zed", "9lives", "edit"];
        v.sort_by(|a, b| locale_cmp(a, b));
        assert_eq!(v, vec!["_x", "9lives", "a_b", "a-b", "bash", "Bash", "edit", "Read", "Zed"]);
    }
}
