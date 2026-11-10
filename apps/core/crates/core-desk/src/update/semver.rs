use regex::Regex;
use std::sync::OnceLock;

pub fn parse_version(raw: &str) -> Option<[u32; 3]> {
    static R: OnceLock<Regex> = OnceLock::new();
    let re = R.get_or_init(|| Regex::new(r"^v?([0-9]{1,6})\.([0-9]{1,6})(?:\.([0-9]{1,6}))?(?:[-+].*)?$").expect("regex"));
    let c = re.captures(crate::claude::js_trim(raw))?;
    let n = |i: usize| c.get(i).map_or(0, |m| m.as_str().parse().unwrap_or(0));
    Some([n(1), n(2), n(3)])
}

pub fn clean_version(raw: &str) -> Option<String> {
    parse_version(raw).map(|v| format!("{}.{}.{}", v[0], v[1], v[2]))
}

pub fn compare_versions(a: &str, b: &str) -> i32 {
    let (Some(x), Some(y)) = (parse_version(a), parse_version(b)) else { return 0 };
    for i in 0..3 {
        if x[i] != y[i] {
            return if x[i] < y[i] { -1 } else { 1 };
        }
    }
    0
}

pub fn is_newer(latest: &str, current: &str) -> bool {
    parse_version(latest).is_some() && parse_version(current).is_some() && compare_versions(latest, current) > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_like_the_ts_twin() {
        assert!(is_newer("2.10.0", "2.9.0"));
        assert!(!is_newer("2.5.0", "2.5.0"));
        assert!(is_newer("v2.5.0", "2.4.0"));
        assert!(!is_newer("2.5.0-beta.1", "2.5.0"));
        assert_eq!(compare_versions("2.4.10", "2.4.9"), 1);
        assert_eq!(compare_versions("1.0", "1.0.0"), 0);
        assert_eq!(parse_version("2.5.0+abc"), Some([2, 5, 0]));
        assert_eq!(parse_version("latest"), None);
        assert!(!is_newer("garbage", "2.4.0"));
        assert_eq!(clean_version("v3.1").as_deref(), Some("3.1.0"));
    }
}
