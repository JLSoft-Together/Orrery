use std::sync::atomic::Ordering;

use serde_json::{json, Value};
use tauri::{AppHandle, Manager};

use super::config::valid_machine_id;
use super::Remote;
use crate::state::lock;

pub const MAX_PANE_KEY: usize = 128;

pub fn valid_pane_key(key: &str) -> bool {
    (1..=MAX_PANE_KEY).contains(&key.len()) && key.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'.' | b':' | b'-'))
}

pub fn pane_of(summary: &Value, terminal: &str) -> Option<String> {
    if terminal.is_empty() {
        return None;
    }
    summary["tabs"].as_array()?.iter().filter_map(|t| t["panes"].as_array()).flatten().find(|p| p["terminalId"].as_str() == Some(terminal)).and_then(|p| p["key"].as_str()).filter(|k| valid_pane_key(k)).map(str::to_owned)
}

pub fn build(relay: &str, machine: &str, pane: &str) -> Option<String> {
    let base = relay.trim_end_matches('/');
    if !base.starts_with("https://") || !valid_machine_id(machine) || !valid_pane_key(pane) {
        return None;
    }
    Some(format!("{base}/o/{machine}/{pane}"))
}

impl Remote {
    pub fn link_for(&self, terminal: &str) -> Option<String> {
        let ctx = lock(&self.ctx).clone()?;
        if !ctx.running.load(Ordering::SeqCst) || lock(&ctx.devices).list().is_empty() {
            return None;
        }
        let pane = pane_of(&ctx.board.summary()?, terminal)?;
        let (relay, machine) = {
            let cfg = lock(&ctx.config);
            (cfg.relay.url.clone(), cfg.machine_id.clone())
        };
        build(&relay, &machine, &pane)
    }
}

pub fn call(app: &AppHandle, p: &Value) -> Value {
    let terminal = p["terminalId"].as_str().unwrap_or("");
    let url = app.try_state::<Remote>().and_then(|r| r.link_for(terminal));
    json!({ "url": url })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::test_util::rig;

    const MACHINE: &str = "abcdefghijklmnopqrstuv";

    fn summary() -> Value {
        json!({"rev": 1, "tabs": [{"key": "t1", "panes": [
            {"key": "p1", "terminalId": "term-a"},
            {"key": "p 2/../x", "terminalId": "term-b"},
            {"key": "p3"}
        ]}]})
    }

    #[test]
    fn maps_terminal_to_safe_pane_key() {
        assert_eq!(pane_of(&summary(), "term-a").as_deref(), Some("p1"));
        assert_eq!(pane_of(&summary(), "term-b"), None);
        assert_eq!(pane_of(&summary(), "missing"), None);
        assert_eq!(pane_of(&summary(), ""), None);
        assert_eq!(pane_of(&json!({}), "term-a"), None);
    }

    #[test]
    fn builds_only_https_relay_links() {
        assert_eq!(build("https://r.example/", MACHINE, "p1").as_deref(), Some("https://r.example/o/abcdefghijklmnopqrstuv/p1"));
        assert_eq!(build("http://r.example", MACHINE, "p1"), None);
        assert_eq!(build("https://r.example", "short", "p1"), None);
        assert_eq!(build("https://r.example", MACHINE, "a/b"), None);
        assert_eq!(build("https://r.example", MACHINE, "a?b=1"), None);
        assert_eq!(build("https://r.example", MACHINE, &"a".repeat(129)), None);
    }

    #[test]
    fn no_link_while_remote_is_off() {
        let rig = rig("pane-link-off", "https://r.example", true);
        assert_eq!(rig.remote.link_for("term-a"), None);
        rig.remote.publish(&summary());
        assert_eq!(rig.remote.link_for("term-a"), None);
    }
}
