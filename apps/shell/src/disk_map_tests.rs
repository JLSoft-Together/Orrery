use super::*;

#[test]
fn every_diskmap_channel_exists_in_the_generated_shim() {
    let shim = include_str!("shim.js");
    for c in CHANNELS {
        assert!(shim.contains(&format!("\"{c}\"")), "{c}");
    }
    for e in [PROGRESS, DONE, DUPES_PROGRESS, DUPES_DONE, crate::disk_snap::SCHEDULE_EVENT, crate::disk_snap::SNAP_EVENT] {
        assert!(shim.contains(&format!("\"{e}\"")), "{e}");
    }
}

#[cfg(windows)]
#[test]
fn scan_children_and_search_requests_are_cleaned() {
    assert_eq!(clean_scan(&json!({ "roots": ["C:\\", "C:\\"], "level": "max", "excludes": ["node_modules"] })), Some(json!({ "roots": ["C:\\"], "level": "max", "excludes": ["node_modules"] })));
    assert_eq!(clean_scan(&json!({ "roots": ["C:\\"], "level": "turbo" })), Some(json!({ "roots": ["C:\\"], "level": "balanced", "excludes": [] })));
    assert_eq!(clean_scan(&json!({ "roots": ["rel"] })), None);
    assert_eq!(clean_scan(&json!({ "roots": [] })), None);
    assert_eq!(clean_scan(&json!({ "roots": ["C:\\"], "excludes": [1] })), None);
    assert_eq!(clean_children(&json!({ "path": "", "sort": "name", "limit": 20 })), Some(json!({ "path": "", "sort": "name", "limit": 20 })));
    assert_eq!(clean_children(&json!({ "path": "C:\\a", "limit": 1e9 })), Some(json!({ "path": "C:\\a", "sort": "size", "limit": 10_000 })));
    assert_eq!(clean_children(&json!({ "path": "a" })), None);
    let q = clean_search(&json!({ "query": "x", "minSize": 10, "kind": "video", "only": "file", "under": "C:\\a", "sort": "mtime" })).unwrap();
    assert_eq!(q["query"], json!("x"));
    assert_eq!(q["minSize"].as_f64(), Some(10.0));
    assert_eq!(q["kind"], json!("video"));
    assert_eq!(q["under"], json!("C:\\a"));
    assert_eq!(q["limit"], json!(100));
    assert_eq!(clean_search(&json!({ "kind": "weird" })), None);
    assert_eq!(clean_search(&json!({ "minSize": -1 })), None);
    assert_eq!(clean_search(&json!({ "under": "rel" })), None);
    assert_eq!(clean_search(&Value::Null), Some(json!({ "limit": 100 })));
    assert_eq!(clean_fast_scan(&json!({ "drive": "c:" })), Some("C".into()));
    assert_eq!(clean_fast_scan(&json!({ "drive": "cd" })), None);
    assert_eq!(clean_sizes(&json!({ "paths": ["C:\\a", "C:\\a"] })), Some(vec!["C:\\a".to_owned()]));
    assert_eq!(clean_dupes(&json!({ "minSize": 10 })).unwrap()["minSize"], json!(1024));
    assert_eq!(clean_dupes(&json!({ "under": ["rel"] })), None);
}

#[test]
fn engine_replies_map_to_the_page_shapes() {
    let st = to_status(&json!({ "scanning": false, "paused": false, "index": { "roots": ["/w", 3], "scannedAt": 5, "entries": 9 } }));
    assert_eq!(st, json!({ "scanning": false, "paused": false, "indexed": true, "roots": ["/w"], "scannedAt": 5, "entries": 9 }));
    assert_eq!(to_status(&json!({ "scanning": true })), json!({ "scanning": true, "paused": false, "indexed": false, "roots": [] }));
    let ch = to_children(&json!({ "path": null, "size": 10, "total": 1, "items": [{ "path": "/w/a", "name": "a", "dir": false, "size": 4, "mtime": 7, "kind": "other", "cloud": false }, { "nope": 1 }] })).unwrap();
    assert_eq!(ch["path"], json!(""));
    assert_eq!(ch["items"].as_array().unwrap().len(), 1);
    assert!(ch["items"][0].get("kind").is_none() && ch["items"][0].get("cloud").is_none());
    let done = to_done(&json!({ "ok": true, "entries": 3, "roots": ["/w"], "saved": {} }));
    assert_eq!(done, json!({ "ok": true, "entries": 3, "roots": ["/w"] }));
    let vols = to_volumes(&json!([{ "mount": "C:\\", "total": 5, "free": 1 }, { "label": "x" }]));
    assert_eq!(vols, json!([{ "mount": "C:\\", "label": "C:\\", "fs": "", "kind": "unknown", "network": false, "total": 5, "free": 1 }]));
    let dd = to_dupes_done(&json!({ "ok": true, "groups": [{ "size": 5, "hash": "h", "files": [{ "path": "/a" }, { "path": "/b" }] }, { "files": [{ "path": "/c" }] }] }));
    assert_eq!(dd["groups"].as_array().unwrap().len(), 1);
    assert_eq!(dd["groups"][0]["count"].as_f64(), Some(2.0));
}

#[test]
fn history_keeps_clean_entries_newest_first() {
    let dir = std::env::temp_dir().join(format!("orrery-disk-history-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let h = History::new(&dir);
    h.append(&json!({ "at": 1, "action": "trash", "count": 1, "freed": 10, "paths": ["/a"], "failed": 0 }));
    h.append(&json!({ "at": 2, "action": "bogus", "count": 1, "freed": 10, "paths": [], "failed": 0 }));
    h.append(&json!({ "at": 3, "action": "delete", "count": 2, "freed": 20, "paths": ["/b", 4], "failed": 1 }));
    let got = h.read(5);
    assert_eq!(got.len(), 2);
    assert_eq!(got[0]["action"], json!("delete"));
    assert_eq!(got[0]["paths"], json!(["/b"]));
    assert_eq!(h.read(1).len(), 1);
    assert_eq!(history_limit(&json!({ "limit": 5000 })), 1000);
    assert_eq!(history_limit(&Value::Null), 50);
    let _ = std::fs::remove_dir_all(dir);
}
