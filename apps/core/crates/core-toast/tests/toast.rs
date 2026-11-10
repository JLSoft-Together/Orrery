mod common;

use std::sync::atomic::Ordering;
use std::sync::Arc;

use common::{iso, t0, tag, term, w, Rig};
use core_store::extra::{BackgroundJob, PlanLimits, QuietHours, ToastHealth, ToastState, ToastTestResult};
use core_store::model::ActivityEvent;
use core_store::persist::notify_file::ToastLocale;
use core_store::persist::snooze::SnoozeStore;
use core_toast::{toast_content, NotifyPatch, ToastOptions, ToastSnapshot, Toaster, TOAST_THROTTLE_MS};
use serde_json::{json, Value};

fn kinds(pairs: &[(&str, bool)]) -> NotifyPatch {
    NotifyPatch { kinds: pairs.iter().map(|(k, v)| ((*k).to_string(), *v)).collect(), ..Default::default() }
}

#[test]
fn shows_a_toast_when_a_known_session_starts_waiting_and_hides_it_when_it_moves_on() {
    let mut r = Rig::new(true);
    r.see(&[term("a", "working", json!({}))]);
    r.see(&[term("a", "waiting", json!({ "waitingFor": "permission" }))]);
    r.see(&[term("a", "working", json!({}))]);
    assert_eq!(r.calls, [format!("show:{}", w("a")), format!("hide:{}", w("a"))]);
    assert_eq!(r.shown[0].title, "title-a đang chờ chủ nhân");
    assert_eq!(r.shown[0].body, "Đang chờ permission");
    assert!(r.shown[0].persistent);
    assert_eq!(r.shown[0].url, format!("http://127.0.0.1:4317/focus?k={}&t=a", r.t.key()));
}

#[test]
fn stays_quiet_for_sessions_already_waiting_when_first_seen() {
    let mut r = Rig::new(true);
    r.see(&[term("a", "waiting", json!({}))]);
    r.see(&[term("a", "waiting", json!({}))]);
    assert!(r.calls.is_empty());
}

#[test]
fn skips_while_a_dashboard_tab_is_in_front_of_the_user() {
    let mut r = Rig::new(true);
    r.t.set_presence(1, true, None);
    r.see(&[term("a", "idle", json!({}))]);
    r.see(&[term("a", "waiting", json!({}))]);
    r.t.drop_client(1);
    r.see(&[term("a", "idle", json!({})), term("b", "idle", json!({}))]);
    r.see(&[term("a", "idle", json!({})), term("b", "waiting", json!({}))]);
    assert_eq!(r.calls, [format!("show:{}", w("b"))]);
}

#[test]
fn hides_open_toasts_when_the_user_comes_back_to_the_dashboard() {
    let mut r = Rig::new(true);
    r.see(&[term("a", "idle", json!({}))]);
    r.see(&[term("a", "waiting", json!({}))]);
    r.t.set_presence(2, true, Some(ToastLocale::En));
    r.flush();
    assert_eq!(r.calls, [format!("show:{}", w("a")), format!("hide:{}", w("a"))]);
}

#[test]
fn throttles_repeat_toasts_per_session() {
    let mut r = Rig::new(true);
    r.see(&[term("a", "idle", json!({}))]);
    r.see(&[term("a", "waiting", json!({}))]);
    r.see(&[term("a", "working", json!({}))]);
    r.see(&[term("a", "waiting", json!({}))]);
    r.advance(TOAST_THROTTLE_MS as i64);
    r.see(&[term("a", "working", json!({}))]);
    r.see(&[term("a", "waiting", json!({}))]);
    assert_eq!(r.shows(), 2);
}

#[test]
fn does_nothing_when_turned_off_or_unsupported_and_turning_off_hides_open_toasts() {
    let mut r = Rig::new(true);
    r.see(&[term("a", "idle", json!({}))]);
    r.see(&[term("a", "waiting", json!({}))]);
    r.t.set_enabled(false);
    assert_eq!(r.t.state(), ToastState::Off);
    r.see(&[term("b", "idle", json!({}))]);
    r.see(&[term("b", "waiting", json!({}))]);
    assert_eq!(r.calls, [format!("show:{}", w("a")), format!("hide:{}", w("a"))]);

    let mut other = Rig::new(false);
    assert_eq!(other.t.state(), ToastState::Unsupported);
    other.t.update(&NotifyPatch { toast: Some(true), ..Default::default() });
    assert_eq!(other.t.state(), ToastState::Unsupported);
    other.see(&[term("a", "idle", json!({}))]);
    other.see(&[term("a", "waiting", json!({}))]);
    assert!(other.calls.is_empty());
}

#[test]
fn hides_the_toast_of_a_removed_session_and_only_accepts_its_own_link_key() {
    let mut r = Rig::new(true);
    r.see(&[term("a", "idle", json!({}))]);
    r.see(&[term("a", "waiting", json!({}))]);
    r.see(&[]);
    assert_eq!(r.calls, [format!("show:{}", w("a")), format!("hide:{}", w("a"))]);
    let key = r.t.key().to_string();
    assert!(r.t.accepts(Some(&key)));
    assert!(!r.t.accepts(Some("nope")));
    assert!(!r.t.accepts(None));
    assert!(!r.t.accepts(Some(&key[1..])));
    assert_eq!(r.t.link_for(None), format!("http://127.0.0.1:4317/focus?k={key}"));
}

#[test]
fn builds_english_text_with_the_alias_first() {
    let c = toast_content(&term("a", "waiting", json!({ "alias": "API fix" })), ToastLocale::En, "http://x", None);
    assert_eq!(c.title, "API fix needs you");
    assert_eq!(c.body, "Claude is waiting for your input");
    assert_eq!(c.open, "Open terminal");
}

#[test]
fn labels_permission_and_question_waits_with_the_open_tool_when_known() {
    let perm = term("a", "waiting", json!({ "waitingFor": "permission prompt" }));
    let en = ToastLocale::En;
    assert_eq!(toast_content(&perm, en, "http://x", Some("Bash")).body, "Needs approval: Bash");
    assert_eq!(toast_content(&perm, en, "http://x", None).body, "Needs approval");
    assert_eq!(toast_content(&perm, en, "http://x", Some("ExitPlanMode")).body, "Plan needs approval");
    assert_eq!(toast_content(&perm, ToastLocale::Vi, "http://x", Some("Bash")).body, "Cần duyệt: Bash");
    assert_eq!(toast_content(&term("a", "waiting", json!({ "waitingFor": "input needed" })), en, "http://x", None).body, "Asking you");
    assert_eq!(toast_content(&term("a", "waiting", json!({ "waitingFor": "foo" })), en, "http://x", None).body, "Waiting for foo");
}

#[test]
fn uses_tool_of_from_observe_for_the_waiting_toast_body() {
    let mut r = Rig::new(true);
    let tool = |_: &str| Some("Bash".to_string());
    for t in [term("a", "working", json!({})), term("a", "waiting", json!({ "waitingFor": "permission prompt" }))] {
        r.t.observe(&ToastSnapshot { terminals: vec![&t], tool_of: Some(&tool), ..Default::default() });
    }
    r.flush();
    assert!(r.shown[0].body.contains("Bash"));
}

#[test]
fn reminds_every_5_minutes_while_still_waiting_at_most_3_times() {
    let mut r = Rig::new(true);
    let since = iso(t0());
    r.see(&[term("a", "working", json!({}))]);
    r.see(&[term("a", "waiting", json!({ "statusSince": since }))]);
    for _ in 0..5 {
        r.advance(5 * 60_000);
        r.see(&[term("a", "waiting", json!({ "statusSince": since }))]);
    }
    assert_eq!(r.titles(), ["title-a đang chờ chủ nhân", "title-a vẫn đang chờ chủ nhân (5 phút)", "title-a vẫn đang chờ chủ nhân (10 phút)", "title-a vẫn đang chờ chủ nhân (15 phút)"]);
    assert_eq!(r.now_ms() - t0(), 25 * 60_000);
}

#[test]
fn does_not_replay_reminders_for_a_session_already_waiting_at_startup() {
    let mut r = Rig::new(true);
    let since = iso(t0() - 12 * 60_000);
    r.see(&[term("a", "waiting", json!({ "statusSince": since }))]);
    r.see(&[term("a", "waiting", json!({ "statusSince": since }))]);
    r.advance(3 * 60_000);
    r.see(&[term("a", "waiting", json!({ "statusSince": since }))]);
    assert_eq!(r.titles(), ["title-a vẫn đang chờ chủ nhân (15 phút)"]);
}

#[test]
fn respects_snooze_for_the_first_toast_and_reminders_and_hides_an_open_one() {
    let mut r = Rig::new(true);
    let clock = r.now.clone();
    let mut snoozes = SnoozeStore::with_clock(None, Box::new(move || clock.load(Ordering::SeqCst)));
    let since = iso(t0());
    let waiting = term("a", "waiting", json!({ "statusSince": since }));
    let observe = |r: &mut Rig, s: &SnoozeStore, t: &core_store::TerminalSession| {
        let f = |x: &core_store::TerminalSession| s.is_snoozed(x, None);
        r.t.observe(&ToastSnapshot { terminals: vec![t], snoozed: Some(&f), ..Default::default() });
        r.flush();
    };
    observe(&mut r, &snoozes, &term("a", "working", json!({})));
    observe(&mut r, &snoozes, &waiting);
    assert!(snoozes.set(&waiting, Some(15.0)));
    r.advance(10 * 60_000);
    observe(&mut r, &snoozes, &waiting);
    r.advance(6 * 60_000);
    observe(&mut r, &snoozes, &waiting);
    assert_eq!(r.calls, [format!("show:{}", w("a")), format!("hide:{}", w("a")), format!("show:{}", w("a"))]);
}

#[test]
fn stays_silent_during_quiet_hours() {
    let mut r = Rig::new(true);
    r.t.update(&NotifyPatch { quiet: Some(QuietHours { enabled: true, from: "09:00".into(), to: "11:00".into() }), ..Default::default() });
    r.see(&[term("a", "idle", json!({}))]);
    r.see(&[term("a", "waiting", json!({}))]);
    r.advance(2 * 3_600_000);
    r.see(&[term("a", "idle", json!({}))]);
    r.see(&[term("a", "waiting", json!({}))]);
    assert_eq!(r.calls, [format!("show:{}", w("a"))]);
}

#[test]
fn toasts_finished_sessions_only_when_that_kind_is_on() {
    let mut r = Rig::new(true);
    r.see(&[term("a", "working", json!({}))]);
    r.see(&[term("a", "idle", json!({}))]);
    r.t.update(&kinds(&[("done", true)]));
    r.see(&[term("a", "working", json!({}))]);
    r.see(&[term("a", "idle", json!({}))]);
    let got: Vec<(&str, bool)> = r.shown.iter().map(|c| (c.title.as_str(), c.persistent)).collect();
    assert_eq!(got, [("title-a đã xong", false)]);
    assert!(r.t.settings().kinds["done"]);
}

#[test]
fn toasts_blocked_jobs_after_the_first_sighting_and_links_to_the_owning_session() {
    let mut r = Rig::new(true);
    let owner = term("a", "working", json!({ "claudeSessionId": "s1" }));
    let job = |state: &str| -> Vec<BackgroundJob> { vec![serde_json::from_value(json!({ "id": "j1", "name": "nightly", "state": state, "sessionId": "s1" })).unwrap()] };
    for jobs in [job("blocked"), job("working"), job("blocked"), Vec::new()] {
        r.t.observe(&ToastSnapshot { terminals: vec![&owner], jobs: Some(&jobs), ..Default::default() });
    }
    r.flush();
    assert_eq!(r.titles(), ["Job nền cần chủ nhân: nightly"]);
    assert_eq!(r.shown[0].url, r.t.link_for(Some("a")));
    assert_eq!(r.calls.last().unwrap(), &format!("hide:{}", tag("job:j1")));
}

pub fn limits(pct: f64, in_min: i64, resets_at: &str) -> PlanLimits {
    serde_json::from_value(json!({
        "updatedAt": iso(t0()),
        "fiveHour": { "usedPct": pct, "resetsAt": resets_at, "forecast": { "pctPerHour": 40, "fullAt": iso(t0() + in_min * 60_000), "beforeReset": true } }
    }))
    .unwrap()
}

#[test]
fn warns_once_per_window_when_the_5_hour_limit_runs_out_within_the_hour() {
    let mut r = Rig::new(true);
    for l in [limits(50.0, 120, "2026-10-08T12:00:00Z"), limits(80.0, 30, "2026-10-08T12:00:00Z"), limits(85.0, 20, "2026-10-08T12:00:00Z")] {
        r.t.observe(&ToastSnapshot { limits: Some(Some(&l)), ..Default::default() });
    }
    r.flush();
    assert_eq!(r.titles(), ["Giới hạn 5h đã dùng 80%"]);
    assert_eq!(r.shown[0].url, r.t.link_for(None));
}

#[test]
fn toasts_a_session_stuck_in_an_error_loop_once_per_episode() {
    let mut r = Rig::new(true);
    r.see(&[term("a", "working", json!({}))]);
    let mut events: Vec<ActivityEvent> = Vec::new();
    for i in 0..6 {
        let at = iso(t0() - 60_000 + i * 1000);
        events.push(serde_json::from_value(json!({ "id": format!("s{i}"), "at": at, "kind": "tool.started", "terminalId": "a", "data": { "tool": "Bash" } })).unwrap());
        events.push(serde_json::from_value(json!({ "id": format!("f{i}"), "at": at, "kind": "tool.failed", "terminalId": "a", "data": { "tool": "Bash" } })).unwrap());
    }
    r.t.check_loops(&events);
    r.t.check_loops(&events);
    r.flush();
    let got: Vec<(&str, &str)> = r.shown.iter().map(|c| (c.title.as_str(), c.body.as_str())).collect();
    assert_eq!(got, [("title-a lỗi liên tục", "Bash lỗi 6 lần trong 5 phút.")]);
}

#[test]
fn snooze_store_holds_a_snooze_only_for_the_waiting_episode_it_was_set_on() {
    let now = Arc::new(std::sync::atomic::AtomicI64::new(t0()));
    let c = now.clone();
    let mut store = SnoozeStore::with_clock(None, Box::new(move || c.load(Ordering::SeqCst)));
    let since = iso(t0());
    let a = term("a", "waiting", json!({ "statusSince": since }));
    assert!(!store.set(&term("b", "working", json!({})), Some(15.0)));
    assert!(!store.set(&a, Some(0.0)));
    assert!(store.set(&a, Some(15.0)));
    assert!(store.is_snoozed(&a, None));
    assert!(!store.is_snoozed(&term("a", "waiting", json!({ "statusSince": "later" })), None));
    assert!(!store.prune([&a]));
    assert!(store.prune([&term("a", "working", json!({}))]));
    assert!(store.all().is_empty());
    store.set(&a, Some(15.0));
    now.fetch_add(16 * 60_000, Ordering::SeqCst);
    assert!(!store.is_snoozed(&a, None));
    assert!(store.set(&a, None));
}

#[test]
fn v11_adds_a_snooze_action_to_waiting_toasts() {
    let mut r = Rig::new(true);
    r.see(&[term("a", "idle", json!({}))]);
    r.see(&[term("a", "waiting", json!({}))]);
    assert_eq!(r.shown[0].snooze_label.as_deref(), Some("Hoãn 15 phút"));
    assert_eq!(r.shown[0].snooze_url.as_deref(), Some(format!("http://127.0.0.1:4317/snooze?k={}&t=a&m=15", r.t.key()).as_str()));
}

#[test]
fn v11_toasts_a_quiet_working_session_once_per_stretch_using_the_collector_threshold() {
    let mut r = Rig::new(true);
    let last = iso(t0());
    r.t.update(&NotifyPatch { stuck_minutes: Some(5.0), ..Default::default() });
    r.see(&[term("a", "working", json!({ "lastActivityAt": last }))]);
    r.advance(6 * 60_000);
    r.see(&[term("a", "working", json!({ "lastActivityAt": last }))]);
    r.see(&[term("a", "working", json!({ "lastActivityAt": last }))]);
    let got: Vec<(&str, &str)> = r.shown.iter().map(|c| (c.title.as_str(), c.body.as_str())).collect();
    assert_eq!(got, [("title-a không có output", "Đang chạy nhưng không có output 6 phút.")]);
    assert_eq!(r.t.settings().stuck_minutes.0, 5.0);
}

#[test]
fn v11_toasts_once_when_context_crosses_the_critical_level() {
    let mut r = Rig::new(true);
    let a = term("a", "working", json!({}));
    for pct in [80.0, 96.0, 96.0, 50.0, 97.0] {
        let f = move |_: &str| Some(pct);
        r.t.observe(&ToastSnapshot { terminals: vec![&a], context_pct: Some(&f), ..Default::default() });
    }
    r.flush();
    assert_eq!(r.titles(), ["title-a đã dùng 96% context", "title-a đã dùng 97% context"]);
}

#[test]
fn v11_sends_a_test_toast_and_reports_windows_health() {
    let mut r = Rig::with(ToastOptions { supported: true, port: 1, file: None, link: None }, t0());
    r.t.health_probe = Some(Box::new(|| ToastHealth::AppOff));
    r.t.set_presence(1, true, None);
    let out = r.t.test();
    r.flush();
    assert_eq!((out.result, out.health), (ToastTestResult::Ok, Some(ToastHealth::AppOff)));
    assert_eq!(r.shown[0].title, "Thông báo thử");
    assert_eq!(r.t.settings().health, Some(ToastHealth::AppOff));
    let mut off = Toaster::new(ToastOptions { supported: false, port: 1, file: None, link: None }, Arc::new(|| 0));
    let out = off.test();
    assert_eq!((out.result, out.health), (ToastTestResult::Unsupported, None));
}

#[test]
fn toast_link_key_survives_a_restart_so_toasts_left_in_the_action_center_keep_working() {
    let dir = std::env::temp_dir().join(format!("ccm-key-{}-{}", std::process::id(), t0()));
    let file = dir.join("notify.json");
    let options = ToastOptions { supported: true, port: 4317, file: Some(file.clone()), link: None };
    let mut first = Toaster::system(options.clone());
    first.load();
    first.flush();
    let mut second = Toaster::system(options);
    second.load();
    assert!(second.key().len() == 32 && second.key().bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));
    assert_eq!(second.key(), first.key());
    assert!(second.accepts(Some(first.key())));
    let saved: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    assert_eq!(saved["key"], json!(first.key()));
    let _ = std::fs::remove_dir_all(&dir);
}
