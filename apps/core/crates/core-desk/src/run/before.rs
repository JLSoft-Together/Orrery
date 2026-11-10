use super::args::expand_args;
use super::proc::{pump_lines, spawn, subst_vars, SpawnSpec};
use super::types::RunContext;
use crate::preview::format::{resolve_command, with_args, Resolved};
use std::sync::mpsc;
use std::time::Duration;

const EXIT_GRACE_MS: u64 = 1_500;

pub fn run_step(ctx: &RunContext, file: &str, args: &[String], verbatim: bool, stream: &str, track: &dyn Fn(Option<u32>)) -> bool {
    let spec = SpawnSpec { file: file.into(), args: args.to_vec(), cwd: ctx.cfg_str("cwd"), env: ctx.cfg_env(), verbatim, ..Default::default() };
    let Ok(mut child) = spawn(&spec, &ctx.env) else {
        return false;
    };
    track(Some(child.id()));
    let (tx, rx) = mpsc::channel::<()>();
    for pipe in [child.stdout.take().map(|p| Box::new(p) as Box<dyn std::io::Read + Send>), child.stderr.take().map(|p| Box::new(p) as Box<dyn std::io::Read + Send>)].into_iter().flatten() {
        let hooks = ctx.hooks.clone();
        let stream = stream.to_string();
        let tx = tx.clone();
        let h = pump_lines(pipe, move |text| hooks.log(&stream, text, None, None));
        std::thread::spawn(move || {
            let _ = h.join();
            let _ = tx.send(());
        });
    }
    drop(tx);
    let code = child.wait().ok().and_then(|s| s.code());
    let mut waited = 0;
    while waited < 2 {
        if rx.recv_timeout(Duration::from_millis(EXIT_GRACE_MS)).is_err() {
            break;
        }
        waited += 1;
    }
    track(None);
    code == Some(0)
}

pub fn run_before_launch(ctx: &RunContext, resolve: &dyn Fn(&str) -> Option<Resolved>, track: &dyn Fn(Option<u32>), cancelled: &dyn Fn() -> bool) -> bool {
    let cwd = ctx.cfg_str("cwd");
    let vars = subst_vars(&cwd, &ctx.root, &ctx.env, &ctx.cfg_env());
    for line in ctx.cfg_before() {
        if cancelled() {
            return false;
        }
        let argv = expand_args(&line, &vars);
        let Some(head) = argv.first() else { continue };
        let Some(spec) = resolve(head) else {
            let base = head.rsplit(['/', '\\']).next().unwrap_or(head);
            ctx.hooks.log("build", &format!("{base}: not found"), None, None);
            return false;
        };
        let Some(full) = with_args(&spec, &argv[1..]) else {
            ctx.hooks.log("build", &format!("{line}: an argument contains a line break or NUL"), None, None);
            return false;
        };
        ctx.hooks.log("build", &format!("> {line}"), None, None);
        if !run_step(ctx, &full.file, &full.args, full.verbatim, "build", track) {
            return false;
        }
    }
    true
}

pub fn resolve_for(ctx: &RunContext, name: &str) -> Option<Resolved> {
    let mut env = ctx.env.clone();
    env.extend(ctx.cfg_env());
    resolve_command(name, &ctx.cfg_str("cwd"), &env, ctx.win)
}
