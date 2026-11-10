use super::host::{Args, ToolHost};
use super::{t_desk, t_files, t_notes, t_open, t_run, t_sessions};
use serde_json::Value;

pub fn call_tool(name: &str, args: &Args, host: &dyn ToolHost) -> Value {
    match name {
        "open_session" => t_open::open_session(args, host),
        "notify_owner" => t_notes::notify_owner(args, host),
        "open_in_workspace" => t_files::open_in_workspace(args, host),
        "who_is_editing" => t_files::who_is_editing(args, host),
        "hand_back" => t_files::hand_back(args, host),
        "preview_layout" => t_desk::preview_layout(args, host),
        "list_servers" => t_desk::list_servers(args, host),
        "free_port" => t_desk::free_port(args, host),
        "telegram_notify" => t_desk::telegram_notify(args, host),
        "telegram_send" => t_desk::telegram_send(args, host),
        "telegram_receive" => t_desk::telegram_receive(args, host),
        "run_app" => t_run::run_app(args, host),
        "run_status" => t_run::run_status(args, host),
        "run_log" => t_run::run_log(args, host),
        "device_screenshot" => t_run::device_screenshot(args, host),
        "list_sessions" => t_sessions::list_sessions(args, host),
        "send_to_session" => t_sessions::send_to_session(args, host),
        "close_session" => t_sessions::close_session(args, host),
        "wait_session" => t_sessions::wait_session(args, host),
        _ => super::host::fail_with("Unknown tool", "failed"),
    }
}
