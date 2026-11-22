use super::*;
use std::path::PathBuf;

const GH: &str = "ghp_A1b2C3d4E5f6G7h8I9j0K1l2M3n4O5p6";
const AWS: &str = "AKIAQ3EGRTWYZ7KJ2M4N";

fn kinds(path: &str, line: &str) -> Vec<&'static str> {
    scan_line(path, line).into_iter().map(|f| f.kind).collect()
}

fn far() -> Instant {
    Instant::now() + Duration::from_secs(30)
}

#[test]
fn detects_known_secrets() {
    assert_eq!(kinds("src/api.ts", &format!("const t = '{GH}'")), vec!["github"]);
    assert_eq!(kinds("deploy.sh", &format!("export AWS_ACCESS_KEY_ID={AWS}")), vec!["aws"]);
    assert_eq!(kinds("keys/id_rsa", "-----BEGIN RSA PRIVATE KEY-----"), vec!["private-key"]);
    assert_eq!(kinds("keys/k.pem", "-----BEGIN OPENSSH PRIVATE KEY-----"), vec!["private-key"]);
    assert_eq!(kinds(".env", "STRIPE_SECRET=Zq81kLmn02pX"), vec!["env"]);
    assert_eq!(kinds("apps/web/.env.local", "DB_PASSWORD=\"s3cr3t-pass\""), vec!["env"]);
    assert_eq!(kinds("config/app.yaml", "api_key: Qw9ErT5yU3iO1pAs"), vec!["assignment"]);
    assert_eq!(kinds("src/a.ts", "const apiKey = 'Qw9ErT5yU3iO1pAs'"), vec!["assignment"]);
    assert_eq!(kinds("relay.json", "\"t\": \"cfut_9aZk3Lm2Qp8Rx7Wv\""), vec!["cloudflare"]);
    assert_eq!(kinds("x.py", "slack = 'xoxb-1234567890-abcdefgh'"), vec!["slack"]);
}

#[test]
fn skips_obvious_samples_and_code() {
    assert!(kinds("docs/aws.md", "AKIAIOSFODNN7EXAMPLE").is_empty());
    assert!(kinds("README.md", "token: ghp_xxxxxxxxxxxxxxxxxxxxxxxx").is_empty());
    assert!(kinds(".env", "API_KEY=your_api_key_here").is_empty());
    assert!(kinds(".env", "SESSION_TIMEOUT=3600").is_empty());
    assert!(kinds(".env", "GITHUB_TOKEN=${GITHUB_TOKEN}").is_empty());
    assert!(kinds(".env", "AUTH_ENABLED=true").is_empty());
    assert!(kinds("src/a.ts", "const token = getToken(user)").is_empty());
    assert!(kinds("src/a.ts", "const password = process.env.DB_PASSWORD").is_empty());
    assert!(kinds("src/a.ts", "const sessionKey = 'ccm.session.v1'").is_empty());
    assert!(kinds("i18n/vi.json", "\"passwordLabel\": \"Mật khẩu của bạn\"").is_empty());
    assert!(kinds("src/a.ts", "let token = abc123XYZ987def").is_empty());
}

#[test]
fn masks_values() {
    for (path, line) in [("a.ts", format!("x = '{GH}'")), (".env", "STRIPE_SECRET=Zq81kLmn02pX".to_owned()), ("b.sh", AWS.to_owned())] {
        let found = scan_line(path, &line);
        assert!(!found.is_empty(), "{line}");
        for f in found {
            assert!(f.masked.contains(MASK), "{}", f.masked);
            assert!(!f.masked.contains("Zq81kLmn02pX") && !f.masked.contains(&GH[4..]) && !f.masked.contains(&AWS[4..]), "{}", f.masked);
        }
    }
    assert_eq!(scan_line("a.ts", &format!("x = '{GH}'"))[0].masked, format!("ghp_{MASK}"));
    assert_eq!(scan_line(".env", "STRIPE_SECRET=Zq81kLmn02pX")[0].masked, format!("STRIPE_SECRET={MASK}"));
}

fn sample_diff() -> String {
    [
        "diff --git a/.env b/.env",
        "new file mode 100644",
        "--- /dev/null",
        "+++ b/.env",
        "@@ -0,0 +1,3 @@",
        "+DEBUG=1",
        "+NAME=demo",
        "+STRIPE_SECRET=Zq81kLmn02pX",
        "diff --git a/src/api.ts b/src/api.ts",
        "--- a/src/api.ts",
        "+++ b/src/api.ts",
        "@@ -10,0 +11,2 @@ fn x()",
        &format!("+const a = '{GH}'"),
        &format!("+const b = '{GH}' // orrery-secret-allow"),
        "@@ -40 +42 @@",
        &format!("+++ AWS {AWS}"),
        "diff --git a/logo.png b/logo.png",
        "Binary files a/logo.png and b/logo.png differ",
        "diff --git a/.env.example b/.env.example",
        "--- /dev/null",
        "+++ b/.env.example",
        "@@ -0,0 +1 @@",
        "+STRIPE_SECRET=Zq81kLmn02pX",
        "",
    ]
    .join("\n")
}

#[test]
fn scans_diff_lines_with_numbers() {
    let s = scan_diff(&sample_diff(), &Allow::default(), far()).unwrap();
    let got: Vec<_> = s.hits.iter().map(|h| (h.path.as_str(), h.line, h.kind)).collect();
    assert_eq!(got, vec![(".env", 3, "env"), ("src/api.ts", 11, "github"), ("src/api.ts", 42, "aws")]);
    assert!(s.skipped.is_empty());
}

#[test]
fn allow_file_skips_lines_and_paths() {
    let allow = Allow::parse("# comment\nZq81kLmn02pX\npath:src/\n");
    let s = scan_diff(&sample_diff(), &allow, far()).unwrap();
    assert!(s.hits.is_empty(), "{:?}", s.hits);
    let allow = Allow::parse("path:*.ts");
    let s = scan_diff(&sample_diff(), &allow, far()).unwrap();
    assert_eq!(s.hits.len(), 1);
}

#[test]
fn big_files_are_skipped_and_deadline_stops() {
    let mut d = String::from("diff --git a/big.txt b/big.txt\n+++ b/big.txt\n@@ -0,0 +1,2000 @@\n");
    let line = format!("+{}\n", "a".repeat(1000));
    for _ in 0..1100 {
        d.push_str(&line);
    }
    d.push_str(&format!("+{GH}\n"));
    let s = scan_diff(&d, &Allow::default(), far()).unwrap();
    assert!(s.hits.is_empty());
    assert_eq!(s.skipped, vec!["big.txt".to_owned()]);
    assert_eq!(scan_diff(&d, &Allow::default(), Instant::now() - Duration::from_secs(1)), Err("timeout"));
}

#[test]
fn commit_markers_tag_hits() {
    let d = format!("\u{1}abc1234\n\ndiff --git a/k b/k\n+++ b/k\n@@ -0,0 +1 @@\n+{GH}\n\u{1}def5678\n\ndiff --git a/k b/k\n+++ /dev/null\n@@ -1 +0,0 @@\n-{GH}\n");
    let s = scan_diff(&d, &Allow::default(), far()).unwrap();
    assert_eq!(s.hits.len(), 1);
    assert_eq!(s.hits[0].commit.as_deref(), Some("abc1234"));
}

#[test]
fn report_never_carries_raw_values() {
    let s = scan_diff(&sample_diff(), &Allow::default(), far()).unwrap();
    let v = report(&s, false);
    let text = v.to_string();
    assert!(!text.contains("Zq81kLmn02pX") && !text.contains(&GH[4..]) && !text.contains(&AWS[4..]));
    let b = blocked(&v).unwrap();
    assert_eq!(b["error"], "secrets");
    assert_eq!(b["hits"].as_array().unwrap().len(), 3);
    assert!(blocked(&json!({"ok": true, "hits": [], "skipped": [], "truncated": false})).is_none());
    assert_eq!(blocked(&json!({"ok": false, "error": "timeout"})).unwrap()["scanError"], "timeout");
}

fn temp_repo(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("ccm-secret-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let g = |args: &[&str]| assert!(exec_git(&dir, args, &ExecOpts::timeout(20_000)).ok(), "{args:?}");
    g(&["init", "-q"]);
    g(&["config", "user.email", "t@t.t"]);
    g(&["config", "user.name", "t"]);
    g(&["config", "core.autocrlf", "false"]);
    dir
}

#[test]
fn scans_staged_changes_in_a_repo() {
    let dir = temp_repo("staged");
    let root = dir.to_string_lossy().to_string();
    std::fs::write(dir.join("ok.txt"), "hello\n").unwrap();
    std::fs::write(dir.join(".env"), "A=1\nSTRIPE_SECRET=Zq81kLmn02pX\n").unwrap();
    std::fs::write(dir.join("unstaged.ts"), format!("x = '{GH}'\n")).unwrap();
    assert!(exec_git(&dir, &["add", "ok.txt", ".env"], &ExecOpts::timeout(20_000)).ok());
    let v = scan_staged(&root);
    assert_eq!(v["ok"], true, "{v}");
    let hits = v["hits"].as_array().unwrap();
    assert_eq!(hits.len(), 1, "{v}");
    assert_eq!(hits[0]["path"], ".env");
    assert_eq!(hits[0]["line"], 2);
    std::fs::write(dir.join(ALLOW_FILE), "STRIPE_SECRET\n").unwrap();
    assert!(scan_staged(&root)["hits"].as_array().unwrap().is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}
