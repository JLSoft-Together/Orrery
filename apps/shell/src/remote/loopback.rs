use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;

use super::config::now_secs;
use super::ctx::Ctx;
use super::{lan, pair};

pub const TICK: Duration = Duration::from_secs(1);

struct Open {
    addr: SocketAddr,
    stop: watch::Sender<bool>,
}

impl Drop for Open {
    fn drop(&mut self) {
        let _ = self.stop.send(true);
    }
}

#[derive(Default)]
struct Listener {
    open: Option<Open>,
    warned: Option<SocketAddr>,
}

pub fn target(ctx: &Ctx, live: bool) -> Option<SocketAddr> {
    if !live {
        return None;
    }
    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, pair::qr_port(ctx)));
    (addr.port() != 0 && !ctx.lan().contains(&addr)).then_some(addr)
}

impl Listener {
    fn addr(&self) -> Option<SocketAddr> {
        self.open.as_ref().map(|o| o.addr)
    }

    fn sync(&mut self, ctx: &Arc<Ctx>, want: Option<SocketAddr>) {
        if self.addr() == want {
            return;
        }
        self.open = None;
        let Some(addr) = want else {
            self.warned = None;
            return;
        };
        let Some(listener) = lan::bind_one(addr) else {
            if self.warned != Some(addr) {
                eprintln!("[remote] usb loopback {addr} is busy, retrying while the usb pairing code is valid");
                self.warned = Some(addr);
            }
            return;
        };
        let (stop, rx) = watch::channel(false);
        tokio::spawn(lan::accept_with(ctx.clone(), listener, rx, true));
        self.open = Some(Open { addr, stop });
        self.warned = None;
    }
}

pub async fn supervise(ctx: Arc<Ctx>, mut stop: watch::Receiver<bool>) {
    let mut usb = ctx.pairing.usb_watch();
    let mut listener = Listener::default();
    loop {
        let live = ctx.pairing.usb_until(now_secs()).is_some();
        listener.sync(&ctx, target(&ctx, live));
        tokio::select! {
            changed = usb.changed() => if changed.is_err() { break },
            _ = tokio::time::sleep(TICK), if live => {}
            _ = stop.changed() => break,
        }
    }
    listener.sync(&ctx, None);
}

#[cfg(test)]
#[path = "loopback_tests.rs"]
mod tests;
