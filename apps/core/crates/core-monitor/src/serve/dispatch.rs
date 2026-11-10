use core_http::{ClientContext, ClientMessage};
use core_store::event::MonitorEvent;
use core_store::extra::{ToolStats, UsageHeatmap};
use core_store::Store;
use serde_json::Value;

use super::Runtime;

fn opt_str(v: Option<String>) -> Value {
    v.map_or(Value::Null, Value::from)
}

fn open_app(a: core_http::client::OpenApp) -> core_store::event::OpenApp {
    match a {
        core_http::client::OpenApp::Explorer => core_store::event::OpenApp::Explorer,
        core_http::client::OpenApp::Vscode => core_store::event::OpenApp::Vscode,
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
            mon.open_favorite(s, &dir, mode, terminal_app);
        }
        ClientMessage::FolderPick { request_id } => mon.pick_folder(s, &request_id, Some(&mut reply)),
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
            mon.focus_window(s, &terminal_id, Some(&mut reply));
        }
        ClientMessage::TerminalOpen { terminal_id, app } => mon.open_folder(&terminal_id, open_app(app), &mut reply),
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
        ClientMessage::TerminalSnooze { terminal_id, minutes } => {
            let Some(t) = s.get_terminal(&terminal_id).cloned() else { return };
            if rt.snoozes.set(&t, minutes.and_then(|n| n.as_f64())) {
                s.set_snoozes(rt.snoozes.all());
            }
        }
        _ => {}
    }
}
