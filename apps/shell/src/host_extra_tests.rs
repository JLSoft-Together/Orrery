use serde_json::{json, Value};

use super::telegram::{ask_request, first_ready, fresh, orrery_action, pick_device, png_bytes, save_shot, start_result, trusted_configs, wants_device, RunLogs};
use super::text::{clip, extra_text};
use super::{bridge_command, e2e_idle, ext_plan, parse_ioreg_idle};

#[test]
fn every_bot_action_maps_to_its_page_action() {
    for (bot, page) in [("status", "session.status"), ("all", "session.sendAll"), ("close", "session.closeOwner"), ("new", "session.create"), ("resumeList", "session.resumeList"), ("answer", "session.answer")] {
        assert_eq!(orrery_action(bot), Some(page));
    }
    assert_eq!(orrery_action("rm"), None);
}

#[test]
fn the_ask_request_carries_the_whole_inbound_as_args_from_telegram() {
    let req = json!({"id": "r1", "action": "send", "project": "/p", "text": "hi"});
    let out = ask_request("x", &req).unwrap();
    assert_eq!(out["id"], "x");
    assert_eq!(out["action"], "session.send");
    assert_eq!(out["args"], req);
    assert_eq!(out["caller"], json!({"source": "telegram"}));
    assert!(ask_request("x", &json!({"action": "nope"})).is_none());
    assert!(ask_request("x", &json!({})).is_none());
}

#[test]
fn screenshots_pick_a_ready_online_device() {
    let snap = json!({"devices": [
        {"id": "A", "name": "Off", "online": false, "state": "ready"},
        {"id": "B", "name": "Pixel", "online": true, "state": "unauthorized"},
        {"id": "C", "name": "Tab", "online": true, "state": "ready"},
    ]});
    assert_eq!(pick_device(&snap, None), Some(("C".into(), "Tab".into())));
    assert_eq!(pick_device(&snap, Some("C")), Some(("C".into(), "Tab".into())));
    assert_eq!(pick_device(&snap, Some("A")), None);
    assert_eq!(pick_device(&json!({}), None), None);
}

#[test]
fn screenshot_png_is_decoded_and_saved_as_a_file() {
    let png = [0x89u8, 0x50, 0x4e, 0x47, 1, 2, 3];
    let url = format!("data:image/png;base64,{}", base64::Engine::encode(&base64::engine::general_purpose::STANDARD, png));
    let bytes = png_bytes(&url).unwrap();
    assert_eq!(bytes, png);
    assert!(png_bytes("data:image/png;base64,aGVsbG8=").is_none());
    let dir = std::env::temp_dir().join(format!("orrery-host-extra-shot-{}", std::process::id()));
    let file = save_shot(&dir, &bytes).unwrap();
    assert_eq!(std::fs::read(&file).unwrap(), png);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn run_configs_drop_the_ones_that_need_trust() {
    let list = json!([
        {"id": "a", "name": "App", "shared": false},
        {"id": "b", "name": "Shared", "shared": true},
        {"id": "imp-1", "name": "Imported", "shared": false},
        {"id": "c"},
    ]);
    assert_eq!(trusted_configs(&list), vec![json!({"id": "a", "name": "App"})]);
    assert!(trusted_configs(&Value::Null).is_empty());
}

#[test]
fn run_start_picks_a_device_only_for_selected_device_configs() {
    assert!(wants_device(&json!({"type": "flutter", "device": "selected"})));
    assert!(!wants_device(&json!({"type": "custom", "device": "selected"})));
    assert!(!wants_device(&json!({"type": "android", "device": "all"})));
    assert_eq!(first_ready(&json!([{"id": "x", "state": "offline"}, {"id": "y", "state": "ready"}])), Some("y".into()));
    assert_eq!(first_ready(&json!([])), None);
}

#[test]
fn run_start_results_read_like_electron() {
    assert_eq!(start_result(&json!({"ok": true, "sessionId": "s1"})), json!({"ok": true, "sessionId": "s1"}));
    assert_eq!(start_result(&json!({"ok": true})), json!({"ok": false, "error": "failed"}));
    assert_eq!(start_result(&json!({"ok": false, "error": {"code": "denied"}})), json!({"ok": false, "error": "denied"}));
    assert_eq!(start_result(&Value::Null), json!({"ok": false, "error": "failed"}));
}

#[test]
fn run_logs_keep_the_last_200_lines_of_the_last_20_sessions() {
    let mut logs = RunLogs::default();
    let lines: Vec<Value> = (0..250).map(|i| json!({"text": format!("l{i}")})).collect();
    logs.push("s0", &lines);
    let got = logs.get("s0");
    assert_eq!(got.len(), 200);
    assert_eq!(got[0], "l50");
    logs.push("s0", &[json!("plain")]);
    assert_eq!(logs.get("s0").last().unwrap(), "plain");
    for i in 1..=20 {
        logs.push(&format!("s{i}"), &[json!("x")]);
    }
    assert!(logs.get("s0").is_empty());
    assert_eq!(logs.get("s20"), vec!["x".to_owned()]);
}

#[test]
fn a_run_event_sent_to_two_windows_counts_once() {
    let mut last = String::new();
    assert!(fresh(&mut last, "{\"a\":1}"));
    assert!(!fresh(&mut last, "{\"a\":1}"));
    assert!(fresh(&mut last, "{\"a\":2}"));
}

#[test]
fn ioreg_idle_reads_nanoseconds() {
    let out = "+-o IOHIDSystem\n    {\n      \"HIDIdleTime\" = 2500000000\n    }\n";
    assert_eq!(parse_ioreg_idle(out), Some(2.5));
    assert_eq!(parse_ioreg_idle("nothing"), None);
}

#[test]
fn e2e_idle_needs_a_finite_non_negative_number() {
    assert_eq!(e2e_idle(Some("90".into())), Some(90.0));
    assert_eq!(e2e_idle(Some("".into())), None);
    assert_eq!(e2e_idle(Some("-1".into())), None);
    assert_eq!(e2e_idle(Some("x".into())), None);
    assert_eq!(e2e_idle(None), None);
}

#[test]
fn bridge_commands_come_from_the_install_result() {
    let v = json!({"ok": true, "statusline": {"command": "\"/b\" statusline-bridge"}, "hook": {"command": "\"/b\" hook-bridge"}});
    assert_eq!(bridge_command(&v, "statusline"), json!("\"/b\" statusline-bridge"));
    assert_eq!(bridge_command(&v, "hook"), json!("\"/b\" hook-bridge"));
    assert_eq!(bridge_command(&json!({}), "hook"), Value::Null);
}

#[test]
fn extension_plans_are_read_leniently() {
    let (kind, commands, scope, dir) = ext_plan(&json!({"kind": "weird", "commands": ["a", 1, "b"], "scope": "team", "dir": "/p"}));
    assert_eq!(kind, "install");
    assert_eq!(commands, vec!["a".to_owned(), "b".to_owned()]);
    assert_eq!(scope, None);
    assert_eq!(dir.as_deref(), Some("/p"));
    let t = extra_text("en");
    assert_eq!(t.ext_where(Some("project"), Some("/p")), "/p, shared with the team (.mcp.json / .claude/settings.json, usually committed)");
    assert_eq!(t.ext_where(Some("project"), None), "Claude Code (user settings)");
    let (title, detail, ok) = t.ext("remove", "x", &["c1".into(), "c2".into()]);
    assert_eq!((title.as_str(), ok.as_str()), ("Remove from Claude Code?", "Run"));
    assert!(detail.contains("c1\n\nc2"));
}

#[test]
fn texts_follow_the_locale() {
    let (en, vi) = (extra_text("en"), extra_text("vi"));
    assert_eq!(en.cancel(), "Cancel");
    assert_eq!(vi.cancel(), "Huỷ");
    assert_eq!(en.tg_command(true, "/f").0, "Install the /bot-tele-receive command?");
    assert!(vi.tg_command(false, "/f").1.contains("/f"));
    assert_eq!(en.hooks("/s", "{}", false).2, "Continue");
    assert_eq!(en.hook_toggle(false, "Stop", "/a/settings.json", "d").1.split("\n\n").next().unwrap(), "Orrery will write /a/settings.json. Other keys stay; the old file is kept as settings.json.ccm-backup. The hook is kept in Orrery so it can be turned back on.");
    let items = [json!({"name": "node", "session": true}), json!({"name": "java"})];
    let (title, detail, ok) = en.res_kill(&items, &["a".into()]);
    assert_eq!((title.as_str(), ok.as_str()), ("Stop 2 forgotten processes?", "Stop all"));
    assert!(detail.ends_with("claude --resume."));
    assert_eq!(vi.res_kill(&items[..1], &[]).0, "Tắt node?");
}

#[test]
fn deepseek_plans_get_their_own_text() {
    let t = extra_text("en");
    assert_eq!(t.deepseek(&json!({"kind": "logout"})).2, "Log out");
    assert_eq!(t.deepseek(&json!({"kind": "download", "commands": ["x"]})).0, "Download the Codewhale program?");
    let sync = t.deepseek(&json!({"kind": "sync", "files": [{"file": "a", "action": "create"}], "mcpAdded": ["m"], "outboxAdded": true}));
    assert!(sync.1.starts_with("Create a\n\nMCP servers added to ~/.codewhale/mcp.json: m\n\nconfig.toml"));
    let long = "y".repeat(400);
    assert_eq!(clip(&long).chars().count(), 301);
    assert_eq!(clip("short"), "short");
}

#[test]
fn vietnamese_texts_avoid_banned_words() {
    let vi = extra_text("vi");
    let all = [
        vi.tg_command(true, "f").1,
        vi.statusline("f", "{}", true).1,
        vi.hooks("f", "{}", true).1,
        vi.hook_test("e", "t", "c").1,
        vi.deepseek(&json!({"kind": "sync", "outboxAdded": true})).1,
    ];
    for s in all {
        assert!(!s.contains("trần") && !s.contains("watchdog"), "{s}");
    }
}
