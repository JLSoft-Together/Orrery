#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OsaError {
    Denied,
    NotInstalled,
    Cancelled,
    Failed,
}

pub fn parse_osa_error(stderr: &str) -> OsaError {
    if stderr.contains("(-1743)") {
        OsaError::Denied
    } else if stderr.contains("(-1728)") || stderr.contains("Can\u{2019}t get application") || stderr.contains("Can't get application") {
        OsaError::NotInstalled
    } else if stderr.contains("(-128)") {
        OsaError::Cancelled
    } else {
        OsaError::Failed
    }
}

pub fn has_control_chars(s: &str) -> bool {
    s.chars().any(|c| (c as u32) < 0x20 || c as u32 == 0x7f)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osascript_errors_map_like_osascript_ts() {
        assert_eq!(parse_osa_error("execution error: Not authorized to send Apple events to Terminal. (-1743)"), OsaError::Denied);
        assert_eq!(parse_osa_error("Can\u{2019}t get application id \"com.googlecode.iterm2\". (-1728)"), OsaError::NotInstalled);
        assert_eq!(parse_osa_error("Can't get application \"iTerm\""), OsaError::NotInstalled);
        assert_eq!(parse_osa_error("User canceled. (-128)"), OsaError::Cancelled);
        assert_eq!(parse_osa_error("boom"), OsaError::Failed);
        assert!(has_control_chars("/a\u{7}b"));
        assert!(!has_control_chars("/Users/me/my app"));
    }
}
