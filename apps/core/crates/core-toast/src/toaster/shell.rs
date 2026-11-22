use core_store::extra::ToastState;

use super::{Draft, Toaster};

pub const SHELL_COMMAND_SHOWN: usize = 80;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellDone {
    pub pane: String,
    pub name: String,
    pub command: String,
    pub code: i64,
    pub ms: u64,
}

impl Toaster {
    pub fn observe_shell(&mut self, m: &ShellDone) {
        if m.pane.is_empty() || self.state() != ToastState::On || self.attentive() || self.quiet_now() {
            return;
        }
        let text = self.text();
        let one_line = m.command.split_whitespace().collect::<Vec<_>>().join(" ");
        let command = if one_line.chars().count() > SHELL_COMMAND_SHOWN { format!("{}…", one_line.chars().take(SHELL_COMMAND_SHOWN - 1).collect::<String>()) } else { one_line };
        let id = format!("shell:{}", m.pane);
        let d = Draft { title: text.shell_title(m.code, m.ms, &command), body: m.name.clone(), url: self.link_for(None), open: text.open(), persistent: false, snooze: false };
        self.show_grouped(&format!("done:{id}"), &id, &m.name, false, d);
    }
}
