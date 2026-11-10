use crate::js;
use crate::run::android::jsre::Js;

const SQL_MAX: usize = 20_000;
const WRITES: [&str; 17] = ["insert", "update", "delete", "create", "drop", "alter", "attach", "detach", "vacuum", "reindex", "analyze", "begin", "commit", "rollback", "savepoint", "release", "upsert"];

pub fn sql_words(sql: &str) -> Option<(Vec<String>, Vec<usize>)> {
    let c: Vec<char> = sql.chars().collect();
    let (mut words, mut semis) = (Vec::<String>::new(), Vec::<usize>::new());
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if matches!(ch, '\'' | '"' | '`' | '[') {
            let close = if ch == '[' { ']' } else { ch };
            let mut j = i + 1;
            loop {
                j += c[j.min(c.len())..].iter().position(|x| *x == close)?;
                if close != ']' && c.get(j + 1) == Some(&close) {
                    j += 2;
                } else {
                    break;
                }
            }
            i = j + 1;
        } else if ch == '-' && c.get(i + 1) == Some(&'-') {
            i = c[i..].iter().position(|x| *x == '\n').map(|n| i + n + 1).unwrap_or(c.len());
        } else if ch == '/' && c.get(i + 1) == Some(&'*') {
            let end = (i + 2..c.len().saturating_sub(1)).find(|k| c[*k] == '*' && c[*k + 1] == '/')?;
            i = end + 2;
        } else if ch == ';' {
            semis.push(words.len());
            i += 1;
        } else if ch.is_ascii_alphabetic() || ch == '_' {
            let mut n = 1;
            while n < 128 && i + n < c.len() && (c[i + n].is_ascii_alphanumeric() || c[i + n] == '_' || c[i + n] == '$') {
                n += 1;
            }
            words.push(c[i..i + n].iter().collect::<String>().to_lowercase());
            i += n;
        } else {
            i += 1;
        }
    }
    Some((words, semis))
}

pub fn check_read_only_sql(raw: &serde_json::Value) -> Result<String, &'static str> {
    static QUOTED: Js = Js::new(r"'[^']*'", "g");
    let sql = raw.as_str().ok_or("empty")?.trim().to_string();
    if sql.is_empty() {
        return Err("empty");
    }
    if js::len(&sql) > SQL_MAX {
        return Err("too-long");
    }
    let (words, semis) = sql_words(&sql).ok_or("syntax")?;
    if words.is_empty() {
        return Err("empty");
    }
    if semis.iter().any(|at| *at < words.len()) {
        return Err("multiple");
    }
    let mut rest: &[String] = &words;
    let mut first = words[0].as_str();
    if first == "explain" {
        rest = if words.get(1).map(String::as_str) == Some("query") && words.get(2).map(String::as_str) == Some("plan") { &words[3..] } else { &words[1..] };
        first = rest.first().map(String::as_str).unwrap_or("");
    }
    if first == "pragma" {
        return if QUOTED.replace_all(&sql, "").contains('=') { Err("not-read") } else { Ok(sql) };
    }
    if first != "select" && first != "with" && first != "values" {
        return Err("not-read");
    }
    if rest.iter().any(|w| WRITES.contains(&w.as_str())) {
        return Err("not-read");
    }
    Ok(sql)
}

pub fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn allows_reads_only() {
        assert!(check_read_only_sql(&json!("select 1")).is_ok());
        assert!(check_read_only_sql(&json!("WITH a AS (select 1) select * from a")).is_ok());
        assert_eq!(check_read_only_sql(&json!("select 1; select 2")), Err("multiple"));
        assert_eq!(check_read_only_sql(&json!("delete from t")), Err("not-read"));
        assert_eq!(check_read_only_sql(&json!("pragma journal_mode = off")), Err("not-read"));
        assert_eq!(check_read_only_sql(&json!("select 'a")), Err("syntax"));
        assert_eq!(check_read_only_sql(&json!("  ")), Err("empty"));
        assert_eq!(quote_ident("a\"b"), "\"a\"\"b\"");
    }
}
