use crate::js;
use regex::Regex;
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::sync::OnceLock;

pub const LIBS_MAX: usize = 300;

struct Res {
    maven: Regex,
    npm_part: Regex,
    pub_name: Regex,
    npm: Regex,
    version: Regex,
    pre: Regex,
    token: Regex,
    lib_version: Regex,
}

fn res() -> &'static Res {
    static R: OnceLock<Res> = OnceLock::new();
    R.get_or_init(|| Res {
        maven: Regex::new(r"^[A-Za-z0-9_][A-Za-z0-9_.-]{0,199}$").unwrap(),
        npm_part: Regex::new(r"^[a-z0-9][a-z0-9._~-]*$").unwrap(),
        pub_name: Regex::new(r"^[a-z][a-z0-9_]{0,63}$").unwrap(),
        npm: Regex::new(r"^(?:@([^/]+)/)?([^/]+)$").unwrap(),
        version: Regex::new(r"^[vV]?([0-9]+(?:\.[0-9]+)*)(?:[-.+_]?([^\n\r\u{2028}\u{2029}]*))?$").unwrap(),
        pre: Regex::new(r"(?:^|[^a-z])(alpha|beta|rc|cr|snapshot|dev|eap|preview|pre|milestone|m[0-9]+|ea|canary|next|nightly|experimental|a[0-9]|b[0-9])").unwrap(),
        token: Regex::new(r"[0-9]+|[a-z]+").unwrap(),
        lib_version: Regex::new(r"^[0-9A-Za-z][0-9A-Za-z.+_-]{0,63}$").unwrap(),
    })
}

pub fn eco_of(v: &Value) -> Option<&'static str> {
    match v.as_str() {
        Some("maven") => Some("maven"),
        Some("pub") => Some("pub"),
        Some("npm") => Some("npm"),
        _ => None,
    }
}

pub fn is_lib_name(eco: &Value, name: &Value) -> bool {
    let Some(name) = name.as_str() else { return false };
    if js::len(name) > 214 {
        return false;
    }
    let r = res();
    match eco.as_str() {
        Some("maven") => {
            let parts: Vec<&str> = name.split(':').collect();
            parts.len() == 2 && parts.iter().all(|p| r.maven.is_match(p) && !p.contains(".."))
        }
        Some("pub") => r.pub_name.is_match(name),
        Some("npm") => match r.npm.captures(name) {
            Some(c) => c.get(1).is_none_or(|s| r.npm_part.is_match(s.as_str())) && c.get(2).is_some_and(|n| r.npm_part.is_match(n.as_str())),
            None => false,
        },
        _ => false,
    }
}

pub fn is_lib_version(v: &Value) -> bool {
    v.as_str().is_some_and(|s| res().lib_version.is_match(s))
}

#[derive(Debug, Clone, PartialEq)]
pub struct LibVersion {
    pub nums: Vec<f64>,
    pub qual: String,
    pub pre: bool,
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

pub fn parse_lib_version(v: &str) -> Option<LibVersion> {
    let r = res();
    let c = r.version.captures(js_trim(v))?;
    let raw = c.get(2).map(|m| m.as_str()).unwrap_or("").to_lowercase();
    let qual = match raw.find('+') {
        Some(i) => raw[..i].to_string(),
        None => raw,
    };
    let nums = c[1].split('.').map(|n| n.parse::<f64>().unwrap_or(f64::NAN)).collect();
    let pre = r.pre.is_match(&qual);
    Some(LibVersion { nums, qual, pre })
}

fn sign(d: f64) -> Option<Ordering> {
    if d > 0.0 {
        Some(Ordering::Greater)
    } else if d < 0.0 {
        Some(Ordering::Less)
    } else {
        None
    }
}

fn rank(t: &str) -> f64 {
    match t {
        "dev" | "snapshot" | "nightly" | "canary" | "experimental" => 0.0,
        "eap" | "ea" | "preview" | "pre" | "alpha" | "a" => 1.0,
        "beta" | "b" | "milestone" | "m" | "next" => 2.0,
        "rc" | "cr" => 3.0,
        _ => 5.0,
    }
}

fn compare_qual(a: &str, b: &str) -> Ordering {
    let r = res();
    let ta: Vec<&str> = r.token.find_iter(a).map(|m| m.as_str()).collect();
    let tb: Vec<&str> = r.token.find_iter(b).map(|m| m.as_str()).collect();
    for i in 0..ta.len().max(tb.len()) {
        let (Some(x), Some(y)) = (ta.get(i), tb.get(i)) else {
            return if ta.get(i).is_none() { Ordering::Less } else { Ordering::Greater };
        };
        let nx = x.as_bytes()[0].is_ascii_digit();
        let ny = y.as_bytes()[0].is_ascii_digit();
        if nx && ny {
            if let Some(o) = sign(x.parse::<f64>().unwrap_or(0.0) - y.parse::<f64>().unwrap_or(0.0)) {
                return o;
            }
        } else if !nx && !ny {
            if let Some(o) = sign(rank(x) - rank(y)) {
                return o;
            }
            if x != y {
                return x.cmp(y);
            }
        } else {
            return if nx { Ordering::Greater } else { Ordering::Less };
        }
    }
    Ordering::Equal
}

pub fn compare_lib_versions(a: &str, b: &str) -> Ordering {
    let (x, y) = match (parse_lib_version(a), parse_lib_version(b)) {
        (Some(x), Some(y)) => (x, y),
        (Some(_), None) => return Ordering::Greater,
        (None, Some(_)) => return Ordering::Less,
        (None, None) => return Ordering::Equal,
    };
    for i in 0..x.nums.len().max(y.nums.len()) {
        if let Some(o) = sign(x.nums.get(i).copied().unwrap_or(0.0) - y.nums.get(i).copied().unwrap_or(0.0)) {
            return o;
        }
    }
    if x.pre != y.pre {
        return if x.pre { Ordering::Less } else { Ordering::Greater };
    }
    compare_qual(&x.qual, &y.qual)
}

pub fn cmp_num(a: &str, b: &str) -> i32 {
    match compare_lib_versions(a, b) {
        Ordering::Less => -1,
        Ordering::Equal => 0,
        Ordering::Greater => 1,
    }
}

pub fn pick_latest(versions: &[String]) -> Value {
    let mut stable: Option<&String> = None;
    let mut pre: Option<&String> = None;
    for v in versions {
        let Some(p) = parse_lib_version(v) else { continue };
        if p.pre {
            if pre.is_none_or(|x| cmp_num(v, x) > 0) {
                pre = Some(v);
            }
        } else if stable.is_none_or(|x| cmp_num(v, x) > 0) {
            stable = Some(v);
        }
    }
    if let (Some(p), Some(s)) = (pre, stable) {
        if cmp_num(p, s) <= 0 {
            pre = None;
        }
    }
    json!({"stable": stable, "pre": pre})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_order() {
        assert!(is_lib_name(&json!("maven"), &json!("androidx.core:core-ktx")));
        assert!(!is_lib_name(&json!("maven"), &json!("a:b:c")));
        assert!(is_lib_name(&json!("npm"), &json!("@vue/compiler-sfc")));
        assert!(!is_lib_name(&json!("npm"), &json!("@a/b/c")));
        assert!(!is_lib_name(&json!("pub"), &json!("Http")));
        let mut v: Vec<String> = ["1.10.0", "1.2.0", "1.10.0-alpha02", "1.10.0-alpha10", "1.10.0-beta01", "1.10.0-rc01", "1.9.9", "2.0.0-SNAPSHOT"].map(String::from).to_vec();
        v.sort_by(|a, b| compare_lib_versions(a, b));
        assert_eq!(v, ["1.2.0", "1.9.9", "1.10.0-alpha02", "1.10.0-alpha10", "1.10.0-beta01", "1.10.0-rc01", "1.10.0", "2.0.0-SNAPSHOT"]);
        assert_eq!(compare_lib_versions("1.0", "1.0.0"), Ordering::Equal);
        assert!(!parse_lib_version("33.0.0-jre").unwrap().pre);
        assert!(parse_lib_version("1.0.0-M1").unwrap().pre);
        let l = pick_latest(&["1.0.0", "1.1.0", "1.2.0-beta1", "junk"].map(String::from));
        assert_eq!(l, json!({"stable": "1.1.0", "pre": "1.2.0-beta1"}));
    }
}
