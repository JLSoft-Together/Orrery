use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use core_store::extra::{Dict, GitDiffStat, PlanLimits, UsageBucket};
use core_store::model::{ActivityKind, AgentRole, AgentStatus, PeerLinkDir, PeerLinkKind, TerminalStatus};
use core_store::{Agent, NewActivity, PeerLink, ServerMessage, Store, StoreOptions, TerminalSession};
use serde_json::{json, Value};

struct Rig {
    store: Store,
    clock: Arc<AtomicI64>,
    sent: Arc<Mutex<Vec<ServerMessage>>>,
}

fn rig(activity_limit: usize, batch_ms: i64) -> Rig {
    let clock = Arc::new(AtomicI64::new(1_800_000_000_000));
    let c = clock.clone();
    let mut store = Store::with_clock(StoreOptions { activity_limit, batch_ms }, Box::new(move || c.load(Ordering::SeqCst)));
    let sent = Arc::new(Mutex::new(Vec::new()));
    let sink = sent.clone();
    store.subscribe(Box::new(move |m| {
        sink.lock().unwrap().push(m.clone());
        Ok(())
    }));
    Rig { store, clock, sent }
}

impl Rig {
    fn messages(&self) -> Vec<Value> {
        self.sent.lock().unwrap().iter().map(|m| serde_json::to_value(m).unwrap()).collect()
    }

    fn events(&mut self) -> Vec<Value> {
        self.store.flush();
        self.messages().iter().flat_map(|m| m["events"].as_array().unwrap().clone()).collect()
    }
}

fn terminal(id: &str, title: &str, status: TerminalStatus) -> TerminalSession {
    serde_json::from_value(json!({ "id": id, "title": title, "status": status.as_str() })).unwrap()
}

fn agent(over: Value) -> Agent {
    let mut base = json!({ "id": "a1", "terminalId": "t1", "role": "main", "name": "x", "type": "main", "status": "working" });
    for (k, v) in over.as_object().unwrap() {
        if v.is_null() {
            base.as_object_mut().unwrap().remove(k);
        } else {
            base[k] = v.clone();
        }
    }
    serde_json::from_value(base).unwrap()
}

#[test]
fn emits_created_field_patches_with_null_and_batches() {
    let mut r = rig(3, 5);
    let mut t = terminal("t", "A", TerminalStatus::Idle);
    t.waiting_for = Some("x".into());
    r.store.upsert_terminal(t);
    r.store.upsert_terminal(terminal("t", "B", TerminalStatus::Idle));
    r.store.upsert_terminal(terminal("t", "B", TerminalStatus::Idle));
    for i in 0..5 {
        let mut a = NewActivity::new(ActivityKind::ToolStarted);
        a.data = Some(serde_json::from_value(json!({ "tool": i.to_string() })).unwrap());
        r.store.push_activity(a);
    }
    assert_eq!(r.store.next_flush_at(), Some(1_800_000_000_005));
    assert!(!r.store.poll(1_800_000_000_004));
    assert!(r.messages().is_empty());
    r.clock.store(1_800_000_000_020, Ordering::SeqCst);
    assert!(r.store.poll(1_800_000_000_020));
    let messages = r.messages();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0]["seq"], 1);
    assert_eq!(messages[0]["events"][0], json!({ "type": "terminal.created", "payload": { "id": "t", "title": "B", "status": "idle" } }));
    assert!(messages[0]["events"].as_array().unwrap().iter().all(|e| e["type"] != "terminal.updated"));
    assert_eq!(r.store.snapshot().activity.len(), 3);

    let mut t = terminal("t", "C", TerminalStatus::Working);
    t.waiting_for = Some("y".into());
    r.store.upsert_terminal(t);
    r.store.upsert_terminal(terminal("t", "C", TerminalStatus::Idle));
    r.store.flush();
    assert_eq!(r.messages()[1]["events"], json!([{ "type": "terminal.updated", "payload": { "id": "t", "title": "C", "status": "idle", "waitingFor": null } }]));
}

#[test]
fn merges_patches_per_id_without_crossing_a_removal() {
    let mut r = rig(3, 5);
    let base = json!({ "role": "subagent", "terminalId": "t", "id": "a" });
    let a = |over: Value| {
        let mut v = base.clone();
        v["name"] = json!("x");
        v["status"] = json!("working");
        for (k, x) in over.as_object().unwrap() {
            v[k] = x.clone();
        }
        serde_json::from_value::<Agent>(v).unwrap()
    };
    r.store.upsert_agent(a(json!({})));
    r.store.flush();
    r.store.upsert_agent(a(json!({ "status": "waiting", "currentTool": "Bash" })));
    r.store.upsert_agent(a(json!({ "id": "b" })));
    r.store.upsert_agent(a(json!({ "status": "completed" })));
    let mut act = NewActivity::new(ActivityKind::AgentCompleted);
    act.agent_id = Some("a".into());
    r.store.push_activity(act);
    r.store.remove_agent("a");
    r.store.upsert_agent(a(json!({ "name": "again" })));
    r.store.upsert_agent(a(json!({ "name": "again", "status": "completed" })));
    r.store.flush();
    let events = r.messages()[1]["events"].as_array().unwrap().clone();
    let types: Vec<&str> = events.iter().map(|e| e["type"].as_str().unwrap()).collect();
    assert_eq!(types, ["agent.updated", "agent.created", "activity", "agent.removed", "agent.created"]);
    assert_eq!(events[0]["payload"], json!({ "id": "a", "status": "completed", "currentTool": null }));
    assert_eq!(events[4]["payload"]["name"], "again");
    assert_eq!(events[4]["payload"]["status"], "completed");
}

#[test]
fn keeps_broadcasting_when_one_listener_fails() {
    let mut store = Store::new(StoreOptions { activity_limit: 3, batch_ms: 5 });
    let got = Arc::new(Mutex::new(0));
    store.subscribe(Box::new(|_| Err("toast observer".into())));
    let g = got.clone();
    store.subscribe(Box::new(move |_| {
        *g.lock().unwrap() += 1;
        Ok(())
    }));
    store.upsert_terminal(terminal("t", "A", TerminalStatus::Idle));
    store.flush();
    assert_eq!(*got.lock().unwrap(), 1);
}

#[test]
fn emits_only_new_roots_and_reports_unknown_projects() {
    let mut store = Store::new(StoreOptions { activity_limit: 10, batch_ms: 1 });
    let asked = Arc::new(Mutex::new(Vec::<Vec<String>>::new()));
    let a = asked.clone();
    store.set_on_new_projects(Some(Box::new(move |dirs| a.lock().unwrap().push(dirs))));
    let b: UsageBucket = serde_json::from_value(json!({
        "key": "k", "day": "2026-10-07", "model": "m", "project": "D:/a/b", "messages": 1, "input": 1, "output": 1,
        "cacheWrite5m": 0, "cacheWrite1h": 0, "cacheRead": 0
    }))
    .unwrap();
    store.update_usage(vec![b.clone()]);
    assert_eq!(*asked.lock().unwrap(), vec![vec!["D:/a/b".to_string()]]);
    let mut roots = Dict::new();
    roots.insert("D:/a/b".to_string(), "D:/a".to_string());
    store.set_project_roots(roots.clone());
    store.update_usage(vec![b]);
    assert_eq!(asked.lock().unwrap().len(), 1);
    assert_eq!(store.usage_snapshot().roots, Some(roots.clone()));
    let mut r = rig(10, 1);
    r.store.set_project_roots(roots.clone());
    r.store.set_project_roots(roots);
    let events = r.events();
    assert_eq!(events, vec![json!({ "type": "usage.roots", "payload": { "roots": { "D:/a/b": "D:/a" } } })]);
}

#[test]
fn identical_upsert_emits_nothing_even_with_absent_fields() {
    let mut r = rig(10, 1000);
    r.store.upsert_agent(agent(json!({ "model": null })));
    r.store.flush();
    let before = r.events().len();
    r.store.upsert_agent(agent(json!({ "model": null })));
    assert_eq!(r.events().len(), before);
}

#[test]
fn patches_only_changed_fields_and_nulls_cleared_ones() {
    let mut r = rig(10, 1000);
    r.store.upsert_agent(agent(json!({ "model": "m" })));
    r.store.flush();
    r.store.upsert_agent(agent(json!({ "model": null, "status": "idle" })));
    let last = r.events().pop().unwrap();
    assert_eq!(last, json!({ "type": "agent.updated", "payload": { "id": "a1", "status": "idle", "model": null } }));
    assert!(r.store.get_agent("a1").unwrap().model.is_none());
}

#[test]
fn agents_of_follows_upsert_move_and_removal() {
    let mut r = rig(10, 1000);
    r.store.upsert_agent(agent(json!({})));
    r.store.upsert_agent(agent(json!({ "id": "a2" })));
    r.store.upsert_agent(agent(json!({ "id": "a3", "terminalId": "t2" })));
    let ids = |s: &Store, t: &str| s.agents_of(t).iter().map(|a| a.id.clone()).collect::<Vec<_>>();
    assert_eq!(ids(&r.store, "t1"), ["a1", "a2"]);
    r.store.upsert_agent(agent(json!({ "id": "a2", "terminalId": "t2" })));
    assert_eq!(ids(&r.store, "t1"), ["a1"]);
    assert_eq!(ids(&r.store, "t2"), ["a3", "a2"]);
    r.store.remove_agent("a1");
    assert!(ids(&r.store, "t1").is_empty());
}

#[test]
fn object_fields_patch_when_identity_changes_like_triple_equals() {
    let mut r = rig(10, 1000);
    let stat = Arc::new(GitDiffStat { files: 1, insertions: 2, deletions: 0, untracked: 0, at: "2027-01-15T08:00:00.000Z".into() });
    let mut t = terminal("t", "A", TerminalStatus::Idle);
    t.diff = Some(stat.clone());
    r.store.upsert_terminal(t.clone());
    r.store.flush();
    r.store.upsert_terminal(t.clone());
    assert!(r.store.next_flush_at().is_none());
    t.diff = Some(Arc::new((*stat).clone()));
    r.store.upsert_terminal(t.clone());
    t.diff = None;
    r.store.upsert_terminal(t);
    let events = r.events();
    assert_eq!(events.len(), 2);
    assert_eq!(events[1]["payload"], json!({ "id": "t", "diff": null }));
}

#[test]
fn created_absorbs_later_patches_and_null_drops_the_field() {
    let mut r = rig(10, 1000);
    let mut t = terminal("t", "A", TerminalStatus::Working);
    t.waiting_for = Some("permission".into());
    r.store.upsert_terminal(t);
    let mut t = terminal("t", "A", TerminalStatus::Waiting);
    t.alias = Some("Ads".into());
    r.store.upsert_terminal(t);
    r.store.remove_terminal("t");
    r.store.upsert_terminal(terminal("t", "Z", TerminalStatus::Idle));
    let events = r.events();
    assert_eq!(
        events,
        vec![
            json!({ "type": "terminal.created", "payload": { "id": "t", "title": "A", "status": "waiting", "alias": "Ads" } }),
            json!({ "type": "terminal.removed", "payload": { "id": "t" } }),
            json!({ "type": "terminal.created", "payload": { "id": "t", "title": "Z", "status": "idle" } }),
        ]
    );
}

#[test]
fn remove_terminal_removes_its_agents_first() {
    let mut r = rig(10, 1000);
    r.store.upsert_terminal(terminal("t1", "A", TerminalStatus::Idle));
    r.store.upsert_agent(agent(json!({})));
    r.store.flush();
    r.store.remove_terminal("t1");
    r.store.remove_terminal("t1");
    let types: Vec<String> = r.events().iter().skip(2).map(|e| e["type"].as_str().unwrap().to_string()).collect();
    assert_eq!(types, ["agent.removed", "terminal.removed"]);
    assert!(r.store.agents_of("t1").is_empty());
}

#[test]
fn links_create_then_patch() {
    let mut r = rig(10, 1000);
    let link = |ab: u64| PeerLink {
        id: "a>b".into(),
        kind: PeerLinkKind::Message,
        a: "a".into(),
        b: "b".into(),
        ab,
        ba: 0,
        first_at: "2027-01-15T08:00:00.000Z".into(),
        last_at: "2027-01-15T08:00:00.000Z".into(),
        last_dir: PeerLinkDir::Ab,
        last_agent_id: None,
    };
    r.store.upsert_link(link(1));
    r.store.flush();
    r.store.upsert_link(link(1));
    r.store.upsert_link(link(2));
    r.store.remove_link("a>b");
    r.store.remove_link("a>b");
    let events = r.events();
    assert_eq!(events[1], json!({ "type": "link.updated", "payload": { "id": "a>b", "ab": 2 } }));
    assert_eq!(events[2], json!({ "type": "link.removed", "payload": { "id": "a>b" } }));
    assert_eq!(events.len(), 3);
    assert!(r.store.peer_links().is_empty());
}

#[test]
fn setters_skip_json_equal_values() {
    let mut r = rig(10, 1000);
    r.store.set_loading(false);
    r.store.set_loading(true);
    r.store.set_loading(true);
    r.store.set_snoozes(Dict::new());
    r.store.set_jobs(Vec::new());
    r.store.set_model_names(Dict::new());
    let limits: PlanLimits = serde_json::from_value(json!({ "updatedAt": "2027-01-15T08:00:00.000Z", "plan": "max" })).unwrap();
    r.store.set_limits(None);
    r.store.set_limits(Some(limits.clone()));
    r.store.set_limits(Some(limits.clone()));
    r.store.set_codex_limits(limits.clone());
    r.store.set_codex_limits(limits);
    r.store.set_resources(json!({ "at": "2027-01-15T08:00:00.000Z", "sessions": [1] }));
    r.store.set_resources(json!({ "at": "2027-01-15T08:00:05.000Z", "sessions": [1] }));
    r.store.set_resources(json!({ "at": "2027-01-15T08:00:10.000Z", "sessions": [2] }));
    let types: Vec<String> = r.events().iter().map(|e| e["type"].as_str().unwrap().to_string()).collect();
    assert_eq!(types, ["sessions.loading", "limits.updated", "codexLimits.updated", "resources", "resources"]);
    let snap = serde_json::to_value(r.store.snapshot()).unwrap();
    assert_eq!(snap["loading"], true);
    assert_eq!(snap["resources"]["at"], "2027-01-15T08:00:10.000Z");
}

#[test]
fn snapshot_sorts_activity_by_time_and_keeps_rings_apart() {
    let mut r = rig(2, 1000);
    let at = |kind, t: &str| {
        let mut a = NewActivity::new(kind);
        a.at = Some(t.to_string());
        a
    };
    r.store.push_activity(at(ActivityKind::TerminalStarted, "2027-01-15T08:00:03.000Z"));
    r.store.push_activity(at(ActivityKind::ToolStarted, "2027-01-15T08:00:01.000Z"));
    r.store.push_activity(at(ActivityKind::TerminalStatus, "2027-01-15T08:00:02.000Z"));
    r.store.push_activity(at(ActivityKind::TerminalEnded, "2027-01-15T08:00:04.000Z"));
    r.store.push_activity(NewActivity::new(ActivityKind::ToolFailed));
    let snap = r.store.snapshot();
    let kinds: Vec<&str> = snap.activity.iter().map(|a| a.kind.as_str()).collect();
    assert_eq!(kinds, ["tool.failed", "tool.started", "terminal.status", "terminal.ended"]);
    assert_eq!(snap.activity[0].at, "2027-01-15T08:00:00.000Z");
    assert_eq!(snap.activity[0].id, "mywpiww0-4");
    assert_eq!(r.store.tool_events().len(), 2);
}

#[test]
fn snapshot_message_flushes_first_and_carries_the_seq() {
    let mut r = rig(10, 1000);
    r.store.status_line_command = Some("node bridge.mjs".into());
    r.store.upsert_terminal(terminal("t", "A", TerminalStatus::Idle));
    let message = serde_json::to_value(r.store.snapshot_message()).unwrap();
    assert_eq!(r.messages().len(), 1);
    assert_eq!(message["seq"], 1);
    let snap = &message["events"][0]["payload"];
    assert_eq!(message["events"][0]["type"], "snapshot");
    assert_eq!(snap["collectorStartedAt"], "2027-01-15T08:00:00.000Z");
    assert_eq!(snap["statusLineCommand"], "node bridge.mjs");
    assert_eq!(snap["terminals"][0]["title"], "A");
    for key in ["favorites", "jobs", "modelNames", "snoozes", "peerLinks", "usage"] {
        assert!(snap.get(key).is_some(), "{key}");
    }
    for key in ["limits", "notify", "loading", "resources", "sensitive"] {
        assert!(snap.get(key).is_none(), "{key}");
    }
}

#[test]
fn context_pct_and_current_tool_come_from_the_main_agent() {
    let mut r = rig(10, 1000);
    r.store.upsert_agent(agent(json!({ "id": "s", "role": "subagent", "contextPct": 90, "currentTool": "Grep" })));
    assert_eq!(r.store.context_pct("t1"), None);
    r.store.upsert_agent(agent(json!({ "contextTokens": 83_500, "contextWindow": 200_000, "currentTool": "Bash" })));
    assert!((r.store.context_pct("t1").unwrap() - 50.0).abs() < 1e-9);
    assert_eq!(r.store.current_tool_of("t1"), Some("Bash"));
    r.store.upsert_agent(agent(json!({ "contextPct": 12.5 })));
    assert_eq!(r.store.context_pct("t1"), Some(12.5));
    let main = r.store.get_agent("a1").unwrap();
    assert_eq!((main.role, main.status), (AgentRole::Main, AgentStatus::Working));
}
