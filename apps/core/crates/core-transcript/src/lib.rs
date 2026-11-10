pub mod cache;
pub mod codex;
pub mod collate;
pub mod conflict;
pub mod extract;
pub mod index;
pub mod jobs;
pub mod json;
pub mod jsonline;
pub mod jsstr;
pub mod omap;
pub mod origin;
pub mod prune;
pub mod query;
pub mod record;
pub mod rules;
pub mod serve;
pub mod sidefiles;
pub mod signal;
pub mod sigserve;
pub mod spans;
pub mod state;
pub mod tail;
pub mod time;
pub mod tracker;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests;
#[cfg(test)]
mod sigtests;
