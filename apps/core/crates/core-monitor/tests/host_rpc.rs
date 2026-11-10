use std::io::Write;
use std::sync::mpsc::{channel, Sender};
use std::sync::{Arc, Mutex};

use core_monitor::host::{HostClient, HostDeps};
use core_monitor::monitor::{Deps, LaunchError, LaunchMode, TerminalApp};
use core_store::event::{FocusWindowResult, FolderPickResult, OpenApp, OpenResult};
use serde_json::{json, Value};

struct Lines(Sender<Value>, Vec<u8>);

impl Write for Lines {
    fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
        self.1.extend_from_slice(b);
        while let Some(i) = self.1.iter().position(|c| *c == b'\n') {
            let line: Vec<u8> = self.1.drain(..=i).collect();
            let _ = self.0.send(serde_json::from_slice(&line).unwrap());
        }
        Ok(b.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

type Seen = Arc<Mutex<Vec<Value>>>;

fn host(answer: impl Fn(&Value) -> Value + Send + 'static) -> (HostDeps, Seen, Arc<HostClient>) {
    let (tx, rx) = channel();
    let client = HostClient::new(Box::new(Lines(tx, Vec::new())));
    let seen: Seen = Arc::default();
    let (c, s) = (client.clone(), seen.clone());
    std::thread::spawn(move || {
        for req in rx {
            s.lock().unwrap().push(req.clone());
            c.on_message(&json!({ "type": "host.res", "id": req["id"], "result": answer(&req) }));
        }
    });
    (HostDeps::new(client.clone()), seen, client)
}

fn dir() -> &'static str {
    if cfg!(windows) {
        "C:\\work\\app"
    } else {
        "/work/app"
    }
}

#[test]
fn focus_goes_to_main_and_bad_pids_never_leave() {
    let (deps, seen, _) = host(|_| json!({ "result": "ambiguous" }));
    assert_eq!(deps.focus_window(4242), Some(FocusWindowResult::Ambiguous));
    assert_eq!(deps.focus_window(3), Some(FocusWindowResult::NotFound));
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0]["type"], "host.req");
    assert_eq!(seen[0]["method"], "focus");
    assert_eq!(seen[0]["params"], json!({ "pid": 4242 }));
}

#[test]
fn unknown_result_text_becomes_failed() {
    let (deps, _, _) = host(|_| json!({ "result": "weird" }));
    assert_eq!(deps.focus_window(100), Some(FocusWindowResult::Failed));
    assert_eq!(deps.open_folder(dir(), OpenApp::Vscode), Some(OpenResult::Failed));
}

#[test]
fn open_validates_the_dir_and_forwards_the_app() {
    let (deps, seen, _) = host(|_| json!({ "result": "noApp" }));
    assert_eq!(deps.open_folder("relative", OpenApp::Explorer), Some(OpenResult::NotFound));
    assert_eq!(deps.open_folder("", OpenApp::Explorer), Some(OpenResult::NotFound));
    assert_eq!(deps.open_folder(dir(), OpenApp::Vscode), Some(OpenResult::NoApp));
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0]["method"], "open");
    assert_eq!(seen[0]["params"], json!({ "dir": dir(), "app": "vscode" }));
}

#[test]
fn launch_maps_ok_and_every_error_code() {
    let (deps, seen, _) = host(|r| match r["params"]["mode"].as_str() {
        Some("new") => json!({ "ok": true }),
        _ => json!({ "ok": false, "error": "notInstalled" }),
    });
    assert_eq!(deps.launch(dir(), LaunchMode::New, Some(TerminalApp::Iterm)), Some(Ok(())));
    assert_eq!(deps.launch(dir(), LaunchMode::Continue, None), Some(Err(LaunchError::NotInstalled)));
    assert_eq!(deps.launch("relative", LaunchMode::New, None), Some(Err(LaunchError::Failed)));
    let seen = seen.lock().unwrap();
    assert_eq!(seen.len(), 2);
    assert_eq!(seen[0]["params"], json!({ "dir": dir(), "mode": "new", "app": "iterm" }));
    assert_eq!(seen[1]["params"], json!({ "dir": dir(), "mode": "continue" }));
    for (code, want) in [("notFound", LaunchError::NotFound), ("denied", LaunchError::Denied), ("failed", LaunchError::Failed), ("whatever", LaunchError::Failed)] {
        let (d, _, _) = host(move |_| json!({ "ok": false, "error": code }));
        assert_eq!(d.launch(dir(), LaunchMode::New, None), Some(Err(want)));
    }
}

#[test]
fn pick_folder_accepts_only_a_valid_absolute_dir() {
    let (deps, _, _) = host(|_| json!({ "result": "ok", "dir": dir() }));
    assert_eq!(deps.pick_folder(), Some((FolderPickResult::Ok, Some(dir().to_owned()))));
    let (deps, _, _) = host(|_| json!({ "result": "ok", "dir": "..\\escape" }));
    assert_eq!(deps.pick_folder(), Some((FolderPickResult::Failed, None)));
    let (deps, _, _) = host(|_| json!({ "result": "ok" }));
    assert_eq!(deps.pick_folder(), Some((FolderPickResult::Failed, None)));
    let (deps, _, _) = host(|_| json!({ "result": "cancelled", "dir": dir() }));
    assert_eq!(deps.pick_folder(), Some((FolderPickResult::Cancelled, None)));
    let (deps, _, _) = host(|_| json!({ "result": "busy" }));
    assert_eq!(deps.pick_folder(), Some((FolderPickResult::Busy, None)));
}

#[test]
fn remote_error_is_failed_and_a_closed_channel_is_unsupported() {
    let (tx, rx) = channel();
    let client = HostClient::new(Box::new(Lines(tx, Vec::new())));
    let c = client.clone();
    std::thread::spawn(move || {
        for req in rx {
            c.on_message(&json!({ "type": "host.res", "id": req["id"], "error": "boom" }));
        }
    });
    let deps = HostDeps::new(client.clone());
    assert_eq!(deps.focus_window(100), Some(FocusWindowResult::Failed));
    assert_eq!(deps.launch(dir(), LaunchMode::New, None), Some(Err(LaunchError::Failed)));
    assert_eq!(deps.pick_folder(), Some((FolderPickResult::Failed, None)));
    client.close();
    assert_eq!(deps.focus_window(100), None);
    assert_eq!(deps.open_folder(dir(), OpenApp::Explorer), None);
    assert_eq!(deps.launch(dir(), LaunchMode::New, None), None);
    assert_eq!(deps.pick_folder(), None);
}
