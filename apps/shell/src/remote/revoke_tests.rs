use std::time::{Duration, Instant};

use core_remote_proto::frame::{Frame, Kind};
use futures_util::SinkExt;
use serde_json::{json, Value};
use tokio_tungstenite::tungstenite::Message;

use super::check::Probes;
use super::config::now_secs;
use super::devices::{Device, Devices};
use super::link::CLOSE_OFF;
use super::revoke::Hold;
use super::test_util::{close_code, hello, recv_frame, recv_json, rig, runtime, session, Phone, Rig};
use crate::state::lock;

fn add(r: &Rig, phone: &Phone) {
    let ctx = r.remote.ctx().unwrap();
    assert!(lock(&ctx.devices).add(Device::new(&phone.id, "Phone", phone.key.public(), now_secs())));
}

fn device(r: &Rig, id: &str) -> Option<Value> {
    r.remote.status()["devices"].as_array().unwrap().iter().find(|d| d["deviceId"] == id).cloned()
}

#[test]
fn a_hold_can_be_undone_only_before_its_deadline() {
    let h = Hold::default();
    let t0 = Instant::now();
    let s1 = h.hold("a", Duration::from_secs(5), t0);
    assert!(h.is_held("a") && !h.is_held("b"));
    assert!(h.undo("a", t0 + Duration::from_secs(4)));
    assert!(!h.is_held("a"));
    let s2 = h.hold("a", Duration::from_secs(5), t0);
    assert!(!h.undo("a", t0 + Duration::from_secs(5)));
    assert!(!h.due("a", s1));
    assert!(h.due("a", s2));
    assert!(!h.due("a", s2));
    h.hold("x", Duration::from_secs(5), t0);
    h.hold("y", Duration::from_secs(5), t0);
    let mut drained = h.drain();
    drained.sort();
    assert_eq!(drained, ["x", "y"]);
    assert!(h.ids().is_empty());
}

#[test]
fn a_held_device_is_cut_off_at_once_and_comes_back_on_undo() {
    let r = rig("revoke-hold", "", false);
    r.remote.enable(true);
    let phone = Phone::new("p-hold");
    add(&r, &phone);
    let (addr, desktop) = (r.addr(), r.desktop_key());
    let rt = runtime();
    let (mut ws, mut t) = rt.block_on(session(addr, &desktop, &phone)).unwrap();
    assert_eq!(rt.block_on(hello(&mut ws, &mut t))["ok"], true);

    assert_eq!(r.remote.call("ccm:remote-revoke", &[json!({ "id": "p-hold", "grace": 2 })]).unwrap(), json!(true));
    let ended = rt.block_on(async {
        let mut events = Vec::new();
        loop {
            match recv_json(&mut ws, &mut t).await {
                Ok(v) => events.push(v),
                Err(code) => return (events, code),
            }
        }
    });
    assert_eq!(ended.1, Some(CLOSE_OFF));
    assert!(!ended.0.iter().any(|v| v["t"] == "device.revoked" || v["t"] == "remote.off"), "{:?}", ended.0);
    assert_eq!(device(&r, "p-hold").unwrap()["revoking"], true);
    let again = rt.block_on(async {
        let (mut ws, _) = session(addr, &desktop, &phone).await?;
        Err::<(), _>(close_code(&mut ws).await)
    });
    assert_eq!(again.err(), Some(Some(CLOSE_OFF)));
    let whys = r.remote.guard_call(&json!({}))["state"]["log"].clone();
    assert!(whys.as_array().unwrap().iter().any(|e| e["why"] == "held"), "{whys}");

    assert_eq!(r.remote.call("ccm:remote-revoke-undo", &[json!({ "id": "p-hold" })]).unwrap(), json!(true));
    assert!(device(&r, "p-hold").unwrap().get("revoking").is_none());
    let (mut ws, mut t) = rt.block_on(session(addr, &desktop, &phone)).unwrap();
    assert_eq!(rt.block_on(hello(&mut ws, &mut t))["ok"], true);
    std::thread::sleep(Duration::from_millis(2300));
    assert!(device(&r, "p-hold").is_some(), "undo cancels the pending revoke");
    assert_eq!(r.remote.call("ccm:remote-revoke-undo", &[json!("p-hold")]).unwrap(), json!(false));
    r.remote.enable(false);
}

#[test]
fn the_revoke_lands_after_the_grace_and_undo_is_then_refused() {
    let r = rig("revoke-late", "", false);
    r.remote.enable(true);
    let phone = Phone::new("p-late");
    add(&r, &phone);
    assert_eq!(r.remote.call("ccm:remote-revoke", &[json!({ "id": "p-late", "grace": 1 })]).unwrap(), json!(true));
    assert!(device(&r, "p-late").is_some());
    std::thread::sleep(Duration::from_millis(1500));
    assert!(device(&r, "p-late").is_none());
    assert_eq!(r.remote.call("ccm:remote-revoke-undo", &[json!("p-late")]).unwrap(), json!(false));
    assert_eq!(r.remote.call("ccm:remote-revoke", &[json!({ "id": "nobody", "grace": 5 })]).unwrap(), json!(false));
    assert_eq!(r.remote.call("ccm:remote-revoke", &[json!({ "grace": 5 })]).unwrap(), json!(false));
    r.remote.enable(false);
}

#[test]
fn shutting_down_inside_the_grace_still_revokes() {
    let r = rig("revoke-exit", "", false);
    r.remote.enable(true);
    let phone = Phone::new("p-exit");
    add(&r, &phone);
    let other = Phone::new("p-stay");
    add(&r, &other);
    assert_eq!(r.remote.call("ccm:remote-revoke", &[json!({ "id": "p-exit", "grace": 30 })]).unwrap(), json!(true));
    r.remote.shutdown();
    let saved = Devices::load(&r.dir);
    assert!(saved.find("p-exit").is_none());
    assert!(saved.find("p-stay").is_some());
}

#[test]
fn a_plain_revoke_still_removes_at_once() {
    let r = rig("revoke-now", "", false);
    let phone = Phone::new("p-now");
    add(&r, &phone);
    assert_eq!(r.remote.call("ccm:remote-revoke", &[json!("p-now")]).unwrap(), json!(true));
    assert!(Devices::load(&r.dir).find("p-now").is_none());
}

#[test]
fn probes_keep_the_latest_rtt_and_rate_limit_push() {
    let p = Probes::default();
    let start = Instant::now();
    assert_eq!(p.since("a", "lan", start), None);
    p.record("a", "lan", Duration::from_millis(42));
    assert_eq!(p.since("a", "lan", start), Some(42));
    assert_eq!(p.since("a", "relay", start), None);
    assert_eq!(p.since("a", "lan", Instant::now() + Duration::from_secs(1)), None);
    let now = Instant::now();
    assert!(p.take_push("a", now));
    assert!(!p.take_push("a", now + Duration::from_secs(29)));
    assert!(p.take_push("b", now));
    assert!(p.take_push("a", now + Duration::from_secs(30)));
}

#[test]
fn remote_check_measures_the_lan_rtt_with_a_ping_and_rate_limits_push() {
    let r = rig("check-rtt", "", false);
    r.remote.enable(true);
    let phone = Phone::new("p-chk");
    add(&r, &phone);
    let (addr, desktop) = (r.addr(), r.desktop_key());
    let rt = runtime();
    let (mut ws, mut t) = rt.block_on(session(addr, &desktop, &phone)).unwrap();
    assert_eq!(rt.block_on(hello(&mut ws, &mut t))["ok"], true);
    let echo = rt.spawn(async move {
        let mut pings = 0;
        while let Ok(f) = recv_frame(&mut ws, &mut t).await {
            if f.kind == Kind::Ping {
                pings += 1;
                let sealed = t.seal(&Frame::new(Kind::Ping, 0, Vec::new()).encode().unwrap()).unwrap();
                if ws.send(Message::Binary(sealed.into())).await.is_err() {
                    break;
                }
            }
        }
        pings
    });
    let out = r.remote.call("ccm:remote-check", &[json!("p-chk")]).unwrap();
    assert_eq!(out["lan"]["ok"], true, "{out}");
    assert!(out["lan"]["ms"].as_u64().is_some_and(|ms| ms < 3000), "{out}");
    assert_eq!(out["relay"], json!({ "state": "off" }));
    assert_eq!(out["push"], json!({ "ok": false, "error": "none" }));
    let again = r.remote.call("ccm:remote-check", &[json!({ "deviceId": "p-chk" })]).unwrap();
    assert_eq!(again["lan"]["ok"], true, "{again}");
    assert_eq!(again["push"], json!({ "ok": false, "error": "rate" }));
    let idle = r.remote.call("ccm:remote-check", &[json!("p-none")]).unwrap();
    assert_eq!(idle["lan"], json!({ "ok": false }));
    assert_eq!(r.remote.call("ccm:remote-check", &[json!(5)]).unwrap(), Value::Null);
    r.remote.enable(false);
    assert_eq!(rt.block_on(echo).unwrap(), 2);
    let off = r.remote.call("ccm:remote-check", &[json!("p-chk")]).unwrap();
    assert_eq!((off["lan"].clone(), off["relay"].clone()), (json!({ "ok": false }), json!({ "state": "off" })));
}
