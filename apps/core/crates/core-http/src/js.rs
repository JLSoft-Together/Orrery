pub fn is_js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{feff}'
    )
}

pub fn trim(s: &str) -> &str {
    s.trim_matches(is_js_space)
}

pub fn len16(s: &str) -> usize {
    s.encode_utf16().count()
}

fn digits(s: &str, n: usize) -> Option<u32> {
    (s.len() == n && s.bytes().all(|b| b.is_ascii_digit())).then(|| s.parse().unwrap_or(0))
}

fn zone_ok(z: &str) -> bool {
    if z.is_empty() || z == "Z" {
        return true;
    }
    let Some(rest) = z.strip_prefix(['+', '-']) else { return false };
    let Some((h, m)) = rest.split_once(':') else { return false };
    matches!((digits(h, 2), digits(m, 2)), (Some(h), Some(m)) if h <= 23 && m <= 59)
}

fn time_ok(t: &str) -> bool {
    let split = t.find(['Z', '+', '-']).unwrap_or(t.len());
    let (clock, zone) = t.split_at(split);
    if !zone_ok(zone) {
        return false;
    }
    let mut parts = clock.split(':');
    let (Some(h), Some(m)) = (parts.next(), parts.next()) else { return false };
    let (Some(h), Some(m)) = (digits(h, 2), digits(m, 2)) else { return false };
    if h > 24 || m > 59 || (h == 24 && m != 0) {
        return false;
    }
    match parts.next() {
        None => true,
        Some(sec) => {
            if parts.next().is_some() {
                return false;
            }
            let (s, frac) = sec.split_once('.').map_or((sec, None), |(s, f)| (s, Some(f)));
            digits(s, 2).is_some_and(|s| s <= 59) && frac.is_none_or(|f| !f.is_empty() && f.bytes().all(|b| b.is_ascii_digit()))
        }
    }
}

pub fn date_parse_ok(s: &str) -> bool {
    let s = trim(s);
    let (date, time) = match s.find(['T', ' ']) {
        Some(i) => (&s[..i], Some(&s[i + 1..])),
        None => (s, None),
    };
    let mut parts = date.splitn(3, '-');
    let year = parts.next().unwrap_or("");
    let year_ok = digits(year, 4).is_some() || (year.len() == 7 && year.starts_with(['+', '-']) && year[1..].bytes().all(|b| b.is_ascii_digit()));
    if !year_ok {
        return false;
    }
    let month = parts.next();
    let day = parts.next();
    if month.is_some_and(|m| !digits(m, 2).is_some_and(|m| (1..=12).contains(&m))) {
        return false;
    }
    if day.is_some_and(|d| !digits(d, 2).is_some_and(|d| (1..=31).contains(&d))) {
        return false;
    }
    match time {
        None => true,
        Some(t) => month.is_some() && day.is_some() && time_ok(t),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_like_js() {
        assert_eq!(trim("\u{feff} a b\u{a0}\n"), "a b");
        assert_eq!(trim("\u{85}x"), "\u{85}x");
    }

    #[test]
    fn counts_utf16_units() {
        assert_eq!(len16("ab"), 2);
        assert_eq!(len16("\u{1F600}"), 2);
    }

    #[test]
    fn accepts_iso_dates_only() {
        for ok in ["2026-10-09T00:00:00.000Z", "2026-10-09", "2026-10", "2026", "2026-10-09T12:30", "2026-10-09T12:30:15+07:00", "2026-10-09 12:30:15"] {
            assert!(date_parse_ok(ok), "{ok}");
        }
        for bad in ["nope", "", "2026-13-01", "2026-10-09T25:00", "2026-10-09T12", "2026-10-09T12:30:15+0700", "x2026-10-09"] {
            assert!(!date_parse_ok(bad), "{bad}");
        }
    }
}
