use std::sync::Arc;

use serde_json::Value;

use crate::client::{parse_client_message, ClientMessage};
use crate::server::{MessageHook, StoreHandle};
use crate::ws::{ClientContext, Clients, TapFn};

pub const TAP_CLIENT_ID: u64 = u64::MAX;

pub const TAP_ALLOWED: [&str; 8] = ["terminal.alias", "terminal.note", "terminal.snooze", "terminal.dismiss", "history.get", "toolSpans.get", "tool-spans.get", "resync"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TapError {
    Denied,
    Invalid,
}

impl std::fmt::Display for TapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TapError::Denied => write!(f, "message type is not allowed through the tap"),
            TapError::Invalid => write!(f, "message is not a valid client message"),
        }
    }
}

impl std::error::Error for TapError {}

pub struct TapGuard {
    clients: Arc<Clients>,
    id: u64,
}

impl Drop for TapGuard {
    fn drop(&mut self) {
        self.clients.remove_tap(self.id);
    }
}

#[derive(Clone)]
pub struct StoreTap {
    store: StoreHandle,
    clients: Arc<Clients>,
    on_message: Option<MessageHook>,
    client_id: u64,
}

fn snapshot_text(store: &mut core_store::Store) -> String {
    serde_json::to_string(&store.snapshot_message()).unwrap_or_default()
}

impl StoreTap {
    pub(crate) fn new(store: StoreHandle, clients: Arc<Clients>, on_message: Option<MessageHook>) -> StoreTap {
        StoreTap { store, clients, on_message, client_id: TAP_CLIENT_ID }
    }

    pub fn snapshot(&self) -> String {
        self.store.with(snapshot_text)
    }

    pub fn subscribe(&self, f: impl Fn(Arc<str>) + Send + Sync + 'static) -> TapGuard {
        let id = self.clients.add_tap(Arc::new(f) as TapFn);
        TapGuard { clients: self.clients.clone(), id }
    }

    pub fn attach(&self, f: impl Fn(Arc<str>) + Send + Sync + 'static) -> (String, TapGuard) {
        let f: TapFn = Arc::new(f);
        self.store.with(|s| {
            let snapshot = snapshot_text(s);
            let id = self.clients.add_tap(f);
            (snapshot, TapGuard { clients: self.clients.clone(), id })
        })
    }

    pub fn send(&self, mut msg: Value, reply: impl Fn(Arc<str>) + Send + Sync + 'static) -> Result<Option<String>, TapError> {
        let kind = msg.get("type").and_then(Value::as_str).ok_or(TapError::Invalid)?;
        if !TAP_ALLOWED.contains(&kind) {
            return Err(TapError::Denied);
        }
        if kind == "tool-spans.get" {
            msg["type"] = Value::from("toolSpans.get");
        }
        let parsed = parse_client_message(&msg).ok_or(TapError::Invalid)?;
        if parsed == ClientMessage::Resync {
            return Ok(Some(self.snapshot()));
        }
        if let Some(hook) = &self.on_message {
            let ctx = ClientContext::callback(Arc::new(reply), self.clients.len(), self.client_id);
            hook(parsed, &ctx);
        }
        Ok(None)
    }
}
