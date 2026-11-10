use crate::util;
use serde_json::{json, Value};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

pub const AWAKE_FILE: &str = "keep-awake.json";
pub const AWAKE_GRACE_MS: f64 = 2.0 * 60_000.0;
pub const MODES: [&str; 3] = ["off", "running", "plugged"];
pub const DEFAULT_MODE: &str = "plugged";

#[derive(Debug, Clone, PartialEq)]
pub struct Decision {
    pub hold: bool,
    pub reason: &'static str,
    pub release_at: Option<f64>,
}

pub struct Input {
    pub mode: &'static str,
    pub agents: i64,
    pub jobs: i64,
    pub on_battery: bool,
    pub last_busy_at: Option<f64>,
    pub now: f64,
    pub grace_ms: f64,
}

pub fn decide(i: &Input) -> Decision {
    if i.mode == "off" {
        return Decision { hold: false, reason: "off", release_at: None };
    }
    let busy = i.agents + i.jobs > 0;
    let release_at = if busy { None } else { i.last_busy_at.map(|t| t + i.grace_ms) };
    let wanted = busy || release_at.is_some_and(|r| i.now < r);
    if !wanted {
        return Decision { hold: false, reason: "idle", release_at: None };
    }
    if i.mode == "plugged" && i.on_battery {
        return Decision { hold: false, reason: "battery", release_at: if busy { None } else { release_at } };
    }
    if busy {
        Decision { hold: true, reason: "working", release_at: None }
    } else {
        Decision { hold: true, reason: "grace", release_at }
    }
}

pub fn mode_of(v: &Value) -> Option<&'static str> {
    v.as_str().and_then(|s| MODES.iter().copied().find(|m| *m == s))
}

pub trait Power: Send + Sync {
    fn start(&self);
    fn stop(&self);
    fn on_battery(&self) -> bool;
}

struct Inner {
    mode: &'static str,
    agents: i64,
    jobs: i64,
    last_busy_at: Option<f64>,
    holding: bool,
    timer: u64,
    last: Value,
}

pub struct KeepAwake {
    inner: Mutex<Inner>,
    power: Box<dyn Power>,
    grace_ms: f64,
    on_change: Box<dyn Fn(Value) + Send + Sync>,
    me: Weak<KeepAwake>,
}

fn state(mode: &str, holding: bool, reason: &str, agents: i64, jobs: i64, release_at: Option<f64>) -> Value {
    json!({"mode": mode, "holding": holding, "reason": reason, "agents": agents, "jobs": jobs, "releaseAt": release_at.map(|r| r as i64)})
}

impl KeepAwake {
    pub fn new(power: Box<dyn Power>, mode: &'static str, grace_ms: f64, on_change: Box<dyn Fn(Value) + Send + Sync>) -> Arc<Self> {
        let reason = if mode == "off" { "off" } else { "idle" };
        Arc::new_cyclic(|me| KeepAwake {
            inner: Mutex::new(Inner { mode, agents: 0, jobs: 0, last_busy_at: None, holding: false, timer: 0, last: state(mode, false, reason, 0, 0, None) }),
            power,
            grace_ms,
            on_change,
            me: me.clone(),
        })
    }

    pub fn state(&self) -> Value {
        self.inner.lock().unwrap().last.clone()
    }

    pub fn set_mode(&self, mode: &'static str) {
        let mut g = self.inner.lock().unwrap();
        g.mode = mode;
        self.apply(&mut g);
    }

    pub fn set_busy(&self, agents: i64, jobs: i64) {
        let mut g = self.inner.lock().unwrap();
        let was = g.agents + g.jobs > 0;
        g.agents = agents;
        g.jobs = jobs;
        if agents + jobs > 0 || was {
            g.last_busy_at = Some(util::now_ms());
        }
        self.apply(&mut g);
    }

    pub fn power_changed(&self) {
        let mut g = self.inner.lock().unwrap();
        self.apply(&mut g);
    }

    pub fn dispose(&self) {
        let mut g = self.inner.lock().unwrap();
        g.timer += 1;
        if g.holding {
            self.power.stop();
            g.holding = false;
        }
    }

    fn apply(&self, g: &mut Inner) {
        g.timer += 1;
        let now = util::now_ms();
        let on_battery = g.mode == "plugged" && self.power.on_battery();
        let d = decide(&Input { mode: g.mode, agents: g.agents, jobs: g.jobs, on_battery, last_busy_at: g.last_busy_at, now, grace_ms: self.grace_ms });
        if d.hold && !g.holding {
            self.power.start();
            g.holding = true;
        }
        if !d.hold && g.holding {
            self.power.stop();
            g.holding = false;
        }
        if let Some(at) = d.release_at {
            let gen = g.timer;
            let me = self.me.clone();
            let wait = Duration::from_millis((at - now).max(0.0) as u64);
            std::thread::spawn(move || {
                std::thread::sleep(wait);
                if let Some(k) = me.upgrade() {
                    let mut g = k.inner.lock().unwrap();
                    if g.timer == gen {
                        k.apply(&mut g);
                    }
                }
            });
        }
        let next = state(g.mode, d.hold, d.reason, g.agents, g.jobs, d.release_at);
        if next != g.last {
            g.last = next.clone();
            (self.on_change)(next);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Fake(Arc<AtomicUsize>, bool);

    impl Power for Fake {
        fn start(&self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
        fn stop(&self) {
            self.0.fetch_sub(1, Ordering::SeqCst);
        }
        fn on_battery(&self) -> bool {
            self.1
        }
    }

    #[test]
    fn decisions() {
        let i = |mode, agents, last: Option<f64>, now, bat| Input { mode, agents, jobs: 0, on_battery: bat, last_busy_at: last, now, grace_ms: 100.0 };
        assert_eq!(decide(&i("off", 3, None, 0.0, false)).reason, "off");
        assert_eq!(decide(&i("running", 1, None, 0.0, true)), Decision { hold: true, reason: "working", release_at: None });
        assert_eq!(decide(&i("running", 0, Some(0.0), 50.0, false)), Decision { hold: true, reason: "grace", release_at: Some(100.0) });
        assert_eq!(decide(&i("plugged", 0, Some(0.0), 50.0, true)), Decision { hold: false, reason: "battery", release_at: Some(100.0) });
        assert_eq!(decide(&i("running", 0, Some(0.0), 150.0, false)).reason, "idle");
    }

    #[test]
    fn grace_releases_on_time() {
        let held = Arc::new(AtomicUsize::new(0));
        let changes = Arc::new(Mutex::new(Vec::new()));
        let c = changes.clone();
        let k = KeepAwake::new(Box::new(Fake(held.clone(), false)), "running", 60.0, Box::new(move |v| c.lock().unwrap().push(v["reason"].as_str().unwrap().to_string())));
        k.set_busy(1, 0);
        assert_eq!(held.load(Ordering::SeqCst), 1);
        k.set_busy(0, 0);
        assert_eq!(k.state()["reason"], "grace");
        std::thread::sleep(Duration::from_millis(400));
        assert_eq!(held.load(Ordering::SeqCst), 0);
        assert_eq!(*changes.lock().unwrap(), vec!["working", "grace", "idle"]);
        k.set_mode("off");
        assert_eq!(k.state()["reason"], "off");
    }
}
