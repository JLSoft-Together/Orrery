use std::net::IpAddr;
use std::sync::Arc;

use serde_json::{json, Value};
use tokio::net::TcpStream;
use tokio::time::Instant;

use super::config::now_secs;
use super::ctx::Ctx;
use super::guard_ip::norm;
use super::install::Reply;
use super::install_http::serve_with;
use super::install_page::pair_url;
use super::pair::PairMeta;

pub const INSTALL_FETCHED: &str = "ccm:remote-install-fetched";

impl Ctx {
    pub fn install_reply(&self, reply: Reply, ip: IpAddr) -> Reply {
        match reply {
            Reply::Pair { token } => self.install_pair(&token, ip),
            Reply::File { first: true, .. } => {
                self.host.event(INSTALL_FETCHED, json!({ "ip": norm(ip).to_string() }));
                self.changed();
                reply
            }
            other => other,
        }
    }

    fn install_pair(&self, token: &str, ip: IpAddr) -> Reply {
        if let Err(why) = self.install.claim_pair(token, ip, now_secs()) {
            self.record_lan(ip, "fail", why.code());
            return Reply::Forbidden;
        }
        if self.pairing.holds_non_lan(now_secs()) {
            self.install.release_pair(token);
            self.record_lan(ip, "fail", "install_busy");
            return Reply::Forbidden;
        }
        let started = self.start_pair(PairMeta::lan(ip));
        let Some(qr) = started.as_ref().ok().and_then(|v| v.get("qr")).and_then(Value::as_str) else {
            self.record_lan(ip, "fail", "install_pair");
            return Reply::Forbidden;
        };
        self.record_lan(ip, "ok", "install_pair");
        Reply::Redirect(pair_url(qr))
    }
}

pub async fn serve(ctx: Arc<Ctx>, stream: TcpStream, ip: IpAddr, deadline: Instant) {
    let hook = ctx.clone();
    serve_with(&ctx.install, stream, ip, deadline, move |r| hook.install_reply(r, ip)).await;
}
