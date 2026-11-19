use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind, MessageDialogResult};

use crate::disk_clean::{CommandAsk, OneTapAsk};
use crate::disk_text::{format_bytes, Ask};
use crate::state::MAIN;

pub struct BoxN {
    pub message: String,
    pub detail: String,
    pub buttons: Vec<String>,
}

fn vi(locale: &str) -> bool {
    locale.to_lowercase().starts_with("vi")
}

fn bin_word(v: bool, mac: bool) -> &'static str {
    if v {
        "Thùng rác"
    } else if mac {
        "Trash"
    } else {
        "Recycle Bin"
    }
}

pub fn one_tap_box(ask: &OneTapAsk, locale: &str, mac: bool) -> BoxN {
    let v = vi(locale);
    let size = format_bytes(ask.bytes);
    let perm = format_bytes(ask.perm_bytes);
    let rest = format_bytes(ask.trash_bytes);
    let bin = bin_word(v, mac);
    let mut lines = Vec::new();
    if ask.perm_count > 0 {
        lines.push(if v {
            format!("File Temp và cache trình duyệt ({perm}) sẽ xoá hẳn, không vào Thùng rác: app tự tạo lại nên để trong Thùng rác chỉ tốn chỗ.")
        } else {
            format!("Temp files and browser caches ({perm}) are deleted permanently: apps make them again, so the {bin} would only fill up.")
        });
    }
    if ask.trash_count > 0 {
        lines.push(if v { format!("Phần còn lại ({rest}) vào Thùng rác.") } else { format!("The rest ({rest}) goes to the {bin}.") });
    }
    let clean = if v { format!("Dọn {size}") } else { format!("Clean {size}") };
    let cancel = if v { "Huỷ" } else { "Cancel" }.to_owned();
    let buttons = if ask.perm_count > 0 { vec![clean, if v { "Tất cả vào Thùng rác".to_owned() } else { format!("Move everything to the {bin}") }, cancel] } else { vec![clean, cancel] };
    BoxN { message: if v { format!("Dọn {size}?") } else { format!("Clean up {size}?") }, detail: lines.join("\n"), buttons }
}

pub fn bin_box(bytes: u64, count: u64, locale: &str, mac: bool) -> BoxN {
    let v = vi(locale);
    let size = format_bytes(bytes);
    BoxN {
        message: if v { "Dọn sạch Thùng rác?".to_owned() } else { format!("Empty the {}?", bin_word(false, mac)) },
        detail: if v { format!("{count} mục, {size}. Không khôi phục được.") } else { format!("{count} items, {size}. They cannot be restored.") },
        buttons: vec![if v { "Dọn sạch" } else { "Empty" }.to_owned(), if v { "Huỷ" } else { "Cancel" }.to_owned()],
    }
}

fn what(kind: &str, v: bool) -> String {
    let w = match (kind, v) {
        ("images", false) => "unused Docker images",
        ("containers", false) => "stopped Docker containers",
        ("volumes", false) => "unused Docker volumes",
        ("buildCache", false) => "the Docker build cache",
        ("simUnavailable", false) => "unavailable iOS simulators",
        ("images", true) => "image Docker không dùng",
        ("containers", true) => "container Docker đã dừng",
        ("volumes", true) => "volume Docker không dùng",
        ("buildCache", true) => "build cache của Docker",
        ("simUnavailable", true) => "simulator iOS không còn dùng được",
        _ => kind,
    };
    w.to_owned()
}

pub fn command_box(ask: &CommandAsk, locale: &str) -> BoxN {
    let v = vi(locale);
    let size = format_bytes(ask.bytes);
    let w = what(&ask.kind, v);
    let mut detail = if v { format!("Orrery chạy: {}\nGiải phóng khoảng {size} ({} mục).", ask.command, ask.count) } else { format!("Orrery runs: {}\nAbout {size} can be freed ({} items).", ask.command, ask.count) };
    if ask.warn {
        detail.push_str(if v { "\n\nDữ liệu trong các volume này mất hẳn. Sao lưu phần còn cần trước." } else { "\n\nData in these volumes is lost for good. Back up anything you still need first." });
    }
    BoxN {
        message: if v { format!("Chạy lệnh dọn chính thức cho {w}?") } else { format!("Run the official clean up for {w}?") },
        detail,
        buttons: vec![if v { "Chạy" } else { "Run" }.to_owned(), if v { "Huỷ" } else { "Cancel" }.to_owned()],
    }
}

pub fn agent_box(ask: &Ask, pane: &str, reason: &str, locale: &str, mac: bool) -> BoxN {
    let v = vi(locale);
    let size = format_bytes(ask.bytes);
    let more = ask.count.saturating_sub(ask.first.len());
    let list = ask.first.join("\n");
    let (message, detail) = if v {
        let tail = if more > 0 { format!("\n…và {more} mục nữa") } else { String::new() };
        (
            format!("Agent ở “{pane}” đề xuất {} {} mục", if ask.permanent { "xoá hẳn" } else { "chuyển vào Thùng rác" }, ask.count),
            format!("Lý do: {reason}\nTổng: {size}\n\n{list}{tail}\n\nAgent đề xuất qua công cụ disk_propose_delete của Orrery. Không có gì bị xoá nếu bạn không đồng ý."),
        )
    } else {
        let tail = if more > 0 { format!("\n…and {more} more") } else { String::new() };
        let action = if ask.permanent { "delete permanently".to_owned() } else { format!("move to the {}", bin_word(false, mac)) };
        let items = if ask.count == 1 { "1 item".to_owned() } else { format!("{} items", ask.count) };
        (
            format!("An agent in “{pane}” proposes to {action} {items}"),
            format!("Reason: {reason}\nTotal: {size}\n\n{list}{tail}\n\nThe agent asked through Orrery's disk_propose_delete tool. Nothing is removed unless you confirm."),
        )
    };
    let ok = match (ask.permanent, v) {
        (true, true) => "Xoá hẳn".to_owned(),
        (true, false) => "Delete permanently".to_owned(),
        (false, true) => "Chuyển vào Thùng rác".to_owned(),
        (false, false) => if mac { "Move to Trash" } else { "Move to Recycle Bin" }.to_owned(),
    };
    BoxN { message, detail, buttons: vec![ok, if v { "Huỷ" } else { "Cancel" }.to_owned()] }
}

pub fn format_ram(bytes: Option<u64>) -> String {
    match bytes {
        None => "?".into(),
        Some(b) if b >= 1024 * 1024 * 1024 => format!("{:.1} GB", b as f64 / (1024f64 * 1024.0 * 1024.0)),
        Some(b) => format!("{} MB", (b as f64 / (1024.0 * 1024.0)).round() as u64),
    }
}

pub fn kill_box(lines: &[String], n: usize, first_name: &str, locale: &str) -> BoxN {
    let v = vi(locale);
    let message = match (v, n) {
        (true, 1) => format!("Kết thúc {first_name}?"),
        (true, _) => format!("Kết thúc {n} process?"),
        (false, 1) => format!("End {first_name}?"),
        (false, _) => format!("End {n} processes?"),
    };
    let detail = if v { format!("{}\n\nViệc chưa lưu trong các process này sẽ mất.", lines.join("\n")) } else { format!("{}\n\nUnsaved work in these processes will be lost.", lines.join("\n")) };
    let ok = match (v, n) {
        (true, 1) => "Kết thúc process".to_owned(),
        (true, _) => format!("Kết thúc {n} process"),
        (false, 1) => "End process".to_owned(),
        (false, _) => format!("End {n} processes"),
    };
    BoxN { message, detail, buttons: vec![ok, if v { "Huỷ" } else { "Cancel" }.to_owned()] }
}

pub fn ask_buttons(app: &AppHandle, b: &BoxN) -> usize {
    let last = b.buttons.len().saturating_sub(1);
    if b.buttons.len() < 3 {
        let ok = crate::hosts::confirm(app, &b.message, &b.detail, &b.buttons[0], b.buttons.get(1).map_or("Cancel", String::as_str));
        return if ok { 0 } else { last };
    }
    if let Ok(answer) = std::env::var("CCM_E2E_CONFIRM") {
        return if answer == "ok" { 0 } else { last };
    }
    let mut d = app
        .dialog()
        .message(format!("{}\n\n{}", b.message, b.detail))
        .title("Orrery")
        .kind(MessageDialogKind::Warning)
        .buttons(MessageDialogButtons::YesNoCancelCustom(b.buttons[0].clone(), b.buttons[1].clone(), b.buttons[2].clone()));
    if let Some(w) = app.get_webview_window(MAIN) {
        if !w.is_visible().unwrap_or(true) {
            let _ = w.show();
            let _ = w.set_focus();
        }
        d = d.parent(&w);
    }
    match d.blocking_show_with_result() {
        MessageDialogResult::Yes => 0,
        MessageDialogResult::No => 1,
        MessageDialogResult::Custom(s) if s == b.buttons[0] => 0,
        MessageDialogResult::Custom(s) if s == b.buttons[1] => 1,
        _ => last,
    }
}
