use crate::jsstr::{js_slice, js_space, word};

const CONFLICT_SCAN_MAX: usize = 20_000;

fn ci_at(c: &[char], at: usize, w: &str) -> bool {
    let n = w.len();
    at + n <= c.len() && c[at..at + n].iter().zip(w.chars()).all(|(a, b)| a.is_ascii() && a.eq_ignore_ascii_case(&b))
}

fn cs_at(c: &[char], at: usize, w: &str) -> bool {
    let n = w.len();
    at + n <= c.len() && c[at..at + n].iter().copied().eq(w.chars())
}

fn digit_run(c: &[char], at: usize) -> usize {
    c[at.min(c.len())..].iter().take_while(|x| x.is_ascii_digit()).count()
}

fn number_at(c: &[char], at: usize, len: usize) -> u32 {
    c[at..at + len].iter().fold(0u32, |a, d| a * 10 + d.to_digit(10).unwrap_or(0))
}

fn after_marker(c: &[char], from: usize) -> Option<u32> {
    let mut j = from;
    while j < c.len() {
        let ch = c[j];
        if ch == ':' || js_space(ch) {
            let run = digit_run(c, j + 1);
            if (2..=5).contains(&run) && c.get(j + 1 + run) != Some(&'.') {
                return Some(number_at(c, j + 1, run));
            }
        }
        if ch == '\n' {
            return None;
        }
        j += 1;
    }
    None
}

fn first_marker(c: &[char], w: &str, ci: bool, word_end: bool) -> Option<u32> {
    for i in 0..c.len() {
        let hit = if ci { ci_at(c, i, w) } else { cs_at(c, i, w) };
        if !hit || (word_end && c.get(i + w.len()).is_some_and(|x| word(*x))) {
            continue;
        }
        if let Some(port) = after_marker(c, i + w.len()) {
            return Some(port);
        }
    }
    None
}

fn spaces(c: &[char], at: usize) -> usize {
    c[at.min(c.len())..].iter().take_while(|x| js_space(**x)).count()
}

fn in_use_phrase(c: &[char]) -> Option<u32> {
    for i in 0..c.len() {
        if (i > 0 && word(c[i - 1])) || !ci_at(c, i, "port") {
            continue;
        }
        let mut j = i + 4;
        let s = spaces(c, j);
        if s == 0 {
            continue;
        }
        j += s;
        if c.get(j) == Some(&':') {
            j += 1;
        }
        let run = digit_run(c, j);
        if !(2..=5).contains(&run) {
            continue;
        }
        let port = number_at(c, j, run);
        let mut k = j + run;
        let mut ok = true;
        for w in ["is", "already", "in", "use"] {
            let s = spaces(c, k);
            if s == 0 || !ci_at(c, k + s, w) {
                ok = false;
                break;
            }
            k += s + w.len();
        }
        if ok {
            return Some(port);
        }
    }
    None
}

pub fn port_conflict_in(text: &str) -> Option<u32> {
    let c: Vec<char> = js_slice(text, CONFLICT_SCAN_MAX).chars().collect();
    let valid = |p: Option<u32>| p.filter(|p| *p > 0 && *p <= 65_535);
    valid(first_marker(&c, "EADDRINUSE", false, true))
        .or_else(|| valid(first_marker(&c, "address already in use", true, false)))
        .or_else(|| valid(in_use_phrase(&c)))
}

#[cfg(test)]
mod tests {
    use super::port_conflict_in;

    #[test]
    fn finds_the_port_like_the_regexes() {
        assert_eq!(port_conflict_in("Error: listen EADDRINUSE: address already in use :::3000"), Some(3000));
        assert_eq!(port_conflict_in("listen EADDRINUSE 127.0.0.1:8080\n"), Some(8080));
        assert_eq!(port_conflict_in("EADDRINUSE\n:5000"), None);
        assert_eq!(port_conflict_in("EADDRINUSE\n5000"), Some(5000));
        assert_eq!(port_conflict_in("EADDRINUSEX :5000"), None);
        assert_eq!(port_conflict_in("Address already in use (os error 98) at 0.0.0.0:7357"), Some(98));
        assert_eq!(port_conflict_in("Port 5173 is already in use"), Some(5173));
        assert_eq!(port_conflict_in("port :5173  is\talready in use"), Some(5173));
        assert_eq!(port_conflict_in("EADDRINUSE on 1.2.3.4 :123456"), None);
        assert_eq!(port_conflict_in("EADDRINUSE :99999"), None);
        assert_eq!(port_conflict_in("EADDRINUSE :00 :8080"), None);
        assert_eq!(port_conflict_in("EADDRINUSE :99999 port 8080 is already in use"), Some(8080));
        assert_eq!(port_conflict_in("nothing here"), None);
    }
}
