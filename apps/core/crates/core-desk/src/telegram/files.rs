use super::clock::Local;
use super::html::Masker;
use super::sanitize::is_secret_file_name;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const GIT_TIMEOUT: Duration = Duration::from_secs(20);
const DIFF_MAX: usize = 45 * 1024 * 1024;
const SECRET_GLOBS: [&str; 10] = [".env*", "*.jks", "*.keystore", "*.p12", "*.pem", "key.properties", "local.properties", "google-services.json", "GoogleService-Info.plist", "*.mobileprovision"];

fn git(cwd: &Path, args: &[String], max: usize) -> Option<String> {
    let bin = crate::git::git_path()?;
    let mut cmd = Command::new(bin);
    cmd.current_dir(cwd).arg("-c").arg("core.quotepath=false").args(args);
    cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let mut child = cmd.spawn().ok()?;
    let mut out = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let mut chunk = [0u8; 64 * 1024];
        let mut over = false;
        while let Ok(n) = out.read(&mut chunk) {
            if n == 0 {
                break;
            }
            if buf.len() + n > max {
                over = true;
            } else {
                buf.extend_from_slice(&chunk[..n]);
            }
        }
        (buf, over)
    });
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if start.elapsed() >= GIT_TIMEOUT => {
                let _ = child.kill();
                let _ = child.wait();
                break None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(10)),
            Err(_) => break None,
        }
    };
    let (buf, over) = reader.join().ok()?;
    (status?.success() && !over).then(|| String::from_utf8_lossy(&buf).into_owned())
}

fn args(list: &[&str]) -> Vec<String> {
    list.iter().map(|s| s.to_string()).collect()
}

fn pathspec() -> Vec<String> {
    let mut v = vec![".".to_string()];
    v.extend(SECRET_GLOBS.iter().map(|g| format!(":(exclude,glob,icase)**/{g}")));
    v
}

pub fn project_diff(cwd: &str, masker: &Masker) -> Result<(String, usize), &'static str> {
    let top = git(Path::new(cwd), &args(&["rev-parse", "--show-toplevel"]), 1024 * 1024).ok_or("not-repo")?;
    let root = PathBuf::from(top.trim());
    let base: Vec<String> = if git(&root, &args(&["rev-parse", "--verify", "-q", "HEAD"]), 1024 * 1024).is_some() { vec!["HEAD".into()] } else { Vec::new() };
    let mut names_args = args(&["diff", "--name-only"]);
    names_args.extend(base.clone());
    names_args.push("--".into());
    names_args.extend(pathspec());
    let names = git(&root, &names_args, 4 * 1024 * 1024).ok_or("failed")?;
    let files = names.split('\n').map(|l| l.trim_end_matches('\r')).filter(|l| !l.is_empty()).filter(|f| !is_secret_file_name(f.rsplit('/').next().unwrap_or(f))).count();
    if files == 0 {
        return Err("clean");
    }
    let mut diff_args = args(&["diff", "--no-color", "--no-ext-diff"]);
    diff_args.extend(base);
    diff_args.push("--".into());
    diff_args.extend(pathspec());
    let diff = git(&root, &diff_args, DIFF_MAX).ok_or("failed")?;
    if super::jsx::trim(&diff).is_empty() {
        return Err("clean");
    }
    Ok((masker.clean(&diff), files))
}

fn stamp(l: &Local) -> String {
    format!("{:04}{:02}{:02}-{:02}{:02}{:02}", l.y, l.mo, l.d, l.h, l.mi, l.s)
}

fn safe_name(name: &str) -> String {
    let mut out = String::new();
    let mut run = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
            out.push(c);
            run = false;
        } else if !run {
            out.push('_');
            run = true;
        }
    }
    let out = super::jsx::tail(out.trim_start_matches('.'), 80);
    if out.is_empty() {
        "file.bin".into()
    } else {
        out
    }
}

fn exclude_orrery(project: &Path) {
    let Some(rel) = git(project, &args(&["rev-parse", "--git-path", "info/exclude"]), 1024 * 1024) else { return };
    let file = crate::util::resolve(&crate::util::path_str(&project.join(rel.trim())));
    let text = std::fs::read_to_string(&file).unwrap_or_default();
    if text.split('\n').any(|l| super::jsx::trim(l) == ".orrery/") {
        return;
    }
    if let Some(dir) = file.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let add = format!("{}.orrery/\n", if !text.is_empty() && !text.ends_with('\n') { "\n" } else { "" });
    let _ = std::fs::OpenOptions::new().append(true).create(true).open(&file).and_then(|mut f| std::io::Write::write_all(&mut f, add.as_bytes()));
}

pub fn save_inbound(project: &str, name: &str, data: &[u8], now: &Local) -> Option<String> {
    let dir = Path::new(project).join(".orrery").join("telegram");
    let safe = safe_name(name);
    let raw_base = name.replace('\\', "/");
    let raw_base = raw_base.rsplit('/').next().unwrap_or("");
    if is_secret_file_name(&safe) || is_secret_file_name(raw_base) {
        return None;
    }
    std::fs::create_dir_all(&dir).ok()?;
    let file = dir.join(format!("{}-{safe}", stamp(now)));
    let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(&file).ok()?;
    std::io::Write::write_all(&mut f, data).ok()?;
    drop(f);
    exclude_orrery(Path::new(project));
    Some(crate::util::path_str(&file))
}

fn body(tool: &str, what: &str) -> String {
    format!(
        "---\ndescription: Orrery {what} on / off (on | off | status)\nargument-hint: on | off | status\nallowed-tools: mcp__orrery__{tool}\n---\nCall the MCP tool `mcp__orrery__{tool}` once with `state` set to \"$ARGUMENTS\" (on, off or status; empty means status).\nThen tell the user the result in one short line, in the language they write in. Do nothing else.\nIf the tool is not available, say that this command only works in a session opened inside Orrery.\n"
    )
}

pub fn command_file(root: &Path, name: &str) -> PathBuf {
    root.join("commands").join(format!("{name}.md"))
}

pub fn install_command(root: &Path, name: &str) -> bool {
    let file = command_file(root, name);
    let text = if name == super::base::RECEIVE_COMMAND { body("telegram_receive", "Telegram commands into sessions (only the paired account)") } else { body("telegram_notify", "Telegram reports for the whole app") };
    let Some(dir) = file.parent() else { return false };
    if std::fs::create_dir_all(dir).is_err() {
        return false;
    }
    let mut tmp = file.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, text).is_ok() && std::fs::rename(&tmp, &file).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saves_files_and_skips_secrets() {
        let dir = std::env::temp_dir().join(format!("ccm-tg-files-{}", crate::util::uuid()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = crate::util::path_str(&dir);
        let now = Local { y: 2026, mo: 11, d: 10, h: 9, mi: 5, s: 7 };
        let f = save_inbound(&p, "my photo.jpg", b"x", &now).unwrap();
        assert!(f.ends_with("20261110-090507-my_photo.jpg"));
        assert!(save_inbound(&p, "my photo.jpg", b"y", &now).is_none());
        assert!(save_inbound(&p, ".env", b"x", &now).is_none());
        assert!(save_inbound(&p, "a/b/key.properties", b"x", &now).is_none());
        let root = dir.join("claude");
        assert!(install_command(&root, super::super::base::RECEIVE_COMMAND));
        let text = std::fs::read_to_string(command_file(&root, "bot-tele-receive")).unwrap();
        assert!(text.contains("allowed-tools: mcp__orrery__telegram_receive\n"));
        assert!(project_diff(&p, &Masker { home: String::new(), platform: "darwin" }) == Err("not-repo") || crate::git::git_path().is_none());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
