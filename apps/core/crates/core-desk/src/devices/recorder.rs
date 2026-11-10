use super::manager::DeviceManager;
use super::parse::{err, err_detail, is_ok, ok};
use super::toggles::sh_quote;
use crate::js;
use crate::run::proc::kill_pid_only;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use std::process::Child;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

pub const REC_LIMIT_S: u64 = 30;
pub const REC_BIT_RATE: u64 = 8_000_000;
pub const FRAMES_KEEP: usize = 16;
pub const FRAMES_PICK: usize = 6;
pub const TOKENS_MAX: usize = 3;

#[derive(Clone, Copy)]
pub struct RecTiming {
    pub frame_ms: u64,
    pub start_check_ms: u64,
    pub stop_wait_ms: u64,
    pub pull_ms: u64,
    pub cmd_ms: u64,
    pub orphan_ms: u64,
}

pub const TIMING: RecTiming = RecTiming { frame_ms: 1_000, start_check_ms: 800, stop_wait_ms: 6_000, pull_ms: 120_000, cmd_ms: 10_000, orphan_ms: (REC_LIMIT_S + 60) * 1_000 };

fn now_ms() -> u64 {
    crate::util::now_ms() as u64
}

static HELD: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub fn held_files() -> Vec<String> {
    HELD.lock().unwrap().clone()
}

pub fn is_rec_token(v: &Value) -> bool {
    v.as_str().is_some_and(|s| s.len() == 32 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')))
}

pub fn remote_name(ts: u64) -> String {
    format!("orrery-rec-{ts}")
}

pub fn remote_path(ts: u64) -> String {
    format!("/sdcard/{}.mp4", remote_name(ts))
}

pub fn record_args(ts: u64) -> Vec<String> {
    vec!["screenrecord".into(), "--time-limit".into(), REC_LIMIT_S.to_string(), "--bit-rate".into(), REC_BIT_RATE.to_string(), remote_path(ts)]
}

pub fn stop_args(ts: u64) -> Vec<String> {
    vec!["pkill".into(), "-INT".into(), "-f".into(), sh_quote(&remote_name(ts))]
}

pub fn remove_args(ts: u64) -> Vec<String> {
    vec!["rm".into(), "-f".into(), sh_quote(&remote_path(ts))]
}

pub fn thin_frames<T: Clone>(frames: &[T], keep: usize) -> Option<Vec<T>> {
    (frames.len() > keep).then(|| frames.iter().enumerate().filter(|(i, _)| i % 2 == 0).map(|(_, f)| f.clone()).collect())
}

pub fn pick_even<T: Clone>(list: &[T], n: usize) -> Vec<T> {
    if list.len() <= n {
        return list.to_vec();
    }
    if n <= 1 {
        return list[..n].to_vec();
    }
    (0..n).map(|i| list[((i as f64 * (list.len() - 1) as f64) / (n - 1) as f64).round() as usize].clone()).collect()
}

pub fn mp4_complete(read: &mut dyn FnMut(u64, usize) -> Vec<u8>, size: u64) -> bool {
    let (mut pos, mut first, mut moov) = (0u64, true, false);
    let mut n = 0;
    while pos < size && n < 10_000 {
        n += 1;
        let h = read(pos, 16);
        if h.len() < 8 {
            return false;
        }
        let mut len = u32::from_be_bytes([h[0], h[1], h[2], h[3]]) as u64;
        let kind = [h[4], h[5], h[6], h[7]];
        if len == 1 {
            if h.len() < 16 {
                return false;
            }
            len = u64::from_be_bytes([h[8], h[9], h[10], h[11], h[12], h[13], h[14], h[15]]);
        } else if len == 0 {
            len = size - pos;
        }
        if len < 8 || pos + len > size {
            return false;
        }
        if first && &kind != b"ftyp" {
            return false;
        }
        first = false;
        if &kind == b"moov" {
            moov = true;
        }
        pos += len;
    }
    moov && pos == size
}

pub fn file_complete(file: &std::path::Path) -> bool {
    let Ok(mut f) = std::fs::File::open(file) else { return false };
    let Ok(size) = f.metadata().map(|m| m.len()) else { return false };
    size > 0
        && mp4_complete(
            &mut |pos, len| {
                let mut buf = vec![0u8; len];
                if f.seek(SeekFrom::Start(pos)).is_err() {
                    return Vec::new();
                }
                let mut got = 0;
                while got < len {
                    match f.read(&mut buf[got..]) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => got += n,
                    }
                }
                buf.truncate(got);
                buf
            },
            size,
        )
}

#[derive(Default)]
struct Gate {
    halted: Mutex<bool>,
    cv: Condvar,
}

impl Gate {
    fn halt(&self) {
        *self.halted.lock().unwrap() = true;
        self.cv.notify_all();
    }

    fn is_halted(&self) -> bool {
        *self.halted.lock().unwrap()
    }

    fn wait(&self, ms: u64) -> bool {
        let g = self.halted.lock().unwrap();
        let (g, _) = self.cv.wait_timeout_while(g, Duration::from_millis(ms), |h| !*h).unwrap();
        *g
    }
}

struct Session {
    ts: u64,
    pid: u32,
    exited: Arc<(Mutex<Option<Option<i32>>>, Condvar)>,
    out: Arc<Mutex<String>>,
    started_at: u64,
    frames: Mutex<Vec<String>>,
    frames_failed: Mutex<u64>,
    frame_ms: Mutex<u64>,
    gate: Gate,
    stopping: Mutex<Option<Value>>,
}

impl Session {
    fn alive(&self) -> bool {
        self.exited.0.lock().unwrap().is_none()
    }

    fn wait_exit(&self, ms: u64) -> bool {
        let g = self.exited.0.lock().unwrap();
        let (g, _) = self.exited.1.wait_timeout_while(g, Duration::from_millis(ms), |e| e.is_none()).unwrap();
        g.is_some()
    }
}

enum Slot {
    Starting,
    Live(Arc<Session>),
}

struct Inner {
    m: Arc<DeviceManager>,
    t: RecTiming,
    sessions: Mutex<HashMap<String, Slot>>,
    files: Mutex<Vec<(String, std::path::PathBuf)>>,
    last_ts: Mutex<u64>,
}

#[derive(Clone)]
pub struct Recorder(Arc<Inner>);

fn fail(code: &str, detail: &str) -> Value {
    static WS: crate::run::android::jsre::Js = crate::run::android::jsre::Js::new(r"\s+", "g");
    let d = js::slice(WS.replace_all(detail, " ").trim(), 300);
    err_detail(code, &d)
}

impl Recorder {
    pub fn new(m: Arc<DeviceManager>, t: RecTiming) -> Self {
        Recorder(Arc::new(Inner { m, t, sessions: Mutex::new(HashMap::new()), files: Mutex::new(Vec::new()), last_ts: Mutex::new(0) }))
    }

    fn publish(&self) {
        let held: Vec<String> = self.0.files.lock().unwrap().iter().map(|(_, p)| crate::util::path_str(p)).collect();
        *HELD.lock().unwrap() = held.clone();
        self.0.m.notify("ccm:dev-rec-held", json!(held));
    }

    fn host_str(&self, method: &str, params: Value) -> Option<String> {
        self.0.m.adb.host.call(method, params).ok().and_then(|v| v.as_str().map(String::from))
    }

    pub fn start(&self, id: &str) -> Value {
        let inner = &self.0;
        {
            let mut s = inner.sessions.lock().unwrap();
            if s.contains_key(id) {
                return err("busy");
            }
            s.insert(id.to_string(), Slot::Starting);
        }
        let ts = {
            let mut last = inner.last_ts.lock().unwrap();
            *last = now_ms().max(*last + 1);
            *last
        };
        let r = self.start_inner(id, ts);
        if !is_ok(&r) {
            let mut s = inner.sessions.lock().unwrap();
            if matches!(s.get(id), Some(Slot::Starting)) {
                s.remove(id);
            }
        }
        r
    }

    fn start_inner(&self, id: &str, ts: u64) -> Value {
        let inner = &self.0;
        let m = &inner.m;
        let ready = m.ready_check(id);
        if !is_ok(&ready) {
            return ready;
        }
        let Some(mut child) = m.adb.spawn_shell(id, &record_args(ts)) else { return err("adb-missing") };
        let pid = child.id();
        let out = Arc::new(Mutex::new(String::new()));
        for pipe in [child.stdout.take().map(|p| Box::new(p) as Box<dyn Read + Send>), child.stderr.take().map(|p| Box::new(p) as Box<dyn Read + Send>)].into_iter().flatten() {
            let out = out.clone();
            std::thread::spawn(move || {
                let mut pipe = pipe;
                let mut buf = [0u8; 4096];
                while let Ok(n) = pipe.read(&mut buf) {
                    if n == 0 {
                        break;
                    }
                    let mut o = out.lock().unwrap();
                    o.push_str(&String::from_utf8_lossy(&buf[..n]));
                    let chars: Vec<char> = o.chars().collect();
                    if chars.len() > 2048 {
                        *o = chars[chars.len() - 2048..].iter().collect();
                    }
                }
            });
        }
        let exited = Arc::new((Mutex::new(None::<Option<i32>>), Condvar::new()));
        let ex = exited.clone();
        std::thread::spawn(move || {
            let code = Child::wait(&mut child).ok().and_then(|s| s.code());
            *ex.0.lock().unwrap() = Some(code);
            ex.1.notify_all();
        });
        let s = Arc::new(Session { ts, pid, exited, out: out.clone(), started_at: now_ms(), frames: Mutex::new(Vec::new()), frames_failed: Mutex::new(0), frame_ms: Mutex::new(inner.t.frame_ms), gate: Gate::default(), stopping: Mutex::new(None) });
        if s.wait_exit(inner.t.start_check_ms) {
            m.adb.shell(id, &remove_args(ts), Some(inner.t.cmd_ms));
            let text = out.lock().unwrap().clone();
            return fail("record-failed", &text);
        }
        inner.sessions.lock().unwrap().insert(id.to_string(), Slot::Live(s.clone()));
        let (me, sid, sess) = (self.clone(), id.to_string(), s.clone());
        std::thread::spawn(move || {
            if !sess.gate.wait(me.0.t.orphan_ms) {
                me.cancel(&sid);
            }
        });
        let (me, sid, sess) = (self.clone(), id.to_string(), s.clone());
        std::thread::spawn(move || me.frames_loop(&sid, &sess));
        json!({"ok": true, "startedAt": s.started_at, "limitMs": REC_LIMIT_S * 1000})
    }

    fn frames_loop(&self, id: &str, s: &Session) {
        while !s.gate.is_halted() {
            let shot = self.0.m.adb.screencap(id);
            if s.gate.is_halted() {
                return;
            }
            if is_ok(&shot) {
                let mut frames = s.frames.lock().unwrap();
                frames.push(shot["png"].as_str().unwrap_or("").to_string());
                if let Some(next) = thin_frames(&frames, FRAMES_KEEP) {
                    *frames = next;
                    *s.frame_ms.lock().unwrap() *= 2;
                }
            } else {
                *s.frames_failed.lock().unwrap() += 1;
            }
            let ms = *s.frame_ms.lock().unwrap();
            if s.gate.wait(ms) {
                return;
            }
        }
    }

    fn live(&self, id: &str) -> Option<Arc<Session>> {
        match self.0.sessions.lock().unwrap().get(id) {
            Some(Slot::Live(s)) => Some(s.clone()),
            _ => None,
        }
    }

    fn finish_process(&self, id: &str, s: &Session) {
        let t = self.0.t;
        if s.alive() {
            self.0.m.adb.shell(id, &stop_args(s.ts), Some(t.cmd_ms));
        }
        if !s.wait_exit(t.stop_wait_ms) {
            kill_pid_only(s.pid);
        }
    }

    pub fn stop(&self, id: &str) -> Value {
        let Some(s) = self.live(id) else { return err("invalid") };
        let mut slot = s.stopping.lock().unwrap();
        if let Some(v) = slot.as_ref() {
            return v.clone();
        }
        let r = self.finish(id, &s);
        *slot = Some(r.clone());
        drop(slot);
        self.0.sessions.lock().unwrap().remove(id);
        r
    }

    fn finish(&self, id: &str, s: &Session) -> Value {
        let (inner, m) = (&self.0, &self.0.m);
        s.gate.halt();
        let duration = (now_ms() - s.started_at).min(REC_LIMIT_S * 1000);
        let dir = self.host_str("devices.recDir", Value::Null).map(std::path::PathBuf::from).unwrap_or_else(|| std::env::temp_dir().join("orrery-rec"));
        let local = dir.join(format!("{}.mp4", remote_name(s.ts)));
        let result = (|| {
            self.finish_process(id, s);
            if let Err(e) = std::fs::create_dir_all(&dir) {
                return fail("record-failed", &e.to_string());
            }
            let Some(pulled) = m.adb.pull(id, &remote_path(s.ts), &crate::util::path_str(&local), inner.t.pull_ms) else { return err("adb-missing") };
            if pulled.timed_out {
                return err("timeout");
            }
            if pulled.code != Some(0) || !file_complete(&local) {
                let _ = std::fs::remove_file(&local);
                return fail("record-failed", &format!("{} {}", pulled.out, s.out.lock().unwrap()));
            }
            let bytes = std::fs::metadata(&local).map(|x| x.len()).unwrap_or(0);
            let mut raw = [0u8; 16];
            let _ = getrandom::fill(&mut raw);
            let token: String = raw.iter().map(|b| format!("{b:02x}")).collect();
            inner.files.lock().unwrap().push((token.clone(), local.clone()));
            loop {
                let oldest = {
                    let f = inner.files.lock().unwrap();
                    (f.len() > TOKENS_MAX).then(|| f[0].0.clone())
                };
                match oldest {
                    Some(t) => self.release(&t),
                    None => break,
                }
            }
            self.publish();
            let frames = pick_even(&s.frames.lock().unwrap(), FRAMES_PICK);
            json!({"ok": true, "token": token, "bytes": bytes, "durationMs": duration, "frames": frames, "framesFailed": *s.frames_failed.lock().unwrap()})
        })();
        m.adb.shell(id, &remove_args(s.ts), Some(inner.t.cmd_ms));
        result
    }

    pub fn cancel(&self, id: &str) -> Value {
        let Some(s) = self.live(id) else { return err("invalid") };
        let mut slot = s.stopping.lock().unwrap();
        if let Some(v) = slot.as_ref() {
            if v.get("ok") == Some(&Value::Bool(true)) {
                if let Some(t) = v["token"].as_str() {
                    self.discard(t);
                }
            }
            return ok();
        }
        s.gate.halt();
        *slot = Some(err("cancelled"));
        self.finish_process(id, &s);
        self.0.m.adb.shell(id, &remove_args(s.ts), Some(self.0.t.cmd_ms));
        self.0.sessions.lock().unwrap().remove(id);
        ok()
    }

    pub fn has_file(&self, token: &str) -> bool {
        self.0.files.lock().unwrap().iter().any(|(t, _)| t == token)
    }

    pub fn save(&self, token: &str, dest: Option<&str>) -> Value {
        let file = self.0.files.lock().unwrap().iter().find(|(t, _)| t == token).map(|(_, p)| p.clone());
        let Some(file) = file else { return err("invalid") };
        let Some(dest) = dest else { return err("cancelled") };
        match std::fs::copy(&file, dest) {
            Ok(_) => json!({"ok": true, "name": std::path::Path::new(dest).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()}),
            Err(e) => fail("record-failed", &e.to_string()),
        }
    }

    pub fn discard(&self, token: &str) -> bool {
        let file = {
            let mut f = self.0.files.lock().unwrap();
            f.iter().position(|(t, _)| t == token).map(|i| f.remove(i).1)
        };
        self.publish();
        match file {
            Some(p) => {
                let _ = std::fs::remove_file(p);
                true
            }
            None => false,
        }
    }

    fn release(&self, token: &str) {
        let file = self.0.files.lock().unwrap().iter().find(|(t, _)| t == token).map(|(_, p)| p.clone());
        let keeps = file.is_some_and(|p| matches!(self.0.m.adb.host.call("devices.recKeeps", json!({"file": crate::util::path_str(&p)})), Ok(Value::Bool(true))));
        if keeps {
            self.0.files.lock().unwrap().retain(|(t, _)| t != token);
            self.publish();
        } else {
            self.discard(token);
        }
    }

    pub fn dispose(&self) {
        let sessions: Vec<(String, Arc<Session>)> = self.0.sessions.lock().unwrap().drain().filter_map(|(id, s)| if let Slot::Live(s) = s { Some((id, s)) } else { None }).collect();
        for (id, s) in sessions {
            s.gate.halt();
            if s.alive() {
                kill_pid_only(s.pid);
            }
            let (m, t) = (self.0.m.clone(), self.0.t);
            std::thread::spawn(move || {
                m.adb.shell(&id, &stop_args(s.ts), Some(t.cmd_ms));
                m.adb.shell(&id, &remove_args(s.ts), Some(t.cmd_ms));
            });
        }
        let tokens: Vec<String> = self.0.files.lock().unwrap().iter().map(|(t, _)| t.clone()).collect();
        for t in tokens {
            self.release(&t);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thins_and_picks_evenly() {
        let v: Vec<u32> = (0..17).collect();
        assert_eq!(thin_frames(&v, 16).unwrap().len(), 9);
        assert!(thin_frames(&v[..16], 16).is_none());
        assert_eq!(pick_even(&v, 6), vec![0, 3, 6, 10, 13, 16]);
        assert_eq!(pick_even(&v[..4], 6), vec![0, 1, 2, 3]);
    }

    #[test]
    fn mp4_needs_ftyp_and_moov() {
        let mut data = Vec::new();
        data.extend_from_slice(&[0, 0, 0, 8]);
        data.extend_from_slice(b"ftyp");
        data.extend_from_slice(&[0, 0, 0, 8]);
        data.extend_from_slice(b"moov");
        let size = data.len() as u64;
        let mut rd = |p: u64, l: usize| data[p as usize..(p as usize + l).min(data.len())].to_vec();
        assert!(mp4_complete(&mut rd, size));
        assert!(!mp4_complete(&mut rd, size + 1));
    }
}
