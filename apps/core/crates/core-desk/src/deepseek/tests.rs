use super::*;
use crate::DeskIo;
use std::collections::BTreeMap;
use std::sync::Mutex;

struct Io {
    answer: Mutex<bool>,
    events: Mutex<Vec<(String, Value)>>,
    plans: Mutex<Vec<Value>>,
}

impl Io {
    fn new() -> Io {
        Io { answer: Mutex::new(true), events: Mutex::new(vec![]), plans: Mutex::new(vec![]) }
    }
}

impl DeskIo for Io {
    fn emit(&self, event: &str, payload: Value) {
        self.events.lock().unwrap().push((event.to_string(), payload));
    }

    fn host(&self, _req: u64, method: &str, params: Value) -> Result<Value, DeskError> {
        assert_eq!(method, "deepseek.confirm");
        self.plans.lock().unwrap().push(params);
        Ok(Value::Bool(*self.answer.lock().unwrap()))
    }
}

fn tmp(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("core-desk-ds-{name}-{}", crate::util::uuid()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[cfg(unix)]
fn script(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::write(path, format!("#!/bin/sh\nPATH=/bin:/usr/bin:$PATH\n{body}")).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
}

#[cfg(unix)]
fn fake_codewhale(path: &Path, state: &Path, log: &Path) {
    let body = format!(
        r#"echo "$@" >> '{log}'
case "$1" in
  --version) echo "codewhale 0.10.2 (abc)";;
  auth)
    case "$2" in
      status)
        echo "provider config keyring env status"
        if [ -f '{state}/key' ]; then echo "deepseek set unset unset ok"; else echo "deepseek unset unset unset unset"; fi;;
      set) cat > '{state}/key'; if grep -q boom '{state}/key'; then echo "bad key $(cat '{state}/key')" >&2; exit 3; fi;;
      clear) cat > /dev/null; rm -f '{state}/key';;
    esac;;
  config)
    if [ "$2" = get ]; then echo "Usage reporting: $(cat '{state}/tele' 2>/dev/null || echo On) (default)"; elif [ "$4" = true ]; then echo On > '{state}/tele'; else echo Off > '{state}/tele'; fi;;
  doctor) printf 'Doctor\n  ok\nUpdates:\n  - current: v0.10.2\n  - latest: v0.11.0\n';;
esac
"#,
        log = log.display(),
        state = state.display()
    );
    script(path, &body);
}

#[cfg(unix)]
struct Fixture {
    root: PathBuf,
    home: PathBuf,
    svc: Service,
    io: Io,
    log: PathBuf,
}

#[cfg(unix)]
fn fixture(name: &str, with_cw: bool) -> Fixture {
    let root = tmp(name);
    let home = root.join("home");
    let bin = root.join("bin");
    let state = root.join("state");
    for d in [&home, &bin, &state] {
        std::fs::create_dir_all(d).unwrap();
    }
    let log = root.join("calls.log");
    if with_cw {
        fake_codewhale(&bin.join("codewhale"), &state, &log);
    }
    let mut env = BTreeMap::new();
    env.insert("PATH".to_string(), bin.to_string_lossy().to_string());
    env.insert("HOME".to_string(), home.to_string_lossy().to_string());
    let child = crate::extensions::status::child_env(&env.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<Vec<_>>());
    let svc = Service::new(env, home.to_string_lossy().to_string(), false, &root.join("data"), runner::default_runner(child));
    Fixture { root, home, svc, io: Io::new(), log }
}

#[cfg(unix)]
#[test]
fn login_status_telemetry_logout_and_update_check() {
    let f = fixture("cli", true);
    let ctx = Ctx::new(&f.io, 1);
    let s = f.svc.status(true);
    assert_eq!((s["installed"].clone(), s["version"].clone(), s["loggedIn"].clone(), s["telemetry"].clone()), (json!(true), json!("0.10.2"), json!(false), json!(true)));
    assert_eq!((s["origin"].clone(), s["sync"].clone(), s["busy"].clone()), (json!("standalone"), json!("never"), Value::Null));
    assert_eq!(f.svc.login(&json!("short")).unwrap(), json!({"ok": false, "code": "badInput"}));
    assert_eq!(f.svc.login(&json!("sk-boom-12345678")).unwrap()["code"], "failed");
    let r = f.svc.login(&json!("sk-boom-12345678")).unwrap();
    assert!(!r.to_string().contains("sk-boom-12345678"), "{r}");
    assert_eq!(f.svc.login(&json!("sk-good-12345678")).unwrap(), json!({"ok": true}));
    assert_eq!(f.svc.status(true)["loggedIn"], json!(true));
    assert_eq!(f.svc.telemetry(&json!("x")).unwrap()["code"], "badInput");
    assert_eq!(f.svc.telemetry(&json!(false)).unwrap(), json!({"ok": true}));
    assert_eq!(f.svc.status(true)["telemetry"], json!(false));
    assert_eq!(f.svc.check_update(), json!({"ok": true, "current": "0.10.2", "latest": "0.11.0", "newer": true}));
    *f.io.answer.lock().unwrap() = false;
    assert_eq!(f.svc.logout(&ctx).unwrap(), json!({"ok": false, "code": "cancelled"}));
    *f.io.answer.lock().unwrap() = true;
    assert_eq!(f.svc.logout(&ctx).unwrap(), json!({"ok": true}));
    assert_eq!(f.svc.status(true)["loggedIn"], json!(false));
    assert_eq!(f.io.plans.lock().unwrap()[1], json!({"kind": "logout"}));
    let _ = std::fs::remove_dir_all(&f.root);
}

#[cfg(unix)]
#[test]
fn not_installed_answers() {
    let f = fixture("none", false);
    if find_system().is_some() {
        return;
    }
    let s = f.svc.status(true);
    assert_eq!((s["installed"].clone(), s["needsDownload"].clone(), s["version"].clone()), (json!(false), json!(false), Value::Null));
    assert_eq!(f.svc.check_update(), json!({"ok": false, "code": "notInstalled"}));
    assert_eq!(f.svc.login(&json!("sk-good-12345678")).unwrap()["code"], "notInstalled");
    assert_eq!(f.svc.cancel_value(), json!(false));
    let _ = std::fs::remove_dir_all(&f.root);
}

#[cfg(unix)]
fn find_system() -> Option<Codewhale> {
    cli::find_codewhale(&Env::new(), "/nonexistent-home", false)
}

#[cfg(unix)]
#[test]
fn install_runs_the_planned_step_and_streams_lines() {
    let f = fixture("install", false);
    if find_system().is_some() {
        return;
    }
    let bin = f.root.join("bin");
    let state = f.root.join("state");
    script(&bin.join("npm"), &format!("echo 'added 1 package'\ncat > '{bin}/codewhale' <<'EOS'\n#!/bin/sh\necho 'codewhale 0.10.2 (x)'\nEOS\nchmod +x '{bin}/codewhale'\n", bin = bin.display()));
    let _ = state;
    let ctx = Ctx::new(&f.io, 2);
    *f.io.answer.lock().unwrap() = false;
    assert_eq!(f.svc.install(&ctx).unwrap(), json!({"ok": false, "code": "cancelled"}));
    assert!(!bin.join("codewhale").exists());
    *f.io.answer.lock().unwrap() = true;
    assert_eq!(f.svc.install(&ctx).unwrap(), json!({"ok": true}));
    assert!(bin.join("codewhale").exists());
    assert_eq!(f.io.plans.lock().unwrap()[0], json!({"kind": "install", "commands": ["npm i -g codewhale", "codewhale --version"]}));
    let lines: Vec<Value> = f.io.events.lock().unwrap().iter().map(|(_, p)| p.clone()).collect();
    assert!(lines.contains(&json!({"phase": "install", "line": "added 1 package"})), "{lines:?}");
    assert_eq!(f.svc.install(&ctx).unwrap(), json!({"ok": false, "code": "installed"}));
    let _ = std::fs::remove_dir_all(&f.root);
}

#[cfg(unix)]
#[test]
fn failed_install_step_reports_masked_detail() {
    let f = fixture("fail", false);
    if find_system().is_some() {
        return;
    }
    let bin = f.root.join("bin");
    script(&bin.join("npm"), "echo \"token=ghp_abcdefghijklmnopqrstuvwxyz0123456789 in $HOME\" >&2\nexit 7\n");
    let r = f.svc.install(&Ctx::new(&f.io, 3)).unwrap();
    assert_eq!(r["code"], "failed");
    let detail = r["detail"].as_str().unwrap();
    assert!(!detail.contains("ghp_abcdefghijklmnopqrstuvwxyz0123456789") && !detail.contains(&f.home.to_string_lossy().to_string()), "{detail}");
    let _ = std::fs::remove_dir_all(&f.root);
}

#[cfg(unix)]
#[test]
fn sync_writes_files_and_goes_current() {
    let f = fixture("sync", true);
    let claude = f.home.join(".claude");
    std::fs::create_dir_all(&claude).unwrap();
    std::fs::write(claude.join("CLAUDE.md"), "rule one\n@extra.md\n").unwrap();
    std::fs::write(claude.join("extra.md"), "extra rule\n").unwrap();
    std::fs::write(f.home.join(".claude.json"), r#"{"mcpServers":{"figma":{"type":"http","url":"https://f.example/mcp"}}}"#).unwrap();
    std::fs::create_dir_all(f.home.join(".codewhale")).unwrap();
    std::fs::write(f.home.join(".codewhale").join("AGENTS.md"), "mine\n").unwrap();
    let ctx = Ctx::new(&f.io, 4);
    *f.io.answer.lock().unwrap() = false;
    let r = f.svc.sync(&ctx).unwrap();
    assert_eq!(r, json!({"ok": false, "code": "cancelled"}));
    assert!(!f.home.join(".codewhale").join("config.toml").exists());
    *f.io.answer.lock().unwrap() = true;
    let r = f.svc.sync(&ctx).unwrap();
    assert_eq!(r["ok"], true);
    assert_eq!(r["mcpAdded"], json!(["figma"]));
    let agents = std::fs::read_to_string(f.home.join(".codewhale").join("AGENTS.md")).unwrap();
    assert!(agents.starts_with("mine\n\n<!-- orrery:begin -->") && agents.contains("rule one\nextra rule") && agents.ends_with("<!-- orrery:end -->\n"));
    assert_eq!(std::fs::read_to_string(f.home.join(".codewhale").join("AGENTS.md.orrery-backup")).unwrap(), "mine\n");
    let cfg = std::fs::read_to_string(f.home.join(".codewhale").join("config.toml")).unwrap();
    assert!(cfg.contains("project_instruction_imports = [\"claude\"]") && cfg.contains("[lifecycle_outbox]"));
    assert!(f.home.join(".codewhale/skills/orrery-handoff/SKILL.md").exists() && f.home.join(".claude/skills/orrery-delegate/SKILL.md").exists());
    let files = r["files"].as_array().unwrap();
    assert_eq!(files[0], json!({"id": "agents", "file": "~/.codewhale/AGENTS.md", "action": "update"}));
    assert_eq!(f.svc.status(true)["sync"], "current");
    let again = f.svc.sync(&ctx).unwrap();
    assert!(again["files"].as_array().unwrap().iter().all(|x| x["action"] == "unchanged"));
    assert_eq!(f.io.plans.lock().unwrap().len(), 2);
    let _ = f.log;
    let _ = std::fs::remove_dir_all(&f.root);
}

#[test]
fn busy_is_refused_and_released() {
    let root = tmp("busy");
    let svc = Service::new(Env::new(), root.to_string_lossy().to_string(), false, &root, runner::default_runner(BTreeMap::new()));
    let inner = svc.exclusive("install", || svc.exclusive("login", || Ok(json!(1)))).unwrap();
    assert_eq!(inner, json!({"ok": false, "code": "busy"}));
    assert_eq!(svc.exclusive("login", || Ok(json!(2))).unwrap(), json!(2));
    let _ = std::fs::remove_dir_all(&root);
}
