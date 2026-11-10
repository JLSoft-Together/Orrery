use super::t_notes::{NOTE_RATE_MAX, NOTE_RATE_MS};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub type NoteSend = Arc<dyn Fn(&str, &str, u64) -> Value + Send + Sync>;
pub type Clock = Arc<dyn Fn() -> f64 + Send + Sync>;

fn rank(level: &str) -> u8 {
    match level {
        "attention" => 2,
        "done" => 1,
        _ => 0,
    }
}

#[derive(Default)]
struct St {
    sent: Vec<f64>,
    pending: Option<(String, String, u64)>,
    timer: bool,
    disposed: bool,
}

#[derive(Clone)]
pub struct NoteQueue {
    st: Arc<Mutex<St>>,
    send: NoteSend,
    now: Clock,
}

impl NoteQueue {
    pub fn new(send: NoteSend, now: Clock) -> Self {
        NoteQueue { st: Arc::new(Mutex::new(St::default())), send, now }
    }

    fn prune(st: &mut St, at: f64) {
        st.sent.retain(|t| at - t < NOTE_RATE_MS);
    }

    pub fn submit(&self, text: &str, level: &str) -> Value {
        let at = (self.now)();
        let mut st = self.st.lock().unwrap();
        Self::prune(&mut st, at);
        if st.sent.len() < NOTE_RATE_MAX && st.pending.is_none() {
            st.sent.push(at);
            drop(st);
            return (self.send)(text, level, 0);
        }
        let (keep, count) = match &st.pending {
            Some((_, l, c)) if rank(l) > rank(level) => (l.clone(), c + 1),
            Some((_, _, c)) => (level.to_string(), c + 1),
            None => (level.to_string(), 1),
        };
        st.pending = Some((text.to_string(), keep, count));
        let in_ms = (st.sent.first().copied().unwrap_or(at) + NOTE_RATE_MS - at).max(0.0);
        if !st.timer {
            st.timer = true;
            self.arm(in_ms);
        }
        json!({ "ok": true, "sent": false, "inMs": super::text::num(in_ms) })
    }

    fn arm(&self, ms: f64) {
        let me = self.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(ms.max(0.0) as u64));
            me.flush();
        });
    }

    pub fn flush(&self) {
        let at = (self.now)();
        let mut st = self.st.lock().unwrap();
        st.timer = false;
        if st.disposed {
            return;
        }
        let Some((text, level, count)) = st.pending.clone() else { return };
        Self::prune(&mut st, at);
        if st.sent.len() >= NOTE_RATE_MAX {
            st.timer = true;
            let wait = (st.sent[0] + NOTE_RATE_MS - at).max(0.0);
            drop(st);
            self.arm(wait);
            return;
        }
        st.pending = None;
        st.sent.push(at);
        drop(st);
        let _ = (self.send)(&text, &level, count.saturating_sub(1));
    }

    pub fn dispose(&self) {
        let mut st = self.st.lock().unwrap();
        st.disposed = true;
        st.pending = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folds_notes_over_the_rate_into_one() {
        let got: Arc<Mutex<Vec<(String, String, u64)>>> = Arc::default();
        let g = got.clone();
        let clock = Arc::new(Mutex::new(1_000_000.0));
        let c = clock.clone();
        let q = NoteQueue::new(Arc::new(move |t: &str, l: &str, m: u64| {
            g.lock().unwrap().push((t.into(), l.into(), m));
            json!({ "ok": true, "sent": true, "name": "x", "inbox": true })
        }), Arc::new(move || *c.lock().unwrap()));
        for i in 0..6 {
            assert_eq!(q.submit(&format!("n{i}"), "info")["sent"], json!(true));
        }
        let held = q.submit("a", "attention");
        assert_eq!(held["sent"], json!(false));
        assert_eq!(held["inMs"], json!(60000));
        q.submit("b", "info");
        *clock.lock().unwrap() += 60_001.0;
        q.flush();
        let last = got.lock().unwrap().last().cloned().unwrap();
        assert_eq!(last, ("b".into(), "attention".into(), 1));
        q.dispose();
    }
}
