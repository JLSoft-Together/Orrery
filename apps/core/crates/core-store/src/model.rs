use std::sync::Arc;

use crate::extra::GitDiffStat;
use crate::num::JsNum;

string_enum! {
    pub enum TerminalStatus { Working = "working", Waiting = "waiting", Idle = "idle", Stale = "stale", Error = "error", Unknown = "unknown" }
}

string_enum! {
    pub enum AgentStatus { Working = "working", Waiting = "waiting", Idle = "idle", Completed = "completed", Error = "error", Cancelled = "cancelled", Unknown = "unknown" }
}

string_enum! {
    pub enum AgentRole { Main = "main", Subagent = "subagent" }
}

string_enum! {
    pub enum Provider { Claude = "claude", Codex = "codex", Deepseek = "deepseek" }
}

string_enum! {
    pub enum CompactTrigger { Manual = "manual", Auto = "auto" }
}

string_enum! {
    pub enum CacheTtl { FiveMinutes = "5m", OneHour = "1h" }
}

string_enum! {
    pub enum PeerLinkKind { Message = "message", Opened = "opened" }
}

string_enum! {
    pub enum PeerLinkDir { Ab = "ab", Ba = "ba" }
}

string_enum! {
    pub enum ActivityKind {
        TerminalStarted = "terminal.started",
        TerminalEnded = "terminal.ended",
        TerminalRenamed = "terminal.renamed",
        TerminalStatus = "terminal.status",
        SessionCleared = "session.cleared",
        AgentStarted = "agent.started",
        AgentCompleted = "agent.completed",
        AgentFailed = "agent.failed",
        AgentCancelled = "agent.cancelled",
        ToolStarted = "tool.started",
        ToolFailed = "tool.failed",
        TerminalCompacted = "terminal.compacted",
        TerminalMode = "terminal.mode",
        LaunchStarted = "launch.started",
        LaunchFailed = "launch.failed",
        JobBlocked = "job.blocked",
        JobDone = "job.done",
        JobStopped = "job.stopped",
        JobStopFailed = "job.stopFailed",
        PeerMessage = "peer.message",
        Sensitive = "sensitive",
        TurnStarted = "turn.started",
        TurnCompleted = "turn.completed",
        TurnFailed = "turn.failed",
        TurnInterrupted = "turn.interrupted",
    }
}

record! {
    pub struct LimitHit {
        req { at: String }
        opt { resets_at: String, window: String, auto_resume: bool }
    }
}

entity! {
    pub struct TerminalSession / TerminalPatch {
        req { title: String, status: TerminalStatus }
        opt {
            process_id: u64,
            parent_process_id: u64,
            shell: String,
            alias: String,
            note: String,
            name_source: String,
            kind: String,
            entrypoint: String,
            cwd: String,
            provider: Provider,
            claude_session_id: String,
            claude_version: String,
            codex_session_id: String,
            codex_version: String,
            deepseek_session_id: String,
            raw_status: String,
            status_since: String,
            waiting_for: String,
            background_shell: bool,
            permission_mode: String,
            git_branch: String,
            compactions: u64,
            cost_usd: JsNum,
            compacting_since: String,
            compact_trigger: CompactTrigger,
            denied_count: u64,
            denied_tools: Arc<Vec<String>>,
            diff: Arc<GitDiffStat>,
            limit_hit: Arc<LimitHit>,
            started_at: String,
            last_activity_at: String,
            ended_at: String,
        }
    }
}

entity! {
    pub struct Agent / AgentPatch {
        req { terminal_id: String, role: AgentRole, status: AgentStatus }
        opt {
            parent_id: String,
            provider: Provider,
            name: String,
            r#type: String,
            description: String,
            tool_use_id: String,
            spawn_depth: u64,
            model: String,
            effort: String,
            current_tool: String,
            started_at: String,
            updated_at: String,
            completed_at: String,
            input_tokens: u64,
            output_tokens: u64,
            cache_read_tokens: u64,
            total_tokens: u64,
            context_tokens: u64,
            first_context_tokens: u64,
            context_window: u64,
            context_pct: JsNum,
            cache_ttl: CacheTtl,
            cache_at: String,
            msg_in: u64,
            msg_out: u64,
        }
    }
}

entity! {
    pub struct PeerLink / PeerLinkPatch {
        req {
            kind: PeerLinkKind,
            a: String,
            b: String,
            ab: u64,
            ba: u64,
            first_at: String,
            last_at: String,
            last_dir: PeerLinkDir,
        }
        opt { last_agent_id: String }
    }
}

record! {
    #[derive(Default)]
    pub struct ActivityData {
        req {}
        opt {
            title: String,
            from: String,
            to: String,
            tool: String,
            agent_type: String,
            description: String,
            waiting_for: String,
            trigger: String,
            mode: String,
            pre_tokens: u64,
            post_tokens: u64,
            error: String,
            job_id: String,
            rule: String,
            path: String,
            file: String,
            project: String,
        }
    }
}

record! {
    pub struct ActivityEvent {
        req { id: String, at: String, kind: ActivityKind }
        opt { terminal_id: String, agent_id: String, data: ActivityData }
    }
}
