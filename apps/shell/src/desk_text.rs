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

impl DeskText {
    pub fn free_port_title(&self, pane: &str, port: i64) -> String {
        if self.vi {
            format!("Agent ở “{pane}” muốn giải phóng cổng {port}")
        } else {
            format!("An agent in “{pane}” wants to free port {port}")
        }
    }

    pub fn free_port_detail(&self, lines: &[String]) -> String {
        let tail = if self.vi {
            "Agent yêu cầu qua công cụ free_port của Orrery. Mỗi process bị dừng cùng mọi thứ nó đã khởi chạy."
        } else {
            "The agent asked through Orrery's free_port tool. Each process is stopped together with everything it started."
        };
        format!("{}\n\n{tail}", lines.join("\n"))
    }

    pub fn free_port_confirm(&self) -> &'static str {
        if self.vi {
            "Dừng"
        } else {
            "Stop"
        }
    }

    pub fn tg_receive_title(&self, pane: &str) -> String {
        if self.vi {
            format!("Agent ở “{pane}” muốn bật nhận lệnh Telegram")
        } else {
            format!("An agent in “{pane}” wants to turn on Telegram commands")
        }
    }

    pub fn tg_receive_detail(&self, owner: &str) -> String {
        if self.vi {
            format!("Khi bật, tài khoản Telegram đã ghép {owner} tag bot trong topic của project để gửi tin vào session của project đó và duyệt các lần xin quyền. Agent yêu cầu qua công cụ telegram_receive của Orrery; chỉ cho phép khi chủ nhân đã yêu cầu. Khởi động lại Orrery thì tự tắt.")
        } else {
            format!("While on, the paired Telegram account {owner} can mention the bot in a project topic to send messages into that project's sessions and approve their permission prompts. The agent asked through Orrery's telegram_receive tool; allow it only if you asked for it. It turns off again when Orrery restarts.")
        }
    }

    pub fn tg_receive_confirm(&self) -> &'static str {
        if self.vi {
            "Bật"
        } else {
            "Turn on"
        }
    }

    pub fn kill_lines(&self, items: &[serde_json::Value]) -> Vec<String> {
        let word = if self.vi { "session agent" } else { "agent session" };
        items
            .iter()
            .map(|i| {
                let ram = match i["ramBytes"].as_f64() {
                    None => "?".to_owned(),
                    Some(b) if b >= 1024f64.powi(3) => format!("{:.1} GB", b / 1024f64.powi(3)),
                    Some(b) => format!("{} MB", (b / 1024f64.powi(2)).round()),
                };
                let cmd = match i["cmd"].as_str().filter(|c| !c.is_empty()) {
                    Some(c) if c.chars().count() > 140 => format!("\n   {}…", c.chars().take(140).collect::<String>()),
                    Some(c) => format!("\n   {c}"),
                    None => String::new(),
                };
                let session = if i["session"] == serde_json::Value::Bool(true) { format!(" ({word})") } else { String::new() };
                format!("• {}{session} — {ram}, pid {}{cmd}", i["name"].as_str().unwrap_or(""), i["pid"])
            })
            .collect()
    }
}

impl DeskText {
    fn pick(&self, en: &str, vi: &str) -> String {
        if self.vi { vi } else { en }.to_owned()
    }

    pub fn agent_pick_title(&self, purpose: &str) -> String {
        if purpose == "target" {
            self.pick("Choose the drive folder to move the agent data into", "Chọn thư mục trên ổ đích để chuyển dữ liệu agent tới")
        } else {
            self.pick("Choose a folder for the original transcripts", "Chọn thư mục giữ bản gốc của transcript")
        }
    }

    fn agent_group(&self, id: &str) -> String {
        const NAMES: [(&str, &str, &str); 6] = [
            ("debugLogs", "Debug logs", "Log debug"),
            ("shellSnapshots", "Shell snapshots", "Ảnh chụp shell"),
            ("appCache", "App caches", "Cache ứng dụng"),
            ("codexLogs", "Codex logs", "Log Codex"),
            ("codexCache", "Codex caches", "Cache Codex"),
            ("codexTemp", "Codex temp files", "File tạm Codex"),
        ];
        NAMES.iter().find(|(k, _, _)| *k == id).map_or_else(|| id.to_owned(), |(_, en, vi)| self.pick(en, vi))
    }

    pub fn agent_data_spec(&self, p: &serde_json::Value) -> Option<(String, String, Vec<String>)> {
        let s = |k: &str| p[k].as_str().unwrap_or("").to_owned();
        let n = |k: &str| p[k].as_f64().filter(|v| v.is_finite()).unwrap_or(0.0);
        let cancel = self.cancel.to_owned();
        let vi = self.vi;
        Some(match p["kind"].as_str()? {
            "clean" => {
                let agent = core_desk::agent_data::rules::Agent::parse(&p["agent"])?;
                let ids: Vec<&str> = p["groups"].as_array().map(|a| a.iter().filter_map(|v| v.as_str()).collect()).unwrap_or_default();
                let lines: Vec<String> = ids
                    .iter()
                    .map(|id| {
                        let names = core_desk::agent_data::rules::group_rule(agent, id).map_or_else(|| (*id).to_owned(), |r| r.names.join(", "));
                        format!("• {} — {names}", self.agent_group(id))
                    })
                    .collect();
                let (files, size, days) = (n("files"), s("size"), n("days"));
                let title = match (vi, files == 1.0) {
                    (true, _) => format!("Xoá {files} file cache ({size})?"),
                    (false, true) => format!("Delete 1 cache file ({size})?"),
                    (false, false) => format!("Delete {files} cache files ({size})?"),
                };
                let detail = if vi {
                    let age = if days > 0.0 { format!("Chỉ file không đổi trong {days} ngày.") } else { "Mọi file trong các nhóm này.".to_owned() };
                    format!("{}\n\n{age} File đổi trong 24 giờ qua được giữ lại. Không bao giờ đụng transcript, cài đặt hay đăng nhập.", lines.join("\n"))
                } else {
                    let age = if days > 0.0 { format!("Only files not changed for {days} days.") } else { "Every file in these groups.".to_owned() };
                    format!("{}\n\n{age} Files changed in the last 24 hours are kept. Transcripts, settings and sign-in are never touched.", lines.join("\n"))
                };
                (title, detail, vec![self.pick("Delete", "Xoá"), cancel])
            }
            "strip" => {
                let (files, images, size, backup) = (n("files"), n("images"), s("size"), s("backup"));
                let title = if vi { format!("Gỡ {images} ảnh khỏi {files} transcript cũ?") } else { format!("Remove {images} images from {files} old transcripts?") };
                let detail = if vi {
                    format!("Khoảng {size} dữ liệu ảnh được thay bằng một dòng ghi chú. File gốc được sao trước vào:\n{backup}\n\nVẫn resume được các session đó, nhưng agent không còn thấy những ảnh đã gỡ. Session đang chạy và session hoạt động gần đây được bỏ qua.")
                } else {
                    format!("About {size} of image data is replaced by a short note. The original files are copied first to:\n{backup}\n\nResuming those sessions later works, but the agent no longer sees the removed images. Running sessions and recent ones are skipped.")
                };
                (title, detail, vec![self.pick("Back up and remove images", "Sao lưu và gỡ ảnh"), cancel])
            }
            "move" => {
                let (from, to, size, original) = (s("from"), s("to"), s("size"), s("original"));
                let title = if vi { format!("Chuyển {from} sang {to}?") } else { format!("Move {from} to {to}?") };
                let detail = if vi {
                    format!("{size} được chép rồi kiểm (số file, dung lượng, hash mẫu). Sau đó bản gốc đổi tên thành {original} và một liên kết thế chỗ, nên Claude Code, Codex và Orrery vẫn dùng đường dẫn cũ.\n\nĐừng mở Claude Code hay Codex tới khi xong. Bộ theo dõi của Orrery khởi động lại một lần. Bản gốc được giữ tới khi bạn chọn Xoá bản gốc.")
                } else {
                    format!("{size} is copied and checked (file count, sizes, sample hashes). Then the original is renamed to {original} and a link takes its place, so Claude Code, Codex and Orrery keep using the same path.\n\nDo not start Claude Code or Codex until it finishes. Orrery's monitor restarts once. The original stays until you choose Delete original.")
                };
                (title, detail, vec![self.pick("Copy and move", "Chép và chuyển"), cancel])
            }
            "undo" => {
                let (folder, target, files) = (s("folder"), s("target"), n("newFiles"));
                let title = if vi { format!("Chuyển {folder} về chỗ cũ?") } else { format!("Move {folder} back?") };
                let detail = match (p["originalDeleted"] == serde_json::Value::Bool(true), files > 0.0, vi) {
                    (true, _, true) => format!("Bản gốc đã bị xoá, nên mọi thứ được chép về từ:\n{target}\n\nBản ở đó vẫn được giữ."),
                    (true, _, false) => format!("The original was deleted, so everything is copied back from:\n{target}\n\nThe copy there is kept."),
                    (false, true, true) => format!("Có {files} file được ghi ở {target} sau khi chuyển. Chép chúng đè lên bản gốc, hoặc quay về bản gốc như lúc chuyển. Bản ở chỗ mới được giữ trong cả hai cách."),
                    (false, true, false) => format!("{files} files were written at {target} after the move. Copy them back over the original, or go back to the original as it was when it was moved. The copy at the new place is kept either way."),
                    (false, false, true) => format!("Liên kết bị gỡ và bản gốc trở lại. Bản ở {target} được giữ."),
                    (false, false, false) => format!("The link is removed and the original comes back. The copy at {target} is kept."),
                };
                let only = self.pick("Undo", "Hoàn tác");
                let back = match (vi, files == 1.0) {
                    (true, _) => format!("Hoàn tác và chép về {files} file"),
                    (false, true) => "Undo and copy back 1 file".to_owned(),
                    (false, false) => format!("Undo and copy back {files} files"),
                };
                let buttons = if p["copyBack"] == serde_json::Value::Bool(true) { vec![back, only, cancel] } else { vec![only, cancel] };
                (title, detail, buttons)
            }
            "delete" => {
                let (folder, size) = (s("folder"), s("size"));
                let title = if vi { format!("Xoá bản gốc {folder}?") } else { format!("Delete the original {folder}?") };
                let detail = if vi {
                    format!("{size} bị xoá hẳn. Bản đã chuyển vẫn chạy bình thường; hoàn tác sau đó sẽ chép mọi thứ về từ bản này.")
                } else {
                    format!("{size} is deleted for good. The moved copy keeps working; Undo after this copies everything back from it.")
                };
                (title, detail, vec![self.pick("Delete original", "Xoá bản gốc"), cancel])
            }
            "discard" => {
                let folder = s("folder");
                let title = if vi { format!("Xoá bản chép dở {folder}?") } else { format!("Delete the unfinished copy {folder}?") };
                let detail = self.pick(
                    "A move was interrupted before it finished. The original folder is in place and in use; only this partial copy is deleted.",
                    "Một lần chuyển bị ngắt giữa chừng. Thư mục gốc vẫn ở chỗ cũ và đang được dùng; chỉ bản chép dở này bị xoá.",
                );
                (title, detail, vec![self.pick("Delete copy", "Xoá bản chép"), cancel])
            }
            _ => return None,
        })
    }
}

#[cfg(test)]
mod agent_data_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn agent_data_texts_match_the_electron_strings() {
        let en = desk_text("en");
        let (t, d, b) = en.agent_data_spec(&json!({"kind": "clean", "agent": "claude", "groups": ["debugLogs"], "files": 1, "size": "2 KB", "days": 30})).unwrap();
        assert_eq!(t, "Delete 1 cache file (2 KB)?");
        assert!(d.starts_with("• Debug logs — debug, logs, telemetry"));
        assert!(d.contains("Only files not changed for 30 days."));
        assert_eq!(b, vec!["Delete".to_owned(), "Cancel".to_owned()]);
        let (_, _, b) = en.agent_data_spec(&json!({"kind": "undo", "folder": "/a", "target": "/b", "newFiles": 2, "copyBack": true})).unwrap();
        assert_eq!(b, vec!["Undo and copy back 2 files".to_owned(), "Undo".to_owned(), "Cancel".to_owned()]);
        assert!(en.agent_data_spec(&json!({"kind": "zzz"})).is_none());
        assert!(en.agent_data_spec(&json!({"kind": "clean", "agent": "x"})).is_none());
        assert_eq!(desk_text("vi").agent_data_spec(&json!({"kind": "discard", "folder": "/x"})).unwrap().0, "Xoá bản chép dở /x?");
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


#[cfg(test)]
mod mcp_tests {
    use super::*;

    #[test]
    fn mcp_texts_match_the_electron_strings() {
        let en = desk_text("en");
        assert_eq!(en.free_port_title("app", 5173), "An agent in “app” wants to free port 5173");
        assert!(en.free_port_detail(&["• x".into()]).starts_with("• x\n\nThe agent asked through Orrery's free_port tool."));
        assert_eq!(desk_text("vi").tg_receive_title("app"), "Agent ở “app” muốn bật nhận lệnh Telegram");
        let lines = en.kill_lines(&[serde_json::json!({"name": "vite", "ramBytes": 2147483648u64, "cmd": "node", "session": true, "pid": 7}), serde_json::json!({"name": "x", "ramBytes": null, "cmd": "", "session": false, "pid": 8})]);
        assert_eq!(lines, vec!["• vite (agent session) — 2.0 GB, pid 7\n   node".to_owned(), "• x — ?, pid 8".to_owned()]);
    }
}
