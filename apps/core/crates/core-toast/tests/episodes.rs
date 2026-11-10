mod common;

use common::{iso, t0, term, Rig};
use core_store::extra::PlanLimits;
use core_store::num::JsNum;
use core_toast::{AgentNote, DiskAlert, NotifyPatch, PhoneEpisode, ToastSnapshot};
use serde_json::{json, Value};

const GB: f64 = 1024.0 * 1024.0 * 1024.0;

fn kinds(pairs: &[(&str, bool)]) -> NotifyPatch {
    NotifyPatch { kinds: pairs.iter().map(|(k, v)| ((*k).to_string(), *v)).collect(), ..Default::default() }
}

fn disk(label: &str, level: &str, free: f64, total: f64) -> DiskAlert {
    DiskAlert { label: label.into(), level: level.into(), free_bytes: free, total_bytes: total }
}

fn note(episode: &str, level: &str, text: &str, merged: f64) -> AgentNote {
    serde_json::from_value(json!({ "type": "agent.note", "episode": episode, "level": level, "text": text, "name": "api", "merged": merged })).unwrap()
}

#[test]
fn disk_shows_the_toast_only_danger_goes_to_the_phone_kind_off_silences_both() {
    let mut r = Rig::new(true);
    r.t.phone_on = true;
    r.t.observe_disk(&disk("C:", "warn", 8.0 * GB, 500.0 * GB));
    r.t.observe_disk(&disk("C:", "danger", 1.5 * GB, 500.0 * GB));
    r.flush();
    assert_eq!(r.titles(), ["Ổ C: sắp đầy: còn 8.0 GB", "Ổ C: sắp đầy: còn 1.5 GB"]);
    assert_eq!(r.phone.len(), 1);
    assert_eq!((r.phone[0].reason, r.phone[0].name.as_str(), r.phone[0].free_bytes), ("disk", "C:", Some(JsNum(1.5 * GB))));
    r.t.update(&kinds(&[("disk", false)]));
    r.t.observe_disk(&disk("D:", "danger", 1.0, 2.0));
    r.flush();
    assert_eq!(r.shown.len(), 2);
    assert_eq!(r.phone.len(), 1);
}

#[test]
fn notes_attention_follows_needs_you_done_follows_the_done_switch_info_never_toasts() {
    let mut r = Rig::new(true);
    r.t.phone_on = true;
    r.t.observe_note(&note("e1", "info", "fyi", 0.0), None);
    r.t.observe_note(&note("e2", "attention", "Pick a schema", 2.0), None);
    r.t.observe_note(&note("e3", "done", "Tests pass", 0.0), None);
    r.flush();
    let got: Vec<(&str, &str)> = r.shown.iter().map(|c| (c.title.as_str(), c.body.as_str())).collect();
    assert_eq!(got, [("api cần chủ nhân xem", "Pick a schema (+2 tin nữa)")]);
    assert_eq!(r.phone.len(), 1);
    assert_eq!((r.phone[0].reason, r.phone[0].name.as_str(), r.phone[0].text.as_deref()), ("note", "api", Some("Pick a schema (+2 tin nữa)")));
    r.t.update(&kinds(&[("done", true), ("waiting", false)]));
    r.t.observe_note(&note("e4", "done", "Tests pass", 0.0), None);
    r.t.observe_note(&note("e5", "attention", "again", 0.0), None);
    r.flush();
    assert_eq!(r.titles(), ["api cần chủ nhân xem", "api báo đã xong"]);
    assert_eq!(r.phone.len(), 2);
}

fn resets() -> String {
    iso(t0() + 4 * 3_600_000 + 1_800_000)
}

fn hit(extra: Value) -> Value {
    let mut v = json!({ "at": iso(t0()), "resetsAt": resets(), "window": "five_hour" });
    if let (Some(o), Value::Object(e)) = (v.as_object_mut(), extra) {
        o.extend(e);
    }
    v
}

fn limited(id: &str, status: &str, h: Value) -> core_store::TerminalSession {
    term(id, status, json!({ "limitHit": h }))
}

#[test]
fn limit_sends_the_window_percent_and_reset_time_with_the_forecast() {
    let mut r = Rig::new(true);
    r.t.phone_on = true;
    let at = |pct: f64, in_min: i64| -> PlanLimits {
        serde_json::from_value(json!({ "updatedAt": iso(t0()), "fiveHour": { "usedPct": pct, "resetsAt": resets(), "forecast": { "pctPerHour": 40, "fullAt": iso(t0() + in_min * 60_000), "beforeReset": true } } })).unwrap()
    };
    for l in [at(50.0, 120), at(82.0, 30)] {
        r.t.observe(&ToastSnapshot { limits: Some(Some(&l)), ..Default::default() });
    }
    r.flush();
    assert_eq!(r.phone.len(), 1);
    let p: &PhoneEpisode = &r.phone[0];
    assert_eq!((p.reason, p.window.as_deref(), p.used_pct, p.resets_at), ("limit", Some("five_hour"), Some(JsNum(82.0)), Some(JsNum((t0() + 16_200_000) as f64))));
    assert_eq!(p.hit, None);
}

#[test]
fn limit_hit_once_per_window_and_reset_however_many_sessions_and_not_one_already_there_at_first_sight() {
    let mut r = Rig::new(true);
    r.t.phone_on = true;
    r.see(&[limited("old", "idle", hit(json!({ "resetsAt": iso(t0() + 3_600_000) })))]);
    assert!(r.phone.is_empty());
    r.see(&[term("a", "working", json!({})), term("b", "working", json!({}))]);
    r.see(&[limited("a", "idle", hit(json!({}))), limited("b", "idle", hit(json!({})))]);
    assert_eq!(r.phone.len(), 1);
    let p = &r.phone[0];
    assert_eq!((p.reason, p.hit, p.used_pct, p.window.as_deref(), p.resets_at), ("limit", Some(true), Some(JsNum(100.0)), Some("five_hour"), Some(JsNum((t0() + 16_200_000) as f64))));
    r.see(&[limited("a", "idle", hit(json!({})))]);
    assert_eq!(r.phone.len(), 1);
    r.see(&[term("c", "working", json!({}))]);
    r.see(&[limited("c", "idle", hit(json!({ "window": "seven_day", "resetsAt": iso(t0() + 4 * 86_400_000) })))]);
    assert_eq!(r.phone.len(), 2);
    assert_eq!(r.phone[1].window.as_deref(), Some("seven_day"));
}

#[test]
fn limit_ignores_a_hit_whose_reset_time_has_passed_and_one_with_the_limit_switch_off() {
    let mut r = Rig::new(true);
    r.t.phone_on = true;
    r.see(&[term("a", "working", json!({}))]);
    r.see(&[limited("a", "idle", hit(json!({ "resetsAt": iso(t0() - 3_600_000) })))]);
    assert!(r.phone.is_empty());
    r.t.update(&kinds(&[("limit", false)]));
    r.see(&[term("b", "working", json!({}))]);
    r.see(&[limited("b", "idle", hit(json!({ "resetsAt": iso(t0() + 86_400_000) })))]);
    assert!(r.phone.is_empty());
}

#[test]
fn telegram_and_phone_episodes_for_waits_questions_and_long_jobs() {
    let mut r = Rig::new(true);
    r.t.phone_on = true;
    r.t.telegram_on = true;
    let cwd = json!({ "cwd": "D:\\work\\shop", "statusSince": iso(t0()) });
    let a = |status: &str, extra: Value| {
        let mut v = cwd.clone();
        if let (Some(o), Value::Object(e)) = (v.as_object_mut(), extra) {
            o.extend(e);
        }
        term("a", status, v)
    };
    let ask = json!({ "question": "Which?", "options": ["A", "B"], "multi": false });
    let reply = |_: &str| Some("All done".to_string());
    let ask_of = |_: &str| Some(ask.clone());
    let tool = |_: &str| Some("AskUserQuestion".to_string());
    let steps = [a("working", json!({})), a("waiting", json!({ "waitingFor": "input needed", "statusSince": iso(t0() + 1000) })), a("working", json!({ "statusSince": iso(t0() + 2000) }))];
    for t in &steps {
        r.t.observe(&ToastSnapshot { terminals: vec![t], reply_of: Some(&reply), ask_of: Some(&ask_of), tool_of: Some(&tool), ..Default::default() });
    }
    r.advance(6 * 60_000);
    let done = a("idle", json!({ "statusSince": iso(t0() + 6 * 60_000) }));
    r.t.observe(&ToastSnapshot { terminals: vec![&done], reply_of: Some(&reply), ..Default::default() });
    r.flush();
    let tg: Vec<Value> = r.telegram.iter().map(|m| serde_json::to_value(m).unwrap()).collect();
    assert_eq!(
        tg,
        [
            json!({ "episode": format!("tw:a|{}", iso(t0() + 1000)), "reason": "question", "terminalId": "a", "name": "shop", "cwd": "D:\\work\\shop", "since": t0() + 1000, "tool": "AskUserQuestion", "ask": ask }),
            json!({ "episode": format!("td:a|{}", iso(t0() + 6 * 60_000)), "reason": "done", "terminalId": "a", "name": "shop", "cwd": "D:\\work\\shop", "worked": 6 * 60_000 - 2000, "reply": "All done" }),
        ]
    );
    let ph: Vec<(&str, String)> = r.phone.iter().map(|p| (p.reason, p.episode.clone())).collect();
    assert_eq!(ph, [("question", format!("w:a|{}", iso(t0() + 1000))), ("done", format!("d:a|{}", iso(t0() + 6 * 60_000)))]);
    assert_eq!(r.phone[1].since, Some(JsNum((t0() + 2000) as f64)));
}

#[test]
fn idle_report_toasts_once_per_new_forgotten_item_after_the_first_report() {
    let mut r = Rig::new(true);
    r.see(&[term("s1", "idle", json!({ "alias": "Shop" }))]);
    let report = |servers: Value| json!({ "idleMinutes": 60, "totals": { "forgottenRamBytes": 1.25 * GB }, "sessions": [{ "sessionId": "s1", "forgotten": true, "idleSinceMs": 5 }], "servers": servers });
    r.t.observe_idle(&report(json!([])));
    r.flush();
    assert!(r.shown.is_empty());
    let gradle = json!([{ "pid": 7, "start": "99", "name": "java.exe", "forgotten": true, "idleSinceMs": 9, "daemon": { "type": "gradle", "version": "8.14" } }]);
    r.t.observe_idle(&report(gradle.clone()));
    r.t.observe_idle(&report(gradle));
    r.flush();
    let got: Vec<(&str, &str)> = r.shown.iter().map(|c| (c.title.as_str(), c.body.as_str())).collect();
    assert_eq!(got, [("2 mục bỏ quên đang dùng 1,3 GB", "Shop, Gradle daemon 8.14. Mở Orrery để xem và tắt.")]);
    r.t.observe_idle(&json!({ "idleMinutes": 60, "totals": { "forgottenRamBytes": 0 }, "sessions": [], "servers": [] }));
    r.flush();
    assert_eq!(r.calls.last().unwrap(), &format!("hide:{}", common::tag("idle:all")));
}
