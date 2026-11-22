use std::collections::HashMap;

use core_procs::Proc;
use core_store::event::FocusWindowResult;

pub const WIN_SCRIPT: &str = r#"
$ErrorActionPreference = 'Stop'
Add-Type -Namespace Ccm -Name Win -MemberDefinition @'
[DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr h);
[DllImport("user32.dll")] public static extern bool BringWindowToTop(IntPtr h);
[DllImport("user32.dll")] public static extern bool ShowWindowAsync(IntPtr h, int n);
[DllImport("user32.dll")] public static extern bool IsIconic(IntPtr h);
[DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, IntPtr pid);
[DllImport("user32.dll")] public static extern bool AttachThreadInput(uint a, uint b, bool attach);
[DllImport("kernel32.dll")] public static extern uint GetCurrentThreadId();
[DllImport("user32.dll")] public static extern void keybd_event(byte vk, byte scan, uint flags, UIntPtr extra);
public delegate bool EnumProc(IntPtr h, IntPtr l);
[DllImport("user32.dll")] public static extern bool EnumWindows(EnumProc cb, IntPtr l);
[DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr h);
[DllImport("user32.dll")] public static extern IntPtr GetWindow(IntPtr h, uint cmd);
[DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
[DllImport("kernel32.dll")] public static extern bool FreeConsole();
[DllImport("kernel32.dll")] public static extern bool AttachConsole(uint pid);
[DllImport("kernel32.dll")] public static extern IntPtr GetConsoleWindow();
[DllImport("user32.dll")] public static extern IntPtr GetAncestor(IntPtr h, uint flags);
public static IntPtr ConsoleHost(uint pid) {
  FreeConsole();
  if (!AttachConsole(pid)) return IntPtr.Zero;
  IntPtr con = GetConsoleWindow();
  FreeConsole();
  if (con == IntPtr.Zero) return IntPtr.Zero;
  IntPtr root = GetAncestor(con, 3);
  if (root != IntPtr.Zero && root != con && IsWindowVisible(root)) return root;
  return IsWindowVisible(con) ? con : IntPtr.Zero;
}
public static int CountTop(uint pid) {
  int n = 0;
  EnumWindows((h, l) => { uint p; GetWindowThreadProcessId(h, out p); if (p == pid && IsWindowVisible(h) && GetWindow(h, 4) == IntPtr.Zero) n++; return true; }, IntPtr.Zero);
  return n;
}
'@
$id = [int]$env:CCM_FOCUS_PID
$chain = @(if ($env:CCM_FOCUS_CHAIN) { $env:CCM_FOCUS_CHAIN -split ',' | ForEach-Object { [int]$_ } })
$h = [Ccm.Win]::ConsoleHost([uint32]$id)
for ($i = 0; $h -eq [IntPtr]::Zero -and $i -lt 8 -and $id -gt 4; $i++) {
  $p = Get-Process -Id $id -ErrorAction SilentlyContinue
  if (-not $p) { break }
  if ($p.MainWindowHandle -ne [IntPtr]::Zero) {
    if ($p.ProcessName -eq 'WindowsTerminal' -and [Ccm.Win]::CountTop([uint32]$p.Id) -gt 1) { 'ambiguous'; exit }
    $h = $p.MainWindowHandle; break
  }
  if ($chain.Count -gt 0) {
    if ($i + 1 -ge $chain.Count) { break }
    $id = $chain[$i + 1]; continue
  }
  $c = Get-CimInstance Win32_Process -Filter "ProcessId=$id"
  if (-not $c) { break }
  $id = [int]$c.ParentProcessId
}
if ($h -eq [IntPtr]::Zero) {
  $wt = @(Get-Process WindowsTerminal -ErrorAction SilentlyContinue | Where-Object { $_.MainWindowHandle -ne [IntPtr]::Zero })
  if ($wt.Count -eq 1 -and [Ccm.Win]::CountTop([uint32]$wt[0].Id) -eq 1) { $h = $wt[0].MainWindowHandle }
  elseif ($wt.Count -ge 1) { 'ambiguous'; exit }
}
if ($h -eq [IntPtr]::Zero) { 'notFound'; exit }
if ([Ccm.Win]::IsIconic($h)) { [void][Ccm.Win]::ShowWindowAsync($h, 9) }
$fg = [Ccm.Win]::GetForegroundWindow()
$fgThread = [Ccm.Win]::GetWindowThreadProcessId($fg, [IntPtr]::Zero)
$me = [Ccm.Win]::GetCurrentThreadId()
$attached = $fgThread -ne 0 -and $fgThread -ne $me -and [Ccm.Win]::AttachThreadInput($me, $fgThread, $true)
[void][Ccm.Win]::BringWindowToTop($h)
[void][Ccm.Win]::SetForegroundWindow($h)
if ($attached) { [void][Ccm.Win]::AttachThreadInput($me, $fgThread, $false) }
if ([Ccm.Win]::GetForegroundWindow() -ne $h) {
  [Ccm.Win]::keybd_event(0x12, 0, 0, [UIntPtr]::Zero)
  [Ccm.Win]::keybd_event(0x12, 0, 2, [UIntPtr]::Zero)
  [void][Ccm.Win]::SetForegroundWindow($h)
}
if ([Ccm.Win]::GetForegroundWindow() -eq $h) { 'ok' } else { 'failed' }
"#;

pub const TERMINAL_SCRIPT: &str = r#"on run argv
  set target to item 1 of argv
  tell application "Terminal"
    repeat with w in windows
      repeat with t in tabs of w
        if tty of t is target then
          set selected of t to true
          set index of w to 1
          activate
          return "ok"
        end if
      end repeat
    end repeat
  end tell
  return "notFound"
end run"#;

pub const ITERM_SCRIPT: &str = r#"on run argv
  set target to item 1 of argv
  tell application "iTerm"
    repeat with w in windows
      repeat with t in tabs of w
        repeat with s in sessions of t
          if tty of s is target then
            tell w to select
            tell t to select
            tell s to select
            activate
            return "ok"
          end if
        end repeat
      end repeat
    end repeat
  end tell
  return "notFound"
end run"#;

pub fn parse_focus_output(stdout: &str) -> FocusWindowResult {
    match stdout.trim().lines().last().map(str::trim) {
        Some("ok") => FocusWindowResult::Ok,
        Some("notFound") => FocusWindowResult::NotFound,
        Some("ambiguous") => FocusWindowResult::Ambiguous,
        _ => FocusWindowResult::Failed,
    }
}

pub fn ancestor_chain(pid: u32, table: &[Proc], max: usize) -> Vec<u32> {
    let all: HashMap<u32, &Proc> = table.iter().map(|p| (p.pid, p)).collect();
    let mut chain = Vec::new();
    let mut cur = all.get(&pid).copied();
    while let Some(c) = cur {
        if chain.len() >= max || c.pid <= 4 {
            break;
        }
        chain.push(c.pid);
        let Some(parent) = all.get(&c.ppid).copied() else { break };
        if parent.pid == c.pid || matches!((parent.start_ms, c.start_ms), (Some(a), Some(b)) if a > b) {
            break;
        }
        cur = Some(parent);
    }
    chain
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PsRow {
    pub pid: u32,
    pub ppid: u32,
    pub tty: String,
    pub comm: String,
}

pub fn parse_ps_tree(stdout: &str) -> HashMap<u32, PsRow> {
    let mut tree = HashMap::new();
    for line in stdout.lines() {
        let mut parts = line.split_whitespace();
        let (Some(pid), Some(ppid), Some(tty)) = (parts.next(), parts.next(), parts.next()) else { continue };
        let (Ok(pid), Ok(ppid)) = (pid.parse::<u32>(), ppid.parse::<u32>()) else { continue };
        let comm = parts.collect::<Vec<_>>().join(" ");
        if comm.is_empty() {
            continue;
        }
        tree.insert(pid, PsRow { pid, ppid, tty: tty.to_owned(), comm });
    }
    tree
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MacKind {
    Terminal,
    Iterm,
    App,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacHost {
    pub kind: MacKind,
    pub bundle: String,
    pub tty: Option<String>,
}

const APP_MARK: &str = ".app/Contents/MacOS/";

fn is_tty(s: &str) -> bool {
    s.strip_prefix("ttys").is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
}

pub fn host_of(tree: &HashMap<u32, PsRow>, pid: u32) -> Option<MacHost> {
    let me = tree.get(&pid)?;
    let mut row = Some(me);
    for _ in 0..=8 {
        let r = row?;
        if r.comm.starts_with('/') && r.comm.contains(APP_MARK) {
            let end = r.comm.find(".app/").map_or(r.comm.len(), |i| i + 4);
            let bundle = r.comm[..end].to_owned();
            let kind = if bundle.ends_with("/Terminal.app") {
                MacKind::Terminal
            } else if bundle.ends_with("/iTerm.app") {
                MacKind::Iterm
            } else {
                MacKind::App
            };
            let tty = is_tty(&me.tty).then(|| me.tty.clone());
            return Some(MacHost { kind, bundle, tty });
        }
        row = if r.ppid > 0 && r.ppid != r.pid { tree.get(&r.ppid) } else { None };
    }
    None
}

pub fn mac_osa_result(out: &str) -> FocusWindowResult {
    match out.trim() {
        "ok" => FocusWindowResult::Ok,
        "notFound" => FocusWindowResult::NotFound,
        _ => FocusWindowResult::Failed,
    }
}

#[cfg(windows)]
pub fn focus(pid: u32) -> Option<FocusWindowResult> {
    use std::time::Duration;
    let chain = core_procs::table().map(|t| ancestor_chain(pid, &t, 9)).unwrap_or_default();
    let chain = chain.iter().map(u32::to_string).collect::<Vec<_>>().join(",");
    let ps = super::launch::windows_powershell(std::env::var("SystemRoot").ok().as_deref());
    let pid_s = pid.to_string();
    let run = crate::job_stop::run_capped(
        std::path::Path::new(&ps),
        &["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", WIN_SCRIPT],
        &[("CCM_FOCUS_PID", &pid_s), ("CCM_FOCUS_CHAIN", &chain)],
        Duration::from_secs(15),
    );
    Some(match run {
        Some(r) if r.code == Some(0) => parse_focus_output(&r.stdout),
        _ => FocusWindowResult::Failed,
    })
}

#[cfg(target_os = "macos")]
pub fn focus(pid: u32) -> Option<FocusWindowResult> {
    use std::time::Duration;

    use super::osa::{parse_osa_error, OsaError};
    if pid <= 1 {
        return Some(FocusWindowResult::NotFound);
    }
    let Some(ps) = core_procs::deadline::run_with_deadline("/bin/ps", &["-A", "-o", "pid=,ppid=,tty=,comm="], Duration::from_secs(10)) else { return Some(FocusWindowResult::Failed) };
    if !ps.success {
        return Some(FocusWindowResult::Failed);
    }
    let Some(host) = host_of(&parse_ps_tree(&ps.stdout), pid) else { return Some(FocusWindowResult::NotFound) };
    let osascript = std::path::Path::new("/usr/bin/osascript");
    if let (MacKind::Terminal | MacKind::Iterm, Some(tty)) = (host.kind, host.tty.as_deref()) {
        let script = if host.kind == MacKind::Terminal { TERMINAL_SCRIPT } else { ITERM_SCRIPT };
        let target = format!("/dev/{tty}");
        let Some(r) = crate::job_stop::run_capped(osascript, &["-e", script, &target], &[], Duration::from_secs(10)) else { return Some(FocusWindowResult::Failed) };
        if r.code != Some(0) {
            return Some(if parse_osa_error(&r.stderr) == OsaError::Denied { FocusWindowResult::Denied } else { FocusWindowResult::Failed });
        }
        return Some(mac_osa_result(&r.stdout));
    }
    let r = crate::job_stop::run_capped(std::path::Path::new("/usr/bin/open"), &["-a", &host.bundle], &[], Duration::from_secs(10));
    Some(if r.is_some_and(|r| r.code == Some(0)) { FocusWindowResult::AppOnly } else { FocusWindowResult::Failed })
}

#[cfg(not(any(windows, target_os = "macos")))]
pub fn focus(_pid: u32) -> Option<FocusWindowResult> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(pid: u32, ppid: u32, start: Option<i64>) -> Proc {
        Proc { pid, ppid, name: String::new(), start_ms: start, start_raw: None }
    }

    #[test]
    fn the_last_output_line_decides_like_focus_ts() {
        assert_eq!(parse_focus_output("noise\r\nok\r\n"), FocusWindowResult::Ok);
        assert_eq!(parse_focus_output("ambiguous"), FocusWindowResult::Ambiguous);
        assert_eq!(parse_focus_output("notFound\n"), FocusWindowResult::NotFound);
        assert_eq!(parse_focus_output(""), FocusWindowResult::Failed);
        assert_eq!(parse_focus_output("Exception"), FocusWindowResult::Failed);
    }

    #[test]
    fn the_chain_stops_at_a_younger_parent_a_loop_or_the_cap() {
        let table = vec![p(100, 50, Some(30)), p(50, 20, Some(20)), p(20, 4, Some(10)), p(4, 0, Some(1))];
        assert_eq!(ancestor_chain(100, &table, 9), vec![100, 50, 20]);
        assert_eq!(ancestor_chain(100, &table, 2), vec![100, 50]);
        let reused = vec![p(100, 50, Some(30)), p(50, 20, Some(40))];
        assert_eq!(ancestor_chain(100, &reused, 9), vec![100]);
        let unknown = vec![p(100, 50, None), p(50, 50, None)];
        assert_eq!(ancestor_chain(100, &unknown, 9), vec![100, 50]);
        assert!(ancestor_chain(7, &table, 9).is_empty());
    }

    #[test]
    fn mac_hosts_are_found_up_the_ps_tree() {
        let out = "  1     0 ??       /sbin/launchd\n 300     1 ??       /System/Applications/Utilities/Terminal.app/Contents/MacOS/Terminal\n 400   300 ttys003  -zsh\n 500   400 ttys003  claude\n 600     1 ??       /Applications/Visual Studio Code.app/Contents/MacOS/Electron\n 700   600 ttys009  /bin/zsh\n 800   700 ttys009  claude\n 900     1 ??       /Applications/iTerm.app/Contents/MacOS/iTerm2\n 910   900 ttys010  claude\n 990     1 ??       orphan\n";
        let tree = parse_ps_tree(out);
        assert_eq!(tree[&600].comm, "/Applications/Visual Studio Code.app/Contents/MacOS/Electron");
        assert_eq!(host_of(&tree, 500), Some(MacHost { kind: MacKind::Terminal, bundle: "/System/Applications/Utilities/Terminal.app".into(), tty: Some("ttys003".into()) }));
        assert_eq!(host_of(&tree, 800), Some(MacHost { kind: MacKind::App, bundle: "/Applications/Visual Studio Code.app".into(), tty: Some("ttys009".into()) }));
        assert_eq!(host_of(&tree, 910).map(|h| h.kind), Some(MacKind::Iterm));
        assert_eq!(host_of(&tree, 990), None);
        assert_eq!(host_of(&tree, 12345), None);
        assert_eq!(mac_osa_result("ok\n"), FocusWindowResult::Ok);
        assert_eq!(mac_osa_result("notFound"), FocusWindowResult::NotFound);
        assert_eq!(mac_osa_result("?"), FocusWindowResult::Failed);
        assert!(!is_tty("??"));
        assert!(is_tty("ttys001"));
    }
}
