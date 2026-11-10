pub mod defs;
pub mod host;
pub mod ipc;
pub mod notes;
pub mod open;
pub mod pane_host;
pub mod parse_desk;
pub mod parse_page;
pub mod parse_run;
pub mod paths;
pub mod registry;
pub mod rpc;
pub mod server;
pub mod t_common;
pub mod t_desk;
pub mod t_files;
pub mod t_notes;
pub mod t_open;
pub mod t_run;
pub mod t_sessions;
pub mod t_telegram;
#[cfg(test)]
mod tests_desk;
#[cfg(test)]
mod tests_tools;
pub mod text;
pub mod tools;

#[cfg(test)]
mod tests_rpc;
