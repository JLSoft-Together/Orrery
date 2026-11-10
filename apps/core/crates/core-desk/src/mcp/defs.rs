use serde_json::Value;
use std::sync::OnceLock;

fn data() -> &'static Value {
    static D: OnceLock<Value> = OnceLock::new();
    D.get_or_init(|| serde_json::from_str(include_str!("tools.json")).unwrap())
}

pub fn protocol_version() -> &'static str {
    data()["protocolVersion"].as_str().unwrap_or("2025-06-18")
}

pub fn instructions(deepseek: bool) -> &'static str {
    data()[if deepseek { "instructions" } else { "instructionsNoDeepseek" }].as_str().unwrap_or("")
}

pub fn tool_defs(deepseek: bool) -> Vec<Value> {
    let all = data()["tools"].as_array().cloned().unwrap_or_default();
    if deepseek {
        return all;
    }
    all.into_iter()
        .filter(|t| t["name"] != "hand_back")
        .map(|t| if t["name"] == "open_session" { data()["openSessionNoDeepseek"].clone() } else { t })
        .collect()
}

pub fn has_tool(defs: &[Value], name: &str) -> bool {
    defs.iter().any(|t| t["name"] == name)
}
