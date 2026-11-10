use core_http::client::{Locale, OpenApp as WireApp};
use core_http::{ClientContext, ClientMessage};
use core_sources::prompt_search::searcher::PromptSearchRequest;
use core_store::event::{MonitorEvent, NotifyTestResult, OpenApp};
use core_store::extra::{QuietHours, ToolStats, UsageHeatmap};
use core_store::persist::notify_file::ToastLocale;
use core_store::Store;
use core_toast::NotifyPatch;
use serde_json::Value;

use crate::alerts::lock;
use crate::runtime::Runtime;

fn opt_str(v: Option<String>) -> Value {
    v.map_or(Value::Null, Value::from)
}

fn open_app(a: WireApp) -> OpenApp {
    match a {
        WireApp::Explorer => OpenApp::Explorer,
        WireApp::Vscode => OpenApp::Vscode,
    }
}

fn locale_of(l: Locale) -> ToastLocale {
    match l {
        Locale::En => ToastLocale::En,
        Locale::Vi => ToastLocale::Vi,
    }
}

pub fn client(rt: &mut Runtime, s: &mut Store, m: ClientMessage, ctx: &ClientContext) {
    let mut reply = |e: MonitorEvent| ctx.reply(e);
    let mon = &mut rt.monitor;
    match m {
        ClientMessage::TerminalAlias { terminal_id, alias } => {
            mon.set_alias(s, &terminal_id, &opt_str(alias));
        }
        ClientMessage::TerminalNote { terminal_id, note } => {
            mon.set_note(s, &terminal_id, &opt_str(note));
        }
        ClientMessage::FavoriteToggle { terminal_id } => {
            mon.toggle_favorite(s, &terminal_id);
        }
        ClientMessage::FavoriteRename { dir, label } => {
            mon.rename_favorite(s, &dir, &opt_str(label));
        }
        ClientMessage::FavoriteRemove { dir } => {
            mon.remove_favorite(s, &dir);
        }
        ClientMessage::FavoriteReorder { dirs } => {
            mon.reorder_favorites(s, &dirs);
        }
        ClientMessage::FavoriteAdd { dir, label } => {
            mon.add_favorite(s, &dir, &opt_str(label.flatten()));
        }
        ClientMessage::FavoriteOpen { dir, mode, terminal_app } => {
            if let Some(target) = mon.favorite_target(&dir) {
                rt.host.launch(target, mode, terminal_app);
            }
        }
        ClientMessage::FolderPick { request_id } => rt.host.pick(request_id, ctx.clone()),
        ClientMessage::HistoryGet => mon.send_history(s, Some(&mut reply)),
        ClientMessage::ToolSpansGet { terminal_id, since } => mon.send_tool_spans(&mut reply, terminal_id.as_deref(), since.as_deref()),
        ClientMessage::TimelineGet => mon.send_timeline(s, Some(&mut reply)),
        ClientMessage::HeatmapGet => {
            if let Some(h) = rt.usage.as_ref().and_then(|u| serde_json::from_value::<UsageHeatmap>(u.heatmap(28)).ok()) {
                reply(MonitorEvent::HeatmapData(h));
            }
        }
        ClientMessage::ToolstatsGet { days, project } => {
            if let Some(t) = rt.usage.as_ref().and_then(|u| serde_json::from_value::<ToolStats>(u.tool_stats(u32::from(days), project.as_deref())).ok()) {
                reply(MonitorEvent::ToolStatsData(t));
            }
        }
        ClientMessage::TerminalFocusWindow { terminal_id } => {
            let pid = mon.focus_pid(&terminal_id);
            rt.host.focus(terminal_id, pid, ctx.clone());
        }
        ClientMessage::TerminalOpen { terminal_id, app } => {
            let dir = mon.open_dir(&terminal_id);
            rt.host.open(terminal_id, dir, open_app(app), ctx.clone());
        }
        ClientMessage::DiagnosticsGet => {
            let usage = rt.usage.as_ref().and_then(|u| u.scan_info());
            reply(MonitorEvent::DiagnosticsData(Box::new(mon.diagnostics(usage, ctx.clients as u64))));
        }
        ClientMessage::TerminalDismiss { terminal_id } => {
            mon.dismiss_ended(s, terminal_id.as_deref());
        }
        ClientMessage::JobDismiss { job_id } => {
            mon.dismiss_jobs(s, job_id.as_deref());
        }
        ClientMessage::JobStop { job_id } => {
            mon.stop_job(&job_id);
        }
        ClientMessage::PromptSearchStart { search_id, query, days, project } => {
            rt.host.search(ctx.client_id, PromptSearchRequest { search_id, query, days: u32::from(days), project }, ctx.clone());
        }
        ClientMessage::PromptSearchCancel { search_id } => rt.host.search_cancel(ctx.client_id, &search_id),
        ClientMessage::PromptSearchOpen { search_id, session_id } => rt.host.search_open(ctx.client_id, search_id, session_id, ctx.clone()),
        ClientMessage::OfficeEnable { on: true } => {
            if !rt.office_noted {
                rt.office_noted = true;
                eprintln!("[collector] Office needs CCM_CORE=node; orrery-core serve has no Office yet");
            }
        }
        m => alerts(rt, s, m, ctx),
    }
}

fn alerts(rt: &mut Runtime, s: &mut Store, m: ClientMessage, ctx: &ClientContext) {
    let mut a = lock(&rt.alerts);
    match m {
        ClientMessage::ClientPresence { attentive, locale } => a.toaster.set_presence(ctx.client_id, attentive, locale.map(locale_of)),
        ClientMessage::NotifyUpdate { toast, kinds, quiet, stuck_minutes, idle_minutes } => {
            let quiet = quiet.and_then(|q| serde_json::from_value::<QuietHours>(q).ok());
            a.toaster.update(&NotifyPatch::from_client(toast, kinds.as_ref(), quiet, stuck_minutes, idle_minutes));
            s.set_notify(a.toaster.settings());
            rt.sampling.sync(a.toaster.idle_watch());
        }
        ClientMessage::NotifyTest => {
            let outcome = a.toaster.test();
            s.set_notify(a.toaster.settings());
            ctx.reply(MonitorEvent::NotifyTestResult(NotifyTestResult { result: outcome.result, health: outcome.health }));
        }
        ClientMessage::TerminalSnooze { terminal_id, minutes } => {
            let Some(t) = s.get_terminal(&terminal_id).cloned() else { return };
            if !a.snoozes.set(&t, minutes.and_then(|n| n.as_f64())) {
                return;
            }
            s.set_snoozes(a.snoozes.all());
            if a.snoozes.is_snoozed(&t, None) {
                a.toaster.hide_for(&t.id);
            }
        }
        ClientMessage::SensitiveUpdate { groups, globs, ignore } => {
            let groups = groups.map(Value::Object);
            let globs = globs.map(|g| Value::Array(g.into_iter().map(Value::from).collect()));
            let ignore = ignore.map(Value::Object);
            if let Ok(mut w) = rt.sensitive.lock() {
                w.update(groups.as_ref(), globs.as_ref(), ignore.as_ref());
                s.set_sensitive(w.settings().clone());
            }
        }
        ClientMessage::SensitiveIgnore { project, rule, on } => {
            if let Ok(mut w) = rt.sensitive.lock() {
                if w.ignore(&project, &rule, on) {
                    s.set_sensitive(w.settings().clone());
                }
            }
        }
        _ => {}
    }
    a.pump();
}
