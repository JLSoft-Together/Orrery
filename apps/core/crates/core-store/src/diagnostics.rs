use crate::extra::UsageScan;

record! {
    pub struct Watchers {
        req { sessions: bool, projects: bool }
        opt {}
    }
}

record! {
    pub struct CodexDiagnostics {
        req { home: String, watching: bool, threads: u64, processes: u64 }
        opt {}
    }
}

record! {
    pub struct DeepseekDiagnostics {
        req { outbox: Option<String>, threads: u64, processes: u64 }
        opt {}
    }
}

record! {
    pub struct SessionCounts {
        req { live: u64, ended: u64 }
        opt {}
    }
}

record! {
    pub struct StatusLineDiagnostics {
        req { reports: u64 }
        opt { last_at: String }
    }
}

record! {
    pub struct Diagnostics {
        req {
            started_at: String,
            platform: String,
            node: String,
            claude_root: String,
            claude_root_found: bool,
            data_dir: String,
            watchers: Watchers,
            verify_processes: bool,
            sessions: SessionCounts,
            status_line: StatusLineDiagnostics,
            clients: u64,
        }
        opt {
            collector_version: String,
            codex: CodexDiagnostics,
            deepseek: DeepseekDiagnostics,
            usage: UsageScan,
        }
    }
}
