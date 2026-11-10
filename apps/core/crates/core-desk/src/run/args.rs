use super::types::Env;
use regex::Regex;
use std::sync::OnceLock;

#[derive(Debug, Clone, Default)]
pub struct SubstituteVars {
    pub project_dir: String,
    pub workspace_dir: String,
    pub env: Env,
    pub mask_env: bool,
}

pub fn split_args(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut in_token = false;
    let mut quote: Option<char> = None;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        match quote {
            Some('\'') => {
                if c == '\'' {
                    quote = None;
                } else {
                    cur.push(c);
                }
            }
            Some(_) => {
                if c == '\\' && chars.get(i + 1) == Some(&'"') {
                    cur.push('"');
                    i += 1;
                } else if c == '"' {
                    quote = None;
                } else {
                    cur.push(c);
                }
            }
            None => {
                if c.is_whitespace() || c == '\u{feff}' {
                    if in_token {
                        out.push(std::mem::take(&mut cur));
                    }
                    cur.clear();
                    in_token = false;
                } else {
                    in_token = true;
                    if c == '"' || c == '\'' {
                        quote = Some(c);
                    } else {
                        cur.push(c);
                    }
                }
            }
        }
        i += 1;
    }
    if in_token {
        out.push(cur);
    }
    out
}

fn var_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"\$\{(projectDir|workspaceDir|env:([A-Za-z_][A-Za-z0-9_]*))\}").expect("regex"))
}

pub fn substitute(arg: &str, vars: &SubstituteVars) -> String {
    var_re()
        .replace_all(arg, |c: &regex::Captures| {
            let name = &c[1];
            if name == "projectDir" {
                return vars.project_dir.clone();
            }
            if name == "workspaceDir" {
                return vars.workspace_dir.clone();
            }
            let value = c.get(2).and_then(|m| vars.env.get(m.as_str())).cloned().unwrap_or_default();
            if vars.mask_env && !value.is_empty() {
                "***".to_string()
            } else {
                value
            }
        })
        .to_string()
}

pub fn expand_args(s: &str, vars: &SubstituteVars) -> Vec<String> {
    split_args(s).iter().map(|a| substitute(a, vars)).collect()
}

fn quote_one(a: &str) -> String {
    if !a.is_empty() && !a.contains(|c: char| c.is_whitespace() || c == '"' || c == '\'') {
        a.to_string()
    } else {
        format!("\"{}\"", a.replace('"', "\\\""))
    }
}

pub fn quote_for_display(argv: &[String]) -> String {
    argv.iter().map(|a| quote_one(a)).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_like_js() {
        assert_eq!(split_args(r#"a  "b c" 'd e' f\g "x\"y""#), vec!["a", "b c", "d e", r"f\g", "x\"y"]);
        assert_eq!(split_args("  "), Vec::<String>::new());
        assert_eq!(split_args("''"), vec![""]);
    }

    #[test]
    fn substitutes() {
        let mut env = Env::new();
        env.insert("K".into(), "v".into());
        let vars = SubstituteVars { project_dir: "/p".into(), workspace_dir: "/w".into(), env, mask_env: false };
        assert_eq!(expand_args("${projectDir}/x ${env:K} ${env:NO} ${workspaceDir}", &vars), vec!["/p/x", "v", "", "/w"]);
        let masked = SubstituteVars { mask_env: true, ..vars };
        assert_eq!(substitute("${env:K}", &masked), "***");
        assert_eq!(quote_for_display(&["a".into(), "b c".into(), "".into(), "q\"".into()]), "a \"b c\" \"\" \"q\\\"\"");
    }
}
