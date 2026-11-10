use super::jsre::{g, Js};
use crate::run::proc::kill_pid;
use crate::run::types::Env;
use regex::Regex;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub struct ProcRow {
    pub pid: u32,
    pub ppid: u32,
    pub cmd: String,
    pub start_ms: Option<f64>,
}

pub type Fallback = Box<dyn FnOnce() + Send>;

pub fn parse_proc_rows(stdout: &str) -> Vec<ProcRow> {
    static ROW: Js = Js::new(r"^\s*(\d+)\s+(\d+)\s?(.*)$", "");
    let mut rows = Vec::new();
    for line in super::detect::split_lines(stdout) {
        let Some(m) = ROW.exec(line) else { continue };
        let num = |i: usize| g(&m, i).and_then(|v| v.parse::<u32>().ok());
        if let (Some(pid), Some(ppid)) = (num(1), num(2)) {
            rows.push(ProcRow { pid, ppid, cmd: g(&m, 3).unwrap_or("").to_string(), start_ms: None });
        }
    }
    rows
}

pub fn tree_pids(root: u32, rows: &[ProcRow], spare: &dyn Fn(&str) -> bool) -> Vec<u32> {
    let mut kids: HashMap<u32, Vec<&ProcRow>> = HashMap::new();
    let start_of: HashMap<u32, Option<f64>> = rows.iter().map(|r| (r.pid, r.start_ms)).collect();
    for r in rows {
        if r.pid == r.ppid {
            continue;
        }
        if let (Some(Some(ps)), Some(rs)) = (start_of.get(&r.ppid), r.start_ms) {
            if *ps > rs {
                continue;
            }
        }
        kids.entry(r.ppid).or_default().push(r);
    }
    let Some(me) = rows.iter().find(|r| r.pid == root) else {
        return walk_all(root, &kids, spare);
    };
    if spare(&me.cmd) {
        return Vec::new();
    }
    walk_all(root, &kids, spare)
}

fn walk_all(root: u32, kids: &HashMap<u32, Vec<&ProcRow>>, spare: &dyn Fn(&str) -> bool) -> Vec<u32> {
    fn walk(pid: u32, depth: u32, kids: &HashMap<u32, Vec<&ProcRow>>, spare: &dyn Fn(&str) -> bool, seen: &mut HashSet<u32>, out: &mut Vec<u32>) {
        if seen.contains(&pid) || depth > 64 {
            return;
        }
        seen.insert(pid);
        for k in kids.get(&pid).map(|v| v.as_slice()).unwrap_or(&[]) {
            if !spare(&k.cmd) {
                walk(k.pid, depth + 1, kids, spare, seen, out);
            }
        }
        out.push(pid);
    }
    let mut out = Vec::new();
    walk(root, 0, kids, spare, &mut HashSet::new(), &mut out);
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct SparingPlan {
    pub single: Vec<u32>,
    pub tree: Vec<u32>,
}

pub fn sparing_plan(root: u32, rows: &[ProcRow], spare: &dyn Fn(&str) -> bool) -> SparingPlan {
    let pids = tree_pids(root, rows, spare);
    let parent_of: HashMap<u32, u32> = rows.iter().map(|r| (r.pid, r.ppid)).collect();
    let mut kids: HashMap<u32, Vec<u32>> = HashMap::new();
    for r in rows {
        if r.pid != r.ppid {
            kids.entry(r.ppid).or_default().push(r.pid);
        }
    }
    let cmd_of: HashMap<u32, &str> = rows.iter().map(|r| (r.pid, r.cmd.as_str())).collect();
    let mut memo: HashMap<u32, bool> = HashMap::new();
    fn holds(pid: u32, depth: u32, cmd_of: &HashMap<u32, &str>, kids: &HashMap<u32, Vec<u32>>, spare: &dyn Fn(&str) -> bool, memo: &mut HashMap<u32, bool>) -> bool {
        if let Some(k) = memo.get(&pid) {
            return *k;
        }
        let hit = depth < 64 && (spare(cmd_of.get(&pid).copied().unwrap_or("")) || kids.get(&pid).map(|v| v.to_vec()).unwrap_or_default().into_iter().any(|k| holds(k, depth + 1, cmd_of, kids, spare, memo)));
        memo.insert(pid, hit);
        hit
    }
    let mut single = Vec::new();
    let mut safe = Vec::new();
    for p in pids {
        if holds(p, 0, &cmd_of, &kids, spare, &mut memo) {
            single.push(p);
        } else {
            safe.push(p);
        }
    }
    let safe_set: HashSet<u32> = safe.iter().copied().collect();
    let tree = safe.iter().copied().filter(|p| parent_of.get(p).is_none_or(|pp| !safe_set.contains(pp))).collect();
    SparingPlan { single, tree }
}

pub fn taskkill_args(pids: &[u32], tree: bool) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    if tree {
        args.push("/T".into());
    }
    args.push("/F".into());
    for p in pids {
        args.push("/PID".into());
        args.push(p.to_string());
    }
    args
}

pub fn proc_table_command(env: &Env) -> (String, Vec<String>) {
    let root = env.get("SystemRoot").or_else(|| env.get("SYSTEMROOT")).map(String::as_str).unwrap_or("C:\\Windows");
    let ps = super::pathx::join(true, &[root, "System32", "WindowsPowerShell", "v1.0", "powershell.exe"]);
    let script = "Get-CimInstance Win32_Process -Property ProcessId,ParentProcessId,CommandLine | ForEach-Object { '{0} {1} {2}' -f $_.ProcessId,$_.ParentProcessId,($_.CommandLine -replace '[\\r\\n]',' ') }";
    (ps, vec!["-NoProfile".into(), "-NonInteractive".into(), "-Command".into(), script.into()])
}

pub fn kill_tree_sparing(pid: u32, spare: &Regex, sync: bool, env: &Env, on_fallback: Option<Fallback>) {
    #[cfg(windows)]
    {
        win::kill(pid, spare.clone(), sync, env.clone(), on_fallback);
    }
    #[cfg(not(windows))]
    {
        let _ = (spare, env, on_fallback);
        kill_pid(pid, sync);
    }
}

#[cfg(windows)]
mod win {
    use super::*;
    use crate::run::collect::{collect, CollectOpts};
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    fn taskkill(pids: &[u32], tree: bool, sync: bool) {
        if pids.is_empty() {
            return;
        }
        let mut c = Command::new("taskkill");
        c.args(taskkill_args(pids, tree)).creation_flags(0x0800_0000).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
        if sync {
            let _ = c.status();
        } else if let Ok(mut child) = c.spawn() {
            std::thread::spawn(move || {
                let _ = child.wait();
            });
        }
    }

    fn rows(env: &Env, timeout_ms: u64) -> Vec<ProcRow> {
        let (file, args) = proc_table_command(env);
        let cwd = std::env::temp_dir().to_string_lossy().to_string();
        let r = collect(&file, &args, &cwd, env, &CollectOpts { timeout_ms: Some(timeout_ms), ..Default::default() });
        if r.code != Some(0) || r.timed_out {
            return Vec::new();
        }
        parse_proc_rows(&r.out)
    }

    pub fn kill(pid: u32, spare: Regex, sync: bool, env: Env, on_fallback: Option<Fallback>) {
        let run = move || {
            let rows = rows(&env, if sync { 5_000 } else { 15_000 });
            if rows.is_empty() {
                if let Some(f) = on_fallback {
                    f();
                }
                kill_pid(pid, sync);
                return;
            }
            let test = |cmd: &str| spare.is_match(cmd);
            if sync {
                let plan = sparing_plan(pid, &rows, &test);
                taskkill(&plan.single, false, true);
                taskkill(&plan.tree, true, true);
            } else {
                taskkill(&tree_pids(pid, &rows, &test), false, false);
            }
        };
        if sync {
            run();
        } else {
            std::thread::spawn(run);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(pid: u32, ppid: u32, cmd: &str) -> ProcRow {
        ProcRow { pid, ppid, cmd: cmd.into(), start_ms: None }
    }

    fn rows() -> Vec<ProcRow> {
        vec![row(100, 1, "cmd /c gradlew"), row(101, 100, "java GradleWrapperMain"), row(102, 101, "java org.gradle.launcher.daemon.bootstrap.GradleDaemon 8.14"), row(103, 102, "java worker of daemon"), row(104, 100, "node tool"), row(105, 104, "node tool child")]
    }

    fn spare(cmd: &str) -> bool {
        cmd.contains("GradleDaemon")
    }

    #[test]
    fn sparing_plan_spares_daemon_tree() {
        let plan = sparing_plan(100, &rows(), &spare);
        assert_eq!(plan, SparingPlan { single: vec![101, 100], tree: vec![104] });
        let mut all = tree_pids(100, &rows(), &spare);
        all.sort();
        assert_eq!(all, vec![100, 101, 104, 105]);
        assert!(tree_pids(102, &rows(), &spare).is_empty());
    }

    #[test]
    fn reused_parent_pid_is_not_walked() {
        let r = vec![ProcRow { start_ms: Some(5_000.0), ..row(100, 1, "a") }, ProcRow { start_ms: Some(6_000.0), ..row(101, 100, "b") }, ProcRow { start_ms: Some(1_000.0), ..row(102, 100, "c") }];
        assert_eq!(tree_pids(100, &r, &|_| false), vec![101, 100]);
    }

    #[test]
    fn subtree_keeps_daemon_alive() {
        let r = vec![
            row(100, 1, "cmd.exe /d /s /c \"gradlew.bat assembleDebug\""),
            row(101, 100, "java.exe org.gradle.wrapper.GradleWrapperMain assembleDebug"),
            row(102, 101, "java.exe -Xmx2g org.gradle.launcher.daemon.bootstrap.GradleDaemon 8.7"),
            row(103, 102, "java.exe org.jetbrains.kotlin.daemon.KotlinCompileDaemon"),
            row(104, 101, "conhost.exe"),
            row(200, 1, ""),
        ];
        let mut got = tree_pids(100, &r, &spare);
        got.sort();
        assert_eq!(got, vec![100, 101, 104]);
    }

    #[test]
    fn parses_cim_rows_and_commands() {
        let out = "100 1 cmd /c gradlew\r\n  101 100 java x\nbad line\n7 3\n";
        let r = parse_proc_rows(out);
        assert_eq!(r, vec![row(100, 1, "cmd /c gradlew"), row(101, 100, "java x"), row(7, 3, "")]);
        assert_eq!(taskkill_args(&[101, 100], false), vec!["/F", "/PID", "101", "/PID", "100"]);
        assert_eq!(taskkill_args(&[104], true), vec!["/T", "/F", "/PID", "104"]);
        let (ps, args) = proc_table_command(&[("SystemRoot".to_string(), "D:\\Win".to_string())].into_iter().collect());
        assert_eq!(ps, "D:\\Win\\System32\\WindowsPowerShell\\v1.0\\powershell.exe");
        assert_eq!(&args[..3], &["-NoProfile", "-NonInteractive", "-Command"]);
    }
}
