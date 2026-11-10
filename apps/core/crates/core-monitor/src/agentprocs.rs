use crate::codex::process::{list_codex_processes, CodexProcess};
use crate::deepseek::process::{list_deepseek_processes, DeepseekProcess};

pub trait AgentProcs: Send + Sync {
    fn codex(&self) -> Option<Vec<CodexProcess>>;
    fn deepseek(&self) -> Option<Vec<DeepseekProcess>>;
}

pub struct SystemProcs;

impl AgentProcs for SystemProcs {
    fn codex(&self) -> Option<Vec<CodexProcess>> {
        list_codex_processes()
    }

    fn deepseek(&self) -> Option<Vec<DeepseekProcess>> {
        list_deepseek_processes()
    }
}
