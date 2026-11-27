use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use serde_json::{json, Value};

use super::super::super::test_util::{rig, runtime, until_json, TestHost};
use super::super::super::viewer_events::{merge, VEvent, WRITE_PER_MINUTE};
use super::super::super::viewer_map::{build_args, lookup, SPECS};
use super::super::super::viewer_tests::{ask, engine, open};
use super::{args, delete, expand, list, list_reply, looks_secret, pane_ctx, save, scope_matches, Vars, BUDGET};
use crate::state::lock;

fn item(id: &str, title: &str, body: &str, scope: &str) -> Value {
    json!({ "id": id, "title": title, "body": body, "scope": scope, "updatedAt": 1 })
}

fn vars(branch: &str, project: &str, cwd: &str) -> Vars {
    Vars { branch: branch.into(), project: project.into(), cwd: cwd.into() }
}

fn summary() -> Value {
    json!({ "rev": 1, "tabs": [{ "key": "t1", "panes": [
        { "key": "p1", "kind": "term", "agent": "claude", "cwd": "C:\\work\\api", "branch": "feat/x" },
        { "key": "p2", "kind": "term", "agent": "claude", "cwd": "/home/me/web/" },
        { "key": "p3", "kind": "term", "agent": "shell" }
    ] }] })
}

type Store = Arc<Mutex<Vec<Value>>>;

fn fake(items: Vec<Value>) -> (Arc<TestHost>, Store) {
    let host = Arc::new(TestHost::default());
    let store: Store = Arc::new(Mutex::new(items));
    let s = store.clone();
    *lock(&host.viewer) = Some(Box::new(move |_o, channel, a, _acc| {
        let mut items = lock(&s);
        match channel {
            "ccm:prompts-get" => json!({ "raw": { "ok": true, "version": 1, "items": items.clone(), "recovered": false } }),
            "ccm:prompts-set" => {
                let req = a[0].clone();
                match req["op"].as_str() {
                    Some("upsert") => {
                        let mut it = req["item"].clone();
                        let id = it["id"].as_str().map_or_else(|| format!("new{}", items.len()), str::to_owned);
                        it["id"] = json!(id);
                        it["updatedAt"] = json!(2);
                        match items.iter_mut().find(|x| x["id"] == it["id"]) {
                            Some(x) => *x = it,
                            None => items.push(it),
                        }
                        json!({ "raw": { "ok": true, "version": 1, "items": items.clone(), "id": id } })
                    }
                    Some("delete") => {
                        let id = req["id"].clone();
                        let before = items.len();
                        items.retain(|x| x["id"] != id);
                        if items.len() == before {
                            json!({ "raw": { "ok": false, "error": "not-found" } })
                        } else {
                            json!({ "raw": { "ok": true, "version": 1, "items": items.clone() } })
                        }
                    }
                    _ => json!({ "raw": { "ok": false, "error": "invalid-op" } }),
                }
            }
            other => panic!("unexpected channel {other}"),
        }
    }));
    (host, store)
}

#[test]
fn the_three_ops_are_in_the_table_and_only_the_writes_cost_quota() {
    assert_eq!(SPECS.len(), 41);
    for (t, write) in [("prompts.list", false), ("prompts.save", true), ("prompts.delete", true)] {
        let spec = lookup(t).unwrap();
        assert!(spec.channel.starts_with("ccm:remote-prompts-"), "{t}");
        assert_eq!(spec.charges_write(&json!({})), write, "{t}");
    }
}

#[test]
fn op_arguments_are_checked_before_they_reach_the_desk() {
    let list_spec = lookup("prompts.list").unwrap();
    assert_eq!(build_args(list_spec, &json!({ "t": "prompts.list" })), Some(json!([{}])));
    assert_eq!(build_args(list_spec, &json!({ "paneKey": null })), Some(json!([{}])));
    assert_eq!(build_args(list_spec, &json!({ "paneKey": "p1", "evil": 1 })), Some(json!([{ "paneKey": "p1" }])));
    assert_eq!(build_args(list_spec, &json!({ "paneKey": "" })), None);
    assert_eq!(build_args(list_spec, &json!({ "paneKey": 5 })), None);
    assert_eq!(build_args(list_spec, &json!({ "paneKey": "x".repeat(129) })), None);

    let save_spec = lookup("prompts.save").unwrap();
    assert_eq!(build_args(save_spec, &json!({ "title": "a", "body": "b" })), Some(json!([{ "title": "a", "body": "b", "scope": "all" }])));
    assert_eq!(
        build_args(save_spec, &json!({ "promptId": "p-1", "title": "a", "body": "b", "scope": "C:\\w", "op": "delete", "x": 1 })),
        Some(json!([{ "id": "p-1", "title": "a", "body": "b", "scope": "C:\\w" }]))
    );
    assert_eq!(build_args(save_spec, &json!({ "title": "a" })), None);
    assert_eq!(build_args(save_spec, &json!({ "title": 1, "body": "b" })), None);
    assert_eq!(build_args(save_spec, &json!({ "promptId": "a/../b", "title": "a", "body": "b" })), None);
    assert_eq!(build_args(save_spec, &json!({ "promptId": 7, "title": "a", "body": "b" })), None);
    assert_eq!(build_args(save_spec, &json!({ "title": "a", "body": "x".repeat(16_001) })), None);
    assert_eq!(build_args(save_spec, &json!({ "title": "a", "body": "b", "scope": 3 })), None);

    let del = lookup("prompts.delete").unwrap();
    assert_eq!(build_args(del, &json!({ "promptId": "abc_1-2" })), Some(json!([{ "id": "abc_1-2" }])));
    assert_eq!(build_args(del, &json!({})), None);
    assert_eq!(build_args(del, &json!({ "promptId": "" })), None);
    assert_eq!(build_args(del, &json!({ "promptId": "a b" })), None);
    assert_eq!(args("prompts.nope", &json!({})), None);
}

#[test]
fn secret_like_text_is_told_apart_from_plain_prompts() {
    for secret in ["API_KEY=abc123 run", "curl https://u:p4ss@h.io/x", "send Bearer abcdefghijkl", "npm run x --auth-token abcdef", "fetch https://x.io/?token=abc&y=1"] {
        assert!(looks_secret(secret), "{secret}");
    }
    for plain in ["Fix the {branch} bug in {project}", "run npm test and report", "Session: 3 live", "read C:\\Users\\me\\proj\\a.ts", "Giải thích đoạn code này", "key: use a map", ""] {
        assert!(!looks_secret(plain), "{plain}");
    }
}

#[test]
fn variables_are_filled_in_one_pass_and_missing_ones_stay_as_text() {
    let v = vars("main", "api", "C:\\work\\api");
    assert_eq!(expand("on {branch} in {project} at {cwd}", &v), ("on main in api at C:\\work\\api".to_owned(), vec![]));
    assert_eq!(expand("{branch}{branch}", &v).0, "mainmain");
    assert_eq!(expand("{other} {BRANCH} { branch } {branch", &v), ("{other} {BRANCH} { branch } {branch".to_owned(), vec![]));
    let none = vars("", "api", "");
    assert_eq!(expand("{branch} {project} {cwd} {branch}", &none), ("{branch} api {cwd} {branch}".to_owned(), vec!["branch", "cwd"]));
    assert_eq!(expand("{{branch}}", &v).0, "{main}");
    assert_eq!(expand("tiếng Việt {project} ✓ {", &v).0, "tiếng Việt api ✓ {");
    let tricky = vars("{cwd}", "", "/x");
    assert_eq!(expand("{branch} {cwd}", &tricky).0, "{cwd} /x");
}

#[test]
fn a_pane_gives_branch_cwd_and_project_and_unknown_panes_give_nothing() {
    let s = summary();
    let p = pane_ctx(&s, "p1").unwrap();
    assert_eq!((p.vars.branch.as_str(), p.vars.project.as_str(), p.dir.as_str()), ("feat/x", "api", "C:\\work\\api"));
    let q = pane_ctx(&s, "p2").unwrap();
    assert_eq!((q.vars.branch.as_str(), q.vars.project.as_str()), ("", "web"));
    let r = pane_ctx(&s, "p3").unwrap();
    assert_eq!((r.vars.cwd.as_str(), r.vars.project.as_str()), ("", ""));
    assert!(pane_ctx(&s, "nope").is_none());
    assert!(pane_ctx(&json!({}), "p1").is_none());
}

#[test]
fn scopes_match_like_the_desktop_does() {
    assert!(scope_matches("all", ""));
    assert!(!scope_matches("C:\\work\\api", ""));
    assert!(scope_matches("C:\\Work\\API", "c:/work/api/"));
    assert!(scope_matches("C:\\work", "C:\\work\\api\\src"));
    assert!(!scope_matches("C:\\work\\ap", "C:\\work\\api"));
    assert!(scope_matches("/home/me/web", "/home/me/web/pkg"));
    assert!(!scope_matches("/home/me/Web", "/home/me/web"));
}

#[test]
fn the_list_hides_secret_looking_items_counts_them_and_puts_pane_items_first() {
    let items = vec![
        item("a", "All", "do {branch} please", "all"),
        item("s", "Deploy", "API_KEY=abc123 deploy", "all"),
        item("l", "Local", "check {project}", "C:\\work\\api"),
        item("o", "Other", "other repo", "D:\\else"),
        item("t", "token=abcdef", "ok", "all"),
    ];
    let p = pane_ctx(&summary(), "p1");
    let out = list_reply(&items, p.as_ref(), BUDGET);
    assert_eq!(out["ok"], true);
    assert_eq!(out["hidden"], 2);
    assert!(out.get("more").is_none());
    let ids: Vec<&str> = out["items"].as_array().unwrap().iter().map(|i| i["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["l", "a", "o"]);
    assert_eq!(out["items"][0]["local"], true);
    assert_eq!(out["items"][0]["text"], "check api");
    assert_eq!(out["items"][1]["text"], "do feat/x please");
    assert_eq!(out["items"][1]["body"], "do {branch} please");
    assert!(out["items"][2].get("text").is_none());
    assert!(!out.to_string().contains("abc123"));
}

#[test]
fn without_a_pane_bodies_stay_raw_and_have_no_text_or_local_flag() {
    let items = vec![item("a", "All", "do {branch}", "all")];
    let out = list_reply(&items, None, BUDGET);
    assert_eq!(out["items"][0], items[0]);
    assert_eq!(out["hidden"], 0);
}

#[test]
fn a_variable_that_would_make_the_text_look_secret_is_not_filled() {
    let mut s = summary();
    s["tabs"][0]["panes"][0]["branch"] = json!("token=abcdef");
    let p = pane_ctx(&s, "p1");
    let out = list_reply(&[item("a", "A", "on {branch}", "all")], p.as_ref(), BUDGET);
    assert!(out["items"][0].get("text").is_none(), "{out}");
}

#[test]
fn missing_variables_are_reported_for_the_pane() {
    let p = pane_ctx(&summary(), "p2");
    let out = list_reply(&[item("a", "A", "{branch} {cwd}", "all")], p.as_ref(), BUDGET);
    assert_eq!(out["items"][0]["missing"], json!(["branch"]));
    assert_eq!(out["items"][0]["text"], "{branch} /home/me/web/");
}

#[test]
fn a_big_library_is_cut_to_the_budget_and_says_how_many_were_left_out() {
    let items: Vec<Value> = (0..200).map(|i| item(&format!("i{i}"), "T", &"word ".repeat(790), "all")).collect();
    let out = list_reply(&items, None, BUDGET);
    let kept = out["items"].as_array().unwrap().len() as u64;
    assert!(kept < 200);
    assert_eq!(kept + out["more"].as_u64().unwrap(), 200);
    assert!(serde_json::to_vec(&out).unwrap().len() < 900_000);
}

#[test]
fn list_asks_the_desk_and_passes_failures_through() {
    let (host, _) = fake(vec![item("a", "A", "b", "all")]);
    let out = list(host.as_ref(), "o", Some(&summary()), &json!([{ "paneKey": "p1" }]));
    assert_eq!(out["raw"]["items"][0]["id"], "a");
    assert_eq!(lock(&host.views)[0].1, "ccm:prompts-get");

    let off = TestHost::default();
    assert_eq!(list(&off, "o", None, &json!([{}])), json!({ "error": "offline" }));

    let broken = TestHost::default();
    *lock(&broken.viewer) = Some(Box::new(|_, _, _, _| json!({ "raw": { "ok": false, "error": "io" } })));
    assert_eq!(list(&broken, "o", None, &json!([{}])), json!({ "raw": { "ok": false, "error": "io" } }));
}

#[test]
fn saving_needs_control_and_never_writes_when_the_desktop_is_view_only() {
    let (host, store) = fake(vec![]);
    host.view_only.store(true, Ordering::SeqCst);
    let a = json!([{ "title": "T", "body": "B", "scope": "all" }]);
    assert_eq!(save(host.as_ref(), "o", &a), json!({ "raw": { "ok": false, "error": "denied" } }));
    assert_eq!(delete(host.as_ref(), "o", &json!([{ "id": "x" }])), json!({ "raw": { "ok": false, "error": "denied" } }));
    assert!(lock(&store).is_empty());
    assert!(lock(&host.views).is_empty());
}

#[test]
fn saving_adds_or_edits_and_never_echoes_the_library_back() {
    let (host, store) = fake(vec![item("a", "A", "old", "all"), item("s", "S", "API_KEY=abc123", "all")]);
    let added = save(host.as_ref(), "o", &json!([{ "title": "T", "body": "B", "scope": "all" }]));
    assert_eq!(added, json!({ "raw": { "ok": true, "id": "new2" } }));
    let edited = save(host.as_ref(), "o", &json!([{ "id": "a", "title": "A2", "body": "new", "scope": "all" }]));
    assert_eq!(edited, json!({ "raw": { "ok": true, "id": "a" } }));
    assert_eq!(lock(&store)[0]["body"], "new");
    assert_eq!(lock(&store).len(), 3);
    assert!(!added.to_string().contains("abc123"));
}

#[test]
fn a_phone_cannot_save_secret_text_nor_touch_a_desktop_only_item() {
    let (host, store) = fake(vec![item("s", "S", "API_KEY=abc123", "all")]);
    for a in [json!([{ "title": "T", "body": "password: hunter2xx" }]), json!([{ "title": "token=abcdef", "body": "x" }])] {
        assert_eq!(save(host.as_ref(), "o", &a), json!({ "raw": { "ok": false, "error": "secret" } }));
    }
    let hit = save(host.as_ref(), "o", &json!([{ "id": "s", "title": "S", "body": "harmless", "scope": "all" }]));
    assert_eq!(hit, json!({ "raw": { "ok": false, "error": "not-found" } }));
    let del = delete(host.as_ref(), "o", &json!([{ "id": "s" }]));
    assert_eq!(del, json!({ "raw": { "ok": false, "error": "not-found" } }));
    assert_eq!(lock(&store).len(), 1);
    assert_eq!(lock(&store)[0]["body"], "API_KEY=abc123");
}

#[test]
fn desk_limit_errors_reach_the_phone_unchanged() {
    let host = TestHost::default();
    *lock(&host.viewer) = Some(Box::new(|_, channel, _, _| match channel {
        "ccm:prompts-get" => json!({ "raw": { "ok": true, "items": [] } }),
        _ => json!({ "raw": { "ok": false, "error": "too-many" } }),
    }));
    assert_eq!(save(&host, "o", &json!([{ "title": "T", "body": "B", "scope": "all" }])), json!({ "raw": { "ok": false, "error": "too-many" } }));
    assert_eq!(delete(&host, "o", &json!([{ "id": "zz" }])), json!({ "raw": { "ok": false, "error": "too-many" } }));
}

#[test]
fn changed_events_merge_into_one_message_that_keeps_the_latest_count_and_the_order_of_the_rest() {
    let out = merge(vec![
        VEvent::Git { root: "/a".into() },
        VEvent::Prompts { count: 1 },
        VEvent::Fs { root: "/a".into(), paths: vec!["/a/1".into()] },
        VEvent::Prompts { count: 4 },
        VEvent::Git { root: "/a".into() },
    ]);
    assert_eq!(out.len(), 3);
    assert_eq!(out[0]["t"], "git.changed");
    assert_eq!(out[1], json!({ "t": "prompts", "count": 4 }));
    assert_eq!(out[2], json!({ "t": "fs.changed", "root": "/a", "paths": ["/a/1"] }));
}

#[test]
fn a_phone_manages_prompts_end_to_end_through_the_real_desk_and_hears_changes() {
    let rig = rig("prompts-e2e", "", false);
    rig.remote.enable(true);
    engine(&rig, vec![]);
    let ctx = rig.remote.ctx().unwrap();
    ctx.publish(summary());
    runtime().block_on(async {
        let (mut ws, mut t) = open(&rig, "pp").await;
        let (mut ws2, mut t2) = open(&rig, "pq").await;
        let empty = ask(&mut ws, &mut t, "l0", json!({ "t": "prompts.list", "paneKey": "p1" })).await;
        assert_eq!((empty["ok"].clone(), empty["data"]["items"].clone(), empty["data"]["hidden"].clone()), (json!(true), json!([]), json!(0)), "{empty}");

        let saved = ask(&mut ws, &mut t, "s1", json!({ "t": "prompts.save", "title": "Review", "body": "Review {project} on {branch}", "scope": "all" })).await;
        assert_eq!(saved["ok"], true, "{saved}");
        let id = saved["data"]["id"].as_str().unwrap().to_owned();
        assert!(saved["data"].get("items").is_none());

        let bad = ask(&mut ws, &mut t, "s2", json!({ "t": "prompts.save", "title": "", "body": "x", "scope": "all" })).await;
        assert_eq!((bad["ok"].clone(), bad["data"]["error"].clone()), (json!(true), json!("invalid-title")), "{bad}");
        let secret = ask(&mut ws, &mut t, "s3", json!({ "t": "prompts.save", "title": "Env", "body": "API_KEY=abc123 x", "scope": "all" })).await;
        assert_eq!(secret["data"]["error"], "secret", "{secret}");

        let listed = ask(&mut ws, &mut t, "l1", json!({ "t": "prompts.list", "paneKey": "p1" })).await;
        assert_eq!(listed["data"]["items"][0]["id"], id.as_str());
        assert_eq!(listed["data"]["items"][0]["text"], "Review api on feat/x");
        assert_eq!(listed["data"]["items"][0]["body"], "Review {project} on {branch}");

        let _ = ask(&mut ws2, &mut t2, "x", json!({ "t": "run.devices" })).await;
        ctx.viewer.push_prompts(2);
        let ev = until_json(&mut ws, &mut t, |v| v["t"] == "prompts").await;
        assert_eq!(ev["count"], 2);
        assert!(tokio::time::timeout(std::time::Duration::from_millis(600), until_json(&mut ws2, &mut t2, |v| v["t"] == "prompts")).await.is_err());

        let edited = ask(&mut ws, &mut t, "e1", json!({ "t": "prompts.save", "promptId": id, "title": "Review 2", "body": "again", "scope": "all" })).await;
        assert_eq!(edited["data"]["id"], id.as_str(), "{edited}");
        let gone = ask(&mut ws, &mut t, "d1", json!({ "t": "prompts.delete", "promptId": id })).await;
        assert_eq!(gone["ok"], true, "{gone}");
        let again = ask(&mut ws, &mut t, "d2", json!({ "t": "prompts.delete", "promptId": id })).await;
        assert_eq!(again["data"]["error"], "not-found", "{again}");
        let invalid = ask(&mut ws, &mut t, "d3", json!({ "t": "prompts.delete", "promptId": "a b" })).await;
        assert_eq!(invalid["error"], "invalid");

        rig.host.view_only.store(true, Ordering::SeqCst);
        let denied = ask(&mut ws, &mut t, "v1", json!({ "t": "prompts.save", "title": "T", "body": "B", "scope": "all" })).await;
        assert_eq!(denied["error"], "denied", "{denied}");
        let still = ask(&mut ws, &mut t, "v2", json!({ "t": "prompts.list" })).await;
        assert_eq!(still["ok"], true, "{still}");
        rig.host.view_only.store(false, Ordering::SeqCst);

        let mut limited = 0;
        for i in 0..WRITE_PER_MINUTE + 2 {
            let r = ask(&mut ws, &mut t, &format!("q{i}"), json!({ "t": "prompts.save", "title": format!("T{i}"), "body": "b", "scope": "all" })).await;
            if r["error"] == "rate" {
                limited += 1;
            }
        }
        assert!(limited >= 1, "{limited}");
    });
    rig.remote.enable(false);
}

#[test]
fn editing_an_item_that_was_deleted_meanwhile_fails_instead_of_bringing_it_back() {
    let (host, store) = fake(vec![item("a", "A", "b", "all")]);
    let gone = save(host.as_ref(), "o", &json!([{ "id": "zzz", "title": "T", "body": "B", "scope": "all" }]));
    assert_eq!(gone, json!({ "raw": { "ok": false, "error": "not-found" } }));
    assert_eq!(lock(&store).len(), 1);
}
