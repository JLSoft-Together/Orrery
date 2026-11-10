use std::path::Path;
use std::sync::Arc;

use core_store::extra::{LimitWindow, PlanLimits};
use core_store::model::CompactTrigger;
use core_transcript::sidefiles::SideFiles;
use serde::Deserialize;
use serde_json::{Map, Value};

pub const COMPACT_STALE_MS: i64 = 15 * 60_000;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookRecord {
    pub session_id: String,
    pub at: String,
    pub compacting_since: Option<String>,
    pub compact_trigger: Option<CompactTrigger>,
    pub denied_count: f64,
    pub denied_tools: Arc<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusLineRecord {
    pub session_id: String,
    pub at: String,
    pub cost_usd: Option<f64>,
    pub context_window: Option<f64>,
    pub context_pct: Option<f64>,
    pub buffer_pct: Option<f64>,
    pub five_hour: Option<LimitWindow>,
    pub seven_day: Option<LimitWindow>,
}

fn read<T: for<'de> Deserialize<'de>>(op: &str, dir: &Path, now: i64) -> Vec<T> {
    let mut m = Map::new();
    m.insert("dir".into(), Value::from(dir.to_string_lossy().into_owned()));
    m.insert("now".into(), Value::from(now as f64));
    let Some(Ok(reply)) = SideFiles.handle(op, &m) else { return Vec::new() };
    let Some(Value::Array(records)) = reply.get("records") else { return Vec::new() };
    records.iter().filter_map(|r| serde_json::from_value(r.clone()).ok()).collect()
}

pub fn read_hooks(dir: &Path, now: i64) -> Vec<HookRecord> {
    read("sidefiles.hooks", dir, now)
}

pub fn read_status_lines(dir: &Path, now: i64) -> Vec<StatusLineRecord> {
    read("sidefiles.statusline", dir, now)
}

pub fn latest_limits(records: &[StatusLineRecord]) -> Option<PlanLimits> {
    let mut best: Option<&StatusLineRecord> = None;
    for r in records {
        if (r.five_hour.is_some() || r.seven_day.is_some()) && best.is_none_or(|b| r.at.encode_utf16().cmp(b.at.encode_utf16()).is_gt()) {
            best = Some(r);
        }
    }
    best.map(|b| PlanLimits { updated_at: b.at.clone(), five_hour: b.five_hour.clone(), seven_day: b.seven_day.clone(), plan: None })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_numbers_and_picks_the_newest_limits() {
        let dir = std::env::temp_dir().join(format!("ccm-sl-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let a = "11111111-2222-3333-4444-555555555555";
        let b = "66666666-7777-8888-9999-000000000000";
        std::fs::write(dir.join(format!("{a}.json")), r#"{"sessionId":"11111111-2222-3333-4444-555555555555","at":"2026-10-07T10:00:00.000Z","costUsd":1.25,"contextWindow":1000000.4,"contextPct":120,"fiveHour":{"usedPct":140,"resetsAt":"2026-10-07T12:00:00.000Z"}}"#).unwrap();
        std::fs::write(dir.join(format!("{b}.json")), r#"{"sessionId":"66666666-7777-8888-9999-000000000000","at":"2026-10-07T11:00:00.000Z","sevenDay":{"usedPct":5}}"#).unwrap();
        let now = core_store::time::now_ms();
        let records = read_status_lines(&dir, now);
        assert_eq!(records.len(), 2);
        let ra = records.iter().find(|r| r.session_id == a).unwrap();
        assert_eq!((ra.cost_usd, ra.context_window, ra.context_pct), (Some(1.25), Some(1_000_000.0), None));
        assert_eq!(ra.five_hour.as_ref().map(|w| w.used_pct.0), Some(100.0));
        let limits = latest_limits(&records).unwrap();
        assert_eq!(limits.updated_at, "2026-10-07T11:00:00.000Z");
        assert!(limits.five_hour.is_none() && limits.seven_day.is_some());
        std::fs::write(dir.join(format!("{a}.json")), r#"{"sessionId":"11111111-2222-3333-4444-555555555555","at":"2026-10-07T10:00:00.000Z","deniedCount":3,"deniedTools":["Bash","Edit","bad tool"],"compactingSince":"2026-10-07T09:59:00.000Z"}"#).unwrap();
        let hooks = read_hooks(&dir, now);
        let h = hooks.iter().find(|h| h.session_id == a).unwrap();
        assert_eq!((h.denied_count, h.denied_tools.as_slice()), (3.0, &["Bash".to_owned(), "Edit".to_owned()][..]));
        assert_eq!(h.compact_trigger, Some(CompactTrigger::Auto));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
