use crate::js;
use serde_json::Value;

const DIFF_MAX_LINES: usize = 1500;

pub fn indent_of(text: &str) -> String {
    let b = text.as_bytes();
    let mut i = 0;
    while let Some(off) = text[i..].find('\n') {
        let start = i + off + 1;
        let mut end = start;
        while end < b.len() && (b[end] == b' ' || b[end] == b'\t') {
            end += 1;
        }
        if end > start && end < b.len() && !text[end..].starts_with(core_transcript::jsstr::js_space) {
            return text[start..end].to_string();
        }
        i = start;
    }
    "  ".into()
}

pub fn indent_text(indent: &str) -> String {
    indent.chars().take(10).collect()
}

pub fn format_settings(next: &Value, before: &str) -> String {
    let eol = if before.contains("\r\n") { "\r\n" } else { "\n" };
    let body = js::stringify_indent(next, &indent_text(&indent_of(before)));
    format!("{}{eol}", body.replace('\n', eol))
}

fn lines(s: &str) -> Vec<String> {
    let t = s.replace("\r\n", "\n");
    let t = t.strip_suffix('\n').unwrap_or(&t);
    t.split('\n').map(str::to_string).collect()
}

pub fn json_diff(before: &str, after: &str) -> String {
    let a = if before.is_empty() { Vec::new() } else { lines(before) };
    let b = lines(after);
    if a.len() > DIFF_MAX_LINES || b.len() > DIFF_MAX_LINES {
        return b.iter().map(|l| format!("+ {l}")).collect::<Vec<_>>().join("\n");
    }
    let (n, m) = (a.len(), b.len());
    let mut dp = vec![vec![0u32; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] { dp[i + 1][j + 1] + 1 } else { dp[i + 1][j].max(dp[i][j + 1]) };
        }
    }
    let mut rows: Vec<String> = Vec::new();
    let (mut i, mut j) = (0, 0);
    while i < n || j < m {
        if i < n && j < m && a[i] == b[j] {
            rows.push(format!("  {}", a[i]));
            i += 1;
            j += 1;
        } else if i < n && (j >= m || dp[i + 1][j] >= dp[i][j + 1]) {
            rows.push(format!("- {}", a[i]));
            i += 1;
        } else {
            rows.push(format!("+ {}", b[j]));
            j += 1;
        }
    }
    let mut out: Vec<String> = Vec::new();
    let mut k = 0;
    while k < rows.len() {
        let mut end = k;
        while end < rows.len() && rows[end].starts_with("  ") {
            end += 1;
        }
        if end - k > 6 {
            out.extend_from_slice(&rows[k..k + 2]);
            out.push("  …".into());
            out.extend_from_slice(&rows[end - 2..end]);
            k = end;
        } else if end > k {
            out.extend_from_slice(&rows[k..end]);
            k = end;
        } else {
            out.push(rows[k].clone());
            k += 1;
        }
    }
    out.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn keeps_indent_and_eol() {
        assert_eq!(indent_of("{\r\n\t\"a\": 1\r\n}"), "\t");
        assert_eq!(indent_of("{}"), "  ");
        assert_eq!(format_settings(&json!({"a": [1]}), "{\r\n    \"x\": 1\r\n}"), "{\r\n    \"a\": [\r\n        1\r\n    ]\r\n}\r\n");
        let d = json_diff("{\n  \"a\": 1\n}\n", "{\n  \"a\": 1,\n  \"b\": 2\n}\n");
        assert_eq!(d, "  {\n-   \"a\": 1\n+   \"a\": 1,\n+   \"b\": 2\n  }");
    }
}
