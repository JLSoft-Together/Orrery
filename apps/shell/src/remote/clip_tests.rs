use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::{ClipHub, ClipSink, MAX_CLIP};
use crate::remote::devices::Device;
use crate::remote::test_util::*;
use crate::state::lock;

#[derive(Default)]
struct FakeClip {
    text: Mutex<Option<String>>,
    fail: Mutex<bool>,
}

impl ClipSink for FakeClip {
    fn write(&self, text: &str) -> bool {
        if *lock(&self.fail) {
            return false;
        }
        *lock(&self.text) = Some(text.to_owned());
        true
    }

    fn read(&self) -> Option<String> {
        lock(&self.text).clone()
    }
}

fn hub() -> (ClipHub, Arc<FakeClip>) {
    let hub = ClipHub::default();
    let fake = Arc::new(FakeClip::default());
    hub.set_sink(fake.clone());
    (hub, fake)
}

#[test]
fn plain_text_is_written_and_empty_or_oversized_or_secret_text_is_refused() {
    let (hub, fake) = hub();
    assert_eq!(hub.receive("hello world"), Ok(()));
    assert_eq!(fake.read().as_deref(), Some("hello world"));
    assert_eq!(hub.receive(""), Err("invalid"));
    assert_eq!(hub.receive(&"a b ".repeat(MAX_CLIP / 4 + 1)), Err("limit"));
    assert_eq!(hub.receive(&"a ".repeat(MAX_CLIP / 2)), Ok(()));
    assert_eq!(hub.receive(&"a".repeat(40)), Err("secret"));
    assert_eq!(hub.receive("deploy with password=hunter22secret now"), Err("secret"));
    assert_eq!(hub.receive("sk-ant-abcdef1234567890abcdef"), Err("secret"));
    assert_ne!(fake.read().as_deref(), Some("deploy with password=hunter22secret now"));
    *lock(&fake.fail) = true;
    assert_eq!(hub.receive("fine"), Err("internal"));
}

#[test]
fn sending_to_the_phone_needs_an_open_session_and_clean_text() {
    let (hub, fake) = hub();
    *lock(&fake.text) = Some("copied text".into());
    assert_eq!(hub.send(), Err("noSession"));
    let mut a = hub.subscribe();
    let mut b = hub.subscribe();
    assert_eq!(hub.send(), Ok(2));
    assert_eq!(&*a.try_recv().unwrap(), "copied text");
    assert_eq!(&*b.try_recv().unwrap(), "copied text");
    *lock(&fake.text) = None;
    assert_eq!(hub.send(), Err("empty"));
    *lock(&fake.text) = Some(String::new());
    assert_eq!(hub.send(), Err("empty"));
    *lock(&fake.text) = Some("x".repeat(MAX_CLIP + 1));
    assert_eq!(hub.send(), Err("limit"));
    *lock(&fake.text) = Some("token=abcd1234secretvalue".into());
    assert_eq!(hub.send(), Err("secret"));
    assert!(a.try_recv().is_err());
    drop((a, b));
    *lock(&fake.text) = Some("ok".into());
    assert_eq!(hub.send(), Err("noSession"));
}

fn paired(rig: &Rig, phone: &Phone) {
    let ctx = rig.remote.ctx().unwrap();
    assert!(lock(&ctx.devices).add(Device::new(&phone.id, "Phone", phone.key.public(), crate::remote::config::now_secs())));
}

async fn hello_with(ws: &mut Ws, t: &mut core_remote_proto::Transport, caps: Value) -> Value {
    send_json(ws, t, json!({ "id": "h1", "t": "hello", "proto": 1, "app": "test", "platform": "android", "caps": caps })).await;
    recv_json(ws, t).await.unwrap()
}

#[test]
fn a_phone_with_the_clip_cap_gets_the_desktop_clipboard_and_can_push_its_own() {
    let rig = rig("clip-e2e", "", false);
    rig.remote.enable(true);
    let ctx = rig.remote.ctx().unwrap();
    let fake = Arc::new(FakeClip::default());
    ctx.board.clip.set_sink(fake.clone());
    let new = Phone::new("old-app");
    let phone = Phone::new("p");
    paired(&rig, &phone);
    paired(&rig, &new);
    let desktop = rig.desktop_key();
    runtime().block_on(async {
        let (mut ow, mut ot) = session(rig.addr(), &desktop, &new).await.unwrap();
        hello_with(&mut ow, &mut ot, json!([])).await;
        assert_eq!(rig.remote.clip_send(), json!({ "ok": false, "error": "noSession" }));
        let (mut ws, mut t) = session(rig.addr(), &desktop, &phone).await.unwrap();
        assert_eq!(hello_with(&mut ws, &mut t, json!(["clip"])).await["ok"], true);
        assert_eq!(rig.remote.clip_send(), json!({ "ok": false, "error": "empty" }));
        *lock(&fake.text) = Some("from desktop".into());
        assert_eq!(rig.remote.clip_send(), json!({ "ok": true, "sent": 1 }));
        let ev = until_json(&mut ws, &mut t, |v| v["t"] == "clip").await;
        assert_eq!(ev["text"], "from desktop");
        send_json(&mut ws, &mut t, json!({ "id": "c1", "t": "clip.push", "text": "from phone" })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "c1").await["ok"], true);
        assert_eq!(fake.read().as_deref(), Some("from phone"));
        send_json(&mut ws, &mut t, json!({ "id": "c2", "t": "clip.push", "text": "api_key=abcd1234secretvalue" })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "c2").await["error"], "secret");
        send_json(&mut ws, &mut t, json!({ "id": "c4", "t": "clip.push", "text": 5 })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "c4").await["error"], "invalid");
        assert_eq!(fake.read().as_deref(), Some("from phone"));
        drop((ws, t));
    });
    rig.remote.enable(false);
}

#[test]
fn view_only_desktop_refuses_clip_push() {
    let rig = rig("clip-view", "", false);
    rig.remote.enable(true);
    let ctx = rig.remote.ctx().unwrap();
    let fake = Arc::new(FakeClip::default());
    ctx.board.clip.set_sink(fake.clone());
    rig.host.view_only.store(true, std::sync::atomic::Ordering::SeqCst);
    let phone = Phone::new("p");
    paired(&rig, &phone);
    let desktop = rig.desktop_key();
    runtime().block_on(async {
        let (mut ws, mut t) = session(rig.addr(), &desktop, &phone).await.unwrap();
        hello_with(&mut ws, &mut t, json!(["clip"])).await;
        send_json(&mut ws, &mut t, json!({ "id": "c1", "t": "clip.push", "text": "nope" })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "c1").await["error"], "denied");
    });
    assert!(fake.read().is_none());
    rig.remote.enable(false);
}
