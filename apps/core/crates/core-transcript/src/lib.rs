pub mod cache;
pub mod collate;
pub mod index;
pub mod jobs;
pub mod json;
pub mod omap;
pub mod query;
pub mod record;
pub mod serve;
pub mod time;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests;
