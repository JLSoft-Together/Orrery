use std::time::Duration;

use tokio::time::{interval_at, Instant, Interval, MissedTickBehavior};

pub const PING_EVERY: Duration = Duration::from_secs(15);
pub const DEAD_AFTER: Duration = Duration::from_secs(45);

pub struct Liveness {
    dead: Duration,
    heard: Instant,
    out: bool,
    tick: Interval,
}

impl Liveness {
    pub fn new(every: Duration, dead: Duration) -> Liveness {
        let mut tick = interval_at(Instant::now() + every, every);
        tick.set_missed_tick_behavior(MissedTickBehavior::Delay);
        Liveness { dead, heard: Instant::now(), out: false, tick }
    }

    pub fn heard(&mut self) {
        self.heard = Instant::now();
    }

    pub fn expired(&self) -> bool {
        self.heard.elapsed() >= self.dead
    }

    pub fn sent(&mut self) {
        self.out = true;
    }

    pub fn answer_is_echo(&mut self) -> bool {
        let echo = !self.out;
        self.out = false;
        echo
    }

    pub async fn due(&mut self) {
        self.tick.tick().await;
    }
}

pub async fn due(alive: &mut Option<Liveness>) {
    match alive {
        Some(a) => a.due().await,
        None => std::future::pending().await,
    }
}
