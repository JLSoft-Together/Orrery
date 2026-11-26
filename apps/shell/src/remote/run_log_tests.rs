use serde_json::{json, Value};

use super::{valid_id, RunLog, MAX_LINES, MAX_SESSIONS, READ_MAX, TEXT_MAX};

fn plain(t: &str) -> String {
    t.to_owned()
}

fn event(session: &str, from: usize, count: usize) -> Value {
    let lines: Vec<Value> = (from..from + count).map(|i| json!({ "stream": "app", "text": format!("line {i}"), "at": 1000 + i as i64 })).collect();
    json!({ "type": "log", "sessionId": session, "lines": lines })
}

fn texts(read: &Value) -> Vec<String> {
    read["lines"].as_array().unwrap().iter().map(|l| l["text"].as_str().unwrap().to_owned()).collect()
}

#[test]
fn the_ring_buffer_keeps_only_the_newest_2000_lines_with_stable_numbers() {
    let log = RunLog::default();
    assert_eq!(log.record(&event("s1", 0, 1500), &plain), 1500);
    assert_eq!(log.record(&event("s1", 1500, 1000), &plain), 1000);
    let all = log.read("s1", Some(0));
    assert_eq!(all["gap"], json!(true));
    let first = all["lines"][0]["n"].as_u64().unwrap();
    assert_eq!(first, (2500 - MAX_LINES + 1) as u64);
    assert_eq!(all["lines"][0]["text"], json!(format!("line {}", 2500 - MAX_LINES)));
    assert_eq!(all["more"], json!(true));
}

#[test]
fn multibyte_logs_are_cut_by_bytes_so_a_window_never_exceeds_the_reply_limit() {
    let log = RunLog::default();
    let lines: Vec<Value> = (0..READ_MAX).map(|i| json!({ "stream": "app", "text": format!("{i:04}{}", "漢".repeat(TEXT_MAX - 4)), "at": 1 })).collect();
    log.record(&json!({ "type": "log", "sessionId": "s1", "lines": lines }), &plain);
    let tail = log.read("s1", None);
    assert!(tail.to_string().len() <= 900_000, "{}", tail.to_string().len());
    assert_eq!(tail["lines"].as_array().unwrap().last().unwrap()["n"], json!(READ_MAX));
    let head = log.read("s1", Some(0));
    assert!(head.to_string().len() <= 900_000);
    assert_eq!(head["more"], json!(true));
    let last = head["last"].as_u64().unwrap();
    assert!(last > 0 && last < READ_MAX as u64);
    let rest = log.read("s1", Some(last));
    assert_eq!(rest["lines"][0]["n"], json!(last + 1));
}

#[test]
fn since_returns_only_newer_lines_and_pages_until_caught_up() {
    let log = RunLog::default();
    log.record(&event("s1", 0, 10), &plain);
    let tail = log.read("s1", None);
    assert_eq!(tail["lines"].as_array().unwrap().len(), 10);
    assert_eq!(tail["last"], json!(10));
    assert_eq!(tail["gap"], json!(false));
    let none = log.read("s1", Some(10));
    assert_eq!(none["lines"], json!([]));
    assert_eq!(none["last"], json!(10));
    log.record(&event("s1", 10, 3), &plain);
    let more = log.read("s1", Some(10));
    assert_eq!(texts(&more), vec!["line 10", "line 11", "line 12"]);
    assert_eq!(more["last"], json!(13));
    assert_eq!(more["gap"], json!(false));
    assert_eq!(more["more"], json!(false));
}

#[test]
fn a_first_read_returns_the_tail_and_flags_the_older_lines_it_left_out() {
    let log = RunLog::default();
    log.record(&event("s1", 0, READ_MAX + 50), &plain);
    let tail = log.read("s1", None);
    assert_eq!(tail["lines"].as_array().unwrap().len(), READ_MAX);
    assert_eq!(tail["lines"][0]["text"], json!("line 50"));
    assert_eq!(tail["gap"], json!(true));
    let paged = log.read("s1", Some(0));
    assert_eq!(paged["lines"].as_array().unwrap().len(), READ_MAX);
    assert_eq!(paged["more"], json!(true));
    assert_eq!(paged["last"], json!(READ_MAX));
}

#[test]
fn secrets_are_masked_and_control_characters_and_long_text_are_cut() {
    let log = RunLog::default();
    let mask = |t: &str| t.replace("hunter2", "***");
    let long = "x".repeat(TEXT_MAX + 500);
    let ev = json!({ "type": "log", "sessionId": "s1", "lines": [
        { "stream": "build", "text": "password=hunter2\u{1b}[31m", "level": "E", "tag": "Auth" },
        { "stream": "logcat", "text": long },
        { "stream": "weird", "text": "dropped" },
        { "stream": "app", "text": 5 }
    ] });
    assert_eq!(log.record(&ev, &mask), 2);
    let read = log.read("s1", None);
    assert_eq!(read["lines"][0]["text"], json!("password=***[31m"));
    assert_eq!(read["lines"][0]["level"], json!("E"));
    assert_eq!(read["lines"][0]["tag"], json!("Auth"));
    assert_eq!(read["lines"][1]["text"].as_str().unwrap().chars().count(), TEXT_MAX);
}

#[test]
fn only_log_events_with_a_valid_session_id_are_recorded() {
    let log = RunLog::default();
    assert_eq!(log.record(&json!({ "type": "session", "session": {} }), &plain), 0);
    assert_eq!(log.record(&event("a/b", 0, 1), &plain), 0);
    assert_eq!(log.record(&event("", 0, 1), &plain), 0);
    assert_eq!(log.record(&json!({ "type": "log", "sessionId": "s1", "lines": "nope" }), &plain), 0);
    assert!(valid_id("abc-123_x"));
    assert!(!valid_id(&"a".repeat(65)));
    assert_eq!(log.read("s1", None)["known"], json!(false));
}

#[test]
fn the_least_recently_written_session_is_dropped_past_the_session_max() {
    let log = RunLog::default();
    for i in 0..MAX_SESSIONS {
        log.record(&event(&format!("s{i}"), 0, 1), &plain);
    }
    log.record(&event("s0", 1, 1), &plain);
    log.record(&event("fresh", 0, 1), &plain);
    assert_eq!(log.read("s0", None)["known"], json!(true));
    assert_eq!(log.read("s1", None)["known"], json!(false));
    assert_eq!(log.read("fresh", None)["known"], json!(true));
    log.forget("fresh");
    assert_eq!(log.read("fresh", None)["known"], json!(false));
}

mod ops {
    use serde_json::json;

    use super::super::super::super::viewer_map::{build_args, lookup, watch_key, Arg, Watch, RUN_LOG, RUN_WATCH, SPECS};

    #[test]
    fn the_run_ops_are_mapped_and_every_op_name_is_unique() {
        let log = lookup("run.log").unwrap();
        assert_eq!((log.channel, log.arg, log.run, log.write), (RUN_LOG, Arg::RunLog, false, false));
        let watch = lookup("run.watch").unwrap();
        assert_eq!((watch.channel, watch.watch, watch.run), (RUN_WATCH, Watch::RunAdd, true));
        let unwatch = lookup("run.unwatch").unwrap();
        assert_eq!((unwatch.channel, unwatch.watch, unwatch.run), (RUN_WATCH, Watch::RunDrop, false));
        assert_eq!(SPECS.len(), 41);
        let mut names: Vec<&str> = SPECS.iter().map(|s| s.t).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), SPECS.len());
    }

    #[test]
    fn run_log_args_accept_a_session_and_an_optional_since_only() {
        let log = lookup("run.log").unwrap();
        assert_eq!(build_args(log, &json!({ "t": "run.log", "session": "abc-1" })), Some(json!([{ "session": "abc-1" }])));
        assert_eq!(build_args(log, &json!({ "session": "abc-1", "since": 40 })), Some(json!([{ "session": "abc-1", "since": 40 }])));
        assert_eq!(build_args(log, &json!({ "session": "abc-1", "since": null })), Some(json!([{ "session": "abc-1" }])));
        assert_eq!(build_args(log, &json!({ "session": "abc-1", "since": -1 })), None);
        assert_eq!(build_args(log, &json!({ "session": "abc-1", "since": "4" })), None);
        assert_eq!(build_args(log, &json!({})), None);
        assert_eq!(build_args(log, &json!({ "session": "a/../b" })), None);
        assert_eq!(build_args(log, &json!({ "session": "x".repeat(65) })), None);
        assert_eq!(build_args(lookup("run.watch").unwrap(), &json!({})), Some(json!([])));
        assert_eq!(watch_key(lookup("run.watch").unwrap(), &json!({})), None);
    }
}
