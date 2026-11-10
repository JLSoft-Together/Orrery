use super::jsre::Js;
use super::runner::Core;
use crate::run::parse::{first_pid, parse_logcat, parse_resolved_activity, pidof_missing, ps_pid};
use crate::run::proc::{kill_pid, pump_lines, spawn, SpawnSpec};
use crate::run::types::{EnvIssue, SessionPatch};
use std::io::Read;
use std::sync::Arc;
use std::time::Instant;

const LAUNCHER: &str = "android.intent.category.LAUNCHER";

fn is_pid(s: &Option<String>) -> bool {
    s.as_deref().is_some_and(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
}

impl Core {
    pub(super) fn launch(self: &Arc<Self>, pkg: &str) -> bool {
        static ERR: Js = Js::new(r"^(Error|Exception)", "m");
        static NO_ACT: Js = Js::new(r"No activities found", "i");
        static ABORTED: Js = Js::new(r"monkey aborted", "i");
        self.state("launching", SessionPatch::default());
        let t = Some(self.t.adb_ms);
        let resolved = self.adb_run(&["shell", "cmd", "package", "resolve-activity", "--brief", "-c", LAUNCHER, pkg], t, false);
        if self.halted() {
            return false;
        }
        let mut launched = false;
        if let Some(component) = parse_resolved_activity(&resolved.out, pkg) {
            let r = self.adb_run(&["shell", "am", "start", "-n", &component], t, true);
            if self.halted() {
                return false;
            }
            launched = r.code == Some(0) && !ERR.test(&r.out);
        }
        if !launched {
            let r = self.adb_run(&["shell", "monkey", "-p", pkg, "-c", LAUNCHER, "1"], t, true);
            if self.halted() {
                return false;
            }
            if NO_ACT.test(&r.out) {
                self.fail(EnvIssue::with("launch", "no-launcher"));
                return false;
            }
            if r.code != Some(0) || ABORTED.test(&r.out) {
                self.fail(EnvIssue::new("launch"));
                return false;
            }
        }
        true
    }

    fn find_pid(self: &Arc<Self>, pkg: &str, use_ps: &mut bool) -> Option<String> {
        let t = Some(self.t.adb_ms);
        if !*use_ps {
            let r = self.adb_run(&["shell", "pidof", "-s", pkg], t, false);
            if !pidof_missing(r.code, &r.out) {
                return if r.code == Some(0) { first_pid(&r.out) } else { None };
            }
            *use_ps = true;
        }
        let r = self.adb_run(&["shell", "ps", "-A", "-o", "PID,NAME"], t, false);
        if self.halted() {
            return None;
        }
        let found = ps_pid(&r.out, pkg);
        if found.is_some() || r.out.contains("PID") {
            return found;
        }
        ps_pid(&self.adb_run(&["shell", "ps"], t, false).out, pkg)
    }

    pub(super) fn watch_pid(self: &Arc<Self>, pkg: &str) {
        let running_at = Instant::now();
        let mut pid: Option<String> = None;
        let mut last_seen = Instant::now();
        let mut use_ps = false;
        let mut noticed = false;
        loop {
            if self.halted() {
                return;
            }
            let next = self.find_pid(pkg, &mut use_ps);
            if self.halted() {
                return;
            }
            let now = Instant::now();
            let since = |at: Instant| now.duration_since(at).as_millis() as u64;
            if pid.is_none() && !noticed && since(running_at) > self.t.no_pid_notice_ms {
                noticed = true;
                self.ctx.hooks.log("logcat", &format!("No running process found for {pkg}; logcat is not attached."), Some("W"), None);
            }
            if is_pid(&next) {
                last_seen = now;
                if next != pid {
                    self.attach(next.as_deref().unwrap_or(""));
                    pid = next;
                }
            } else if pid.is_some() && since(last_seen) > self.t.gone_ms {
                {
                    let mut st = self.lock();
                    if st.halted() {
                        return;
                    }
                    st.ended = true;
                    st.phase = "stopped".into();
                }
                self.kill_all(false);
                self.ctx.hooks.state("stopped", SessionPatch::default());
                return;
            }
            let searching = pid.is_none() && since(running_at) < self.t.gone_ms;
            if !self.wait_ms(if searching { self.t.pid_search_ms } else { self.t.pid_watch_ms }) {
                return;
            }
        }
    }

    fn attach(self: &Arc<Self>, pid: &str) {
        let old = {
            let mut st = self.lock();
            let old = st.logcat.take();
            if let Some(o) = old {
                st.children.remove(&o);
            }
            old
        };
        if let Some(o) = old {
            kill_pid(o, false);
        }
        let Some(adb) = self.lock().adb.clone() else { return };
        let args = vec!["-s".to_string(), self.serial.clone(), "logcat".into(), "-v".into(), "threadtime".into(), format!("--pid={pid}")];
        self.adb_use();
        let Ok(mut child) = spawn(&SpawnSpec { file: adb, args, cwd: self.cwd.clone(), ..Default::default() }, &self.ctx.env) else { return };
        let cpid = child.id();
        if self.add_child(cpid, false) {
            self.lock().logcat = Some(cpid);
        }
        let hooks = self.ctx.hooks.clone();
        let pump = child.stdout.take().map(|out| {
            pump_lines(out, move |line| {
                if line.trim().is_empty() || line.starts_with("--------- beginning of") {
                    return;
                }
                match parse_logcat(line) {
                    Some(p) => hooks.log("logcat", &p.text, Some(&p.level), Some(&p.tag)),
                    None => hooks.log("logcat", line, None, None),
                }
            })
        });
        if let Some(mut err) = child.stderr.take() {
            std::thread::spawn(move || {
                let mut sink = [0u8; 4096];
                while matches!(err.read(&mut sink), Ok(n) if n > 0) {}
            });
        }
        let me = self.clone();
        std::thread::spawn(move || {
            let _ = child.wait();
            if let Some(h) = pump {
                let _ = h.join();
            }
            let mut st = me.lock();
            st.children.remove(&cpid);
            if st.logcat == Some(cpid) {
                st.logcat = None;
            }
        });
    }
}
