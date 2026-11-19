use core_http::{Config as HttpConfig, Hooks, RunningServer, StoreHandle};
use core_remote_proto::frame::{Frame, Kind, Reassembler, MAX_MESSAGE};
use core_remote_proto::Transport;
use core_store::{Agent, Store, StoreOptions, TerminalSession};
use serde_json::{json, Value};

use super::devices::Device;
use super::fit::{fit_data, fit_snapshot, oversized, size, Fit, Trimmed, ACTIVITY_KEEP, SNAPSHOT_BUDGET, TEXT_MAX};
use super::test_util::*;
use crate::state::lock;

fn agent(id: &str, role: &str, status: &str, at: &str, text: usize) -> Value {
    json!({ "id": id, "terminalId": "t1", "role": role, "status": status, "updatedAt": at, "description": "d".repeat(text) })
}

fn snapshot(agents: Vec<Value>, activity: usize) -> Value {
    let activity: Vec<Value> = (0..activity).map(|i| json!({ "id": format!("a{i}"), "at": format!("2026-11-19T00:{:02}:00Z", i % 60), "kind": "tool", "data": { "title": "x".repeat(200) } })).collect();
    json!({ "protocolVersion": 1, "terminals": [{ "id": "t1", "title": "t1", "status": "working" }], "agents": agents, "activity": activity })
}

fn ids(data: &Value) -> Vec<String> {
    data["agents"].as_array().unwrap().iter().map(|a| a["id"].as_str().unwrap().to_owned()).collect()
}

#[test]
fn a_small_snapshot_is_left_whole() {
    let mut data = snapshot(vec![agent("m", "main", "working", "2026-11-19T01:00:00Z", 10)], 20);
    let before = data.clone();
    assert_eq!(fit_data(&mut data, SNAPSHOT_BUDGET), Fit::Whole);
    assert_eq!(data, before);
}

#[test]
fn old_activity_goes_first_and_the_newest_is_kept() {
    let mut data = snapshot(vec![agent("m", "main", "working", "", 10)], 3000);
    let budget = size(&data) / 2;
    let fit = fit_data(&mut data, budget);
    assert_eq!(fit, Fit::Trimmed(Trimmed { activity: 3000 - ACTIVITY_KEEP, agents: 0, text: false }));
    let list = data["activity"].as_array().unwrap();
    assert_eq!(list.len(), ACTIVITY_KEEP);
    assert_eq!(list.last().unwrap()["id"], "a2999");
    assert!(size(&data) <= budget);
}

#[test]
fn finished_subagents_go_oldest_first_and_main_or_running_agents_stay() {
    let mut agents = vec![agent("main", "main", "completed", "2026-11-19T00:00:00Z", 5000), agent("run", "subagent", "working", "2026-11-19T00:00:01Z", 5000)];
    for i in 0..20 {
        agents.push(agent(&format!("s{i:02}"), "subagent", "completed", &format!("2026-11-19T01:{i:02}:00Z"), 5000));
    }
    let mut data = snapshot(agents, 0);
    let budget = size(&data) - 5 * 5000;
    let Fit::Trimmed(t) = fit_data(&mut data, budget) else { panic!("not trimmed") };
    assert!(t.agents >= 5 && t.agents <= 6, "{t:?}");
    assert!(!t.text);
    let kept = ids(&data);
    assert!(kept.contains(&"main".to_owned()) && kept.contains(&"run".to_owned()));
    assert!(!kept.contains(&"s00".to_owned()) && kept.contains(&"s19".to_owned()));
    assert!(size(&data) <= budget);
}

#[test]
fn long_text_is_cut_before_live_subagents_are_dropped() {
    let agents = vec![agent("main", "main", "working", "", 10), agent("run", "subagent", "working", "", 50_000)];
    let mut data = snapshot(agents, 0);
    let Fit::Trimmed(t) = fit_data(&mut data, 20_000) else { panic!("not trimmed") };
    assert!(t.text && t.agents == 0, "{t:?}");
    assert_eq!(ids(&data), ["main", "run"]);
    assert_eq!(data["agents"][1]["description"].as_str().unwrap().chars().count(), TEXT_MAX + 1);
}

#[test]
fn a_snapshot_that_cannot_fit_reports_failed_with_its_size() {
    let terminals: Vec<Value> = (0..400).map(|i| json!({ "id": format!("t{i}"), "title": "x".repeat(900), "status": "working" })).collect();
    let mut data = json!({ "terminals": terminals, "agents": [], "activity": [] });
    let Fit::Failed(n) = fit_data(&mut data, 100_000) else { panic!("should fail") };
    assert!(n > 100_000);
}

#[test]
fn the_snapshot_event_carries_the_trimmed_marker_only_when_cut() {
    let mut small = json!({ "t": "snapshot", "seq": 1, "data": snapshot(vec![], 2) });
    assert_eq!(fit_snapshot(&mut small, SNAPSHOT_BUDGET), Fit::Whole);
    assert!(small.get("trimmed").is_none());
    let mut big = json!({ "t": "snapshot", "seq": 1, "data": snapshot(vec![], 3000) });
    let budget = size(&big["data"]) / 2;
    assert!(matches!(fit_snapshot(&mut big, budget), Fit::Trimmed(_)));
    assert_eq!(big["trimmed"]["activity"], 3000 - ACTIVITY_KEEP);
    assert_eq!(big["trimmed"]["agents"], 0);
}

#[test]
fn an_oversized_reply_becomes_a_limit_error_and_an_event_a_limit_event() {
    assert_eq!(oversized(&json!({ "re": "c1", "ok": true, "data": 1 }), MAX_MESSAGE + 1), json!({ "re": "c1", "ok": false, "error": "limit" }));
    let ev = oversized(&json!({ "t": "ws.snapshot", "tabs": [] }), MAX_MESSAGE + 9);
    assert_eq!((ev["t"].as_str(), ev["what"].as_str(), ev["bytes"].as_u64()), (Some("limit"), Some("ws.snapshot"), Some((MAX_MESSAGE + 9) as u64)));
}

async fn core_server(dir: &std::path::Path) -> (RunningServer, StoreHandle) {
    let dist = dir.join("dist");
    std::fs::create_dir_all(&dist).unwrap();
    std::fs::write(dist.join("index.html"), "ok").unwrap();
    let store = StoreHandle::new(Store::new(StoreOptions { activity_limit: 10, batch_ms: 10 }));
    let server = core_http::start(HttpConfig::new(0, dist), store.clone(), Hooks::default()).await.unwrap();
    (server, store)
}

async fn recv_whole(ws: &mut Ws, t: &mut Transport, reasm: &mut Reassembler) -> Value {
    loop {
        let bytes = next_binary(ws).await.expect("socket open");
        let frame = Frame::decode(&t.open(&bytes).unwrap()).unwrap();
        if let Some(m) = reasm.push(frame).unwrap() {
            if m.kind == Kind::Json {
                return serde_json::from_slice(&m.payload).unwrap();
            }
        }
    }
}

#[test]
fn a_snapshot_over_one_mebibyte_reaches_the_phone_trimmed_and_the_session_keeps_working() {
    let rig = rig("fit-big", "", false);
    let rt = runtime();
    let (server, store) = rt.block_on(core_server(&rig.dir));
    store.with(|s| {
        s.upsert_terminal(serde_json::from_value::<TerminalSession>(json!({ "id": "t1", "title": "t1", "status": "working" })).unwrap());
        s.upsert_agent(serde_json::from_value::<Agent>(agent("main", "main", "working", "2026-11-19T00:00:00Z", 100)).unwrap());
        for i in 0..700 {
            s.upsert_agent(serde_json::from_value::<Agent>(agent(&format!("s{i:03}"), "subagent", "completed", &format!("2026-11-19T{:02}:{:02}:00Z", i / 60, i % 60), 2000)).unwrap());
        }
    });
    let tap = server.tap();
    assert!(tap.snapshot().len() > MAX_MESSAGE);
    *lock(&rig.host.tap) = Some(tap);
    rig.remote.enable(true);
    assert!(rig.remote.publish(&json!({ "rev": 1, "tabs": [], "closedCount": 0 })));
    let phone = Phone::new("p");
    let ctx = rig.remote.ctx().unwrap();
    assert!(lock(&ctx.devices).add(Device::new(&phone.id, "Phone", phone.key.public(), super::config::now_secs())));
    let desktop = rig.desktop_key();
    rt.block_on(async {
        let (mut ws, mut t) = session(rig.addr(), &desktop, &phone).await.unwrap();
        let mut reasm = Reassembler::new();
        assert_eq!(hello(&mut ws, &mut t).await["ok"], true);
        let mut snap = recv_whole(&mut ws, &mut t, &mut reasm).await;
        while snap["t"] != "snapshot" {
            snap = recv_whole(&mut ws, &mut t, &mut reasm).await;
        }
        assert!(size(&snap) <= MAX_MESSAGE);
        assert!(snap["trimmed"]["agents"].as_u64().unwrap() > 0, "{}", snap["trimmed"]);
        let kept = ids(&snap["data"]);
        assert!(kept.contains(&"main".to_owned()) && kept.contains(&"s699".to_owned()) && !kept.contains(&"s000".to_owned()));
        assert_eq!(recv_whole(&mut ws, &mut t, &mut reasm).await["t"], "ws.snapshot");
        send_json(&mut ws, &mut t, json!({ "id": "r", "t": "resync" })).await;
        assert_eq!(recv_whole(&mut ws, &mut t, &mut reasm).await["re"], "r");
        let again = recv_whole(&mut ws, &mut t, &mut reasm).await;
        assert_eq!(again["t"], "snapshot");
        assert!(again["trimmed"].is_object());
    });
    rig.remote.enable(false);
    rt.block_on(server.shutdown());
}
