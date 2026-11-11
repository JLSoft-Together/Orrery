use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::watch;

use super::ctx::Ctx;
use super::{lan, mdns};

pub const REBIND_EVERY: Duration = Duration::from_secs(30);

#[derive(Clone, Copy)]
pub struct Plan {
    pub port: u16,
    pub dynamic: bool,
    pub mdns: bool,
}

type Loops = HashMap<SocketAddr, watch::Sender<bool>>;

fn spawn_loop(ctx: &Arc<Ctx>, listener: std::net::TcpListener, loops: &mut Loops) {
    let Ok(addr) = listener.local_addr() else { return };
    let (tx, rx) = watch::channel(false);
    tokio::spawn(lan::accept_loop(ctx.clone(), listener, rx));
    loops.insert(addr, tx);
}

fn ips(loops: &Loops) -> Vec<Ipv4Addr> {
    let mut out: Vec<Ipv4Addr> = loops
        .keys()
        .filter_map(|a| match a.ip() {
            IpAddr::V4(v4) => Some(v4),
            IpAddr::V6(_) => None,
        })
        .collect();
    out.sort();
    out
}

fn addrs(loops: &Loops) -> Vec<SocketAddr> {
    let mut out: Vec<SocketAddr> = loops.keys().copied().collect();
    out.sort();
    out
}

fn advertise(ctx: &Ctx, loops: &Loops, plan: Plan) -> Option<mdns::Mdns> {
    if !plan.mdns {
        return None;
    }
    let port = loops.keys().next().map_or(plan.port, SocketAddr::port);
    mdns::start(&ctx.name, &ips(loops), port, ctx.keys.noise.public())
}

pub fn changed_set(have: &[Ipv4Addr], want: &[Ipv4Addr]) -> (Vec<Ipv4Addr>, Vec<Ipv4Addr>) {
    let gone = have.iter().filter(|ip| !want.contains(ip)).copied().collect();
    let fresh = want.iter().filter(|ip| !have.contains(ip)).copied().collect();
    (gone, fresh)
}

pub async fn supervise(ctx: Arc<Ctx>, initial: Vec<std::net::TcpListener>, plan: Plan, mut stop: watch::Receiver<bool>) {
    let mut loops = Loops::new();
    for l in initial {
        spawn_loop(&ctx, l, &mut loops);
    }
    let mut advert = advertise(&ctx, &loops, plan);
    loop {
        tokio::select! {
            _ = tokio::time::sleep(REBIND_EVERY), if plan.dynamic => {}
            _ = stop.changed() => break,
        }
        let want = lan::lan_addrs();
        let (gone, fresh) = changed_set(&ips(&loops), &want);
        if gone.is_empty() && fresh.is_empty() {
            continue;
        }
        loops.retain(|addr, tx| match addr.ip() {
            IpAddr::V4(v4) if gone.contains(&v4) => {
                let _ = tx.send(true);
                false
            }
            _ => true,
        });
        for l in lan::bind(&fresh, plan.port) {
            spawn_loop(&ctx, l, &mut loops);
        }
        ctx.set_lan(addrs(&loops));
        drop(advert.take());
        advert = advertise(&ctx, &loops, plan);
        ctx.changed();
    }
    for tx in loops.values() {
        let _ = tx.send(true);
    }
    drop(advert);
}
