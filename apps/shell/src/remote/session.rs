use std::sync::Arc;
use std::time::Duration;

use core_remote_proto::session::accept;
use core_remote_proto::Error;
use tokio::sync::mpsc;
use tokio::time::{timeout_at, Instant};

use super::config::now_secs;
use super::ctx::{Admit, Ctx, MAX_PAIR_TRIES, MAX_RELAY_PAIR_TRIES};
use super::dashboard::Phone;
use super::guard_hooks::link_ip;
use super::guard_ip::key;
use super::limits::Pending;
use super::link::{Link, Out, Peer, Secure, CLOSE_DEVICE, CLOSE_HANDSHAKE, CLOSE_PROTO, CLOSE_RATE};
use super::pair;
use super::text::remote_text;
use crate::state::lock;

pub const HELLO_WAIT: Duration = Duration::from_secs(5);

pub fn try_keys(link: &Link) -> Vec<(String, usize)> {
    match (link.peer, link_ip(link)) {
        (Peer::Lan(ip), _) => vec![(format!("ip:{}", key(ip)), MAX_PAIR_TRIES)],
        (Peer::Relay(_), Some(ip)) => vec![(format!("relay:{}", key(ip)), MAX_PAIR_TRIES), ("relay".to_owned(), MAX_RELAY_PAIR_TRIES)],
        (Peer::Relay(_), None) => vec![("relay".to_owned(), MAX_RELAY_PAIR_TRIES)],
    }
}

pub async fn serve(ctx: Arc<Ctx>, mut link: Link, pending: Option<Pending>, deadline: Instant) {
    let Ok(Some(msg1)) = timeout_at(deadline, link.rx.recv()).await else {
        return ctx.refuse(&link, CLOSE_HANDSHAKE, "timeout", "timeout").await;
    };
    if link.origin.pairing && !pair::is_pair_msg1(&msg1) {
        return ctx.refuse(&link, CLOSE_HANDSHAKE, "pairing", "pairing").await;
    }
    if pair::is_pair_msg1(&msg1) {
        if !ctx.take_pair_try(&try_keys(&link), std::time::Instant::now()) {
            return ctx.refuse(&link, CLOSE_RATE, "pairing", "rate").await;
        }
        pair::run(ctx, link, msg1, deadline).await;
        drop(pending);
        return;
    }
    let now = now_secs();
    let accepted = {
        let devices = lock(&ctx.devices);
        accept(&ctx.keys.noise, &msg1, |id, key| devices.authorize(id, key, now))
    };
    let (session, msg2) = match accepted {
        Ok(v) => v,
        Err(Error::Unauthorized) => {
            ctx.notify_unknown(std::time::Instant::now());
            return ctx.refuse(&link, CLOSE_DEVICE, "device", "device").await;
        }
        Err(Error::ProtoTooOld { .. }) => return link.out.close(CLOSE_PROTO, "old").await,
        Err(Error::ProtoTooNew { .. }) => return link.out.close(CLOSE_PROTO, "new").await,
        Err(_) => return ctx.refuse(&link, CLOSE_HANDSHAKE, "handshake", "handshake").await,
    };
    if !link.out.send(Out::Msg(msg2)).await {
        return;
    }
    let device = session.device_id.clone();
    let key = session.device_static;
    let sec = Secure { transport: session.transport, out: link.out.clone() };
    let mut phone = Phone::new(ctx.clone(), sec, device.clone());
    let Ok(Some(first)) = timeout_at(Instant::now() + HELLO_WAIT, link.rx.recv()).await else {
        return ctx.refuse(&link, CLOSE_HANDSHAKE, "hello", "hello").await;
    };
    let Some((id, hello)) = phone.first_hello(&first) else {
        return ctx.refuse(&link, CLOSE_HANDSHAKE, "hello", "hello").await;
    };
    let via = match link.peer {
        Peer::Lan(_) => "lan",
        Peer::Relay(_) => "relay",
    };
    let (ctl_tx, ctl_rx) = mpsc::channel(4);
    if matches!(link.peer, Peer::Relay(_)) {
        let (every, dead) = *lock(&ctx.keepalive);
        phone.watch(every, dead);
    }
    let live = match ctx.admit_live(&device, &key, via, ctl_tx, now_secs()) {
        Ok(id) => id,
        Err(Admit::Revoked) => return link.out.close(CLOSE_DEVICE, "device").await,
        Err(Admit::Full) => return link.out.close(CLOSE_RATE, "sessions").await,
    };
    link.budget.open();
    let _ = link.out.send(Out::Good).await;
    ctx.board.mark_connect();
    drop(pending);
    let touched = lock(&ctx.devices).touch(&device, now_secs());
    if let Some((name, true)) = touched {
        let t = remote_text(&ctx.host.welcome().locale);
        ctx.host.notify(t.connected_title, &t.connected_body(&name));
    }
    let ip = ctx.session_opened(&link, &device);
    ctx.changed();
    phone.start(&id, &hello, link.rx, ctl_rx).await;
    ctx.session_closed(ip);
    ctx.drop_live(live);
    ctx.changed();
}
