use super::exec::{exec_git, ExecOpts};
use super::paths::{dirname, real_or_null, relative, resolve_str, safe_repo_path, SafePath};
use regex::Regex;
use serde_json::{json, Map, Value};
use std::path::Path;
use std::sync::OnceLock;

pub const MAX_DIFF_BYTES: usize = 1024 * 1024;
pub const MAX_FULL_DIFF_BYTES: usize = 3 * 1024 * 1024;
pub const MAX_ENTRIES: usize = 5000;

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub path: String,
    pub orig_path: Option<String>,
    pub x: String,
    pub y: String,
    pub kind: &'static str,
    pub staged: bool,
    pub counts: Option<Counts>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Counts {
    pub added: Option<f64>,
    pub removed: Option<f64>,
    pub binary: bool,
}

impl Entry {
    pub fn to_json(&self) -> Value {
        let mut m = Map::new();
        m.insert("path".into(), json!(self.path));
        if let Some(o) = &self.orig_path {
            m.insert("origPath".into(), json!(o));
        }
        m.insert("x".into(), json!(self.x));
        m.insert("y".into(), json!(self.y));
        m.insert("kind".into(), json!(self.kind));
        m.insert("staged".into(), json!(self.staged));
        if let Some(c) = &self.counts {
            if let Some(a) = c.added {
                m.insert("added".into(), json!(a as i64));
            }
            if let Some(r) = c.removed {
                m.insert("removed".into(), json!(r as i64));
            }
            m.insert("binary".into(), json!(c.binary));
        }
        Value::Object(m)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiffOpts {
    pub full: bool,
    pub whitespace: &'static str,
}

pub fn clean_diff_options(raw: &Value) -> DiffOpts {
    let ws = raw.get("whitespace").and_then(Value::as_str).unwrap_or("");
    let whitespace = ["none", "eol", "all", "all-blank", "imports"].into_iter().find(|w| *w == ws).unwrap_or("none");
    DiffOpts { full: raw.get("context").and_then(Value::as_str) == Some("full"), whitespace }
}

pub fn diff_args(o: DiffOpts) -> Vec<&'static str> {
    let mut a = vec!["--histogram"];
    if o.full {
        a.push("-U1000000");
    }
    match o.whitespace {
        "eol" => a.push("--ignore-space-at-eol"),
        "all" => a.push("-w"),
        "all-blank" | "imports" => a.extend(["-w", "--ignore-blank-lines"]),
        _ => {}
    }
    a
}

fn re(cell: &'static OnceLock<Regex>, pat: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pat).expect("regex"))
}

pub struct Status {
    pub branch: Option<String>,
    pub upstream: Option<String>,
    pub ahead: f64,
    pub behind: f64,
    pub entries: Vec<Entry>,
}

fn slice_from(s: &str, n: usize) -> String {
    s.chars().skip(n).collect()
}

pub fn parse_status_v2(out: &str) -> Status {
    static HEAD: OnceLock<Regex> = OnceLock::new();
    static OID: OnceLock<Regex> = OnceLock::new();
    static UP: OnceLock<Regex> = OnceLock::new();
    static AB: OnceLock<Regex> = OnceLock::new();
    let parts: Vec<&str> = out.split('\0').collect();
    let mut st = Status { branch: None, upstream: None, ahead: 0.0, behind: 0.0, entries: Vec::new() };
    let mut oid: Option<String> = None;
    let mut i = 0;
    while i < parts.len() {
        let rec = parts[i];
        i += 1;
        if rec.is_empty() {
            continue;
        }
        let t = rec.chars().next().unwrap_or(' ');
        if t == '#' {
            if let Some(c) = re(&HEAD, r"^# branch\.head (.+)$").captures(rec) {
                st.branch = if &c[1] == "(detached)" { None } else { Some(c[1].to_string()) };
            }
            if let Some(c) = re(&OID, r"^# branch\.oid ([0-9a-f]+)$").captures(rec) {
                oid = Some(c[1].to_string());
            }
            if let Some(c) = re(&UP, r"^# branch\.upstream (.+)$").captures(rec) {
                st.upstream = Some(c[1].to_string());
            }
            if let Some(c) = re(&AB, r"^# branch\.ab \+([0-9]+) -([0-9]+)$").captures(rec) {
                st.ahead = c[1].parse().unwrap_or(0.0);
                st.behind = c[2].parse().unwrap_or(0.0);
            }
            continue;
        }
        if t == '?' {
            st.entries.push(Entry { path: slice_from(rec, 2), orig_path: None, x: "?".into(), y: "?".into(), kind: "untracked", staged: false, counts: None });
            continue;
        }
        if t == '!' {
            continue;
        }
        let fields: Vec<&str> = rec.split(' ').collect();
        let xy = fields.get(1).copied().unwrap_or("..");
        let mut xc = xy.chars();
        let x = xc.next().map(String::from).unwrap_or_else(|| ".".into());
        let y = xc.next().map(String::from).unwrap_or_else(|| ".".into());
        let rest = |n: usize| if fields.len() > n { fields[n..].join(" ") } else { String::new() };
        if t == '1' {
            let kind = if x == "A" {
                "added"
            } else if x == "D" || y == "D" {
                "deleted"
            } else {
                "modified"
            };
            let staged = x != ".";
            st.entries.push(Entry { path: rest(8), orig_path: None, x, y, kind, staged, counts: None });
        } else if t == '2' {
            let orig = parts.get(i).copied().unwrap_or("").to_string();
            i += 1;
            let kind = if x == "C" { "added" } else { "renamed" };
            let staged = x != ".";
            st.entries.push(Entry { path: rest(9), orig_path: Some(orig), x, y, kind, staged, counts: None });
        } else if t == 'u' {
            st.entries.push(Entry { path: rest(10), orig_path: None, x, y, kind: "conflict", staged: false, counts: None });
        }
    }
    if st.branch.is_none() {
        if let Some(o) = oid {
            st.branch = Some(o.chars().take(7).collect());
        }
    }
    st
}

pub fn parse_numstat(out: &str) -> Vec<(String, Counts)> {
    static NUM: OnceLock<Regex> = OnceLock::new();
    let parts: Vec<&str> = out.split('\0').collect();
    let mut map: Vec<(String, Counts)> = Vec::new();
    let mut i = 0;
    while i < parts.len() {
        let rec = parts[i];
        let Some(c) = re(&NUM, r"(?s)^(-|[0-9]+)\t(-|[0-9]+)\t(.*)$").captures(rec) else {
            i += 1;
            continue;
        };
        let mut file = c[3].to_string();
        if file.is_empty() {
            i += 1;
            i += 1;
            file = parts.get(i).copied().unwrap_or("").to_string();
        }
        let binary = &c[1] == "-";
        let counts = if binary { Counts { added: None, removed: None, binary } } else { Counts { added: c[1].parse().ok(), removed: c[2].parse().ok(), binary } };
        map.retain(|(k, _)| k != &file);
        map.push((file, counts));
        i += 1;
    }
    map
}

pub fn counts_for<'a>(map: &'a [(String, Counts)], path: &str) -> Option<&'a Counts> {
    map.iter().find(|(k, _)| k == path).map(|(_, c)| c)
}

pub fn hunk_patch_for(rel: &str, patch: &Value) -> Option<String> {
    static HUNK: OnceLock<Regex> = OnceLock::new();
    let patch = patch.as_str()?;
    if crate::js::len(patch) > MAX_DIFF_BYTES {
        return None;
    }
    let lines: Vec<&str> = patch.split('\n').collect();
    let start = lines.iter().position(|l| l.starts_with("@@ "))?;
    let head = re(&HUNK, r"^@@ -([0-9]+)(?:,([0-9]+))? \+([0-9]+)(?:,([0-9]+))? @@").captures(lines[start])?;
    let old_count: f64 = head.get(2).map(|m| m.as_str().parse().unwrap_or(0.0)).unwrap_or(1.0);
    let new_count: f64 = head.get(4).map(|m| m.as_str().parse().unwrap_or(0.0)).unwrap_or(1.0);
    let mut body: Vec<&str> = Vec::new();
    let (mut olds, mut news) = (0.0, 0.0);
    for line in &lines[start + 1..] {
        if line.is_empty() && olds == old_count && news == new_count {
            break;
        }
        match line.chars().next() {
            Some(' ') => {
                olds += 1.0;
                news += 1.0;
            }
            Some('-') => olds += 1.0,
            Some('+') => news += 1.0,
            Some('\\') => {}
            _ => return None,
        }
        body.push(line);
        if olds > old_count || news > new_count {
            return None;
        }
    }
    if olds != old_count || news != new_count {
        return None;
    }
    let mut out = vec![format!("diff --git a/{rel} b/{rel}"), format!("--- a/{rel}"), format!("+++ b/{rel}"), lines[start].to_string()];
    out.extend(body.iter().map(|s| s.to_string()));
    out.push(String::new());
    Some(out.join("\n"))
}

pub fn new_file_diff(rel: &str, text: &str) -> String {
    if text.is_empty() {
        return [format!("diff --git a/{rel} b/{rel}"), "new file mode 100644".into(), String::new()].join("\n");
    }
    let mut body: Vec<&str> = text.split('\n').collect();
    let ends = text.ends_with('\n');
    if ends {
        body.pop();
    }
    let mut out = vec![format!("diff --git a/{rel} b/{rel}"), "new file mode 100644".into(), "--- /dev/null".into(), format!("+++ b/{rel}"), format!("@@ -0,0 +1,{} @@", body.len())];
    out.extend(body.iter().map(|l| format!("+{l}")));
    if !ends {
        out.push("\\ No newline at end of file".into());
    }
    out.join("\n") + "\n"
}

pub fn toplevel(dir: &str) -> Result<String, &'static str> {
    let res = exec_git(Path::new(dir), &["rev-parse", "--show-toplevel"], &ExecOpts::max(16_384).with_timeout(10_000));
    if res.no_git() {
        return Err("no-git");
    }
    if !res.ok() {
        return Err("not-repo");
    }
    real_or_null(&resolve_str(res.stdout.trim())).ok_or("not-repo")
}

impl ExecOpts {
    pub fn with_timeout(mut self, ms: u64) -> Self {
        self.timeout_ms = ms;
        self
    }
}

pub fn common_dir(dir: &str) -> Option<String> {
    let opts = ExecOpts::max(16_384).with_timeout(10_000);
    let mut res = exec_git(Path::new(dir), &["rev-parse", "--path-format=absolute", "--git-common-dir"], &opts);
    if !res.ok() {
        res = exec_git(Path::new(dir), &["rev-parse", "--git-common-dir"], &opts);
    }
    let out = if res.ok() { res.stdout.trim().to_string() } else { String::new() };
    if out.is_empty() {
        return None;
    }
    real_or_null(&resolve_str(&Path::new(dir).join(out).to_string_lossy()))
}

fn has_head(root: &str) -> bool {
    exec_git(Path::new(root), &["rev-parse", "--verify", "-q", "HEAD"], &ExecOpts::max(4096)).ok()
}

pub fn empty_tree(root: &str) -> Option<String> {
    let res = exec_git(Path::new(root), &["hash-object", "-t", "tree", "--stdin"], &ExecOpts::max(4096).with_input(""));
    if res.ok() {
        Some(res.stdout.trim().to_string())
    } else {
        None
    }
}

pub fn base_rev(root: &str) -> Option<String> {
    if has_head(root) {
        return Some("HEAD".into());
    }
    empty_tree(root)
}

fn count_lines(abs: &str) -> Counts {
    let none = Counts { added: None, removed: None, binary: false };
    let Ok(meta) = std::fs::metadata(abs) else { return none };
    if !meta.is_file() || meta.len() as usize > MAX_DIFF_BYTES {
        return none;
    }
    let Ok(buf) = std::fs::read(abs) else { return none };
    if buf[..buf.len().min(8192)].contains(&0) {
        return Counts { added: None, removed: None, binary: true };
    }
    let text = String::from_utf8_lossy(&buf);
    if text.is_empty() {
        return Counts { added: Some(0.0), removed: None, binary: false };
    }
    let n = text.split('\n').count() - usize::from(text.ends_with('\n'));
    Counts { added: Some(n as f64), removed: None, binary: false }
}

fn err(e: &str) -> Value {
    json!({"ok": false, "error": e})
}

pub fn status(root: &str) -> Value {
    let r = Path::new(root);
    let res = exec_git(r, &["status", "--porcelain=v2", "-z", "--branch", "--untracked-files=all"], &ExecOpts::timeout(20_000));
    if res.no_git() {
        return err("no-git");
    }
    if !res.ok() {
        return err("failed");
    }
    let parsed = parse_status_v2(&res.stdout);
    let truncated = parsed.entries.len() > MAX_ENTRIES || res.truncated;
    let mut entries: Vec<Entry> = parsed.entries.into_iter().take(MAX_ENTRIES).collect();
    if let Some(base) = base_rev(root) {
        let num = exec_git(r, &["diff", &base, "--histogram", "--numstat", "-z", "-M", "--no-ext-diff", "--no-textconv"], &ExecOpts::timeout(20_000));
        if num.ok() {
            let counts = parse_numstat(&num.stdout);
            for e in entries.iter_mut() {
                if let Some(c) = counts_for(&counts, &e.path) {
                    e.counts = Some(c.clone());
                }
            }
        }
    }
    let mut n = 0;
    for e in entries.iter_mut() {
        if e.kind != "untracked" {
            continue;
        }
        if n >= 300 {
            break;
        }
        n += 1;
        let c = count_lines(&Path::new(root).join(&e.path).to_string_lossy());
        let mut merged = e.counts.clone().unwrap_or(Counts { added: None, removed: None, binary: false });
        if c.added.is_some() {
            merged.added = c.added;
        }
        merged.binary = c.binary;
        e.counts = Some(merged);
    }
    json!({
        "ok": true, "root": root, "branch": parsed.branch, "upstream": parsed.upstream,
        "ahead": parsed.ahead as i64, "behind": parsed.behind as i64,
        "entries": entries.iter().map(Entry::to_json).collect::<Vec<_>>(), "truncated": truncated,
    })
}

pub fn entry_for(root: &str, rel: &str) -> Option<Entry> {
    let res = exec_git(Path::new(root), &["status", "--porcelain=v2", "-z", "--untracked-files=all", "--", rel], &ExecOpts::timeout(15_000));
    if !res.ok() {
        return None;
    }
    parse_status_v2(&res.stdout).entries.into_iter().find(|e| e.path == rel)
}

pub fn diff(root: &str, rel: &Value, opts: &Value) -> Value {
    let Some(safe) = safe_repo_path(root, rel) else { return err("denied") };
    let entry = entry_for(root, &safe.rel);
    diff_entry(root, &safe, entry.as_ref(), clean_diff_options(opts))
}

pub fn is_binary_diff(out: &str) -> bool {
    static BIN: OnceLock<Regex> = OnceLock::new();
    static HUNK: OnceLock<Regex> = OnceLock::new();
    re(&BIN, r"(?m)^Binary files .* differ$").is_match(out) && !re(&HUNK, r"(?m)^@@ ").is_match(out)
}

fn diff_result(rel: &str, text: &str, binary: bool, too_large: bool, untracked: bool) -> Value {
    json!({"ok": true, "path": rel, "text": text, "binary": binary, "tooLarge": too_large, "untracked": untracked})
}

pub fn run_capped_diff(root: &str, head: &[&str], paths: &[&str], opts: DiffOpts, rel: &str) -> Value {
    let run = |o: DiffOpts, max: usize| {
        let mut args: Vec<&str> = head.to_vec();
        args.extend(["-M", "--no-ext-diff", "--no-textconv"]);
        args.extend(diff_args(o));
        args.push("--");
        args.extend_from_slice(paths);
        exec_git(Path::new(root), &args, &ExecOpts::timeout(20_000).with_max(max))
    };
    let mut res = run(opts, if opts.full { MAX_FULL_DIFF_BYTES } else { MAX_DIFF_BYTES });
    let mut partial = false;
    if res.truncated && opts.full {
        partial = true;
        res = run(DiffOpts { full: false, ..opts }, MAX_DIFF_BYTES);
    }
    if !res.ok() && !res.truncated {
        return err("failed");
    }
    if res.truncated {
        return diff_result(rel, "", false, true, false);
    }
    let mut v = diff_result(rel, &res.stdout, is_binary_diff(&res.stdout), false, false);
    if partial {
        v["partial"] = json!(true);
    }
    v
}

fn diff_entry(root: &str, safe: &SafePath, entry: Option<&Entry>, opts: DiffOpts) -> Value {
    let Some(entry) = entry else { return diff_result(&safe.rel, "", false, false, false) };
    if entry.kind == "untracked" {
        let meta = std::fs::metadata(&safe.abs).ok();
        let Some(meta) = meta.filter(|m| m.is_file()) else { return err("missing") };
        if meta.len() as usize > MAX_DIFF_BYTES {
            return diff_result(&safe.rel, "", false, true, true);
        }
        let Ok(buf) = std::fs::read(&safe.abs) else { return err("missing") };
        if buf[..buf.len().min(8192)].contains(&0) {
            return diff_result(&safe.rel, "", true, false, true);
        }
        return diff_result(&safe.rel, &new_file_diff(&safe.rel, &String::from_utf8_lossy(&buf)), false, false, true);
    }
    let Some(base) = base_rev(root) else { return err("failed") };
    let mut paths: Vec<&str> = Vec::new();
    if let Some(o) = &entry.orig_path {
        paths.push(o);
    }
    paths.push(&entry.path);
    run_capped_diff(root, &["diff", &base], &paths, opts, &safe.rel)
}

pub fn file_diff(abs: &str, repo_root: impl Fn(&str) -> Result<String, String>) -> Value {
    let root = match repo_root(&dirname(abs)) {
        Ok(r) => r,
        Err(e) => return err(&e),
    };
    let rel = relative(&root, abs);
    let Some(safe) = safe_repo_path(&root, &json!(rel)) else { return err("denied") };
    let entry = entry_for(&root, &safe.rel);
    let none = |kind: Value, staged: bool| json!({"ok": true, "root": root, "rel": safe.rel, "kind": kind, "staged": staged, "text": "", "binary": false, "tooLarge": false, "untracked": false});
    match &entry {
        None => return none(Value::Null, false),
        Some(e) if e.kind == "deleted" || e.kind == "conflict" => return none(json!(e.kind), e.staged),
        _ => {}
    }
    let e = entry.as_ref().expect("entry");
    let res = diff_entry(&root, &safe, Some(e), DiffOpts { full: false, whitespace: "none" });
    if res["ok"] != json!(true) {
        return res;
    }
    json!({"ok": true, "root": root, "rel": safe.rel, "kind": e.kind, "staged": e.staged, "text": res["text"], "binary": res["binary"], "tooLarge": res["tooLarge"], "untracked": res["untracked"]})
}

fn failed(detail: &str) -> Value {
    json!({"ok": false, "error": "failed", "detail": crate::js::slice(detail.trim(), 400)})
}

pub fn discard_file(root: &str, rel: &Value, trash: &dyn Fn(&str) -> Result<(), String>) -> Value {
    let Some(safe) = safe_repo_path(root, rel) else { return err("denied") };
    let Some(entry) = entry_for(root, &safe.rel) else { return err("stale") };
    if entry.kind == "conflict" {
        return err("conflict");
    }
    let trash_if_there = |abs: &str| -> Value {
        if std::fs::symlink_metadata(abs).is_err() {
            return json!({"ok": true});
        }
        match trash(abs) {
            Ok(()) => json!({"ok": true}),
            Err(e) => failed(&e),
        }
    };
    let r = Path::new(root);
    if entry.kind == "untracked" {
        return trash_if_there(&safe.abs);
    }
    if entry.kind == "added" || entry.kind == "renamed" {
        let unstage = exec_git(r, &["rm", "--cached", "-q", "--ignore-unmatch", "--", &entry.path], &ExecOpts::default());
        if !unstage.ok() {
            return failed(&unstage.stderr);
        }
        let trashed = trash_if_there(&safe.abs);
        let Some(orig) = entry.orig_path.as_ref().filter(|_| trashed["ok"] == json!(true)) else { return trashed };
        let Some(orig) = safe_repo_path(root, &json!(orig)) else { return err("denied") };
        let res = exec_git(r, &["restore", "--source=HEAD", "--staged", "--worktree", "--", &orig.rel], &ExecOpts::default());
        return if res.ok() { json!({"ok": true}) } else { failed(&res.stderr) };
    }
    let res = exec_git(r, &["restore", "--source=HEAD", "--staged", "--worktree", "--", &entry.path], &ExecOpts::default());
    if res.ok() {
        json!({"ok": true})
    } else {
        failed(&res.stderr)
    }
}

pub fn discard_hunk(root: &str, rel: &Value, patch: &Value) -> Value {
    let Some(safe) = safe_repo_path(root, rel) else { return err("denied") };
    let Some(entry) = entry_for(root, &safe.rel) else { return err("stale") };
    if entry.kind == "conflict" {
        return err("conflict");
    }
    if entry.kind != "modified" {
        return err("denied");
    }
    let Some(text) = hunk_patch_for(&safe.rel, patch) else { return err("denied") };
    let r = Path::new(root);
    let check = exec_git(r, &["apply", "-R", "--whitespace=nowarn", "--check", "-"], &ExecOpts::default().with_input(&text));
    if !check.ok() {
        return json!({"ok": false, "error": "stale", "detail": crate::js::slice(check.stderr.trim(), 400)});
    }
    let res = exec_git(r, &["apply", "-R", "--whitespace=nowarn", "-"], &ExecOpts::default().with_input(&text));
    if res.ok() {
        json!({"ok": true})
    } else {
        failed(&res.stderr)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_status_and_numstat() {
        let out = "# branch.oid abcdef1234567\0# branch.head (detached)\0# branch.ab +2 -1\x001 .M N... 100644 100644 100644 a b x y.txt\x002 R. N... 100644 100644 100644 a b R100 new.txt\0old.txt\0? un tracked.txt\0";
        let st = parse_status_v2(out);
        assert_eq!(st.branch.as_deref(), Some("abcdef1"));
        assert_eq!((st.ahead, st.behind), (2.0, 1.0));
        assert_eq!(st.entries.len(), 3);
        assert_eq!(st.entries[0].path, "x y.txt");
        assert_eq!(st.entries[1].orig_path.as_deref(), Some("old.txt"));
        assert_eq!(st.entries[2].path, "un tracked.txt");
        let n = parse_numstat("1\t2\ta.txt\0-\t-\tbin\0" );
        assert_eq!(counts_for(&n, "a.txt").unwrap().added, Some(1.0));
        assert!(counts_for(&n, "bin").unwrap().binary);
        let r = parse_numstat("3\t0\t\0old\0new\0");
        assert_eq!(counts_for(&r, "new").unwrap().added, Some(3.0));
    }

    #[test]
    fn hunks_and_new_files() {
        let p = hunk_patch_for("a.txt", &json!("@@ -1,2 +1,2 @@\n a\n-b\n+c\n")).unwrap();
        assert_eq!(p, "diff --git a/a.txt b/a.txt\n--- a/a.txt\n+++ b/a.txt\n@@ -1,2 +1,2 @@\n a\n-b\n+c\n");
        assert!(hunk_patch_for("a", &json!("@@ -1,2 +1,2 @@\n a\n")).is_none());
        assert_eq!(new_file_diff("n", "x\ny"), "diff --git a/n b/n\nnew file mode 100644\n--- /dev/null\n+++ b/n\n@@ -0,0 +1,2 @@\n+x\n+y\n\\ No newline at end of file\n");
        assert_eq!(new_file_diff("n", ""), "diff --git a/n b/n\nnew file mode 100644\n");
    }
}
