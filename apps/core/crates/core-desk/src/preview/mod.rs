pub mod branches;
pub mod changes;
pub mod checkpoints;
pub mod commit;
pub mod envcheck;
pub mod exec;
pub mod explorer;
pub mod files;
pub mod gate;
pub mod gitmisc;
pub mod history;
pub mod i18n;
pub mod i18n_parse;
pub mod ipc_files;
pub mod ipc_git;
pub mod ipc_tools;
pub mod ipc_wt;
pub mod log;
pub mod paths;
pub mod scripts;
pub mod search;
pub mod worktrees;

use crate::{Desk, DeskEnv};

pub fn register(desk: &mut Desk, env: &DeskEnv) {
    ipc_git::register(desk);
    ipc_wt::register(desk, env);
    ipc_files::register(desk, env);
    ipc_tools::register(desk, env);
}
