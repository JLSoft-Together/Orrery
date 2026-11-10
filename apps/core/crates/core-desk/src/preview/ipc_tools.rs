use super::gate::{self, denied, Busy};
use super::history::LocalHistory;
use super::http::{clean_request, send_http, HttpState};
use super::paths::{is_absolute, real_or_null, root_key};
use super::replace::{apply_replace, clean_replace, matcher, Cleaned};
use super::{checks, ctx, deny, envcheck, files, format, i18n, memdoctor, perm, scripts};
use crate::claude::Sys;
use crate::{arg, Ctx, Desk, DeskEnv};
use serde_json::{json, Value};
use std::sync::Arc;

pub const MODULE: &str = "previewTools";

fn field<'a>(req: &'a Value, key: &str) -> &'a Value {
    req.get(key).unwrap_or(&Value::Null)
}

fn strings(v: Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
}

fn is_dir(p: &str) -> bool {
    std::fs::metadata(p).map(|m| m.is_dir()).unwrap_or(false)
}

fn cap(ctx: &Ctx, name: &str) -> bool {
    gate::has_cap(&gate::caps(ctx), name)
}

pub fn register(desk: &mut Desk, env: &DeskEnv, history: Arc<LocalHistory>) {
    desk.on(MODULE, "ccm:env-check", |ctx, args| Ok(gate::check_dir(ctx, arg(args, 0)).map(|d| envcheck::check_env(&d)).unwrap_or_else(denied)));
    desk.on(MODULE, "ccm:env-reveal", |ctx, args| {
        let req = arg(args, 0);
        Ok(gate::check_dir(ctx, field(req, "root")).map(|d| envcheck::reveal_env(&d, field(req, "file"), field(req, "key"))).unwrap_or_else(denied))
    });
    desk.on(MODULE, "ccm:i18n-scan", |ctx, args| Ok(gate::check_dir(ctx, arg(args, 0)).map(|d| i18n::scan_i18n(&d)).unwrap_or_else(denied)));
    desk.on(MODULE, "ccm:scripts", |ctx, args| {
        let dir = arg(args, 0);
        let real = gate::repo_of(ctx, dir).or_else(|| gate::allowed(ctx, dir, false).map(|a| a.real));
        Ok(match real.filter(|r| is_dir(r)) {
            Some(r) => scripts::detect_scripts(&r),
            None => json!(null),
        })
    });
    register_run(desk);
    register_claude(desk, env);
    register_net(desk, history);
}

fn register_run(desk: &mut Desk) {
    let formatter = Arc::new(format::Formatter::default());
    desk.on(MODULE, "ccm:format", move |ctx, args| {
        let req = arg(args, 0);
        let Some(text) = field(req, "text").as_str() else { return Ok(denied()) };
        if format::too_large(text, files::MAX_WRITE_BYTES) {
            return Ok(json!({"ok": false, "error": "too-large"}));
        }
        Ok(match gate::editable_file(ctx, field(req, "path")) {
            Some((real, root)) => formatter.format(&real, &root, text),
            None => denied(),
        })
    });
    let runner = Arc::new(checks::CheckRunner::default());
    let r = runner.clone();
    desk.on(MODULE, "ccm:check-run", move |ctx, args| {
        let req = arg(args, 0);
        let real = gate::check_dir(ctx, field(req, "dir"));
        let spec = checks::clean_check(field(req, "check"));
        let (Some(real), Some(spec)) = (real, spec) else { return Ok(denied()) };
        Ok(match checks::resolve_check(&real, &spec) {
            Some(line) => r.run(&root_key(&real), &real, &line),
            None => json!({"ok": false, "error": "unknown-script"}),
        })
    });
    desk.on(MODULE, "ccm:check-cancel", move |ctx, args| Ok(json!(gate::check_dir(ctx, arg(args, 0)).map(|d| runner.cancel(&root_key(&d))).unwrap_or(false))));
}

fn register_claude(desk: &mut Desk, env: &DeskEnv) {
    let data_dir = env.data_dir.to_string_lossy().to_string();
    desk.on(MODULE, "ccm:perm-read", move |ctx, args| {
        let dir = if cap(ctx, "history") { Some(data_dir.as_str()) } else { None };
        Ok(gate::check_dir(ctx, arg(args, 0)).map(|d| perm::read_permissions(&d, dir, crate::util::now_ms())).unwrap_or_else(denied))
    });
    desk.on(MODULE, "ccm:ctx-sources", |ctx, args| Ok(gate::check_dir(ctx, arg(args, 0)).map(|d| ctx::context_sources(&d, &Sys::from_env())).unwrap_or_else(denied)));
    desk.on(MODULE, "ccm:memory-doctor", |ctx, args| Ok(gate::check_dir(ctx, arg(args, 0)).map(|d| memdoctor::memory_doctor(&d, &Sys::from_env())).unwrap_or_else(denied)));
    let writing = Arc::new(Busy::default());
    let w = writing.clone();
    desk.on(MODULE, "ccm:perm-allow", move |ctx, args| {
        let req = arg(args, 0);
        let real = gate::check_dir(ctx, field(req, "root"));
        let Some(real) = real.filter(|_| cap(ctx, "confirmPermRules")) else { return Ok(denied()) };
        let Some(_guard) = w.enter() else { return Ok(json!({"ok": false, "error": "failed"})) };
        Ok(perm::write_allow(&real, field(req, "rules"), &|r| gate::confirm(ctx, "perm", r)))
    });
    desk.on(MODULE, "ccm:sensitive-deny", move |ctx, args| {
        let req = arg(args, 0);
        let caps = gate::caps(ctx);
        let config = caps.get("claudeConfigDir").and_then(Value::as_str).filter(|c| !c.is_empty()).map(String::from);
        let project = field(req, "project").as_str().filter(|p| crate::js::len(p) <= 1024 && is_absolute(p));
        let (Some(config), Some(project), true) = (config, project, gate::has_cap(&caps, "confirmDenyRules")) else { return Ok(denied()) };
        let Some(real) = real_or_null(project).filter(|r| std::path::Path::new(r).parent().is_some() && is_dir(r)) else { return Ok(denied()) };
        if gate::check_dir(ctx, &json!(real)).is_none() && !deny::is_session_project(&real, &deny::session_cwds(&config)) {
            return Ok(denied());
        }
        let Some(_guard) = writing.enter() else { return Ok(json!({"ok": false, "error": "failed"})) };
        let dreq = deny::DenyReq { project: &real, rule: field(req, "rule"), tool: field(req, "tool"), scope: field(req, "scope"), config_dir: &config };
        Ok(deny::write_deny(&dreq, &|r| gate::confirm(ctx, "deny", r)))
    });
}

fn register_net(desk: &mut Desk, history: Arc<LocalHistory>) {
    let replacing = Arc::new(Busy::default());
    desk.on(MODULE, "ccm:replace-files", move |ctx, args| {
        let req = arg(args, 0);
        if !cap(ctx, "confirmReplace") {
            return Ok(denied());
        }
        let clean = match clean_replace(req) {
            Cleaned::Denied => return Ok(denied()),
            Cleaned::Error(e) => return Ok(json!({"ok": false, "error": e})),
            Cleaned::Ok(c) => c,
        };
        let Some(ok) = gate::allowed(ctx, field(req, "dir"), false).filter(|a| is_dir(&a.real)) else { return Ok(denied()) };
        if matcher(&clean.query).is_none() {
            return Ok(json!({"ok": false, "error": "unsupported"}));
        }
        let Some(_guard) = replacing.enter() else { return Ok(json!({"ok": false, "error": "cancelled"})) };
        let lines: usize = clean.files.iter().map(|f| f.lines.len()).sum();
        let mut paths: Vec<Value> = clean.files.iter().take(12).map(|f| json!(f.path)).collect();
        if clean.files.len() > 12 {
            paths.push(json!("…"));
        }
        let asked = json!({"find": clean.query.q, "replacement": clean.replacement, "lines": lines, "files": clean.files.len(), "paths": paths});
        if !gate::confirm(ctx, "replace", asked) {
            return Ok(json!({"ok": false, "error": "cancelled"}));
        }
        let roots = strings(ctx.host("preview.writeRoots", json!({})).unwrap_or(Value::Null));
        let on = cap(ctx, "history");
        let res = apply_replace(&ok.real, &clean, &roots, &|real, prev| {
            if on {
                history.record(real, prev);
            }
        });
        gate::invalidate(ctx, &ok.real);
        Ok(res)
    });
    let state = Arc::new(HttpState::default());
    let s = state.clone();
    desk.on(MODULE, "ccm:http-send", move |ctx, args| {
        let Some(req) = clean_request(arg(args, 0)) else { return Ok(json!({"ok": false, "error": "bad-request"})) };
        if !s.host_ok(&req, &|host, method| gate::confirm(ctx, "http", json!({"host": host, "method": method}))) {
            return Ok(json!({"ok": false, "error": "declined"}));
        }
        let id = req.id.clone();
        let flag = match &id {
            Some(i) => s.start(i),
            None => Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };
        let res = send_http(&req, &flag);
        if let Some(i) = &id {
            s.finish(i, &flag);
        }
        Ok(res)
    });
    desk.on(MODULE, "ccm:http-cancel", move |_ctx, args| Ok(json!(arg(args, 0).as_str().map(|id| state.cancel(id)).unwrap_or(false))));
}
