use serde_json::{json, Value};

use super::*;

fn parsed(v: Value) -> Option<Value> {
    parse_client_message(&v).map(|m| serde_json::to_value(m).unwrap())
}

#[test]
fn tool_spans_accepts_only_valid_fields() {
    for extra in [json!({"since": "nope"}), json!({"since": "x".repeat(41)}), json!({"terminalId": 5})] {
        let mut m = json!({"type": "toolSpans.get"});
        m.as_object_mut().unwrap().extend(extra.as_object().unwrap().clone());
        assert_eq!(parsed(m), None);
    }
    assert_eq!(parsed(json!({"type": "toolSpans.get"})), Some(json!({"type": "toolSpans.get"})));
    let ok = json!({"type": "toolSpans.get", "terminalId": "x", "since": "2026-10-09T00:00:00.000Z"});
    assert_eq!(parsed(ok.clone()), Some(ok));
}

#[test]
fn toolstats_accepts_7_or_30_days_with_a_project_folder_or_null_only() {
    for bad in [json!({"days": 14, "project": null}), json!({"days": 7}), json!({"days": 30, "project": ""}), json!({"days": 30, "project": "x".repeat(1025)}), json!({"days": "7", "project": null})] {
        let mut m = json!({"type": "toolstats.get"});
        m.as_object_mut().unwrap().extend(bad.as_object().unwrap().clone());
        assert_eq!(parsed(m), None);
    }
    assert_eq!(parsed(json!({"type": "toolstats.get", "days": 7, "project": null})), Some(json!({"type": "toolstats.get", "days": 7, "project": null})));
    assert_eq!(
        parsed(json!({"type": "toolstats.get", "days": 30, "project": "D:\\work\\demo", "extra": 1})),
        Some(json!({"type": "toolstats.get", "days": 30, "project": "D:\\work\\demo"}))
    );
}

#[test]
fn favorite_open_keeps_terminal_and_iterm_and_drops_any_other_terminal_app() {
    let open = |app: Value| {
        let mut m = json!({"type": "favorite.open", "dir": "/x", "mode": "new"});
        if !app.is_null() {
            m["terminalApp"] = app;
        }
        parsed(m).unwrap()
    };
    assert_eq!(open(json!("iterm")), json!({"type": "favorite.open", "dir": "/x", "mode": "new", "terminalApp": "iterm"}));
    assert_eq!(open(json!("terminal"))["terminalApp"], "terminal");
    for other in [json!("warp"), json!(5), Value::Null] {
        assert_eq!(open(other), json!({"type": "favorite.open", "dir": "/x", "mode": "new"}));
    }
    assert_eq!(parsed(json!({"type": "favorite.open", "dir": "/x", "mode": "old"})), None);
}

#[test]
fn simple_messages_round_trip_and_wrong_shapes_are_dropped() {
    let ok = [
        json!({"type": "resync"}),
        json!({"type": "office.enable", "on": true}),
        json!({"type": "terminal.alias", "terminalId": "t", "alias": null}),
        json!({"type": "terminal.alias", "terminalId": "t", "alias": "A"}),
        json!({"type": "favorite.toggle", "terminalId": "t"}),
        json!({"type": "favorite.rename", "dir": "d", "label": null}),
        json!({"type": "favorite.remove", "dir": "d"}),
        json!({"type": "favorite.reorder", "dirs": ["a", "b"]}),
        json!({"type": "favorite.add", "dir": "d"}),
        json!({"type": "favorite.add", "dir": "d", "label": null}),
        json!({"type": "favorite.add", "dir": "d", "label": "L"}),
        json!({"type": "folder.pick", "requestId": "r1"}),
        json!({"type": "terminal.open", "terminalId": "t", "app": "vscode"}),
        json!({"type": "history.get"}),
        json!({"type": "terminal.dismiss"}),
        json!({"type": "terminal.dismiss", "terminalId": "t"}),
        json!({"type": "job.dismiss"}),
        json!({"type": "job.stop", "jobId": "j"}),
        json!({"type": "client.presence", "attentive": false}),
        json!({"type": "client.presence", "attentive": true, "locale": "vi"}),
        json!({"type": "terminal.note", "terminalId": "t", "note": null}),
        json!({"type": "terminal.snooze", "terminalId": "t", "minutes": 15}),
        json!({"type": "terminal.snooze", "terminalId": "t", "minutes": null}),
        json!({"type": "sensitive.ignore", "project": "p", "rule": "rm-rf", "on": true}),
        json!({"type": "promptSearch.cancel", "searchId": "s-1"}),
        json!({"type": "notify.test"}),
    ];
    for m in ok {
        assert_eq!(parsed(m.clone()), Some(m));
    }
    let bad = [
        json!({"type": "office.enable", "on": "yes"}),
        json!({"type": "terminal.alias", "terminalId": "t"}),
        json!({"type": "terminal.alias", "terminalId": 5, "alias": null}),
        json!({"type": "favorite.rename", "dir": "d"}),
        json!({"type": "favorite.add", "dir": "d", "label": 5}),
        json!({"type": "favorite.reorder", "dirs": "a"}),
        json!({"type": "favorite.reorder", "dirs": ["a", 1]}),
        json!({"type": "favorite.reorder", "dirs": vec!["a"; 101]}),
        json!({"type": "favorite.reorder", "dirs": ["x".repeat(1025)]}),
        json!({"type": "folder.pick", "requestId": "x".repeat(65)}),
        json!({"type": "terminal.open", "terminalId": "t", "app": "emacs"}),
        json!({"type": "job.dismiss", "jobId": null}),
        json!({"type": "job.stop", "jobId": "j".repeat(65)}),
        json!({"type": "client.presence"}),
        json!({"type": "terminal.note", "terminalId": "t", "note": "n".repeat(1001)}),
        json!({"type": "terminal.snooze", "terminalId": "t", "minutes": "5"}),
        json!({"type": "terminal.snooze", "terminalId": "t"}),
        json!({"type": "sensitive.ignore", "project": "p", "rule": "nope", "on": true}),
        json!({"type": "promptSearch.cancel", "searchId": "has space"}),
        json!({"type": "nope"}),
        json!({"type": 5}),
        json!({}),
        json!([1, 2]),
        json!("resync"),
        json!(null),
    ];
    for m in bad {
        assert_eq!(parsed(m.clone()), None, "{m}");
    }
}

#[test]
fn prompt_search_start_checks_every_field() {
    let abs = if cfg!(windows) { "D:\\work\\demo" } else { "/work/demo" };
    let base = |q: Value, days: Value, project: Value| json!({"type": "promptSearch.start", "searchId": "s1", "query": q, "days": days, "project": project});
    assert!(parsed(base(json!("hello world"), json!(30), json!(abs))).is_some());
    assert!(parsed(base(json!("hello"), json!(0), Value::Null)).is_some());
    assert_eq!(parsed(base(json!("  "), json!(30), Value::Null)), None);
    assert_eq!(parsed(base(json!("q".repeat(201)), json!(30), Value::Null)), None);
    assert_eq!(parsed(base(json!("q"), json!(14), Value::Null)), None);
    assert_eq!(parsed(base(json!("q"), json!(30), json!("relative/dir"))), None);
    assert_eq!(parsed(base(json!("q"), json!(30), json!(format!("{abs}\u{1}x")))), None);
}

#[test]
fn prompt_search_open_needs_a_session_uuid() {
    let id = "0123abcd-0123-4abc-8def-0123456789ab";
    let ok = json!({"type": "promptSearch.open", "searchId": "s1", "sessionId": id});
    assert_eq!(parsed(ok.clone()), Some(ok));
    assert_eq!(parsed(json!({"type": "promptSearch.open", "searchId": "s1", "sessionId": "x"})), None);
}

#[test]
fn notify_update_sanitizes_and_drops_on_a_bad_part() {
    let ok = json!({"type": "notify.update", "toast": true, "kinds": {"done": true, "bogus": 1}, "quiet": {"enabled": true, "from": "22:00", "to": "08:00"}, "stuckMinutes": 10, "idleMinutes": 60});
    assert_eq!(
        parsed(ok),
        Some(json!({"type": "notify.update", "toast": true, "kinds": {"done": true}, "quiet": {"enabled": true, "from": "22:00", "to": "08:00"}, "stuckMinutes": 10, "idleMinutes": 60}))
    );
    assert_eq!(parsed(json!({"type": "notify.update"})), Some(json!({"type": "notify.update"})));
    for bad in [json!({"toast": "yes"}), json!({"kinds": 5}), json!({"quiet": {"enabled": true}}), json!({"stuckMinutes": 11}), json!({"idleMinutes": "60"})] {
        let mut m = json!({"type": "notify.update"});
        m.as_object_mut().unwrap().extend(bad.as_object().unwrap().clone());
        assert_eq!(parsed(m), None);
    }
}

#[test]
fn sensitive_update_sanitizes_and_drops_on_a_bad_part() {
    let ok = json!({"type": "sensitive.update", "groups": {"git": false, "zz": true}, "globs": [" *.secret ", "*.secret"], "ignore": {"D:\\a": ["rm-rf", "nope"]}});
    assert_eq!(parsed(ok), Some(json!({"type": "sensitive.update", "groups": {"git": false}, "globs": ["*.secret"], "ignore": {"D:\\a": ["rm-rf"]}})));
    assert_eq!(parsed(json!({"type": "sensitive.update"})), Some(json!({"type": "sensitive.update"})));
    for bad in [json!({"groups": null}), json!({"globs": "x"}), json!({"ignore": []})] {
        let mut m = json!({"type": "sensitive.update"});
        m.as_object_mut().unwrap().extend(bad.as_object().unwrap().clone());
        assert_eq!(parsed(m), None);
    }
}
