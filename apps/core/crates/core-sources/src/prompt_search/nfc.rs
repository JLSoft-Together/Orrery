use super::nfc_table::PAIRS;

fn ccc(c: char) -> u8 {
    match c as u32 {
        0x300..=0x314 | 0x33D..=0x344 | 0x346 | 0x34A..=0x34C | 0x350..=0x352 | 0x357 | 0x35B | 0x363..=0x36F => 230,
        0x315 | 0x31A | 0x358 => 232,
        0x316..=0x319 | 0x31C..=0x320 | 0x323..=0x326 | 0x329..=0x333 | 0x339..=0x33C | 0x347..=0x349 | 0x34D | 0x34E | 0x353..=0x356 | 0x359 | 0x35A => 220,
        0x31B => 216,
        0x321 | 0x322 | 0x327 | 0x328 => 202,
        0x334..=0x338 => 1,
        0x345 => 240,
        0x35C | 0x35F | 0x362 => 233,
        0x35D | 0x35E | 0x360 | 0x361 => 234,
        _ => 0,
    }
}

fn is_mark(c: char) -> bool {
    (0x300..=0x36F).contains(&(c as u32)) && ccc(c) != 0
}

fn compose(base: char, mark: char) -> Option<char> {
    let key = (base as u32, mark as u32);
    PAIRS.binary_search_by(|p| (p.0, p.1).cmp(&key)).ok().and_then(|i| char::from_u32(PAIRS[i].2))
}

pub fn nfc(s: &str) -> String {
    if s.is_ascii() {
        return s.to_owned();
    }
    let chars: Vec<char> = s.chars().collect();
    let mut out: Vec<char> = Vec::with_capacity(chars.len());
    let mut base_at: Option<usize> = None;
    let mut last_ccc = 0u8;
    let mut i = 0;
    while i < chars.len() {
        if !is_mark(chars[i]) {
            out.push(chars[i]);
            base_at = Some(out.len() - 1);
            last_ccc = 0;
            i += 1;
            continue;
        }
        let mut j = i;
        while j < chars.len() && is_mark(chars[j]) {
            j += 1;
        }
        let mut run: Vec<char> = chars[i..j].to_vec();
        run.sort_by_key(|c| ccc(*c));
        for m in run {
            let cc = ccc(m);
            let blocked = last_ccc != 0 && last_ccc >= cc;
            if let (Some(b), false) = (base_at, blocked) {
                if let Some(x) = compose(out[b], m) {
                    out[b] = x;
                    continue;
                }
            }
            out.push(m);
            last_ccc = cc;
        }
        i = j;
    }
    out.into_iter().collect()
}

pub fn fold_text(s: &str) -> String {
    nfc(s).to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn composes_decomposed_vietnamese_and_latin() {
        assert_eq!(nfc("e\u{0302}\u{0301}"), "\u{1ebf}");
        assert_eq!(nfc("e\u{0323}\u{0302}"), "\u{1ec7}");
        assert_eq!(nfc("e\u{0302}\u{0323}"), "\u{1ec7}");
        assert_eq!(nfc("Vie\u{0302}\u{0323}t Nam"), "Vi\u{1ec7}t Nam");
        assert_eq!(nfc("a\u{0301}"), "\u{e1}");
        assert_eq!(nfc("\u{e1}"), "\u{e1}");
        assert_eq!(nfc("o\u{031B}\u{0301}"), "\u{1edb}");
        assert_eq!(nfc("x\u{0301}"), "x\u{0301}");
        assert_eq!(nfc(" \u{0301}"), " \u{0301}");
        assert_eq!(fold_text("PAYWALL Ca\u{0301}"), "paywall c\u{e1}");
    }
}
