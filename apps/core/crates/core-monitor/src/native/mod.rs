pub mod focus;
pub mod launch;
pub mod open;
pub mod osa;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use core_store::event::{FocusWindowResult, FolderPickResult, OpenApp, OpenResult};

use crate::host::protocol::{valid_dir, valid_pid};
use crate::job_stop::JobStopResult;
use crate::monitor::{Deps, LaunchError, LaunchMode, SystemDeps, TerminalApp};
use crate::process::ProcessInfo;

pub type Picker = Box<dyn Fn() -> (FolderPickResult, Option<String>) + Send + Sync>;

pub struct NativeDeps {
    system: SystemDeps,
    picker: Option<Picker>,
    picking: AtomicBool,
    focusing: Mutex<()>,
}

impl NativeDeps {
    pub fn new(picker: Option<Picker>) -> NativeDeps {
        NativeDeps { system: SystemDeps, picker, picking: AtomicBool::new(false), focusing: Mutex::new(()) }
    }
}

pub fn checked_pick(result: FolderPickResult, dir: Option<String>) -> (FolderPickResult, Option<String>) {
    match (result, dir) {
        (FolderPickResult::Ok, Some(d)) if valid_dir(&d) => (FolderPickResult::Ok, Some(d)),
        (FolderPickResult::Ok, _) => (FolderPickResult::Failed, None),
        (r, _) => (r, None),
    }
}

impl Deps for NativeDeps {
    fn now(&self) -> i64 {
        self.system.now()
    }

    fn is_pid_alive(&self, pid: u32) -> bool {
        self.system.is_pid_alive(pid)
    }

    fn query_processes(&self, pids: &[u32]) -> Option<HashMap<u32, ProcessInfo>> {
        self.system.query_processes(pids)
    }

    fn stop_job(&self, job_id: &str) -> JobStopResult {
        self.system.stop_job(job_id)
    }

    fn launch(&self, dir: &str, mode: LaunchMode, app: Option<TerminalApp>) -> Option<Result<(), LaunchError>> {
        if !valid_dir(dir) {
            return Some(Err(LaunchError::Failed));
        }
        launch::launch(dir, mode, app.unwrap_or(TerminalApp::Terminal))
    }

    fn focus_window(&self, pid: u32) -> Option<FocusWindowResult> {
        if !valid_pid(pid) {
            return Some(FocusWindowResult::NotFound);
        }
        let _turn = self.focusing.lock().unwrap_or_else(|e| e.into_inner());
        focus::focus(pid)
    }

    fn open_folder(&self, dir: &str, app: OpenApp) -> Option<OpenResult> {
        if !valid_dir(dir) {
            return Some(OpenResult::NotFound);
        }
        open::open_folder(dir, app)
    }

    fn pick_folder(&self) -> Option<(FolderPickResult, Option<String>)> {
        let picker = self.picker.as_ref()?;
        if self.picking.swap(true, Ordering::SeqCst) {
            return Some((FolderPickResult::Busy, None));
        }
        let (result, dir) = picker();
        self.picking.store(false, Ordering::SeqCst);
        Some(checked_pick(result, dir))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    fn dir() -> &'static str {
        if cfg!(windows) {
            "C:\\work\\app"
        } else {
            "/work/app"
        }
    }

    #[test]
    fn bad_input_never_reaches_the_os() {
        let d = NativeDeps::new(None);
        assert_eq!(d.focus_window(3), Some(FocusWindowResult::NotFound));
        assert_eq!(d.open_folder("relative", OpenApp::Explorer), Some(OpenResult::NotFound));
        assert_eq!(d.open_folder("", OpenApp::Vscode), Some(OpenResult::NotFound));
        assert_eq!(d.launch("relative", LaunchMode::New, None), Some(Err(LaunchError::Failed)));
        assert_eq!(d.pick_folder(), None);
    }

    #[test]
    fn a_missing_folder_is_not_found_before_any_spawn() {
        let missing = std::env::temp_dir().join("orrery-native-missing-dir-x9");
        let missing = missing.to_string_lossy().into_owned();
        let d = NativeDeps::new(None);
        if cfg!(any(windows, target_os = "macos")) {
            assert_eq!(d.open_folder(&missing, OpenApp::Explorer), Some(OpenResult::NotFound));
            assert_eq!(d.launch(&missing, LaunchMode::New, None), Some(Err(LaunchError::NotFound)));
        } else {
            assert_eq!(d.open_folder(&missing, OpenApp::Explorer), None);
            assert_eq!(d.launch(&missing, LaunchMode::New, None), None);
        }
    }

    #[test]
    fn the_picker_answer_is_checked_and_one_dialog_runs_at_a_time() {
        let ok = NativeDeps::new(Some(Box::new(|| (FolderPickResult::Ok, Some(dir().to_owned())))));
        assert_eq!(ok.pick_folder(), Some((FolderPickResult::Ok, Some(dir().to_owned()))));
        let rel = NativeDeps::new(Some(Box::new(|| (FolderPickResult::Ok, Some("rel".into())))));
        assert_eq!(rel.pick_folder(), Some((FolderPickResult::Failed, None)));
        let none = NativeDeps::new(Some(Box::new(|| (FolderPickResult::Ok, None))));
        assert_eq!(none.pick_folder(), Some((FolderPickResult::Failed, None)));
        let cancel = NativeDeps::new(Some(Box::new(|| (FolderPickResult::Cancelled, Some(dir().to_owned())))));
        assert_eq!(cancel.pick_folder(), Some((FolderPickResult::Cancelled, None)));

        let (enter_tx, enter_rx) = std::sync::mpsc::channel::<()>();
        let (go_tx, go_rx) = std::sync::mpsc::channel::<()>();
        let go_rx = Mutex::new(go_rx);
        let enter_tx = Mutex::new(enter_tx);
        let slow = Arc::new(NativeDeps::new(Some(Box::new(move || {
            let _ = enter_tx.lock().unwrap().send(());
            let _ = go_rx.lock().unwrap().recv();
            (FolderPickResult::Cancelled, None)
        }))));
        let first = slow.clone();
        let t = std::thread::spawn(move || first.pick_folder());
        enter_rx.recv().unwrap();
        assert_eq!(slow.pick_folder(), Some((FolderPickResult::Busy, None)));
        go_tx.send(()).unwrap();
        assert_eq!(t.join().unwrap(), Some((FolderPickResult::Cancelled, None)));
        go_tx.send(()).unwrap();
        assert_eq!(slow.pick_folder(), Some((FolderPickResult::Cancelled, None)));
    }
}
