pub struct DeskText {
    pub cancel: &'static str,
    pub backup_confirm: &'static str,
    pub backup_detail_tail: &'static str,
    pub build_confirm: &'static str,
    pub build_confirm_trash: &'static str,
    vi: bool,
}

const SECTIONS: [(&str, &str, &str); 9] = [
    ("appearance", "Appearance and fonts", "Giao diện và font"),
    ("settings", "Settings and notifications", "Cài đặt và thông báo"),
    ("workspace", "Workspace tabs and layouts", "Tab và bố cục Workspace"),
    ("notes", "Notes, prompts and bookmarks", "Ghi chú, prompt và bookmark"),
    ("names", "Names, aliases and favourites", "Tên gọi, alias và yêu thích"),
    ("devices", "Remembered Android devices", "Thiết bị Android đã nhớ"),
    ("run", "Run settings and templates", "Cài đặt Run và mẫu cấu hình"),
    ("updates", "Update preferences", "Tuỳ chọn cập nhật"),
    ("github", "GitHub account names (tokens are added again)", "Tên tài khoản GitHub (cần thêm lại token)"),
];

const EN: DeskText = DeskText {
    cancel: "Cancel",
    backup_confirm: "Import and restart",
    backup_detail_tail: "These values replace the ones on this machine. The current setup is saved first in the backups folder of the Orrery data folder. Orrery restarts to finish.",
    build_confirm: "Delete",
    build_confirm_trash: "Move to Trash",
    vi: false,
};

const VI: DeskText = DeskText {
    cancel: "Huỷ",
    backup_confirm: "Nhập và khởi động lại",
    backup_detail_tail: "Các giá trị này sẽ thay giá trị trên máy này. Cấu hình hiện tại được lưu trước vào thư mục backups trong thư mục dữ liệu Orrery. Orrery sẽ khởi động lại để hoàn tất.",
    build_confirm: "Xoá",
    build_confirm_trash: "Chuyển vào Thùng rác",
    vi: true,
};

pub fn desk_text(locale: &str) -> &'static DeskText {
    if locale == "en" {
        &EN
    } else {
        &VI
    }
}

impl DeskText {
    pub fn section(&self, id: &str) -> String {
        SECTIONS.iter().find(|(k, _, _)| *k == id).map_or_else(|| id.to_owned(), |(_, en, vi)| if self.vi { (*vi).to_owned() } else { (*en).to_owned() })
    }

    pub fn backup_title(&self, n: usize) -> String {
        match (self.vi, n) {
            (true, _) => format!("Nhập {n} mục và khởi động lại Orrery?"),
            (false, 1) => "Import 1 section and restart Orrery?".to_owned(),
            (false, _) => format!("Import {n} sections and restart Orrery?"),
        }
    }

    pub fn backup_detail(&self, names: &[String]) -> String {
        let list: Vec<String> = names.iter().map(|n| format!("• {n}")).collect();
        format!("{}\n\n{}", list.join("\n"), self.backup_detail_tail)
    }

    pub fn build_title(&self, dirs: i64, size: &str) -> String {
        match (self.vi, dirs) {
            (true, _) => format!("Xoá {dirs} thư mục build ({size})?"),
            (false, 1) => format!("Delete 1 build folder ({size})?"),
            (false, _) => format!("Delete {dirs} build folders ({size})?"),
        }
    }

    pub fn build_detail(&self, lines: &[String], more: i64, reinstall: bool, trash: bool) -> String {
        let more = match (more, self.vi) {
            (0, _) => String::new(),
            (m, true) => format!("\n…và {m} thư mục nữa"),
            (m, false) => format!("\n…and {m} more"),
        };
        let (again, gone, safe) = if self.vi {
            (
                if reinstall { "Phải cài lại gói (node_modules / .venv), và lần build sau sẽ lâu hơn." } else { "Lần build sau sẽ tạo lại và lâu hơn." },
                if trash { "Chúng được chuyển vào Thùng rác." } else { "Chúng bị xoá hẳn, không vào Thùng rác." },
                "Mã nguồn không bao giờ bị đụng tới.",
            )
        } else {
            (
                if reinstall { "Packages (node_modules / .venv) must be installed again, and the next build takes longer." } else { "The next build rebuilds them and takes longer." },
                if trash { "They go to the Recycle Bin / Trash." } else { "They are deleted for good, not moved to the Recycle Bin / Trash." },
                "Source files are never touched.",
            )
        };
        format!("{}{more}\n\n{again} {gone} {safe}", lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backup_texts_match_the_electron_strings() {
        let en = desk_text("en");
        assert_eq!(en.backup_title(1), "Import 1 section and restart Orrery?");
        assert_eq!(en.backup_title(3), "Import 3 sections and restart Orrery?");
        assert_eq!(en.section("github"), "GitHub account names (tokens are added again)");
        assert_eq!(en.section("zzz"), "zzz");
        let detail = en.backup_detail(&["A".into(), "B".into()]);
        assert!(detail.starts_with("• A\n• B\n\nThese values replace"));
        assert_eq!(desk_text("vi").backup_title(1), "Nhập 1 mục và khởi động lại Orrery?");
    }

    #[test]
    fn build_clean_texts_match_the_electron_strings() {
        let en = desk_text("en");
        assert_eq!(en.build_title(1, "2 MB"), "Delete 1 build folder (2 MB)?");
        assert_eq!(en.build_title(2, "2 MB"), "Delete 2 build folders (2 MB)?");
        assert_eq!(
            en.build_detail(&["a".into()], 2, false, true),
            "a\n…and 2 more\n\nThe next build rebuilds them and takes longer. They go to the Recycle Bin / Trash. Source files are never touched."
        );
        assert!(desk_text("vi").build_detail(&[], 0, true, false).starts_with("\n\nPhải cài lại gói"));
    }
}
