use core_store::persist::notify_file::ToastLocale;

use crate::util::{wait_reason, WaitReason};

const RULE_LABELS: [(&str, &str, &str); 9] = [
    ("outside-project", "file outside the project", "file ngoài project"),
    ("git-force-push", "git push --force", "git push --force"),
    ("git-reset-hard", "git reset --hard", "git reset --hard"),
    ("git-clean", "git clean -fd", "git clean -fd"),
    ("rm-rf", "rm -rf outside the project", "rm -rf ngoài project"),
    ("del-s", "del /s outside the project", "del /s ngoài project"),
    ("remove-item-recurse", "Remove-Item -Recurse outside the project", "Remove-Item -Recurse ngoài project"),
    ("adb-uninstall", "adb uninstall", "adb uninstall"),
    ("drop-table", "DROP TABLE", "DROP TABLE"),
];

pub fn rule_label(rule: &str, locale: ToastLocale) -> String {
    match RULE_LABELS.iter().find(|(k, _, _)| *k == rule) {
        Some((_, en, vi)) => (if locale == ToastLocale::Vi { vi } else { en }).to_string(),
        None => rule.strip_prefix("custom:").unwrap_or(rule).to_string(),
    }
}

#[derive(Clone, Copy)]
pub struct Text(pub ToastLocale);

impl Text {
    fn vi(self) -> bool {
        self.0 == ToastLocale::Vi
    }

    fn pick(self, en: &str, vi: &str) -> String {
        (if self.vi() { vi } else { en }).to_string()
    }

    pub fn waiting_title(self, name: &str) -> String {
        if self.vi() { format!("{name} đang chờ chủ nhân") } else { format!("{name} needs you") }
    }

    pub fn waiting_body(self, reason: Option<&str>, tool: Option<&str>) -> String {
        match wait_reason(reason) {
            WaitReason::Question => self.pick("Asking you", "Đang hỏi bạn"),
            WaitReason::Permission => match tool {
                Some("ExitPlanMode") => self.pick("Plan needs approval", "Cần duyệt plan"),
                Some(t) if !t.is_empty() => if self.vi() { format!("Cần duyệt: {t}") } else { format!("Needs approval: {t}") },
                _ => self.pick("Needs approval", "Cần duyệt"),
            },
            WaitReason::Other => match reason.filter(|r| !r.is_empty()) {
                Some(r) => if self.vi() { format!("Đang chờ {r}") } else { format!("Waiting for {r}") },
                None => self.pick("Claude is waiting for your input", "Claude đang chờ chủ nhân trả lời"),
            },
        }
    }

    pub fn reminder_title(self, name: &str, minutes: &str) -> String {
        if self.vi() { format!("{name} vẫn đang chờ chủ nhân ({minutes} phút)") } else { format!("{name} still needs you ({minutes} min)") }
    }

    pub fn done_title(self, name: &str) -> String {
        if self.vi() { format!("{name} đã xong") } else { format!("{name} finished") }
    }

    pub fn done_body(self) -> String {
        self.pick("Claude finished replying", "Claude đã trả lời xong")
    }

    pub fn job_title(self, name: &str) -> String {
        if self.vi() { format!("Job nền cần chủ nhân: {name}") } else { format!("Background job needs you: {name}") }
    }

    pub fn job_body(self) -> String {
        self.pick("Open it with claude agents to answer.", "Mở bằng claude agents để trả lời.")
    }

    pub fn loop_title(self, name: &str) -> String {
        if self.vi() { format!("{name} lỗi liên tục") } else { format!("{name} keeps failing") }
    }

    pub fn loop_body(self, n: u64, tool: &str) -> String {
        if self.vi() { format!("{tool} lỗi {n} lần trong 5 phút.") } else { format!("{tool} failed {n} times in 5 minutes.") }
    }

    pub fn limit_title(self, pct: &str) -> String {
        if self.vi() { format!("Giới hạn 5h đã dùng {pct}%") } else { format!("5-hour limit at {pct}%") }
    }

    pub fn limit_body(self, at: &str) -> String {
        if self.vi() { format!("Với tốc độ này sẽ hết vào khoảng {at}, trước khi reset.") } else { format!("At this pace it runs out around {at}, before the window resets.") }
    }

    pub fn stuck_title(self, name: &str) -> String {
        if self.vi() { format!("{name} không có output") } else { format!("{name} has gone quiet") }
    }

    pub fn stuck_body(self, minutes: &str) -> String {
        if self.vi() { format!("Đang chạy nhưng không có output {minutes} phút.") } else { format!("Working but no output for {minutes} min.") }
    }

    pub fn context_title(self, name: &str, pct: &str) -> String {
        if self.vi() { format!("{name} đã dùng {pct}% context") } else { format!("{name} context at {pct}%") }
    }

    pub fn context_body(self) -> String {
        self.pick("Run /compact before Claude compacts on its own.", "Nên /compact trước khi Claude tự compact.")
    }

    pub fn idle_title(self, n: usize, size: &str) -> String {
        if self.vi() {
            format!("{n} mục bỏ quên đang dùng {size}")
        } else if n == 1 {
            format!("1 forgotten item uses {size}")
        } else {
            format!("{n} forgotten items use {size}")
        }
    }

    pub fn idle_body(self, names: &str) -> String {
        if self.vi() { format!("{names}. Mở Orrery để xem và tắt.") } else { format!("{names}. Open Orrery to review and stop them.") }
    }

    pub fn sensitive_title(self, name: &str) -> String {
        if self.vi() { format!("{name} vừa đụng vào thứ nhạy cảm") } else { format!("{name} touched something sensitive") }
    }

    pub fn sensitive_body(self, rule: &str, tool: &str, path: Option<&str>) -> String {
        let path = path.filter(|p| !p.is_empty()).map(|p| format!(" · {p}")).unwrap_or_default();
        format!("{tool} · {}{path}", rule_label(rule, self.0))
    }

    pub fn disk_title(self, label: &str, free: &str) -> String {
        if self.vi() { format!("Ổ {label} sắp đầy: còn {free}") } else { format!("Drive {label} almost full: {free} left") }
    }

    pub fn disk_body(self, danger: bool) -> String {
        match (danger, self.vi()) {
            (true, false) => "Builds and agents may fail soon. Open Orrery to clean build output or agent data.".into(),
            (true, true) => "Build và agent sắp lỗi vì hết chỗ. Mở Orrery để dọn đồ build hoặc dữ liệu agent.".into(),
            (false, false) => "Open Orrery to see what can be cleaned.".into(),
            (false, true) => "Mở Orrery để xem có thể dọn gì.".into(),
        }
    }

    pub fn note_title(self, name: &str, done: bool) -> String {
        match (done, self.vi()) {
            (true, false) => format!("{name} reports done"),
            (true, true) => format!("{name} báo đã xong"),
            (false, false) => format!("{name} asks for your attention"),
            (false, true) => format!("{name} cần chủ nhân xem"),
        }
    }

    pub fn note_more(self, n: &str) -> String {
        if self.vi() { format!(" (+{n} tin nữa)") } else { format!(" (+{n} more)") }
    }

    pub fn test_title(self) -> String {
        self.pick("Test notification", "Thông báo thử")
    }

    pub fn test_body(self) -> String {
        self.pick("Windows notifications from Orrery work.", "Thông báo Windows của Orrery đang chạy.")
    }

    pub fn snooze(self, minutes: u64) -> String {
        if self.vi() { format!("Hoãn {minutes} phút") } else { format!("Snooze {minutes} min") }
    }

    pub fn open(self) -> String {
        self.pick("Open terminal", "Mở terminal")
    }

    pub fn open_dashboard(self) -> String {
        self.pick("Open dashboard", "Mở dashboard")
    }

    pub fn dismiss(self) -> String {
        self.pick("Dismiss", "Bỏ qua")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rule_labels_follow_the_locale() {
        assert_eq!(rule_label("rm-rf", ToastLocale::Vi), "rm -rf ngoài project");
        assert_eq!(rule_label("rm-rf", ToastLocale::En), "rm -rf outside the project");
        assert_eq!(rule_label("custom:*.pem", ToastLocale::En), "*.pem");
        let en = Text(ToastLocale::En);
        assert_eq!(en.sensitive_body("git-clean", "Bash", Some("x")), "Bash · git clean -fd · x");
        assert_eq!(en.idle_title(1, "2 GB"), "1 forgotten item uses 2 GB");
        assert_eq!(en.idle_title(2, "2 GB"), "2 forgotten items use 2 GB");
    }
}
