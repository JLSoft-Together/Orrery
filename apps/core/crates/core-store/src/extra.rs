use indexmap::IndexMap;

use crate::num::JsNum;

pub type Dict<T> = IndexMap<String, T>;

string_enum! {
    pub enum JobState { Working = "working", Blocked = "blocked", Done = "done", Unknown = "unknown" }
}

string_enum! {
    pub enum UsageScanState { Idle = "idle", Scanning = "scanning", Ready = "ready" }
}

string_enum! {
    pub enum ToastState { On = "on", Off = "off", Unsupported = "unsupported" }
}

string_enum! {
    pub enum ToastHealth { Ok = "ok", GlobalOff = "globalOff", AppOff = "appOff", Unknown = "unknown" }
}

string_enum! {
    pub enum ToastTestResult { Ok = "ok", Failed = "failed", Unsupported = "unsupported" }
}

string_enum! {
    pub enum ToolOutcome { Ok = "ok", Error = "error", Interrupted = "interrupted" }
}

string_enum! {
    pub enum LaneStatus { Working = "working", Waiting = "waiting", Idle = "idle" }
}

record! {
    pub struct GitDiffStat {
        req { files: u64, insertions: u64, deletions: u64, untracked: u64, at: String }
        opt {}
    }
}

record! {
    pub struct Favorite {
        req { dir: String, label: String, added_at: String }
        opt {}
    }
}

record! {
    pub struct BackgroundJob {
        req { id: String, state: JobState }
        opt {
            name: String,
            tempo: String,
            tasks: u64,
            queued: u64,
            tokens: u64,
            state_since: String,
            updated_at: String,
            session_id: String,
        }
    }
}

record! {
    pub struct LimitForecast {
        req { pct_per_hour: JsNum }
        opt { full_at: String, before_reset: bool }
    }
}

record! {
    pub struct LimitWindow {
        req { used_pct: JsNum }
        opt { resets_at: String, forecast: LimitForecast }
    }
}

record! {
    pub struct PlanLimits {
        req { updated_at: String }
        opt { five_hour: LimitWindow, seven_day: LimitWindow, plan: String }
    }
}

record! {
    pub struct ResponseStats {
        req { day: String, count: u64, total_ms: JsNum, long_waits: u64 }
        opt { median_ms: JsNum }
    }
}

record! {
    pub struct UsageBucket {
        req {
            key: String,
            day: String,
            model: String,
            project: String,
            messages: u64,
            input: u64,
            output: u64,
            cache_write5m: u64,
            cache_write1h: u64,
            cache_read: u64,
        }
        opt { fast: bool }
    }
}

record! {
    pub struct UsageScan {
        req { state: UsageScanState, files_done: u64, files_total: u64 }
        opt {}
    }
}

record! {
    pub struct UsageSnapshot {
        req { buckets: Vec<UsageBucket>, scan: UsageScan }
        opt { roots: Dict<String> }
    }
}

record! {
    pub struct QuietHours {
        req { enabled: bool, from: String, to: String }
        opt {}
    }
}

record! {
    pub struct NotifySettings {
        req { toast: ToastState, kinds: Dict<bool>, quiet: QuietHours, stuck_minutes: JsNum, idle_minutes: JsNum }
        opt { health: ToastHealth }
    }
}

record! {
    pub struct SnoozeEntry {
        req { until: JsNum }
        opt { since: String }
    }
}

record! {
    pub struct ToolSpan {
        req { id: String, terminal_id: String, agent_id: String, tool: String, start: String }
        opt { end: String, outcome: ToolOutcome, path: String, access: String }
    }
}

record! {
    pub struct ToolSpansData {
        req { spans: Vec<ToolSpan> }
        opt { terminal_id: String, since: String }
    }
}

record! {
    pub struct SessionRecord {
        req {
            id: String,
            title: String,
            ended_at: String,
            input_tokens: u64,
            output_tokens: u64,
            cache_read_tokens: u64,
            subagents: u64,
        }
        opt {
            session_id: String,
            cwd: String,
            git_branch: String,
            model: String,
            started_at: String,
            compactions: u64,
            cost_usd: JsNum,
            working_ms: JsNum,
            waiting_ms: JsNum,
        }
    }
}

record! {
    pub struct TimelineSegment {
        req { from: String, status: LaneStatus }
        opt { to: String }
    }
}

record! {
    pub struct TimelineLane {
        req { id: String, title: String, segments: Vec<TimelineSegment> }
        opt {}
    }
}

record! {
    pub struct DayTimeline {
        req { day: String, lanes: Vec<TimelineLane> }
        opt {}
    }
}

record! {
    pub struct HeatmapDay {
        req { day: String, hours: Vec<u64> }
        opt {}
    }
}

record! {
    pub struct UsageHeatmap {
        req { days: Vec<HeatmapDay> }
        opt {}
    }
}

record! {
    pub struct ToolStatEntry {
        req { name: String, n: u64 }
        opt { errors: u64 }
    }
}

record! {
    pub struct ToolStatsUsed {
        req { skills: Vec<String>, mcp: Vec<String> }
        opt {}
    }
}

record! {
    pub struct ToolStats {
        req {
            days: u64,
            project: Option<String>,
            projects: Vec<String>,
            tools: Vec<ToolStatEntry>,
            skills: Vec<ToolStatEntry>,
            agents: Vec<ToolStatEntry>,
            mcp: Vec<ToolStatEntry>,
            used30: ToolStatsUsed,
        }
        opt {}
    }
}

record! {
    pub struct SensitiveSettings {
        req { groups: Dict<bool>, globs: Vec<String>, ignore: Dict<Vec<String>> }
        opt {}
    }
}
