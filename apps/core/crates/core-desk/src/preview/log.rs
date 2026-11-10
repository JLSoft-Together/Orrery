use super::changes::{clean_diff_options, empty_tree, parse_numstat, run_capped_diff, counts_for};
use super::exec::{exec_git, ExecOpts, ExecResult};
use super::paths::safe_repo_path;
use regex::Regex;
use serde_json::{json, Map, Value};
use std::path::Path;
use std::sync::OnceLock;

pub const FS: char = '\x1f';
pub const LOG_FORMAT: &str = "%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%D%x1f%s";
pub const LOG_PAGE: usize = 500;
pub const MAX_COMMIT_FILES: usize = 5000;
const MAX_PATHS: usize = 20;

pub fn ref_re() -> &'static Regex {
    static R: OnceLock<Regex> = OnceLock::new();
    R.get_or_init(|| Regex::new(r"^refs/(heads|remotes|tags)/[^\s~^:?*\[\\]+$").expect("regex"))
}

pub fn is_hash(v: &Value) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    v.as_str().map(|s| R.get_or_init(|| Regex::new(r"(?i)^[0-9a-f]{4,40}$").expect("regex")).is_match(s)).unwrap_or(false)
}

fn js_number(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return 0.0;
    }
    t.parse::<f64>().ok().filter(|f| f.is_finite()).unwrap_or(0.0)
}

pub fn parse_log(out: &str) -> Vec<Value> {
    let mut commits = Vec::new();
    for rec in out.split('\0') {
        let rec = rec.strip_prefix('\n').unwrap_or(rec);
        let f: Vec<&str> = rec.split(FS).collect();
        if f.len() < 7 || f[0].is_empty() {
            continue;
        }
        let parents: Vec<&str> = if f[1].is_empty() { Vec::new() } else { f[1].split(' ').collect() };
        commits.push(json!({
            "hash": f[0], "parents": parents, "author": f[2], "email": f[3], "time": js_number(f[4]) as i64,
            "refs": f[5], "subject": f[6..].join(&FS.to_string()),
        }));
    }
    commits
}

fn track_num(s: &str, word: &str) -> i64 {
    let re = Regex::new(&format!(r"{word} ([0-9]+)")).expect("regex");
    re.captures(s).and_then(|c| c[1].parse().ok()).unwrap_or(0)
}

pub fn parse_ref_lines(out: &str, remotes: &[String]) -> Value {
    let mut local = Vec::new();
    let mut tags = Vec::new();
    let mut groups: Vec<(String, Vec<Value>)> = Vec::new();
    let mut names: Vec<&String> = remotes.iter().collect();
    names.sort_by_key(|n| std::cmp::Reverse(crate::js::len(n)));
    for line in out.split('\n') {
        let p: Vec<&str> = line.split(FS).collect();
        let (r, obj) = (p.first().copied().unwrap_or(""), p.get(1).copied().unwrap_or(""));
        if r.is_empty() || obj.is_empty() {
            continue;
        }
        let peeled = p.get(2).copied().unwrap_or("");
        let hash = if peeled.is_empty() { obj } else { peeled };
        if let Some(name) = r.strip_prefix("refs/heads/") {
            let upstream = p.get(3).copied().unwrap_or("");
            let track = p.get(4).copied().unwrap_or("");
            let (ahead, behind, gone) = if track.trim() == "gone" { (0, 0, true) } else { (track_num(track, "ahead"), track_num(track, "behind"), false) };
            local.push(json!({"ref": r, "name": name, "hash": hash, "upstream": if upstream.is_empty() { Value::Null } else { json!(upstream) }, "ahead": ahead, "behind": behind, "gone": gone}));
        } else if let Some(name) = r.strip_prefix("refs/tags/") {
            tags.push(json!({"ref": r, "name": name, "hash": hash}));
        } else if let Some(short) = r.strip_prefix("refs/remotes/") {
            if short.ends_with("/HEAD") {
                continue;
            }
            let remote = names.iter().find(|n| short.starts_with(&format!("{n}/"))).map(|s| s.to_string()).unwrap_or_else(|| short.split('/').next().unwrap_or("").to_string());
            let item = json!({"ref": r, "name": short, "hash": hash});
            match groups.iter_mut().find(|(n, _)| *n == remote) {
                Some((_, l)) => l.push(item),
                None => groups.push((remote, vec![item])),
            }
        }
    }
    groups.sort_by(|a, b| locale_compare(&a.0, &b.0));
    let remotes: Vec<Value> = groups.into_iter().map(|(name, refs)| json!({"name": name, "refs": refs})).collect();
    json!({"local": local, "remotes": remotes, "tags": tags})
}

pub fn locale_compare(a: &str, b: &str) -> std::cmp::Ordering {
    let ka: Vec<char> = a.chars().flat_map(|c| c.to_lowercase()).collect();
    let kb: Vec<char> = b.chars().flat_map(|c| c.to_lowercase()).collect();
    ka.cmp(&kb).then_with(|| b.cmp(a))
}

pub fn parse_name_status(out: &str) -> Vec<Map<String, Value>> {
    let parts: Vec<&str> = out.split('\0').collect();
    let mut files = Vec::new();
    let mut i = 0;
    while i < parts.len() {
        let status = parts[i];
        i += 1;
        if status.is_empty() {
            continue;
        }
        let kind = match status.chars().next() {
            Some('A') => "added",
            Some('D') => "deleted",
            Some('R') => "renamed",
            Some('C') => "copied",
            Some('T') => "typechange",
            _ => "modified",
        };
        let mut m = Map::new();
        if kind == "renamed" || kind == "copied" {
            let orig = parts.get(i).copied().unwrap_or("");
            let p = parts.get(i + 1).copied().unwrap_or("");
            i += 2;
            if !p.is_empty() {
                m.insert("path".into(), json!(p));
                m.insert("origPath".into(), json!(orig));
                m.insert("kind".into(), json!(kind));
                files.push(m);
            }
        } else {
            let p = parts.get(i).copied().unwrap_or("");
            i += 1;
            if !p.is_empty() {
                m.insert("path".into(), json!(p));
                m.insert("kind".into(), json!(kind));
                files.push(m);
            }
        }
    }
    files
}

fn clean_str(v: &Value, max: usize) -> Option<String> {
    v.as_str().map(str::trim).filter(|s| !s.is_empty()).map(|s| crate::js::slice(s, max))
}

fn epoch(v: &Value) -> Option<f64> {
    v.as_f64().filter(|f| f.is_finite() && *f > 0.0).map(f64::floor)
}

pub fn ref_ok(v: &Value) -> bool {
    v.as_str().map(|s| s == "HEAD" || (ref_re().is_match(s) && !s.contains("..") && !s.ends_with(".lock"))).unwrap_or(false)
}

pub struct LogQuery {
    pub branch: Option<String>,
    pub not_ref: Option<String>,
    pub author: Option<String>,
    pub since: Option<f64>,
    pub until: Option<f64>,
    pub text: Option<String>,
    pub regex: bool,
    pub match_case: bool,
    pub paths: Vec<String>,
    pub skip: f64,
}

pub fn clean_log_query(raw: &Value, root: &str) -> LogQuery {
    let g = |k: &str| raw.get(k).cloned().unwrap_or(Value::Null);
    let branch = if ref_ok(&g("branch")) { g("branch").as_str().map(String::from) } else { None };
    let not_ref = match &branch {
        Some(b) if ref_ok(&g("notRef")) && g("notRef").as_str() != Some(b) => g("notRef").as_str().map(String::from),
        _ => None,
    };
    let paths = g("paths").as_array().map(|a| a.iter().filter_map(|p| safe_repo_path(root, p).map(|s| s.rel)).take(MAX_PATHS).collect()).unwrap_or_default();
    let skip = g("skip").as_f64().filter(|f| f.is_finite() && *f > 0.0).map(|f| f.floor().min(10_000_000.0)).unwrap_or(0.0);
    LogQuery {
        branch,
        not_ref,
        author: clean_str(&g("author"), 200),
        since: epoch(&g("since")),
        until: epoch(&g("until")),
        text: clean_str(&g("text"), 500),
        regex: g("regex") == json!(true),
        match_case: g("matchCase") == json!(true),
        paths,
        skip,
    }
}

fn escape_regex(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if ".*+?^${}()|[]\\".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

pub fn log_args(q: &LogQuery) -> Vec<String> {
    let mut a: Vec<String> = vec!["log".into(), "--topo-order".into(), "-z".into(), format!("--format={LOG_FORMAT}"), format!("--skip={}", q.skip as u64), "-n".into(), (LOG_PAGE + 1).to_string()];
    if let Some(au) = &q.author {
        a.push(format!("--author={}", if q.regex { escape_regex(au) } else { au.clone() }));
    }
    if let Some(s) = q.since {
        a.push(format!("--since={}", crate::util::iso_string(s * 1000.0)));
    }
    if let Some(u) = q.until {
        a.push(format!("--until={}", crate::util::iso_string(u * 1000.0)));
    }
    if let Some(t) = &q.text {
        a.push(format!("--grep={t}"));
    }
    if q.text.is_some() || q.author.is_some() {
        a.push(if q.regex { "--extended-regexp" } else { "--fixed-strings" }.into());
        if !q.match_case {
            a.push("--regexp-ignore-case".into());
        }
    }
    match &q.branch {
        Some(b) => {
            a.push(b.clone());
            if let Some(n) = &q.not_ref {
                a.push("--not".into());
                a.push(n.clone());
            }
        }
        None => a.extend(["--branches", "--remotes", "--tags", "HEAD"].map(String::from)),
    }
    a.push("--".into());
    a.extend(q.paths.iter().cloned());
    a
}

fn t() -> ExecOpts {
    ExecOpts::timeout(20_000)
}

fn fail(res: &ExecResult) -> &'static str {
    if res.no_git() {
        "no-git"
    } else {
        "failed"
    }
}

fn git(root: &str, args: &[&str], o: &ExecOpts) -> ExecResult {
    exec_git(Path::new(root), args, o)
}

fn opt_trim(res: &ExecResult) -> Value {
    if res.ok() && !res.stdout.trim().is_empty() {
        json!(res.stdout.trim())
    } else {
        Value::Null
    }
}

pub fn list_refs(root: &str) -> Value {
    let jobs: Vec<Vec<&str>> = vec![
        vec!["for-each-ref", "--format=%(refname)%1f%(objectname)%1f%(*objectname)%1f%(upstream:short)%1f%(upstream:track,nobracket)", "refs/heads", "refs/remotes", "refs/tags"],
        vec!["remote"],
        vec!["symbolic-ref", "-q", "--short", "HEAD"],
        vec!["rev-parse", "-q", "--verify", "HEAD"],
        vec!["config", "user.name"],
    ];
    let res: Vec<ExecResult> = std::thread::scope(|s| {
        let hs: Vec<_> = jobs.iter().map(|a| s.spawn(move || git(root, a, &t()))).collect();
        hs.into_iter().map(|h| h.join().unwrap_or_default()).collect()
    });
    if !res[0].ok() {
        return json!({"ok": false, "error": fail(&res[0])});
    }
    let remotes: Vec<String> = res[1].stdout.split('\n').filter(|s| !s.is_empty()).map(String::from).collect();
    let parsed = parse_ref_lines(&res[0].stdout, &remotes);
    json!({
        "ok": true, "root": root,
        "head": {"branch": opt_trim(&res[2]), "hash": opt_trim(&res[3])},
        "local": parsed["local"], "remotes": parsed["remotes"], "tags": parsed["tags"],
        "me": opt_trim(&res[4]),
    })
}

pub fn resolve_commit(root: &str, hash: &Value) -> Result<String, &'static str> {
    if !is_hash(hash) {
        return Err("denied");
    }
    let spec = format!("{}^{{commit}}", hash.as_str().unwrap_or(""));
    let res = git(root, &["rev-parse", "--verify", "-q", &spec], &ExecOpts::max(4096));
    if res.ok() {
        Ok(res.stdout.trim().to_string())
    } else {
        Err("missing")
    }
}

pub fn log_page(root: &str, raw: &Value) -> Value {
    let q = clean_log_query(raw, root);
    let has_head = git(root, &["rev-parse", "-q", "--verify", "HEAD"], &ExecOpts::max(4096)).ok();
    if !has_head && q.branch.is_none() {
        return json!({"ok": true, "commits": [], "more": false});
    }
    let args = log_args(&q);
    let refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let res = git(root, &refs, &t());
    if !res.ok() && !res.truncated {
        return json!({"ok": false, "error": fail(&res)});
    }
    let mut commits = parse_log(&res.stdout);
    let more = commits.len() > LOG_PAGE || res.truncated;
    commits.truncate(LOG_PAGE);
    let git_rows = commits.len();
    if let Some(text) = q.text.as_ref().filter(|t| q.skip == 0.0 && is_hash(&json!(t))) {
        if let Ok(full) = resolve_commit(root, &json!(text)) {
            let fmt = format!("--format={LOG_FORMAT}");
            let one = git(root, &["log", "-z", &fmt, "-n", "1", &full, "--"], &t());
            if one.ok() {
                if let Some(found) = parse_log(&one.stdout).into_iter().next() {
                    commits.retain(|c| c["hash"] != found["hash"]);
                    commits.insert(0, found);
                }
            }
        }
    }
    let mut v = json!({"ok": true, "commits": commits, "more": more});
    if commits_len(&v) > git_rows {
        v["injected"] = json!(commits_len(&v) - git_rows);
    }
    v
}

fn commits_len(v: &Value) -> usize {
    v["commits"].as_array().map(Vec::len).unwrap_or(0)
}

fn parent_of(root: &str, parents: &[&str]) -> Option<String> {
    parents.first().filter(|p| !p.is_empty()).map(|p| p.to_string()).or_else(|| empty_tree(root))
}

pub fn commit_detail(root: &str, hash: &Value) -> Value {
    let full = match resolve_commit(root, hash) {
        Ok(f) => f,
        Err(e) => return json!({"ok": false, "error": e}),
    };
    let show = git(root, &["show", "-s", "--format=%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%cn%x1f%ce%x1f%ct%x1f%B", &full], &t());
    if !show.ok() {
        return json!({"ok": false, "error": fail(&show)});
    }
    let f: Vec<&str> = show.stdout.split(FS).collect();
    let parents: Vec<&str> = match f.get(1) {
        Some(p) if !p.is_empty() => p.split(' ').collect(),
        _ => Vec::new(),
    };
    let Some(base) = parent_of(root, &parents) else { return json!({"ok": false, "error": "failed"}) };
    let (names, nums) = std::thread::scope(|s| {
        let a = s.spawn(|| git(root, &["diff-tree", "-r", "-z", "-M", "--name-status", &base, &full], &t()));
        let b = s.spawn(|| git(root, &["diff-tree", "-r", "-z", "-M", "--histogram", "--numstat", &base, &full], &t()));
        (a.join().unwrap_or_default(), b.join().unwrap_or_default())
    });
    if !names.ok() {
        return json!({"ok": false, "error": fail(&names)});
    }
    let counts = if nums.ok() { parse_numstat(&nums.stdout) } else { Vec::new() };
    let all = parse_name_status(&names.stdout);
    let total = all.len();
    let files: Vec<Value> = all
        .into_iter()
        .take(MAX_COMMIT_FILES)
        .map(|mut m| {
            let p = m["path"].as_str().unwrap_or("").to_string();
            if let Some(c) = counts_for(&counts, &p) {
                if let Some(a) = c.added {
                    m.insert("added".into(), json!(a as i64));
                }
                if let Some(r) = c.removed {
                    m.insert("removed".into(), json!(r as i64));
                }
                m.insert("binary".into(), json!(c.binary));
            }
            Value::Object(m)
        })
        .collect();
    let field = |i: usize| f.get(i).copied().unwrap_or("");
    let message = if f.len() > 8 { f[8..].join(&FS.to_string()) } else { String::new() };
    json!({
        "ok": true, "hash": field(0), "parents": parents, "author": field(2), "email": field(3),
        "authorTime": js_number(field(4)) as i64, "committer": field(5), "committerEmail": field(6),
        "commitTime": js_number(field(7)) as i64, "message": message.trim_end_matches('\n'),
        "files": files, "truncated": total > MAX_COMMIT_FILES || names.truncated,
    })
}

pub fn branches_containing(root: &str, hash: &Value) -> Value {
    static RH: OnceLock<Regex> = OnceLock::new();
    let full = match resolve_commit(root, hash) {
        Ok(f) => f,
        Err(e) => return json!({"ok": false, "error": e}),
    };
    let res = git(root, &["branch", "-a", "--contains", &full, "--format=%(refname)"], &t());
    if !res.ok() {
        return json!({"ok": false, "error": fail(&res)});
    }
    let rh = RH.get_or_init(|| Regex::new(r"^refs/remotes/.+/HEAD$").expect("regex"));
    let list: Vec<String> = res
        .stdout
        .split('\n')
        .map(str::trim)
        .filter(|b| b.starts_with("refs/") && !rh.is_match(b))
        .map(|b| b.strip_prefix("refs/heads/").or_else(|| b.strip_prefix("refs/remotes/")).unwrap_or(b).to_string())
        .take(500)
        .collect();
    json!({"ok": true, "branches": list})
}

pub fn commit_diff(root: &str, hash: &Value, rel: &Value, opts: &Value, orig_rel: Option<&Value>) -> Value {
    let safe = safe_repo_path(root, rel);
    let orig = orig_rel.and_then(|o| safe_repo_path(root, o));
    let (Some(safe), true) = (safe, orig_rel.is_none() || orig.is_some()) else { return json!({"ok": false, "error": "denied"}) };
    let full = match resolve_commit(root, hash) {
        Ok(f) => f,
        Err(e) => return json!({"ok": false, "error": e}),
    };
    let rl = git(root, &["rev-list", "--parents", "-n", "1", &full], &ExecOpts::max(16_384));
    let line = rl.stdout.trim().to_string();
    let parents: Vec<&str> = line.split(' ').skip(1).collect();
    let Some(base) = parent_of(root, &parents) else { return json!({"ok": false, "error": "failed"}) };
    let mut paths: Vec<&str> = Vec::new();
    if let Some(o) = &orig {
        paths.push(&o.rel);
    }
    paths.push(&safe.rel);
    run_capped_diff(root, &["diff", &base, &full], &paths, clean_diff_options(opts), &safe.rel)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_refs_and_logs() {
        let out = "refs/heads/main\x1fabc\x1f\x1forigin/main\x1fahead 2, behind 1\nrefs/remotes/my/fork/x\x1fdef\x1f\x1f\x1f\nrefs/remotes/origin/HEAD\x1fdef\x1f\x1f\x1f\nrefs/tags/v1\x1f111\x1f222\x1f\x1f\n";
        let v = parse_ref_lines(out, &["my".into(), "my/fork".into()]);
        assert_eq!(v["local"][0]["ahead"], 2);
        assert_eq!(v["remotes"][0]["name"], "my/fork");
        assert_eq!(v["tags"][0]["hash"], "222");
        let l = parse_log("\nabc\x1fp1 p2\x1fA\x1fa@x\x1f12\x1fHEAD\x1fsub\x1fject\0");
        assert_eq!(l[0]["subject"], "sub\x1fject");
        assert_eq!(l[0]["parents"][1], "p2");
        let n = parse_name_status("M\0a\0R087\0old\0new\0");
        assert_eq!(n[1]["origPath"], "old");
        assert_eq!(locale_compare("b", "A"), std::cmp::Ordering::Greater);
    }
}
