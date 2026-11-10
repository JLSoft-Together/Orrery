use super::versions::is_lib_name;
use crate::js;
use regex::Regex;
use serde_json::Value;
use std::sync::OnceLock;

pub fn registry_base(registry: &str) -> &'static str {
    match registry {
        "google" => "https://dl.google.com/android/maven2",
        "central" => "https://repo1.maven.org/maven2",
        "pub" => "https://pub.dev/api/packages",
        _ => "https://registry.npmjs.org",
    }
}

pub fn is_allowed_override(base: &str) -> bool {
    let b = base.trim();
    let Some(scheme_end) = b.find("://") else { return false };
    if !b[..scheme_end].eq_ignore_ascii_case("http") {
        return false;
    }
    let rest = &b[scheme_end + 3..];
    let auth_end = rest.find(['/', '?', '#', '\\']).unwrap_or(rest.len());
    let authority = &rest[..auth_end];
    let host_port = authority.rsplit('@').next().unwrap_or("");
    let host = if host_port.starts_with('[') {
        match host_port.find(']') {
            Some(i) => &host_port[..=i],
            None => return false,
        }
    } else {
        host_port.split(':').next().unwrap_or("")
    };
    let host = host.to_ascii_lowercase();
    if !matches!(host.as_str(), "127.0.0.1" | "localhost" | "[::1]") {
        return false;
    }
    let tail = &rest[auth_end..];
    let (before_hash, hash) = match tail.find('#') {
        Some(i) => (&tail[..i], &tail[i + 1..]),
        None => (tail, ""),
    };
    let search = before_hash.find('?').map(|i| &before_hash[i + 1..]).unwrap_or("");
    search.is_empty() && hash.is_empty()
}

pub fn registries_for(eco: &str, name: &str) -> Vec<&'static str> {
    match eco {
        "pub" => vec!["pub"],
        "npm" => vec!["npm"],
        _ => {
            static G: OnceLock<Regex> = OnceLock::new();
            let re = G.get_or_init(|| Regex::new(r"^(androidx|android\.arch|com\.android|com\.google)(\.|$)").unwrap());
            let group = name.split(':').next().unwrap_or("");
            if re.is_match(group) {
                vec!["google", "central"]
            } else {
                vec!["central"]
            }
        }
    }
}

pub fn encode_uri_component(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

pub fn lookup_url(registry: &str, eco: &str, name: &str, over: Option<&str>) -> Option<String> {
    if !is_lib_name(&Value::from(eco), &Value::from(name)) {
        return None;
    }
    let base = match over {
        Some(o) => format!("{}/{}", o.strip_suffix('/').unwrap_or(o), registry),
        None => registry_base(registry).to_string(),
    };
    if registry == "google" || registry == "central" {
        let (group, artifact) = name.split_once(':')?;
        return Some(format!("{base}/{}/{artifact}/maven-metadata.xml", group.replace('.', "/")));
    }
    if registry == "pub" {
        return Some(format!("{base}/{name}"));
    }
    Some(match name.strip_prefix('@') {
        Some(rest) => format!("{base}/@{}", encode_uri_component(rest)),
        None => format!("{base}/{}", encode_uri_component(name)),
    })
}

struct Xml {
    metadata: Regex,
    versions: Regex,
    version: Regex,
    release: Regex,
}

fn xml() -> &'static Xml {
    static X: OnceLock<Xml> = OnceLock::new();
    X.get_or_init(|| Xml {
        metadata: Regex::new(r"<metadata[\s>]").unwrap(),
        versions: Regex::new(r"<versions>([\s\S]*?)</versions>").unwrap(),
        version: Regex::new(r"<version>\s*([^<\s]+)\s*</version>").unwrap(),
        release: Regex::new(r"<release>\s*([^<\s]+)\s*</release>").unwrap(),
    })
}

pub fn parse_maven_metadata(text: &str) -> Option<Vec<String>> {
    let x = xml();
    if !x.metadata.is_match(text) {
        return None;
    }
    let block = x.versions.captures(text).and_then(|c| c.get(1)).map(|m| m.as_str()).unwrap_or("");
    let mut list: Vec<String> = x.version.captures_iter(block).map(|c| c[1].to_string()).collect();
    if let Some(release) = x.release.captures(text).map(|c| c[1].to_string()) {
        if !list.contains(&release) {
            list.push(release);
        }
    }
    Some(list)
}

pub fn parse_pub_versions(text: &str) -> Option<Vec<String>> {
    let doc: Value = serde_json::from_str(text).ok()?;
    let versions = doc.as_object()?.get("versions")?.as_array()?;
    Some(
        versions
            .iter()
            .filter_map(|v| {
                let o = v.as_object()?;
                let ver = o.get("version")?.as_str()?;
                (o.get("retracted") != Some(&Value::Bool(true))).then(|| ver.to_string())
            })
            .collect(),
    )
}

pub fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Some(Value::String(s)) => !s.is_empty(),
        Some(_) => true,
    }
}

pub fn parse_npm_versions(text: &str) -> Option<Vec<String>> {
    let doc: Value = serde_json::from_str(text).ok()?;
    let versions = doc.as_object()?.get("versions")?;
    let deprecated = |v: &Value| truthy(v.as_object().and_then(|o| o.get("deprecated")));
    match versions {
        Value::Object(m) => Some(js::entries(m).into_iter().filter(|(_, v)| !deprecated(v)).map(|(k, _)| k.clone()).collect()),
        Value::Array(a) => Some(a.iter().enumerate().filter(|(_, v)| !deprecated(v)).map(|(i, _)| i.to_string()).collect()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urls_and_overrides() {
        assert!(is_allowed_override("http://127.0.0.1:5000"));
        assert!(is_allowed_override("http://localhost/x/"));
        assert!(!is_allowed_override("https://127.0.0.1"));
        assert!(!is_allowed_override("http://example.com"));
        assert!(!is_allowed_override("http://127.0.0.1/?a=1"));
        assert_eq!(registries_for("maven", "com.google.firebase:x"), ["google", "central"]);
        assert_eq!(lookup_url("google", "maven", "androidx.core:core", None).unwrap(), "https://dl.google.com/android/maven2/androidx/core/core/maven-metadata.xml");
        assert_eq!(lookup_url("npm", "npm", "@vue/x", Some("http://127.0.0.1:1/")).unwrap(), "http://127.0.0.1:1/npm/@vue%2Fx");
        let xml = "<metadata><versioning><release>2.0</release><versions><version>1.0</version><version> 1.1 </version></versions></versioning></metadata>";
        assert_eq!(parse_maven_metadata(xml).unwrap(), ["1.0", "1.1", "2.0"]);
        assert_eq!(parse_npm_versions(r#"{"versions":{"1.0.0":{},"2.0.0":{"deprecated":"x"}}}"#).unwrap(), ["1.0.0"]);
    }
}
