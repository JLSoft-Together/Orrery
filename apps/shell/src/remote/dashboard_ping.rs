use std::time::{Duration, Instant};

use core_remote_proto::frame::Kind;

use super::dashboard::Phone;
use super::link::CLOSE_HANDSHAKE;

pub const PING_LOST: Duration = Duration::from_secs(10);
pub const MAX_PINGS_OUT: usize = 3;

impl Phone {
    fn forget_lost(&mut self, now: Instant) {
        while self.pings.front().is_some_and(|t| now.duration_since(*t) >= PING_LOST) {
            self.pings.pop_front();
        }
    }

    async fn ping(&mut self, now: Instant) -> bool {
        if self.pings.len() >= MAX_PINGS_OUT {
            self.pings.pop_front();
        }
        self.pings.push_back(now);
        if let Some(a) = self.alive.as_mut() {
            a.sent();
        }
        self.sec.frame(Kind::Ping, 0, &[]).await
    }

    pub(super) async fn keepalive(&mut self) -> bool {
        let Some(a) = self.alive.as_ref() else { return true };
        if a.expired() {
            self.sec.close(CLOSE_HANDSHAKE, "idle").await;
            return false;
        }
        let now = Instant::now();
        self.forget_lost(now);
        self.ping(now).await
    }

    pub(super) async fn probe(&mut self) -> bool {
        let now = Instant::now();
        self.forget_lost(now);
        if !self.pings.is_empty() {
            return true;
        }
        self.ping(now).await
    }

    pub(super) async fn pong(&mut self) -> bool {
        if let Some(a) = self.alive.as_mut() {
            a.answer_is_echo();
        }
        match self.pings.pop_front() {
            Some(sent) => {
                self.ctx.probes.record(&self.device, self.via, sent.elapsed());
                true
            }
            None => self.sec.frame(Kind::Ping, 0, &[]).await,
        }
    }
}
