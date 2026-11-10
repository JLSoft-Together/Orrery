#[macro_use]
mod macros;

pub mod diagnostics;
pub mod event;
pub mod extra;
pub mod jseq;
pub mod model;
pub mod num;
pub mod replay;
pub mod store;
pub mod time;

pub use event::{MonitorEvent, MonitorSnapshot, ServerMessage};
pub use model::{ActivityEvent, Agent, AgentPatch, PeerLink, PeerLinkPatch, TerminalPatch, TerminalSession};
pub use num::JsNum;
pub use store::{NewActivity, Store, StoreOptions};

pub const PROTOCOL_VERSION: u64 = 1;
pub const SUPPORTED_CLAUDE_VERSION: &str = "2.1.289";
