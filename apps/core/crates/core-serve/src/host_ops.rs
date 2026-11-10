use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use core_http::ClientContext;
use core_monitor::monitor::{LaunchMode, TerminalApp};
use core_monitor::Deps;
use core_sources::prompt_search::searcher::{PromptSearchRequest, PromptSearcher, SearcherDeps};
use core_store::event::{FocusResult, FocusWindowResult, FolderPickResult, FolderPicked, MonitorEvent, OpenApp, OpenResult, TerminalOpenResult};
use serde_json::Value;
use tokio::sync::oneshot;

use crate::runtime::Cmd;

pub struct HostOps {
    deps: Arc<dyn Deps>,
    searcher: Arc<PromptSearcher>,
    back: Mutex<Sender<Cmd>>,
}

fn spawn(name: &str, f: impl FnOnce() + Send + 'static) {
    if let Err(e) = std::thread::Builder::new().name(name.into()).spawn(f) {
        eprintln!("[collector] {name}: {e}");
    }
}

fn reply_value(ctx: &ClientContext, v: Value) {
    if let Ok(e) = serde_json::from_value::<MonitorEvent>(v) {
        ctx.reply(e);
    }
}

pub fn focus_of(deps: &dyn Deps, pid: Option<u32>) -> FocusWindowResult {
    match pid {
        None => FocusWindowResult::NotFound,
        Some(p) => deps.focus_window(p).unwrap_or(FocusWindowResult::Unsupported),
    }
}

impl HostOps {
    pub fn new(deps: Arc<dyn Deps>, projects: PathBuf, back: Sender<Cmd>) -> HostOps {
        let open = deps.clone();
        let searcher_deps = SearcherDeps { open_folder: Some(Box::new(move |dir: &str, _app: &str| open.open_folder(dir, OpenApp::Explorer).unwrap_or(OpenResult::Unsupported).as_str())), ..SearcherDeps::default() };
        HostOps { deps, searcher: Arc::new(PromptSearcher::new(projects, searcher_deps)), back: Mutex::new(back) }
    }

    pub fn pick(&self, request_id: String, ctx: ClientContext) {
        let deps = self.deps.clone();
        spawn("host-pick", move || {
            let (result, dir) = deps.pick_folder().unwrap_or((FolderPickResult::Unsupported, None));
            ctx.reply(MonitorEvent::FolderPicked(FolderPicked { request_id, result, dir }));
        });
    }

    pub fn focus(&self, terminal_id: String, pid: Option<u32>, ctx: ClientContext) {
        let deps = self.deps.clone();
        spawn("host-focus", move || {
            let result = focus_of(deps.as_ref(), pid);
            ctx.reply(MonitorEvent::FocusResult(FocusResult { terminal_id, result }));
        });
    }

    pub fn focus_run(&self, pid: Option<u32>, done: oneshot::Sender<Option<FocusWindowResult>>) {
        let deps = self.deps.clone();
        spawn("host-focus", move || {
            let _ = done.send(Some(focus_of(deps.as_ref(), pid)));
        });
    }

    pub fn open(&self, terminal_id: String, dir: Option<String>, app: OpenApp, ctx: ClientContext) {
        let deps = self.deps.clone();
        spawn("host-open", move || {
            let result = match dir {
                None => OpenResult::NotFound,
                Some(d) => deps.open_folder(&d, app).unwrap_or(OpenResult::Unsupported),
            };
            ctx.reply(MonitorEvent::TerminalOpenResult(TerminalOpenResult { terminal_id, app, result }));
        });
    }

    pub fn launch(&self, target: (String, String), mode: LaunchMode, app: Option<TerminalApp>) {
        let deps = self.deps.clone();
        let back = self.back.lock().ok().map(|b| b.clone());
        spawn("host-launch", move || {
            let (dir, label) = target;
            if let (Some(result), Some(tx)) = (deps.launch(&dir, mode, app), back) {
                let _ = tx.send(Cmd::LaunchDone(label, mode, result));
            }
        });
    }

    pub fn search(&self, client: u64, req: PromptSearchRequest, ctx: ClientContext) {
        let searcher = self.searcher.clone();
        spawn("prompt-search", move || {
            searcher.start(client, &req, &mut |v| reply_value(&ctx, v));
        });
    }

    pub fn search_cancel(&self, client: u64, search_id: &str) {
        self.searcher.cancel(client, Some(search_id));
    }

    pub fn search_open(&self, client: u64, search_id: String, session_id: String, ctx: ClientContext) {
        let searcher = self.searcher.clone();
        spawn("prompt-open", move || {
            searcher.open(client, &search_id, &session_id, &mut |v| reply_value(&ctx, v));
        });
    }

    pub fn drop_client(&self, client: u64) {
        self.searcher.drop_client(client);
    }
}
