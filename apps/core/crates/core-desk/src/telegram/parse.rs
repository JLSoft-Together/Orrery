use super::jsx::{bad_path, collapse, int_of, is_bidi, is_ctl, is_ctl_c1, len, num_of, s, slice, trim};
use super::sanitize::{INBOUND_MAX, NAME_MAX};
use serde_json::{json, Value};

const SINCE_MAX: f64 = 8_640_000_000_000_000.0;
const SECRET_FILE_GLOBS: [&str; 10] = [".env*", "*.jks", "*.keystore", "*.p12", "*.pem", "key.properties", "local.properties", "google-services.json", "GoogleService-Info.plist", "*.mobileprovision"];

pub fn inbound_prompt(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\x1b' && i + 1 < chars.len() {
            let n = chars[i + 1];
            if n == '[' {
                let mut j = i + 2;
                while j < chars.len() && (chars[j].is_ascii_digit() || chars[j] == ';' || chars[j] == '?') {
                    j += 1;
                }
                while j < chars.len() && (' '..='/').contains(&chars[j]) {
                    j += 1;
                }
                if j < chars.len() && ('@'..='~').contains(&chars[j]) {
                    i = j + 1;
                    continue;
                }
            } else if matches!(n, ']' | 'P' | '^' | '_') {
                let mut j = i + 2;
                let mut end = None;
                while j < chars.len() {
                    if chars[j] == '\x07' {
                        end = Some(j + 1);
                        break;
                    }
                    if chars[j] == '\x1b' && j + 1 < chars.len() && chars[j + 1] == '\\' {
                        end = Some(j + 2);
                        break;
                    }
                    j += 1;
                }
                if let Some(e) = end {
                    i = e;
                    continue;
                }
            }
        }
        out.push(c);
        i += 1;
    }
    let out: String = out.chars().filter(|c| { let n = *c as u32; !(n <= 0x08 || (0x0b..=0x1f).contains(&n) || (0x7f..=0x9f).contains(&n)) && !is_bidi(*c) }).collect();
    let mut flat = String::new();
    let mut nl = 0;
    for c in out.chars() {
        if c == '\n' {
            nl += 1;
            continue;
        }
        if nl > 0 {
            flat.push_str(if nl >= 2 { "\n\n" } else { "\n" });
            nl = 0;
        }
        flat.push(c);
    }
    if nl > 0 {
        flat.push_str(if nl >= 2 { "\n\n" } else { "\n" });
    }
    slice(trim(&flat), INBOUND_MAX)
}

fn glob_match(glob: &str, name: &str) -> bool {
    let g: Vec<char> = glob.to_lowercase().chars().collect();
    let n: Vec<char> = name.to_lowercase().chars().collect();
    fn go(g: &[char], n: &[char]) -> bool {
        match g.first() {
            None => n.is_empty(),
            Some('*') => (0..=n.len()).any(|i| go(&g[1..], &n[i..])),
            Some('?') => !n.is_empty() && go(&g[1..], &n[1..]),
            Some(c) => n.first() == Some(c) && go(&g[1..], &n[1..]),
        }
    }
    go(&g, &n)
}

pub fn is_secret_file_name(name: &str) -> bool {
    SECRET_FILE_GLOBS.iter().any(|g| glob_match(g, name))
}

fn one_line(v: &str, max: usize) -> String {
    slice(trim(&collapse(v, is_ctl, " ")), max)
}

fn tool_ok(v: &Value) -> Option<&str> {
    v.as_str().filter(|t| (1..=80).contains(&t.len()) && t.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b':' | b'-')))
}

fn parse_ask(raw: &Value) -> Option<Value> {
    let q = raw.get("question")?.as_str()?;
    let opts = raw.get("options")?.as_array()?;
    let multi = raw.get("multi")?.as_bool()?;
    let question = one_line(q, 200);
    if question.is_empty() {
        return None;
    }
    let mut options = Vec::new();
    for o in opts {
        let label = one_line(o.as_str()?, 60);
        if label.is_empty() {
            return None;
        }
        options.push(label);
    }
    if options.is_empty() {
        return None;
    }
    let more = options.len() > 6 || raw.get("more") == Some(&Value::Bool(true));
    options.truncate(6);
    let mut m = json!({"question": question, "options": options, "multi": multi});
    if more {
        m["more"] = json!(true);
    }
    Some(m)
}

pub fn parse_episode(raw: &Value) -> Option<Value> {
    if s(raw, "type") != Some("telegram") {
        return None;
    }
    let episode = s(raw, "episode").filter(|e| !e.is_empty() && len(e) <= 300)?;
    let reason = s(raw, "reason").filter(|r| ["done", "waiting", "question", "permission"].contains(r))?;
    let terminal = s(raw, "terminalId").filter(|t| !t.is_empty() && len(t) <= 200)?;
    let name = s(raw, "name").filter(|n| !n.is_empty())?;
    let cwd = s(raw, "cwd").filter(|c| !c.is_empty() && len(c) <= 4096 && !bad_path(c))?;
    let ms = |k: &str| num_of(&raw[k]).filter(|v| *v >= 0.0 && *v < SINCE_MAX);
    let mut m = json!({"type": "telegram", "episode": episode, "reason": reason, "terminalId": terminal, "name": slice(name, NAME_MAX), "cwd": cwd});
    if let Some(v) = ms("since") {
        m["since"] = json!(v);
    }
    if let Some(v) = ms("worked") {
        m["worked"] = json!(v);
    }
    if let Some(t) = tool_ok(&raw["tool"]) {
        m["tool"] = json!(t);
    }
    if let Some(r) = s(raw, "reply").filter(|r| !trim(r).is_empty()) {
        m["reply"] = json!(slice(r, 3000));
    }
    if let Some(a) = raw.get("ask").and_then(parse_ask) {
        m["ask"] = a;
    }
    Some(m)
}

pub fn parse_buttons(raw: &Value) -> Option<Option<Vec<(String, String)>>> {
    if raw.is_null() {
        return Some(None);
    }
    let list = raw.as_array().filter(|l| (1..=6).contains(&l.len()))?;
    let mut out = Vec::new();
    for b in list {
        let o = b.as_object()?;
        let label = o.get("label")?.as_str()?;
        let value = o.get("value")?.as_str()?;
        let label = trim(&collapse(label, |c| is_ctl_c1(c) || is_bidi(c) || matches!(c, '\u{2028}' | '\u{2029}'), " ")).to_string();
        let value = trim(value).to_string();
        if label.is_empty() || label.chars().count() > 32 || value.is_empty() || value.chars().count() > 200 || value.contains('\0') {
            return None;
        }
        out.push((label, value));
    }
    Some(Some(out))
}

pub fn pane_infos(raw: &Value) -> Vec<Value> {
    let Some(list) = raw.as_array() else { return Vec::new() };
    list.iter()
        .take(20)
        .filter_map(|p| {
            let name = s(p, "name").filter(|n| !n.is_empty())?;
            let status = s(p, "status")?;
            let mut m = json!({"name": slice(&collapse(name, is_ctl, " "), NAME_MAX), "status": slice(status, 20)});
            if let Some(t) = s(p, "terminalId").filter(|t| len(t) < 200) {
                m["terminalId"] = json!(t);
            }
            if let Some(t) = tool_ok(&p["tool"]) {
                m["tool"] = json!(t);
            }
            if let Some(v) = num_of(&p["since"]).filter(|v| *v > 0.0) {
                m["since"] = json!(v);
            }
            Some(m)
        })
        .collect()
}

pub fn parse_inbound_result(raw: &Value) -> Value {
    let unavailable = json!({"ok": false, "error": "unavailable"});
    if !raw.is_object() {
        return unavailable;
    }
    if raw.get("ok") != Some(&Value::Bool(true)) {
        let e = s(raw, "error").filter(|e| ["none", "many", "busy", "gone", "unavailable", "state"].contains(e)).unwrap_or("unavailable");
        let panes = pane_infos(&raw["panes"]);
        let mut m = json!({"ok": false, "error": e});
        if !panes.is_empty() {
            m["panes"] = Value::Array(panes);
        }
        return m;
    }
    let nm = |v: &Value| v.as_str().filter(|s| !s.is_empty()).map(|s| slice(&collapse(s, is_ctl, " "), NAME_MAX));
    let action = s(raw, "action").unwrap_or("");
    match action {
        "status" => json!({"ok": true, "action": "status", "panes": pane_infos(&raw["panes"])}),
        "keys" => {
            let Some(name) = s(raw, "name").filter(|n| !n.is_empty()) else { return unavailable };
            let Some(key) = s(raw, "key").filter(|k| ["approve", "deny", "stop", "option", "esc"].contains(k)) else { return unavailable };
            let option = int_of(&raw["option"]).filter(|o| (1..=6).contains(o) && raw["option"].as_f64() == Some(*o as f64));
            if key == "option" && option.is_none() {
                return unavailable;
            }
            let mut m = json!({"ok": true, "action": "keys", "name": slice(name, NAME_MAX), "key": key});
            if let (Some(o), "option") = (option, key) {
                m["option"] = json!(o);
            }
            m
        }
        "tail" => {
            let (Some(name), Some(text)) = (nm(&raw["name"]), s(raw, "text")) else { return unavailable };
            let text: String = text.chars().filter(|c| { let n = *c as u32; !(n <= 0x09 || (0x0b..=0x1f).contains(&n) || n == 0x7f) }).collect();
            json!({"ok": true, "action": "tail", "name": name, "text": super::jsx::tail(&text, 20_000)})
        }
        "new" | "resume" | "compact" | "close" | "focus" => match nm(&raw["name"]) {
            Some(name) => json!({"ok": true, "action": action, "name": name}),
            None => unavailable,
        },
        "answer" => match nm(&raw["name"]) {
            Some(name) => json!({"ok": true, "action": "answer", "name": name, "submitted": raw.get("submitted") == Some(&Value::Bool(true))}),
            None => unavailable,
        },
        "all" => {
            let names: Vec<String> = raw["names"].as_array().map(|l| l.iter().take(20).filter_map(nm).collect()).unwrap_or_default();
            json!({"ok": true, "action": "all", "names": names})
        }
        "resumeList" => {
            let items: Vec<Value> = raw["items"]
                .as_array()
                .map(|l| {
                    l.iter()
                        .take(8)
                        .filter_map(|i| {
                            let name = nm(&i["name"])?;
                            let id = s(i, "id").filter(|id| id.len() == 36 && id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-'))?;
                            let cwd = s(i, "cwd").filter(|c| !c.is_empty() && len(c) <= 4096 && !bad_path(c))?;
                            Some(json!({"id": id, "name": name, "cwd": cwd, "endedAt": s(i, "endedAt").map(|e| slice(e, 40)).unwrap_or_default()}))
                        })
                        .collect()
                })
                .unwrap_or_default();
            json!({"ok": true, "action": "resumeList", "items": items})
        }
        "send" => {
            let Some(name) = s(raw, "name").filter(|n| !n.is_empty()) else { return unavailable };
            let mut m = json!({"ok": true, "action": "send", "name": slice(name, NAME_MAX), "queued": raw.get("queued") == Some(&Value::Bool(true))});
            if let Some(t) = s(raw, "terminalId").filter(|t| len(t) < 200) {
                m["terminalId"] = json!(t);
            }
            if let Some(n) = int_of(&raw["images"]).filter(|n| (1..=10).contains(n)) {
                m["images"] = json!(n);
            }
            m
        }
        _ => unavailable,
    }
}
