pub struct Strings {
    pub app_name: &'static str,
    pub tray_open: &'static str,
    pub tray_compact: &'static str,
    pub tray_login: &'static str,
    pub tray_logs: &'static str,
    pub tray_quit: &'static str,
    pub hidden_notice: &'static str,
    pub link_bad_folder: &'static str,
    pub link_failed: &'static str,
    pub link_live: &'static str,
    pub port_in_use: &'static str,
    pub start_failed: &'static str,
}

const EN: Strings = Strings {
    app_name: "Orrery",
    tray_open: "Open dashboard",
    tray_compact: "Compact window",
    tray_login: if cfg!(target_os = "macos") { "Open at login" } else { "Start with Windows" },
    tray_logs: "Open log folder",
    tray_quit: "Quit",
    hidden_notice: "Still running in the tray. Quit from the tray menu.",
    link_bad_folder: "Orrery cannot open this folder",
    link_failed: "Orrery could not open the terminal. Open the Workspace and try again.",
    link_live: "This session is still running in another terminal. Close it there first, then open the link again.",
    port_in_use: "Port {port} is taken: an older Orrery (.bat / npm start) or another app (4317 is also the OpenTelemetry collector default). Close it or set CCM_PORT.",
    start_failed: "Orrery could not start its monitor: {reason}",
};

const VI: Strings = Strings {
    app_name: "Orrery",
    tray_open: "Mở dashboard",
    tray_compact: "Cửa sổ gọn",
    tray_login: if cfg!(target_os = "macos") { "Mở khi đăng nhập" } else { "Chạy cùng Windows" },
    tray_logs: "Mở thư mục log",
    tray_quit: "Thoát",
    hidden_notice: "Monitor vẫn chạy dưới khay hệ thống. Thoát từ menu khay.",
    link_bad_folder: "Orrery không mở được thư mục này",
    link_failed: "Orrery không mở được terminal. Hãy mở Workspace rồi thử lại.",
    link_live: "Session này vẫn đang chạy ở một terminal khác. Hãy đóng nó ở đó trước rồi mở lại liên kết.",
    port_in_use: "Cổng {port} đang bị chiếm: Orrery bản cũ (.bat / npm start) hoặc app khác (4317 cũng là cổng mặc định của OpenTelemetry collector). Hãy tắt nó hoặc đặt CCM_PORT.",
    start_failed: "Orrery không khởi động được phần monitor: {reason}",
};

pub fn strings(locale: &str) -> &'static Strings {
    if locale == "en" {
        &EN
    } else {
        &VI
    }
}
