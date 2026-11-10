use super::watcher::Unwatch;
use crate::run::flutter::machine::MachineClient;
use crate::run::types::BuildError;
use super::host::HostContext;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

#[derive(Default)]
pub struct LiveState {
    pub app_id: Option<String>,
    pub started: bool,
    pub stop_requested: bool,
    pub build_error: Option<BuildError>,
    pub reload_error: Option<BuildError>,
    pub device_lost: bool,
    pub reloading: bool,
    pub reload_queued: bool,
    pub syncing: bool,
    pub sync_queued: bool,
    pub pub_pid: Option<u32>,
    pub unwatch: Vec<Unwatch>,
}

pub struct Live {
    pub pid: u32,
    pub machine: Arc<MachineClient>,
    pub root: String,
    pub pkg: String,
    pub host: HostContext,
    pub sdk: String,
    pub device_id: String,
    pub flavor: Option<String>,
    exited: (Mutex<bool>, Condvar),
    st: Mutex<LiveState>,
}

impl Live {
    #[allow(clippy::too_many_arguments)]
    pub fn new(pid: u32, machine: Arc<MachineClient>, root: String, pkg: String, host: HostContext, sdk: String, device_id: String, flavor: Option<String>) -> Arc<Live> {
        Arc::new(Live { pid, machine, root, pkg, host, sdk, device_id, flavor, exited: (Mutex::new(false), Condvar::new()), st: Mutex::new(LiveState::default()) })
    }

    pub fn st<R>(&self, f: impl FnOnce(&mut LiveState) -> R) -> R {
        f(&mut self.st.lock().unwrap())
    }

    pub fn mark_exited(&self) {
        *self.exited.0.lock().unwrap() = true;
        self.exited.1.notify_all();
    }

    pub fn is_exited(&self) -> bool {
        *self.exited.0.lock().unwrap()
    }

    pub fn wait_exit(&self, timeout: Duration) -> bool {
        let g = self.exited.0.lock().unwrap();
        let (g, _) = self.exited.1.wait_timeout_while(g, timeout, |done| !*done).unwrap();
        *g
    }
}
