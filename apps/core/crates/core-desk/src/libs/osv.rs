use super::registry::{encode_uri_component, truthy};
use super::versions::cmp_num;
use crate::js;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::sync::OnceLock;

pub const OSV_BASE: &str = "https://api.osv.dev";

pub fn osv_ecosystem(eco: &str) -> &'static str {
    match eco {
        "maven" => "Maven",
        "pub" => "Pub",
        _ => "npm",
    }
}

pub fn is_osv_id(v: &Value) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    let re = R.get_or_init(|| Regex::new(r"^[A-Za-z0-9][A-Za-z0-9._:-]{0,79}$").unwrap());
    v.as_str().is_some_and(|s| re.is_match(s))
}

fn base_of(base: Option<&str>) -> &str {
    let b = base.unwrap_or(OSV_BASE);
    b.strip_suffix('/').unwrap_or(b)
}

pub fn osv_batch_url(base: Option<&str>) -> String {
    format!("{}/v1/querybatch", base_of(base))
}

pub fn osv_vuln_url(id: &str, base: Option<&str>) -> String {
    format!("{}/v1/vulns/{}", base_of(base), encode_uri_component(id))
}

pub fn parse_osv_batch(text: &str, count: usize) -> Option<Vec<Vec<String>>> {
    let doc: Value = serde_json::from_str(text).ok()?;
    let results = match &doc {
        Value::Null => return None,
        Value::Object(o) => o.get("results")?.as_array()?,
        _ => return None,
    };
    if results.len() != count {
        return None;
    }
    Some(
        results
            .iter()
            .map(|r| match r.as_object().and_then(|o| o.get("vulns")).and_then(Value::as_array) {
                Some(vs) => vs
                    .iter()
                    .filter_map(|v| v.as_object().and_then(|o| o.get("id")))
                    .filter(|id| is_osv_id(id))
                    .filter_map(|id| id.as_str().map(str::to_string))
                    .collect(),
                None => Vec::new(),
            })
            .collect(),
    )
}

fn severity_word(v: Option<&Value>) -> Option<&'static str> {
    match v?.as_str()?.trim().to_lowercase().as_str() {
        "critical" => Some("critical"),
        "high" => Some("high"),
        "moderate" | "medium" => Some("moderate"),
        "low" => Some("low"),
        _ => None,
    }
}

fn get<'a>(v: &'a Value, k: &str) -> Option<&'a Value> {
    v.as_object().and_then(|o| o.get(k))
}

fn same_name(eco: &str, a: Option<&Value>, b: &str) -> bool {
    match a.and_then(Value::as_str) {
        Some(a) if eco == "npm" || eco == "pub" => a == b,
        Some(a) => a.to_lowercase() == b.to_lowercase(),
        None => false,
    }
}

fn lower(a: Option<String>, b: &str) -> Option<String> {
    match a {
        Some(a) if !a.is_empty() && cmp_num(b, &a) >= 0 => Some(a),
        _ => Some(b.to_string()),
    }
}

fn fixed_for(affected: &[&Value], version: &str) -> Option<String> {
    let mut in_range: Option<String> = None;
    let mut above: Option<String> = None;
    for a in affected {
        let ranges = get(a, "ranges").and_then(Value::as_array).cloned().unwrap_or_default();
        for r in &ranges {
            if get(r, "type").and_then(Value::as_str) == Some("GIT") {
                continue;
            }
            let mut introduced = "0".to_string();
            for e in get(r, "events").and_then(Value::as_array).cloned().unwrap_or_default() {
                if let Some(i) = get(&e, "introduced").and_then(Value::as_str) {
                    introduced = i.to_string();
                }
                let Some(fixed) = get(&e, "fixed").and_then(Value::as_str) else { continue };
                if cmp_num(fixed, version) <= 0 {
                    continue;
                }
                above = lower(above, fixed);
                if introduced == "0" || cmp_num(version, &introduced) >= 0 {
                    in_range = lower(in_range, fixed);
                }
            }
        }
    }
    in_range.or(above)
}

pub fn parse_osv_vuln(text: &str, eco: &str, name: &str, version: &str) -> Option<Value> {
    let doc: Value = serde_json::from_str(text).ok()?;
    if !doc.is_object() || !is_osv_id(get(&doc, "id").unwrap_or(&Value::Null)) || truthy(get(&doc, "withdrawn")) {
        return None;
    }
    let all = get(&doc, "affected").and_then(Value::as_array).cloned().unwrap_or_default();
    let affected: Vec<&Value> = all
        .iter()
        .filter(|a| {
            let pkg = get(a, "package");
            pkg.and_then(|p| get(p, "ecosystem")).and_then(Value::as_str) == Some(osv_ecosystem(eco)) && same_name(eco, pkg.and_then(|p| get(p, "name")), name)
        })
        .collect();
    let mut severity = severity_word(get(&doc, "database_specific").and_then(|d| get(d, "severity")));
    for a in &affected {
        if severity.is_none() {
            severity = severity_word(get(a, "database_specific").and_then(|d| get(d, "severity"))).or_else(|| severity_word(get(a, "ecosystem_specific").and_then(|d| get(d, "severity"))));
        }
    }
    let summary = get(&doc, "summary").and_then(Value::as_str).filter(|s| !s.trim().is_empty());
    let text = summary.or_else(|| get(&doc, "details").and_then(Value::as_str)).unwrap_or("");
    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
    let aliases: Vec<Value> = get(&doc, "aliases").and_then(Value::as_array).map(|a| a.iter().filter(|x| is_osv_id(x)).take(5).cloned().collect()).unwrap_or_default();
    let mut m = Map::new();
    m.insert("id".into(), get(&doc, "id").cloned().unwrap_or(Value::Null));
    m.insert("aliases".into(), Value::Array(aliases));
    m.insert("summary".into(), json!(js::slice(&collapsed, 200)));
    m.insert("severity".into(), json!(severity.unwrap_or("unknown")));
    m.insert("fixed".into(), json!(fixed_for(&affected, version)));
    Some(Value::Object(m))
}

pub fn bare_vuln(id: &str) -> Value {
    json!({"id": id, "aliases": [], "summary": "", "severity": "unknown", "fixed": null})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_and_record() {
        assert_eq!(parse_osv_batch(r#"{"results":[{"vulns":[{"id":"GHSA-1"}]},{}]}"#, 2).unwrap(), vec![vec!["GHSA-1".to_string()], vec![]]);
        assert!(parse_osv_batch(r#"{"results":[]}"#, 1).is_none());
        let rec = r#"{"id":"GHSA-1","summary":"  Bad\n thing ","affected":[{"package":{"ecosystem":"npm","name":"x"},"ranges":[{"type":"SEMVER","events":[{"introduced":"0"},{"fixed":"1.2.0"}]},{"type":"SEMVER","events":[{"introduced":"2.0.0"},{"fixed":"2.1.0"}]}],"database_specific":{"severity":"HIGH"}}]}"#;
        let v = parse_osv_vuln(rec, "npm", "x", "1.0.0").unwrap();
        assert_eq!(v, json!({"id":"GHSA-1","aliases":[],"summary":"Bad thing","severity":"high","fixed":"1.2.0"}));
        assert_eq!(osv_vuln_url("GHSA-1", Some("http://127.0.0.1:9/")), "http://127.0.0.1:9/v1/vulns/GHSA-1");
    }
}
