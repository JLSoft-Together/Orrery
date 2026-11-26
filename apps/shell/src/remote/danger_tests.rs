use serde_json::{json, Value};

use super::danger::{default_rules, matching, needs_auth, rules_json, valid_rules, MAX_RULES, MAX_RULE_LEN};
use super::devices::Device;
use super::test_util::*;
use crate::state::lock;

fn hit(text: &str) -> bool {
    matching(&default_rules(), text).is_some()
}

#[test]
fn every_default_rule_matches_its_command_and_ignores_safe_ones() {
    for text in [
        "git push -f",
        "git push origin main --force",
        "GIT PUSH --FORCE",
        "  git push -f origin  ",
        "git reset --hard HEAD~1",
        "rm -rf /tmp/x",
        "rm -fr build",
        "sudo rm -Rf node_modules",
        "git clean -fd",
        "git clean -xdf",
        "DROP TABLE users;",
        "drop database prod",
        "git branch -D old",
    ] {
        assert!(hit(text), "{text}");
    }
    for text in [
        "git push",
        "git push --force-with-lease",
        "git push origin main --force-with-lease",
        "git reset --soft HEAD~1",
        "rm file.txt",
        "rm -r dir",
        "git clean -n",
        "select * from tables",
        "git branch feature",
        "echo hello",
        "",
    ] {
        assert!(!hit(text), "{text}");
    }
}

#[test]
fn invalid_long_duplicate_and_surplus_rules_are_dropped() {
    let mut rules = vec!["(unclosed".to_owned(), String::new(), "a".repeat(MAX_RULE_LEN + 1), "ok1".to_owned(), "ok1".to_owned(), "[".to_owned()];
    assert_eq!(valid_rules(&rules), vec!["ok1".to_owned()]);
    assert!(matching(&rules, "ok1 now").is_some());
    rules = (0..50).map(|i| format!("rule{i}x")).collect();
    assert_eq!(valid_rules(&rules).len(), MAX_RULES);
    assert_eq!(rules_json(&["(".to_owned(), "x".to_owned()]), json!(["x"]));
    assert!(matching(&["a".repeat(MAX_RULE_LEN)], &"a".repeat(MAX_RULE_LEN)).is_some());
}

#[test]
fn needs_auth_blocks_unless_authed_and_ignores_other_text() {
    let rules = default_rules();
    let blocked = needs_auth(&rules, &json!({ "t": "ws.send", "paneKey": "p1", "text": "rm -rf /" }), false).unwrap();
    assert_eq!((blocked["ok"].clone(), blocked["error"].clone()), (json!(false), json!("needsAuth")));
    assert!(blocked["rule"].as_str().is_some_and(|r| r.contains("rm")));
    assert!(blocked["message"].as_str().unwrap().contains("authentication"));
    let vi = needs_auth(&rules, &json!({ "text": "rm -rf /" }), true).unwrap();
    assert!(vi["message"].as_str().unwrap().contains("xác thực"));
    assert!(needs_auth(&rules, &json!({ "text": "rm -rf /", "authed": true }), false).is_none());
    assert!(needs_auth(&rules, &json!({ "text": "rm -rf /", "authed": "true" }), false).is_some());
    assert!(needs_auth(&rules, &json!({ "text": "ls" }), false).is_none());
    assert!(needs_auth(&rules, &json!({ "paneKey": "p1" }), false).is_none());
    assert!(needs_auth(&[], &json!({ "text": "rm -rf /" }), false).is_none());
}

#[test]
fn ws_send_without_authed_is_refused_and_never_reaches_the_pane() {
    let rig = rig("danger-send", "", false);
    rig.remote.enable(true);
    let phone = Phone::new("p");
    let ctx = rig.remote.ctx().unwrap();
    assert!(lock(&ctx.devices).add(Device::new(&phone.id, "Phone", phone.key.public(), super::config::now_secs())));
    *lock(&rig.host.answer_with) = Some(Box::new(|action, _| (action == "ws.send").then(|| json!({ "ok": true }))));
    let desktop = rig.desktop_key();
    runtime().block_on(async {
        let (mut ws, mut t) = session(rig.addr(), &desktop, &phone).await.unwrap();
        let welcome = hello(&mut ws, &mut t).await;
        assert_eq!(welcome["dangerRules"].as_array().map(Vec::len), Some(default_rules().len()));
        send_json(&mut ws, &mut t, json!({ "id": "a", "t": "ws.send", "paneKey": "p1", "text": "git push --force" })).await;
        let denied = until_json(&mut ws, &mut t, |v| v["re"] == "a").await;
        assert_eq!((denied["ok"].clone(), denied["error"].clone()), (json!(false), json!("needsAuth")));
        assert!(denied["rule"].is_string() && denied["message"].is_string());
        send_json(&mut ws, &mut t, json!({ "id": "b", "t": "ws.send", "paneKey": "p1", "text": "git push --force", "authed": true })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "b").await["ok"], true);
        send_json(&mut ws, &mut t, json!({ "id": "c", "t": "ws.send", "paneKey": "p1", "text": "ls -la" })).await;
        assert_eq!(until_json(&mut ws, &mut t, |v| v["re"] == "c").await["ok"], true);
    });
    let asks: Vec<(String, Value)> = lock(&rig.host.asks).clone();
    let sends: Vec<&Value> = asks.iter().filter(|(a, _)| a == "ws.send").map(|(_, v)| v).collect();
    assert_eq!(sends.len(), 2);
    assert_eq!(sends[0]["cmd"]["text"], "git push --force");
    assert!(sends[0]["cmd"].get("authed").is_none());
    assert_eq!(sends[1]["cmd"]["text"], "ls -la");
    rig.remote.enable(false);
}
