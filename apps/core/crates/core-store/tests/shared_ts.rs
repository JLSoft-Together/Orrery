use std::collections::BTreeSet;
use std::path::PathBuf;

use core_store::diagnostics::Diagnostics;
use core_store::event::{FavoriteRejectReason, FocusWindowResult, FolderPickResult, MonitorSnapshot, OpenApp, OpenResult};
use core_store::extra::{
    BackgroundJob, DayTimeline, Favorite, GitDiffStat, JobState, LaneStatus, LimitForecast, LimitWindow, NotifySettings, PlanLimits, QuietHours,
    ResponseStats, SensitiveSettings, SessionRecord, SnoozeEntry, ToastHealth, ToastState, ToastTestResult, ToolOutcome, ToolSpan, ToolSpansData,
    ToolStats, TimelineLane, TimelineSegment, UsageBucket, UsageScan, UsageScanState, UsageSnapshot,
};
use core_store::model::{
    ActivityData, ActivityEvent, ActivityKind, Agent, AgentPatch, AgentRole, AgentStatus, CacheTtl, LimitHit, PeerLink, PeerLinkDir, PeerLinkKind,
    PeerLinkPatch, Provider, TerminalPatch, TerminalSession, TerminalStatus,
};
use core_store::{MonitorEvent, PROTOCOL_VERSION, SUPPORTED_CLAUDE_VERSION};

fn shared(file: &str) -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../packages/shared/src").join(file);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn camel(field: &str) -> String {
    let field = field.trim_start_matches("r#");
    let mut out = String::new();
    let mut upper = false;
    for c in field.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            out.extend(c.to_uppercase());
            upper = false;
        } else {
            out.push(c);
        }
    }
    out
}

fn rust_fields(fields: &[&str]) -> BTreeSet<String> {
    fields.iter().map(|f| camel(f)).collect()
}

fn block<'a>(src: &'a str, head: &str) -> &'a str {
    let start = src.find(head).unwrap_or_else(|| panic!("{head} not found"));
    let body = &src[start..];
    let open = body.find('{').unwrap();
    let mut depth = 0usize;
    for (i, c) in body[open..].char_indices() {
        match c {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &body[open + 1..open + i];
                }
            }
            _ => {}
        }
    }
    panic!("{head} not closed")
}

fn fields_at(body: &str, indent: usize) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for line in body.lines() {
        let trimmed = line.trim_start_matches(' ');
        if line.len() - trimmed.len() != indent {
            continue;
        }
        let name: String = trimmed.chars().take_while(|c| c.is_ascii_alphanumeric() || *c == '_').collect();
        if !name.is_empty() && trimmed[name.len()..].starts_with([':', '?']) {
            out.insert(name);
        }
    }
    out
}

fn ts_fields(src: &str, name: &str) -> BTreeSet<String> {
    fields_at(block(src, &format!("export interface {name} ")), 2)
}

fn ts_union(src: &str, name: &str) -> BTreeSet<String> {
    let head = format!("export type {name} =");
    let start = src.find(&head).unwrap_or_else(|| panic!("{head} not found"));
    let rest = &src[start + head.len()..];
    let end = rest.find("\nexport ").unwrap_or(rest.len());
    let mut out = BTreeSet::new();
    for line in rest[..end].lines() {
        let line = line.trim();
        if line.starts_with("/**") || line.starts_with("//") {
            continue;
        }
        for (i, part) in line.split('\'').enumerate() {
            if i % 2 == 1 {
                out.insert(part.to_string());
            }
        }
    }
    out
}

fn strs(all: &[&str]) -> BTreeSet<String> {
    all.iter().map(|s| s.to_string()).collect()
}

#[test]
fn interfaces_match_rust_fields() {
    let src = shared("types.ts");
    let checks: &[(&str, &[&str])] = &[
        ("TerminalSession", TerminalSession::FIELDS),
        ("LimitHit", LimitHit::FIELDS),
        ("Agent", Agent::FIELDS),
        ("PeerLink", PeerLink::FIELDS),
        ("ActivityEvent", ActivityEvent::FIELDS),
        ("Favorite", Favorite::FIELDS),
        ("MonitorSnapshot", MonitorSnapshot::FIELDS),
        ("QuietHours", QuietHours::FIELDS),
        ("NotifySettings", NotifySettings::FIELDS),
        ("SnoozeEntry", SnoozeEntry::FIELDS),
        ("BackgroundJob", BackgroundJob::FIELDS),
        ("LimitWindow", LimitWindow::FIELDS),
        ("LimitForecast", LimitForecast::FIELDS),
        ("PlanLimits", PlanLimits::FIELDS),
        ("ResponseStats", ResponseStats::FIELDS),
        ("ToolStats", ToolStats::FIELDS),
        ("UsageScan", UsageScan::FIELDS),
        ("UsageSnapshot", UsageSnapshot::FIELDS),
        ("SessionRecord", SessionRecord::FIELDS),
        ("TimelineSegment", TimelineSegment::FIELDS),
        ("TimelineLane", TimelineLane::FIELDS),
        ("DayTimeline", DayTimeline::FIELDS),
        ("GitDiffStat", GitDiffStat::FIELDS),
        ("Diagnostics", Diagnostics::FIELDS),
        ("ToolSpan", ToolSpan::FIELDS),
        ("ToolSpansData", ToolSpansData::FIELDS),
    ];
    for (name, fields) in checks {
        assert_eq!(rust_fields(fields), ts_fields(&src, name), "{name}");
    }
    let mut bucket = ts_fields(&src, "UsageBucket");
    bucket.extend(ts_fields(&src, "UsageTotals"));
    assert_eq!(rust_fields(UsageBucket::FIELDS), bucket, "UsageBucket");
    let data = block(block(&src, "export interface ActivityEvent "), "data?:");
    assert_eq!(rust_fields(ActivityData::FIELDS), fields_at(data, 4), "ActivityEvent.data");
    assert_eq!(rust_fields(SensitiveSettings::FIELDS), ts_fields(&shared("sensitive.ts"), "SensitiveSettings"));
}

#[test]
fn patches_cover_every_entity_field() {
    assert_eq!(rust_fields(TerminalPatch::FIELDS), rust_fields(TerminalSession::FIELDS));
    assert_eq!(rust_fields(AgentPatch::FIELDS), rust_fields(Agent::FIELDS));
    assert_eq!(rust_fields(PeerLinkPatch::FIELDS), rust_fields(PeerLink::FIELDS));
}

#[test]
fn string_unions_match_rust_enums() {
    let src = shared("types.ts");
    let checks: &[(&str, &[&str])] = &[
        ("TerminalStatus", TerminalStatus::ALL),
        ("AgentStatus", AgentStatus::ALL),
        ("AgentRole", AgentRole::ALL),
        ("Provider", Provider::ALL),
        ("CacheTtl", CacheTtl::ALL),
        ("PeerLinkKind", PeerLinkKind::ALL),
        ("PeerLinkDir", PeerLinkDir::ALL),
        ("ActivityKind", ActivityKind::ALL),
        ("JobState", JobState::ALL),
        ("UsageScanState", UsageScanState::ALL),
        ("ToastState", ToastState::ALL),
        ("ToastHealth", ToastHealth::ALL),
        ("ToastTestResult", ToastTestResult::ALL),
        ("ToolOutcome", ToolOutcome::ALL),
        ("LaneStatus", LaneStatus::ALL),
        ("FavoriteRejectReason", FavoriteRejectReason::ALL),
        ("FocusWindowResult", FocusWindowResult::ALL),
        ("FolderPickResult", FolderPickResult::ALL),
        ("OpenApp", OpenApp::ALL),
        ("OpenResult", OpenResult::ALL),
    ];
    for (name, all) in checks {
        assert_eq!(strs(all), ts_union(&src, name), "{name}");
    }
}

#[test]
fn every_monitor_event_type_is_known() {
    let src = shared("types.ts");
    let start = src.find("export type MonitorEvent =").unwrap();
    let end = start + src[start..].find("\nexport ").unwrap();
    let mut types = Vec::new();
    for part in src[start..end].split("type: '").skip(1) {
        types.push(part.split('\'').next().unwrap().to_string());
    }
    assert_eq!(types.len(), 43);
    for t in &types {
        if let Err(err) = serde_json::from_value::<MonitorEvent>(serde_json::json!({ "type": t, "payload": 0 })) {
            assert!(!err.to_string().contains("unknown variant"), "{t}: {err}");
        }
    }
    let unknown = serde_json::from_value::<MonitorEvent>(serde_json::json!({ "type": "nope", "payload": 0 })).unwrap_err().to_string();
    assert!(unknown.contains("unknown variant"), "{unknown}");
}

#[test]
fn constants_match_shared() {
    let status = shared("status.ts");
    let lines: Vec<&str> = status.lines().map(str::trim_end).collect();
    assert!(lines.contains(&format!("export const PROTOCOL_VERSION = {PROTOCOL_VERSION}").as_str()));
    assert!(lines.contains(&format!("export const SUPPORTED_CLAUDE_VERSION = '{SUPPORTED_CLAUDE_VERSION}'").as_str()));
}
