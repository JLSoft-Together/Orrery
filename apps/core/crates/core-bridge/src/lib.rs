pub mod hook;
pub mod jsfmt;
pub mod statusline;

use std::path::PathBuf;

pub const SESSION_LEN: usize = 36;

pub fn is_session_id(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != SESSION_LEN {
        return false;
    }
    b.iter().enumerate().all(|(i, c)| match i {
        8 | 13 | 18 | 23 => *c == b'-',
        _ => c.is_ascii_hexdigit(),
    })
}

fn var(k: &str) -> Option<String> {
    std::env::var(k).ok().filter(|v| !v.is_empty())
}

pub fn home_dir() -> PathBuf {
    let h = if cfg!(windows) { var("USERPROFILE").or_else(|| var("HOME")) } else { var("HOME") };
    h.map(PathBuf::from).unwrap_or_default()
}

pub fn data_dir() -> PathBuf {
    if let Some(d) = var("CCM_DATA_DIR") {
        return PathBuf::from(d);
    }
    let home = home_dir();
    if cfg!(windows) {
        return var("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|| home.join("AppData").join("Local")).join("ccm");
    }
    home.join("Library").join("Application Support").join("ccm")
}

pub fn is_js_space(c: char) -> bool {
    c.is_whitespace() || c == '\u{feff}'
}

pub fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

pub fn write_replace(file: &std::path::Path, tmp: &std::path::Path, text: &str) -> std::io::Result<()> {
    std::fs::write(tmp, text)?;
    std::fs::rename(tmp, file)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_ids() {
        assert!(is_session_id("0123abcd-0000-4000-8000-ABCDEF012345"));
        assert!(!is_session_id("0123abcd-0000-4000-8000-ABCDEF01234"));
        assert!(!is_session_id("0123abcd00000-4000-8000-ABCDEF012345"));
        assert!(!is_session_id("0123abcg-0000-4000-8000-ABCDEF012345"));
    }
}
