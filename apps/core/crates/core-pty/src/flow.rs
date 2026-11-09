use std::sync::{Arc, Condvar, Mutex, MutexGuard};

pub const PAUSE_AT: usize = 512 * 1024;
pub const RESUME_AT: usize = 128 * 1024;
pub const INFLIGHT_MAX: usize = 256 * 1024;
pub const CONTROL_INFLIGHT_MAX: usize = 4 * 1024 * 1024;
pub const CONN_MAX: usize = 8 * 1024 * 1024;
pub const INBOUND_MAX: usize = 2 * 1024 * 1024;

#[derive(Default)]
struct GateState {
    inflight: usize,
    paused: bool,
    draining: bool,
    closed: bool,
}

pub struct Gate {
    max: usize,
    m: Mutex<GateState>,
    cv: Condvar,
}

pub struct Credit {
    gate: Arc<Gate>,
    n: usize,
}

impl Drop for Credit {
    fn drop(&mut self) {
        self.gate.release(self.n);
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

impl Gate {
    pub fn new() -> Arc<Gate> {
        Gate::with_max(INFLIGHT_MAX)
    }

    pub fn with_max(max: usize) -> Arc<Gate> {
        Arc::new(Gate { max: max.max(1), m: Mutex::new(GateState::default()), cv: Condvar::new() })
    }

    pub fn acquire(self: &Arc<Self>, n: usize) -> Credit {
        let mut s = lock(&self.m);
        loop {
            let blocked_by_socket = s.paused && !s.draining;
            let blocked_by_queue = s.inflight > 0 && s.inflight + n > self.max;
            if s.closed || (!blocked_by_socket && !blocked_by_queue) {
                break;
            }
            s = self.cv.wait(s).unwrap_or_else(|e| e.into_inner());
        }
        s.inflight += n;
        Credit { gate: self.clone(), n }
    }

    fn release(&self, n: usize) {
        let mut s = lock(&self.m);
        s.inflight = s.inflight.saturating_sub(n);
        self.cv.notify_all();
    }

    pub fn set_paused(&self, paused: bool) {
        let mut s = lock(&self.m);
        if s.paused != paused {
            s.paused = paused;
            if !paused {
                self.cv.notify_all();
            }
        }
    }

    pub fn drain(&self) {
        lock(&self.m).draining = true;
        self.cv.notify_all();
    }

    pub fn close(&self) {
        lock(&self.m).closed = true;
        self.cv.notify_all();
    }

    pub fn paused(&self) -> bool {
        lock(&self.m).paused
    }

    pub fn inflight(&self) -> usize {
        lock(&self.m).inflight
    }
}

#[derive(Default)]
struct MeterState {
    unsent: usize,
    gate: Option<Arc<Gate>>,
}

#[derive(Default)]
pub struct Meter {
    m: Mutex<MeterState>,
}

impl Meter {
    pub fn new() -> Arc<Meter> {
        Arc::new(Meter::default())
    }

    pub fn queued(&self, n: usize, droppable: bool) -> bool {
        let mut s = lock(&self.m);
        if droppable && s.unsent + n > CONN_MAX {
            return false;
        }
        s.unsent += n;
        if s.unsent > PAUSE_AT {
            if let Some(g) = &s.gate {
                g.set_paused(true);
            }
        }
        true
    }

    pub fn sent(&self, n: usize) {
        let mut s = lock(&self.m);
        s.unsent = s.unsent.saturating_sub(n);
        if s.unsent < RESUME_AT {
            if let Some(g) = &s.gate {
                g.set_paused(false);
            }
        }
    }

    pub fn bind(&self, gate: Option<Arc<Gate>>) {
        let mut s = lock(&self.m);
        if let Some(old) = s.gate.take() {
            old.set_paused(false);
        }
        if let Some(g) = &gate {
            g.set_paused(s.unsent > PAUSE_AT);
        }
        s.gate = gate;
    }

    pub fn bound_to(&self, gate: &Arc<Gate>) -> bool {
        lock(&self.m).gate.as_ref().is_some_and(|g| Arc::ptr_eq(g, gate))
    }

    pub fn unsent(&self) -> usize {
        lock(&self.m).unsent
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    fn blocks(gate: &Arc<Gate>, n: usize) -> mpsc::Receiver<Credit> {
        let (tx, rx) = mpsc::channel();
        let g = gate.clone();
        std::thread::spawn(move || {
            let _ = tx.send(g.acquire(n));
        });
        rx
    }

    #[test]
    fn inflight_is_bounded_and_released_on_drop() {
        let g = Gate::with_max(100);
        let a = g.acquire(60);
        let b = g.acquire(40);
        assert_eq!(g.inflight(), 100);
        let waiting = blocks(&g, 10);
        assert!(waiting.recv_timeout(Duration::from_millis(100)).is_err());
        drop(a);
        let c = waiting.recv_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(g.inflight(), 50);
        drop(b);
        drop(c);
        assert_eq!(g.inflight(), 0);
    }

    #[test]
    fn a_chunk_bigger_than_max_passes_when_nothing_is_in_flight() {
        let g = Gate::with_max(10);
        let big = g.acquire(50);
        assert_eq!(g.inflight(), 50);
        drop(big);
    }

    #[test]
    fn meter_pauses_over_512k_and_resumes_under_128k() {
        let g = Gate::new();
        let m = Meter::new();
        m.bind(Some(g.clone()));
        assert!(m.queued(PAUSE_AT, true));
        assert!(!g.paused());
        assert!(m.queued(1, true));
        assert!(g.paused());
        let waiting = blocks(&g, 1);
        assert!(waiting.recv_timeout(Duration::from_millis(100)).is_err());
        m.sent(PAUSE_AT - RESUME_AT);
        assert!(g.paused(), "hysteresis: still over 128 KB");
        assert!(waiting.recv_timeout(Duration::from_millis(50)).is_err());
        m.sent(2);
        assert!(!g.paused());
        drop(waiting.recv_timeout(Duration::from_secs(2)).unwrap());
    }

    #[test]
    fn unbinding_a_paused_socket_resumes_and_rebinding_follows_the_new_backlog() {
        let g = Gate::new();
        let old = Meter::new();
        old.bind(Some(g.clone()));
        old.queued(PAUSE_AT + 1, true);
        assert!(g.paused());
        let fresh = Meter::new();
        old.bind(None);
        fresh.bind(Some(g.clone()));
        assert!(!g.paused());
        old.sent(PAUSE_AT + 1);
        fresh.queued(PAUSE_AT + 10, true);
        assert!(g.paused());
        assert!(fresh.bound_to(&g));
        assert!(!old.bound_to(&g));
    }

    #[test]
    fn draining_ignores_the_socket_but_not_the_queue_and_close_ignores_both() {
        let g = Gate::with_max(10);
        g.set_paused(true);
        let w = blocks(&g, 5);
        assert!(w.recv_timeout(Duration::from_millis(100)).is_err());
        g.drain();
        let first = w.recv_timeout(Duration::from_secs(2)).unwrap();
        let w2 = blocks(&g, 8);
        assert!(w2.recv_timeout(Duration::from_millis(100)).is_err());
        g.close();
        let second = w2.recv_timeout(Duration::from_secs(2)).unwrap();
        drop((first, second));
    }

    #[test]
    fn droppable_output_stops_at_conn_max_texts_still_pass() {
        let m = Meter::new();
        assert!(m.queued(CONN_MAX, true));
        assert!(!m.queued(1, true));
        assert!(m.queued(10, false));
        assert_eq!(m.unsent(), CONN_MAX + 10);
    }
}
