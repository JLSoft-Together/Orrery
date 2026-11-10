use super::paths::{posix_join, win_join};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    Win32,
    Darwin,
}

impl Platform {
    pub const HOST: Platform = if cfg!(windows) { Platform::Win32 } else { Platform::Darwin };
}

pub fn default_data_dir(local_app_data: Option<&str>, platform: Platform, home: &str) -> String {
    match platform {
        Platform::Win32 => {
            let base = match local_app_data {
                Some(v) if !v.is_empty() => v.to_string(),
                _ => win_join(&[home, "AppData", "Local"]),
            };
            win_join(&[&base, "ccm"])
        }
        Platform::Darwin => posix_join(&[home, "Library", "Application Support", "ccm"]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_node_cases() {
        let cases = [
            (Some("C:\\Users\\a\\AppData\\Local"), Platform::Win32, "C:\\Users\\a", "C:\\Users\\a\\AppData\\Local\\ccm"),
            (None, Platform::Win32, "C:\\Users\\a", "C:\\Users\\a\\AppData\\Local\\ccm"),
            (Some(""), Platform::Win32, "C:\\Users\\a", "C:\\Users\\a\\AppData\\Local\\ccm"),
            (None, Platform::Darwin, "/Users/a", "/Users/a/Library/Application Support/ccm"),
        ];
        for (env, platform, home, expected) in cases {
            assert_eq!(default_data_dir(env, platform, home), expected);
        }
    }
}
