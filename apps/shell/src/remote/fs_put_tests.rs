use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant, SystemTime};

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;
use serde_json::{json, Value};

use super::super::super::test_util::{temp_dir, TestHost};
use super::super::super::viewer_map::{build_args, lookup};
use super::{parse, run, save, stamp, Chunk, Fail, Puts, Step, MAX_CHUNK_B64, MAX_FILE, MAX_OPEN, TTL};
use crate::state::lock;

fn jpeg(extra: usize) -> Vec<u8> {
    let mut v = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00];
    v.extend(std::iter::repeat_n(7u8, extra));
    v
}

fn png() -> Vec<u8> {
    let mut v = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    v.extend([0u8; 16]);
    v
}

fn chunk(root: &str, up: &str, seq: u64, last: bool, data: &[u8]) -> Chunk {
    Chunk { root: root.into(), up: up.into(), seq, last, data: data.to_vec() }
}

fn args(root: &str, up: &str, seq: u64, last: bool, data: &[u8]) -> Value {
    json!([{ "root": root, "up": up, "seq": seq, "last": last, "b64": B64.encode(data) }])
}

fn host(allowed: &str) -> TestHost {
    let host = TestHost::default();
    let allowed = allowed.to_owned();
    *lock(&host.viewer) = Some(Box::new(move |_o, channel, a, _acc| {
        assert_eq!(channel, "ccm:list-dir");
        let root = a[0]["root"].as_str().unwrap_or("");
        if root == allowed {
            json!({ "raw": [] })
        } else {
            json!({ "raw": null })
        }
    }));
    host
}

fn inbox(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(root.join(".orrery").join("inbox")).map(|rd| rd.filter_map(Result::ok).map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
    names.sort();
    names
}

fn call(h: &TestHost, puts: &Puts, a: Value, now: Instant) -> Value {
    run(h, "remote:p:1", &[], &a, puts, now, SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000))
}

#[test]
fn chunks_are_joined_in_order_and_written_under_the_inbox_with_a_desktop_name() {
    let root = temp_dir("put-ok");
    let r = root.to_string_lossy().into_owned();
    let (h, puts, now) = (host(&r), Puts::default(), Instant::now());
    let data = jpeg(3000);
    let (a, b) = data.split_at(1000);
    let first = call(&h, &puts, args(&r, "u1", 0, false, a), now);
    assert_eq!(first, json!({ "ok": true, "done": false, "next": 1 }));
    let done = call(&h, &puts, args(&r, "u1", 1, true, b), now);
    assert_eq!(done["ok"], json!(true));
    assert_eq!(done["done"], json!(true));
    assert_eq!(done["name"], json!("20231114-221320.jpg"));
    let path = root.join(".orrery").join("inbox").join("20231114-221320.jpg");
    assert_eq!(done["path"], json!(path.to_string_lossy()));
    assert_eq!(std::fs::read(&path).unwrap(), data);
    assert_eq!(puts.open_count(), 0);
}

#[test]
fn a_png_is_kept_as_png_and_other_bytes_are_refused() {
    let root = temp_dir("put-png");
    let r = root.to_string_lossy().into_owned();
    let (h, puts, now) = (host(&r), Puts::default(), Instant::now());
    let ok = call(&h, &puts, args(&r, "p1", 0, true, &png()), now);
    assert!(ok["name"].as_str().unwrap().ends_with(".png"));
    let bad = call(&h, &puts, args(&r, "p2", 0, true, b"<html>not an image</html>"), now);
    assert_eq!(bad, json!({ "ok": false, "error": "image" }));
    assert_eq!(inbox(&root).len(), 1);
}

#[test]
fn an_existing_name_is_never_overwritten_a_suffix_is_added() {
    let root = temp_dir("put-dup");
    let at = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    let (p1, n1) = save(&root, &jpeg(1), at).unwrap();
    let (p2, n2) = save(&root, &jpeg(2), at).unwrap();
    let (_, n3) = save(&root, &png(), at).unwrap();
    assert_eq!((n1.as_str(), n2.as_str(), n3.as_str()), ("20231114-221320.jpg", "20231114-221320-1.jpg", "20231114-221320.png"));
    assert_eq!(std::fs::read(p1).unwrap(), jpeg(1));
    assert_eq!(std::fs::read(p2).unwrap(), jpeg(2));
}

#[test]
fn out_of_order_and_missing_chunks_are_rejected_and_drop_the_buffer() {
    let puts = Puts::default();
    let now = Instant::now();
    assert!(matches!(puts.take("o", chunk("/r", "a", 0, false, b"x"), now), Ok(Step::Next(1))));
    assert_eq!(puts.take("o", chunk("/r", "a", 2, false, b"x"), now), Err(Fail::Order));
    assert_eq!(puts.open_count(), 0);
    assert_eq!(puts.take("o", chunk("/r", "a", 1, true, b"x"), now), Err(Fail::Expired));
    assert!(matches!(puts.take("o", chunk("/r", "b", 0, false, b"x"), now), Ok(Step::Next(1))));
    assert_eq!(puts.take("o", chunk("/r", "b", 0, false, b"x"), now), Err(Fail::Order));
    assert_eq!(puts.open_count(), 0);
}

#[test]
fn a_chunk_cannot_switch_the_root_of_a_running_upload() {
    let puts = Puts::default();
    let now = Instant::now();
    assert!(puts.take("o", chunk("/a", "u", 0, false, b"x"), now).is_ok());
    assert_eq!(puts.take("o", chunk("/b", "u", 1, true, b"x"), now), Err(Fail::Order));
}

#[test]
fn more_than_8_mb_is_refused_and_the_buffer_is_freed() {
    let puts = Puts::default();
    let now = Instant::now();
    let big = vec![1u8; MAX_FILE / 2 + 1];
    assert!(puts.take("o", chunk("/r", "u", 0, false, &big), now).is_ok());
    assert_eq!(puts.take("o", chunk("/r", "u", 1, false, &big), now), Err(Fail::Size));
    assert_eq!(puts.open_count(), 0);
}

#[test]
fn a_buffer_expires_after_60_seconds_of_silence() {
    let puts = Puts::default();
    let now = Instant::now();
    assert!(puts.take("o", chunk("/r", "u", 0, false, b"x"), now).is_ok());
    assert_eq!(puts.take("o", chunk("/r", "u", 1, true, b"x"), now + TTL + Duration::from_secs(1)), Err(Fail::Expired));
    assert_eq!(puts.open_count(), 0);
    assert!(puts.take("o", chunk("/r", "v", 0, false, b"x"), now).is_ok());
    assert!(puts.take("o", chunk("/r", "v", 1, false, b"x"), now + Duration::from_secs(50)).is_ok());
    assert!(puts.take("o", chunk("/r", "v", 2, true, b"x"), now + Duration::from_secs(100)).is_ok());
}

#[test]
fn two_uploads_at_once_keep_separate_buffers_and_the_open_count_is_capped() {
    let puts = Puts::default();
    let now = Instant::now();
    assert!(puts.take("o", chunk("/r", "a", 0, false, b"AA"), now).is_ok());
    assert!(puts.take("o", chunk("/r", "b", 0, false, b"BB"), now).is_ok());
    let a = puts.take("o", chunk("/r", "a", 1, true, b"aa"), now).unwrap();
    let b = puts.take("o", chunk("/r", "b", 1, true, b"bb"), now).unwrap();
    assert_eq!(a, Step::Done { root: "/r".into(), bytes: b"AAaa".to_vec() });
    assert_eq!(b, Step::Done { root: "/r".into(), bytes: b"BBbb".to_vec() });
    for i in 0..MAX_OPEN {
        assert!(puts.take("o", chunk("/r", &format!("k{i}"), 0, false, b"x"), now).is_ok());
    }
    assert_eq!(puts.take("o", chunk("/r", "extra", 0, false, b"x"), now), Err(Fail::Busy));
    assert!(puts.take("other", chunk("/r", "k0", 0, false, b"x"), now).is_ok());
}

#[test]
fn a_root_the_viewer_guard_does_not_allow_is_denied_and_nothing_is_written() {
    let root = temp_dir("put-deny");
    let other = temp_dir("put-other");
    let h = host(&root.to_string_lossy());
    let puts = Puts::default();
    let out = call(&h, &puts, args(&other.to_string_lossy(), "u", 0, true, &jpeg(10)), Instant::now());
    assert_eq!(out, json!({ "ok": false, "error": "denied" }));
    assert!(inbox(&other).is_empty());
    assert!(!other.join(".orrery").exists());
    assert_eq!(puts.open_count(), 0);
}

#[test]
fn a_view_only_desktop_refuses_before_touching_the_guard() {
    let root = temp_dir("put-view");
    let r = root.to_string_lossy().into_owned();
    let h = host(&r);
    h.view_only.store(true, Ordering::SeqCst);
    let out = call(&h, &Puts::default(), args(&r, "u", 0, true, &jpeg(10)), Instant::now());
    assert_eq!(out["error"], json!("denied"));
    assert!(lock(&h.views).is_empty());
}

#[test]
fn a_root_with_parent_segments_or_a_missing_folder_is_never_created() {
    let base = temp_dir("put-dots");
    let sneaky = base.join("..").join(base.file_name().unwrap());
    assert_eq!(save(&sneaky, &jpeg(1), SystemTime::now()).unwrap_err(), Fail::Denied);
    let gone = base.join("not-there");
    assert_eq!(save(&gone, &jpeg(1), SystemTime::now()).unwrap_err(), Fail::Denied);
    assert!(!gone.exists());
}

#[test]
fn a_dot_orrery_that_is_a_file_is_refused() {
    let root = temp_dir("put-file");
    std::fs::write(root.join(".orrery"), b"x").unwrap();
    assert_eq!(save(&root, &jpeg(1), SystemTime::now()).unwrap_err(), Fail::Denied);
}

#[test]
fn bad_requests_are_invalid() {
    let h = host("/r");
    let puts = Puts::default();
    let now = Instant::now();
    let cases = [
        json!([]),
        json!([{ "root": "/r", "up": "a b", "seq": 0, "last": true, "b64": "AAAA" }]),
        json!([{ "root": "/r", "up": "a", "seq": 0, "last": true, "b64": "***" }]),
        json!([{ "root": "/r", "up": "a", "seq": 0, "last": true, "b64": "" }]),
        json!([{ "root": "/r", "up": "a", "seq": -1, "last": true, "b64": "AAAA" }]),
        json!([{ "root": "/r", "up": "a", "seq": 0, "b64": "AAAA" }]),
    ];
    for a in cases {
        assert_eq!(call(&h, &puts, a, now), json!({ "ok": false, "error": "invalid" }));
    }
    let huge = "A".repeat(MAX_CHUNK_B64 + 4);
    assert!(parse(&json!([{ "root": "/r", "up": "a", "seq": 0, "last": true, "b64": huge }])).is_none());
}

#[test]
fn the_op_is_registered_and_arguments_are_checked_by_the_map() {
    let spec = lookup("fs.put").expect("fs.put registered");
    let good = json!({ "root": "D:\\proj", "up": "u1", "seq": 0, "last": false, "b64": "AAAA" });
    assert_eq!(build_args(spec, &good).unwrap()[0]["up"], json!("u1"));
    for bad in [
        json!({ "root": "\\\\srv\\share", "up": "u1", "seq": 0, "last": false, "b64": "AAAA" }),
        json!({ "root": "D:\\proj", "up": "../x", "seq": 0, "last": false, "b64": "AAAA" }),
        json!({ "root": "D:\\proj", "up": "u1", "seq": 65, "last": false, "b64": "AAAA" }),
        json!({ "root": "D:\\proj", "up": "u1", "seq": 0, "last": "no", "b64": "AAAA" }),
        json!({ "root": "D:\\proj", "up": "u1", "seq": 0, "last": false, "b64": "" }),
        json!({ "up": "u1", "seq": 0, "last": false, "b64": "AAAA" }),
    ] {
        assert!(build_args(spec, &bad).is_none(), "{bad}");
    }
}

#[test]
fn the_stamp_is_a_utc_date_and_time() {
    assert_eq!(stamp(SystemTime::UNIX_EPOCH), "19700101-000000");
    assert_eq!(stamp(SystemTime::UNIX_EPOCH + Duration::from_secs(1_709_164_799)), "20240228-235959");
    assert_eq!(stamp(SystemTime::UNIX_EPOCH + Duration::from_secs(1_709_251_200)), "20240301-000000");
}
