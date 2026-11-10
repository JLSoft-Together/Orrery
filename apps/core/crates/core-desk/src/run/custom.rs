use super::args::expand_args;
use super::before::{resolve_for, run_before_launch, run_step};
use super::flutter::runner::{create_with_command, CustomCommand};
use super::proc::{kill_pid, subst_vars};
use super::types::{EnvIssue, RunContext, RunResult, Runner, SessionPatch};
use crate::preview::format::with_args;
use regex::Regex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const STOP_GRACE_MS: u64 = 3_000;

fn base(a: &str) -> String {
    a.replace('\\', "/").rsplit('/').next().unwrap_or(a).to_string()
}

pub fn flutter_machine_mode(argv: &[String]) -> Option<&'static str> {
    if !argv.iter().any(|a| a == "--machine") {
        return None;
    }
    let flutter = Regex::new(r"(?i)^flutter(\.bat|\.cmd|\.exe)?$").expect("regex");
    let head = Regex::new(r"(?i)^(flutter|fvm|puro)(\.bat|\.cmd|\.exe)?$").expect("regex");
    let heads: Vec<String> = argv.iter().take(3).map(|a| base(a)).collect();
    let at = heads.iter().position(|h| flutter.is_match(h))?;
    if !heads[..=at].iter().all(|h| head.is_match(h)) || argv.get(at + 1).map(String::as_str) != Some("run") {
        return None;
    }
    Some(if argv.iter().any(|a| a == "--release") {
        "release"
    } else if argv.iter().any(|a| a == "--profile") {
        "profile"
    } else {
        "debug"
    })
}

pub struct CustomRunner {
    ctx: RunContext,
    pid: AtomicU32,
    stopping: AtomicBool,
    ended: AtomicBool,
    machine: Mutex<Option<Arc<dyn Runner>>>,
}

pub fn create(ctx: RunContext) -> Arc<dyn Runner> {
    Arc::new(CustomRunner { ctx, pid: AtomicU32::new(0), stopping: AtomicBool::new(false), ended: AtomicBool::new(false), machine: Mutex::new(None) })
}

impl CustomRunner {
    fn track(&self, pid: Option<u32>) {
        self.pid.store(pid.unwrap_or(0), Ordering::SeqCst);
    }

    fn cancel(&self) {
        self.ended.store(true, Ordering::SeqCst);
        self.ctx.hooks.state("cancelled", SessionPatch::default());
    }
}

impl Runner for CustomRunner {
    fn start(&self) {
        let ctx = &self.ctx;
        let hooks = &ctx.hooks;
        if ctx.config["type"] != "custom" {
            return hooks.fail(EnvIssue::new("spawn"));
        }
        let cwd = ctx.cfg_str("cwd");
        let argv = expand_args(&ctx.cfg_str("command"), &subst_vars(&cwd, &ctx.root, &ctx.env, &ctx.cfg_env()));
        let head = argv.first().and_then(|h| resolve_for(ctx, h));
        let Some(head) = head else { return hooks.fail(EnvIssue::with("spawn", argv.first().cloned().unwrap_or_default())) };
        let Some(cmd) = with_args(&head, &argv[1..]) else { return hooks.fail(EnvIssue::new("bad-arg")) };
        if let Some(mode) = flutter_machine_mode(&argv) {
            let machine = create_with_command(ctx.clone(), CustomCommand { file: cmd.file.clone(), args: cmd.args.clone(), verbatim: cmd.verbatim, mode: mode.to_string() });
            *self.machine.lock().unwrap() = Some(machine.clone());
            if self.stopping.load(Ordering::SeqCst) {
                return self.cancel();
            }
            return machine.start();
        }
        if !ctx.cfg_before().is_empty() {
            hooks.state("before-launch", SessionPatch::default());
            let ok = run_before_launch(ctx, &|c| resolve_for(ctx, c), &|p| self.track(p), &|| self.stopping.load(Ordering::SeqCst));
            if self.stopping.load(Ordering::SeqCst) {
                return self.cancel();
            }
            if !ok {
                return hooks.fail(EnvIssue::new("before-launch"));
            }
        }
        if self.stopping.load(Ordering::SeqCst) {
            return self.cancel();
        }
        hooks.state("running", SessionPatch::default());
        let ok = run_step(ctx, &cmd.file, &cmd.args, cmd.verbatim, "app", &|p| self.track(p));
        self.ended.store(true, Ordering::SeqCst);
        if self.stopping.load(Ordering::SeqCst) || ok {
            hooks.state("stopped", SessionPatch::default());
        } else {
            hooks.fail(EnvIssue::new("exit"));
        }
    }

    fn stop(&self) {
        if let Some(m) = self.machine.lock().unwrap().clone() {
            return m.stop();
        }
        if self.stopping.load(Ordering::SeqCst) || self.ended.load(Ordering::SeqCst) {
            return;
        }
        self.stopping.store(true, Ordering::SeqCst);
        let pid = self.pid.load(Ordering::SeqCst);
        if pid == 0 {
            return;
        }
        self.ctx.hooks.state("stopping", SessionPatch::default());
        kill_pid(pid, false);
        let start = Instant::now();
        while self.pid.load(Ordering::SeqCst) == pid && start.elapsed() < Duration::from_millis(STOP_GRACE_MS) {
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn kill(&self) {
        self.stopping.store(true, Ordering::SeqCst);
        if let Some(m) = self.machine.lock().unwrap().clone() {
            return m.kill();
        }
        let pid = self.pid.load(Ordering::SeqCst);
        if pid != 0 {
            kill_pid(pid, true);
        }
    }

    fn reload(&self, full: bool) -> RunResult {
        match self.machine.lock().unwrap().clone() {
            Some(m) => m.reload(full),
            None => RunResult::err("reload-unsupported"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn detects_flutter_machine() {
        assert_eq!(flutter_machine_mode(&v(&["flutter", "run", "--machine"])), Some("debug"));
        assert_eq!(flutter_machine_mode(&v(&["fvm", "flutter", "run", "--machine", "--release"])), Some("release"));
        assert_eq!(flutter_machine_mode(&v(&["/sdk/bin/flutter.bat", "run", "--machine", "--profile"])), Some("profile"));
        assert_eq!(flutter_machine_mode(&v(&["flutter", "run"])), None);
        assert_eq!(flutter_machine_mode(&v(&["node", "flutter", "run", "--machine"])), None);
    }
}
