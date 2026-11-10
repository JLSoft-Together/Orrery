pub mod branches;
pub mod changes;
pub mod checkpoints;
pub mod checks;
pub mod commit;
pub mod ctx;
pub mod deny;
pub mod envcheck;
pub mod exec;
pub mod explorer;
pub mod files;
pub mod format;
pub mod gate;
pub mod gitmisc;
pub mod history;
pub mod http;
pub mod i18n;
pub mod i18n_parse;
pub mod ipc_files;
pub mod ipc_git;
pub mod ipc_tools;
pub mod ipc_wt;
pub mod log;
pub mod memdoctor;
pub mod memlint;
pub mod paths;
pub mod perm;
pub mod replace;
pub mod scripts;
pub mod search;
pub mod spawn;
pub mod worktrees;

use crate::{Desk, DeskEnv};

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    ipc_git::register(desk);
    ipc_wt::register(desk, env);
    let history = std::sync::Arc::new(history::LocalHistory::new(env.data_dir.clone()));
    ipc_files::register(desk, env, history.clone());
    ipc_tools::register(desk, env, history);
}
