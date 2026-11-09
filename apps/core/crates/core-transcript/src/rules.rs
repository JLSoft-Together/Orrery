use crate::jsstr::{js_space, units, word};
use serde_json::{Map, Value};

#[derive(Debug, Clone, PartialEq)]
pub struct CmdHit {
    pub rule: &'static str,
    pub targets: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Touch {
    File(String),
    Cmds(Vec<CmdHit>),
}

impl Touch {
    pub fn to_json(&self) -> Value {
        let mut m = Map::new();
        match self {
            Touch::File(f) => {
                m.insert("file".into(), Value::from(f.as_str()));
            }
            Touch::Cmds(c) => {
                let list = c
                    .iter()
                    .map(|h| {
                        let mut o = Map::new();
                        o.insert("rule".into(), Value::from(h.rule));
                        if let Some(t) = &h.targets {
                            o.insert("targets".into(), Value::from(t.clone()));
                        }
                        Value::Object(o)
                    })
                    .collect::<Vec<_>>();
                m.insert("cmds".into(), Value::Array(list));
            }
        }
        Value::Object(m)
    }
}

const MAX_COMMAND: usize = 20_000;
pub const SHELL_TOOLS: [&str; 2] = ["Bash", "PowerShell"];

fn file_key(name: &str) -> Option<&'static str> {
    match name {
        "Edit" | "Write" | "MultiEdit" => Some("file_path"),
        "NotebookEdit" => Some("notebook_path"),
        _ => None,
    }
}

pub fn touch_of(name: &str, input: &Value) -> Option<Touch> {
    let input = input.as_object()?;
    if let Some(key) = file_key(name) {
        let file = input.get(key)?.as_str()?;
        let n = units(file);
        return (n > 0 && n < 1024).then(|| Touch::File(file.to_owned()));
    }
    if SHELL_TOOLS.contains(&name) {
        let cmd = input.get("command")?.as_str()?;
        if units(cmd) > MAX_COMMAND {
            return None;
        }
        let cmds = command_hits(cmd, 0);
        return (!cmds.is_empty()).then_some(Touch::Cmds(cmds));
    }
    None
}

fn split_segments(cmd: &str) -> Vec<Vec<String>> {
    let mut segments = Vec::new();
    let mut tokens: Vec<String> = Vec::new();
    let mut cur = String::new();
    let mut has = false;
    let mut quote: Option<char> = None;
    let chars: Vec<char> = cmd.chars().collect();
    let mut i = 0;
    let end_token = |tokens: &mut Vec<String>, cur: &mut String, has: &mut bool| {
        if *has {
            tokens.push(std::mem::take(cur));
        }
        cur.clear();
        *has = false;
    };
    while i < chars.len() {
        let c = chars[i];
        if let Some(q) = quote {
            if c == q {
                quote = None;
            } else {
                cur.push(c);
            }
            i += 1;
            continue;
        }
        if c == '"' || c == '\'' {
            quote = Some(c);
            has = true;
        } else if c == ' ' || c == '\t' {
            end_token(&mut tokens, &mut cur, &mut has);
        } else if c == '\n' || c == '\r' || c == ';' || c == '|' || (c == '&' && chars.get(i + 1) == Some(&'&')) {
            if c == '&' {
                i += 1;
            }
            end_token(&mut tokens, &mut cur, &mut has);
            if !tokens.is_empty() {
                segments.push(std::mem::take(&mut tokens));
            }
        } else {
            cur.push(c);
            has = true;
        }
        i += 1;
    }
    end_token(&mut tokens, &mut cur, &mut has);
    if !tokens.is_empty() {
        segments.push(tokens);
    }
    segments
}

fn base_cmd(t: &str) -> String {
    let line_end = t.find(['\n', '\r', '\u{2028}', '\u{2029}']).unwrap_or(t.len());
    let s = match t[..line_end].rfind(['/', '\\']) {
        Some(i) => &t[i + 1..],
        None => t,
    };
    let s = if s.len() >= 4 && s.is_char_boundary(s.len() - 4) && s[s.len() - 4..].eq_ignore_ascii_case(".exe") { &s[..s.len() - 4] } else { s };
    s.to_lowercase()
}

fn is_assign(t: &str) -> bool {
    let b = t.as_bytes();
    if !b.first().is_some_and(|c| c.is_ascii_alphabetic() || *c == b'_') {
        return false;
    }
    let mut i = 1;
    while b.get(i).is_some_and(|c| c.is_ascii_alphanumeric() || *c == b'_') {
        i += 1;
    }
    b.get(i) == Some(&b'=')
}

const NESTED: [&str; 6] = ["bash", "sh", "zsh", "cmd", "pwsh", "powershell"];

fn prefix_of(t: &str, word: &str, min: usize) -> bool {
    t.len() >= min && t.len() <= word.len() && word.as_bytes()[..t.len()].eq_ignore_ascii_case(t.as_bytes())
}

fn nested_flag(t: &str) -> bool {
    ["-c", "/c", "/k"].iter().any(|f| t.eq_ignore_ascii_case(f)) || prefix_of(t, "-command", 2)
}

fn ps_recurse(t: &str) -> bool {
    prefix_of(t, "-recurse", 2)
}

fn ps_value_param(t: &str) -> bool {
    ["-path", "-literalpath", "-include", "-exclude", "-filter", "-credential", "-stream"].iter().any(|f| t.eq_ignore_ascii_case(f))
}

fn ps_path_param(t: &str) -> bool {
    t.eq_ignore_ascii_case("-path") || t.eq_ignore_ascii_case("-literalpath")
}

fn letters_flag(t: &str) -> bool {
    t.len() > 1 && t.starts_with('-') && t[1..].bytes().all(|c| c.is_ascii_alphabetic())
}

fn short_flags<'a>(tokens: impl Iterator<Item = &'a String>) -> String {
    tokens.filter(|t| letters_flag(t)).map(|t| &t[1..]).collect()
}

fn git_hit(args: &[String]) -> Option<&'static str> {
    let mut i = 0;
    while i < args.len() && args[i].starts_with('-') {
        i += if args[i] == "-C" || args[i] == "-c" { 2 } else { 1 };
    }
    let sub = args.get(i).map(String::as_str);
    let rest = args.get(i + 1..).unwrap_or(&[]);
    if sub == Some("push") && rest.iter().any(|t| t == "--force" || (letters_flag(t) && t.contains('f'))) {
        return Some("git-force-push");
    }
    if sub == Some("reset") && rest.iter().any(|t| t == "--hard") {
        return Some("git-reset-hard");
    }
    if sub == Some("clean") {
        let flags = short_flags(rest.iter());
        if (flags.contains('f') || rest.iter().any(|t| t == "--force")) && flags.contains('d') {
            return Some("git-clean");
        }
    }
    None
}

fn hit(rule: &'static str, targets: Option<Vec<String>>) -> Vec<CmdHit> {
    vec![CmdHit { rule, targets }]
}

fn segment_hits(tokens: &[String], depth: u32) -> Vec<CmdHit> {
    let mut i = 0;
    while i < tokens.len() && (tokens[i] == "sudo" || is_assign(&tokens[i])) {
        i += 1;
    }
    let Some(head) = tokens.get(i) else { return Vec::new() };
    if head.is_empty() {
        return Vec::new();
    }
    let name = base_cmd(head);
    let args = &tokens[i + 1..];
    let has = |w: &str| args.iter().any(|t| t == w);
    if NESTED.contains(&name.as_str()) && depth < 2 {
        if let Some(at) = args.iter().position(|t| nested_flag(t)) {
            if args.get(at + 1).is_some_and(|t| !t.is_empty()) {
                return command_hits(&args[at + 1..].join(" "), depth + 1);
            }
        }
    }
    if name == "git" {
        return git_hit(args).map_or_else(Vec::new, |r| hit(r, None));
    }
    if name == "adb" && (has("uninstall") || (has("pm") && has("uninstall"))) {
        return hit("adb-uninstall", None);
    }
    let n = name.as_str();
    if matches!(n, "remove-item" | "ri" | "rm" | "rmdir" | "del" | "erase" | "rd")
        && (args.iter().any(|t| ps_recurse(t) && units(t) > 2) || (n == "remove-item" && args.iter().any(|t| ps_recurse(t))))
    {
        return hit("remove-item-recurse", Some(ps_targets(args)));
    }
    if n == "rm" {
        let flags = short_flags(args.iter().filter(|t| !t.starts_with("--")));
        let recursive = flags.contains(['r', 'R']) || has("--recursive");
        let force = flags.contains('f') || has("--force");
        if recursive && force {
            return hit("rm-rf", Some(positional(args)));
        }
    }
    if matches!(n, "del" | "erase" | "rd" | "rmdir") && args.iter().any(|t| t.eq_ignore_ascii_case("/s")) {
        let targets = args.iter().filter(|t| !t.starts_with('/') || !slash_letter(t)).cloned().collect();
        return hit("del-s", Some(targets));
    }
    Vec::new()
}

fn slash_letter(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 2 && b[0] == b'/' && b[1].is_ascii_alphabetic()
}

fn positional(args: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = false;
    for a in args {
        if rest || !a.starts_with('-') {
            out.push(a.clone());
        } else if a == "--" {
            rest = true;
        }
    }
    out
}

fn ps_targets(args: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        if ps_value_param(a) {
            if ps_path_param(a) {
                if let Some(next) = args.get(i + 1).filter(|t| !t.is_empty()) {
                    out.extend(next.split(',').map(str::to_owned));
                }
            }
            i += 1;
        } else if !a.starts_with('-') {
            out.extend(a.split(',').map(str::to_owned));
        }
        i += 1;
    }
    out.retain(|t| !t.is_empty());
    out
}

fn ci_at(chars: &[char], at: usize, w: &str) -> bool {
    let n = w.len();
    at + n <= chars.len() && chars[at..at + n].iter().zip(w.chars()).all(|(a, b)| a.is_ascii() && a.eq_ignore_ascii_case(&b))
}

fn mentions_drop(cmd: &str) -> bool {
    let c: Vec<char> = cmd.chars().collect();
    for i in 0..c.len() {
        if (i > 0 && word(c[i - 1])) || !ci_at(&c, i, "drop") {
            continue;
        }
        let mut j = i + 4;
        let start = j;
        while j < c.len() && js_space(c[j]) {
            j += 1;
        }
        if j == start {
            continue;
        }
        for w in ["table", "database"] {
            if ci_at(&c, j, w) && c.get(j + w.len()).is_none_or(|x| !word(*x)) {
                return true;
            }
        }
    }
    false
}

pub fn command_hits(cmd: &str, depth: u32) -> Vec<CmdHit> {
    let mut out = Vec::new();
    for seg in split_segments(cmd) {
        out.extend(segment_hits(&seg, depth));
    }
    if depth == 0 && mentions_drop(cmd) {
        out.push(CmdHit { rule: "drop-table", targets: None });
    }
    out
}
