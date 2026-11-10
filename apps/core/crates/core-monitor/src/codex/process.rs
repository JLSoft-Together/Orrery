use crate::cliprocs::{list_named, list_named_in, split_args, top_level, CliProcess, Rooted};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessMode {
    Exec,
    Tui,
    AppServer,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CodexProcess {
    pub pid: u32,
    pub start: i64,
    pub mode: ProcessMode,
    pub resume_id: Option<String>,
    pub ancestors: Vec<u32>,
    pub orphan: bool,
}

impl Rooted for CodexProcess {
    fn pid(&self) -> u32 {
        self.pid
    }
    fn ancestors(&self) -> &[u32] {
        &self.ancestors
    }
}

const VALUE_FLAGS: [&str; 22] = [
    "-c", "--config", "-m", "--model", "-p", "--profile", "-s", "--sandbox", "-a", "--ask-for-approval", "-C", "--cd", "-i", "--image", "--add-dir", "--enable", "--disable", "--local-provider", "--remote",
    "--remote-auth-token-env", "--output-schema", "--color",
];
const EXTRA_VALUE_FLAGS: [&str; 2] = ["-o", "--output-last-message"];
const OTHER_COMMANDS: [&str; 18] = [
    "login", "logout", "mcp", "mcp-server", "completion", "sandbox", "debug", "apply", "a", "cloud", "features", "help", "update", "plugin", "marketplace", "proto", "responses-api-proxy", "stdio-to-uds",
];
const OTHER_COMMANDS_TAIL: [&str; 2] = ["generate-ts", "generate-json-schema"];

const CODEX_EXE: [&str; 1] = ["codex.exe"];
const CODEX_NAMES: [&str; 1] = ["codex"];

pub fn is_uuid(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 36 && b.iter().enumerate().all(|(i, c)| if matches!(i, 8 | 13 | 18 | 23) { *c == b'-' } else { c.is_ascii_hexdigit() })
}

fn is_value_flag(a: &str) -> bool {
    VALUE_FLAGS.contains(&a) || EXTRA_VALUE_FLAGS.contains(&a)
}

fn is_other_command(a: &str) -> bool {
    OTHER_COMMANDS.contains(&a) || OTHER_COMMANDS_TAIL.contains(&a)
}

fn positionals(args: &[String]) -> Vec<&str> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if a == "--" {
            break;
        }
        if a.starts_with('-') {
            if !a.contains('=') && is_value_flag(a) {
                i += 1;
            }
            i += 1;
            continue;
        }
        out.push(a);
        i += 1;
    }
    out
}

pub fn mode_of_command_line(cmd: &str) -> (ProcessMode, Option<String>) {
    let args = split_args(cmd);
    let pos = positionals(args.get(1..).unwrap_or(&[]));
    let uuid_at = |i: usize| pos.get(i).filter(|s| is_uuid(s)).map(|s| (*s).to_owned());
    match pos.first().copied() {
        Some("exec" | "e") => (ProcessMode::Exec, if pos.get(1) == Some(&"resume") { uuid_at(2) } else { None }),
        Some("resume") => (ProcessMode::Tui, uuid_at(1)),
        Some("fork") => (ProcessMode::Tui, None),
        Some("app-server") => (ProcessMode::AppServer, None),
        Some("review") => (ProcessMode::Exec, None),
        Some(f) if is_other_command(f) => (ProcessMode::Other, None),
        _ => (ProcessMode::Tui, None),
    }
}

fn to_codex(p: CliProcess) -> CodexProcess {
    let (mode, resume_id) = mode_of_command_line(&p.cmd);
    CodexProcess { pid: p.pid, start: p.start, mode, resume_id, ancestors: p.ancestors, orphan: p.orphan }
}

pub fn codex_processes(rows: Vec<CliProcess>) -> Vec<CodexProcess> {
    top_level(rows.into_iter().map(to_codex).collect())
}

pub fn list_codex_processes() -> Option<Vec<CodexProcess>> {
    list_named(&CODEX_EXE, &CODEX_NAMES).map(codex_processes)
}

pub fn list_codex_processes_in(table: &[core_procs::Proc]) -> Option<Vec<CodexProcess>> {
    list_named_in(table, &CODEX_EXE, &CODEX_NAMES).map(codex_processes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TID: &str = "01a125e6-f212-7711-a885-025fd33ab05a";

    #[test]
    fn reads_the_subcommand_and_resume_id_only() {
        let exe = "\"C:\\npm\\codex.exe\"";
        assert_eq!(mode_of_command_line(&format!("{exe} exec -s read-only -i logo.png -")), (ProcessMode::Exec, None));
        assert_eq!(mode_of_command_line(&format!("{exe} -m gpt-6 exec resume {TID}")), (ProcessMode::Exec, Some(TID.to_owned())));
        assert_eq!(mode_of_command_line(&format!("{exe} resume {TID}")), (ProcessMode::Tui, Some(TID.to_owned())));
        assert_eq!(mode_of_command_line(&format!("{exe} \"resume my work\"")), (ProcessMode::Tui, None));
        assert_eq!(mode_of_command_line(&format!("{exe} app-server")), (ProcessMode::AppServer, None));
        assert_eq!(mode_of_command_line(&format!("{exe} login")), (ProcessMode::Other, None));
        assert_eq!(mode_of_command_line(exe), (ProcessMode::Tui, None));
        assert_eq!(mode_of_command_line(&format!("{exe} fork {TID}")), (ProcessMode::Tui, None));
        assert_eq!(mode_of_command_line(&format!("{exe} review --base main")), (ProcessMode::Exec, None));
    }

    #[test]
    fn drops_helpers_started_by_another_codex_process() {
        let row = |pid: u32, anc: Vec<u32>, cmd: &str| CliProcess { pid, start: 1, ancestors: anc, orphan: false, cmd: cmd.into() };
        let out = codex_processes(vec![row(10, vec![9, 8], "codex.exe exec -"), row(11, vec![10], "codex.exe sandbox"), row(13, vec![30], "codex.exe")]);
        assert_eq!(out.iter().map(|p| p.pid).collect::<Vec<_>>(), vec![10, 13]);
        assert_eq!(out[1].mode, ProcessMode::Tui);
    }
}
