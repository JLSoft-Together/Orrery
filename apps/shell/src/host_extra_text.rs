use serde_json::Value;

pub struct ExtraText {
    vi: bool,
}

pub fn extra_text(locale: &str) -> ExtraText {
    ExtraText { vi: locale != "en" }
}

fn file_name(file: &str) -> &str {
    file.rsplit(['/', '\\']).next().unwrap_or(file)
}

pub fn clip(s: &str) -> String {
    if s.encode_utf16().count() > 300 {
        let head: String = s.chars().scan(0usize, |n, c| {
            *n += c.len_utf16();
            (*n <= 300).then_some(c)
        }).collect();
        format!("{head}…")
    } else {
        s.to_owned()
    }
}

impl ExtraText {
    fn pick(&self, en: &str, vi: &str) -> String {
        if self.vi { vi } else { en }.to_owned()
    }

    pub fn cancel(&self) -> String {
        self.pick("Cancel", "Huỷ")
    }

    pub fn tg_command(&self, receive: bool, file: &str) -> (String, String, String) {
        let (title, detail) = match (receive, self.vi) {
            (false, false) => ("Install the /bot-tele-send-notify command?".to_owned(), format!("Orrery writes {file}. In any Claude Code session opened in Orrery, /bot-tele-send-notify on | off | status then turns Telegram reports on or off for the whole app.")),
            (false, true) => ("Cài lệnh /bot-tele-send-notify?".to_owned(), format!("Orrery sẽ ghi file {file}. Sau đó trong mọi session Claude Code mở trong Orrery, gõ /bot-tele-send-notify on | off | status để bật / tắt báo Telegram cho cả app.")),
            (true, false) => ("Install the /bot-tele-receive command?".to_owned(), format!("Orrery writes {file}. In a Claude Code session opened in Orrery, /bot-tele-receive on lets your paired Telegram account mention the bot in a project topic to type a message into that project's session. Nobody else can. It turns off again when Orrery restarts.")),
            (true, true) => ("Cài lệnh /bot-tele-receive?".to_owned(), format!("Orrery sẽ ghi file {file}. Trong session Claude Code mở trong Orrery, gõ /bot-tele-receive on thì tài khoản Telegram đã ghép của chủ nhân tag bot trong topic của project để gửi tin vào session của project đó. Người khác không gửi được. Khởi động lại Orrery thì tự tắt.")),
        };
        (title, detail, self.pick("Install", "Cài"))
    }

    pub fn statusline(&self, file: &str, json: &str, chained: bool) -> (String, String, String) {
        let detail = if self.vi {
            let tail = if chained { " (status line hiện tại của bạn vẫn chạy sau bridge)" } else { "" };
            format!("Orrery sẽ ghi vào {file}{tail}:\n\n{json}\n\nTệp cũ được giữ lại thành settings.json.ccm-backup. Các session Claude mới sẽ dùng cấu hình này.")
        } else {
            let tail = if chained { " (your current status line keeps running after the bridge)" } else { "" };
            format!("Orrery will set this in {file}{tail}:\n\n{json}\n\nThe old file is kept as settings.json.ccm-backup. New Claude sessions pick it up.")
        };
        (self.pick("Turn on the status line bridge?", "Bật status line bridge?"), detail, self.pick("Turn on", "Bật"))
    }

    pub fn hooks(&self, file: &str, json: &str, on: bool) -> (String, String, String) {
        let title = match on {
            true => self.pick("Turn on the hook bridge?", "Bật hook bridge?"),
            false => self.pick("Turn off the hook bridge?", "Tắt hook bridge?"),
        };
        let detail = if self.vi {
            let what = if on { "thêm hook chạy nền (PreCompact, PostCompact, PermissionDenied, PermissionRequest) vào" } else { "gỡ hook của mình khỏi" };
            format!("Orrery sẽ {what} {file}; các hook khác của bạn giữ nguyên:\n\n{json}\n\nHook chạy nền và chỉ giữ tên sự kiện, trigger, tên tool, thư mục của session và, với hộp hỏi quyền, luật được gợi ý hoặc vài chữ đầu của lệnh. Tệp cũ được giữ lại thành settings.json.ccm-backup. Các session Claude mới sẽ dùng cấu hình này.")
        } else {
            let what = if on { "add async hooks (PreCompact, PostCompact, PermissionDenied, PermissionRequest) to" } else { "remove its hooks from" };
            format!("Orrery will {what} {file}; your other hooks stay:\n\n{json}\n\nThey run in the background and keep only the event name, trigger, tool name, the session folder and, for permission dialogs, the suggested rule or the first words of the command. The old file is kept as settings.json.ccm-backup. New Claude sessions pick it up.")
        };
        let ok = if on { self.pick("Turn on", "Bật") } else { self.pick("Continue", "Tiếp tục") };
        (title, detail, ok)
    }

    pub fn hook_toggle(&self, on: bool, event: &str, file: &str, diff: &str) -> (String, String, String) {
        let name = file_name(file);
        if self.vi {
            let title = if on { format!("Bật lại hook {event} này?") } else { format!("Tắt hook {event} này?") };
            let kept = if on { "" } else { " Hook được cất trong Orrery để bật lại sau." };
            let detail = format!("Orrery sẽ ghi {file}. Các khoá khác giữ nguyên; file cũ được giữ thành {name}.ccm-backup.{kept}\n\n{diff}\n\nCác phiên Claude đang chạy sẽ nhận thay đổi ở lần khởi động sau.");
            (title, detail, if on { "Bật" } else { "Tắt" }.to_owned())
        } else {
            let title = if on { format!("Turn the {event} hook back on?") } else { format!("Turn off this {event} hook?") };
            let kept = if on { "" } else { " The hook is kept in Orrery so it can be turned back on." };
            let detail = format!("Orrery will write {file}. Other keys stay; the old file is kept as {name}.ccm-backup.{kept}\n\n{diff}\n\nRunning Claude sessions pick this up on their next start.");
            (title, detail, if on { "Turn on" } else { "Turn off" }.to_owned())
        }
    }

    pub fn hook_test(&self, event: &str, text: &str, cwd: &str) -> (String, String, String) {
        if self.vi {
            (format!("Chạy thử hook {event} này một lần?"), format!("{text}\n\nLệnh chạy trong {cwd} với input JSON giả trên stdin và môi trường tối thiểu, dừng sau 10 giây. Hook nào sửa file hay gọi dịch vụ thì vẫn làm thật."), "Chạy thử".to_owned())
        } else {
            (format!("Run this {event} hook once as a test?"), format!("{text}\n\nIt runs in {cwd} with a made-up JSON input on stdin and a minimal environment, stopped after 10 seconds. A hook that changes files or calls a service really does so."), "Run test".to_owned())
        }
    }

    pub fn gh_remove(&self, login: &str) -> (String, String, String) {
        if self.vi {
            (format!("Gỡ tài khoản GitHub {login}?"), "Orrery sẽ quên token của tài khoản này. Các repo đang dùng nó quay về chế độ tự chọn. Token vẫn còn hiệu lực trên GitHub cho tới khi bạn thu hồi ở đó.".to_owned(), "Gỡ".to_owned())
        } else {
            (format!("Remove GitHub account {login}?"), "Orrery forgets its token. Repos that used it go back to automatic choice. The token stays valid on GitHub until you revoke it there.".to_owned(), "Remove".to_owned())
        }
    }

    pub fn res_kill(&self, items: &[Value], lines: &[String]) -> (String, String, String) {
        let n = items.len();
        let name = items.first().and_then(|i| i["name"].as_str()).unwrap_or("");
        let session = items.iter().any(|i| i["session"] == Value::Bool(true));
        let body = lines.join("\n");
        if self.vi {
            let title = if n == 1 { format!("Tắt {name}?") } else { format!("Tắt {n} process bị bỏ quên?") };
            let tail = if session { " Session agent bị tắt sẽ mất câu trả lời hoặc tool đang chạy dở; mở lại sau bằng claude --resume." } else { "" };
            (title, format!("{body}\n\nMỗi process bị tắt cùng mọi thứ nó đã khởi chạy.{tail}"), if n == 1 { "Tắt" } else { "Tắt tất cả" }.to_owned())
        } else {
            let title = if n == 1 { format!("Stop {name}?") } else { format!("Stop {n} forgotten processes?") };
            let tail = if session { " A stopped agent session loses the reply or tool call it is in the middle of; reopen it later with claude --resume." } else { "" };
            (title, format!("{body}\n\nEach process is stopped together with everything it started.{tail}"), if n == 1 { "Stop" } else { "Stop all" }.to_owned())
        }
    }

    pub fn ext_where(&self, scope: Option<&str>, dir: Option<&str>) -> String {
        match (scope, dir) {
            (Some("user"), _) => self.pick("all your projects (user config)", "mọi project của bạn (cấu hình user)"),
            (Some("project"), Some(d)) => {
                if self.vi {
                    format!("{d}, chia sẻ cho cả team (.mcp.json / .claude/settings.json, thường được commit)")
                } else {
                    format!("{d}, shared with the team (.mcp.json / .claude/settings.json, usually committed)")
                }
            }
            (Some("local"), Some(d)) => {
                if self.vi {
                    format!("{d}, chỉ mình bạn (không commit)")
                } else {
                    format!("{d}, only you (not committed)")
                }
            }
            _ => self.pick("Claude Code (user settings)", "Claude Code (cấu hình user)"),
        }
    }

    pub fn ext(&self, kind: &str, place: &str, commands: &[String]) -> (String, String, String) {
        let title = match (kind, self.vi) {
            ("remove", false) => "Remove from Claude Code?",
            ("source", false) => "Add to Claude Code?",
            (_, false) => "Install into Claude Code?",
            ("remove", true) => "Gỡ khỏi Claude Code?",
            ("source", true) => "Thêm vào Claude Code?",
            (_, true) => "Cài vào Claude Code?",
        };
        let list = commands.join("\n\n");
        let detail = if self.vi { format!("Orrery sẽ chạy, cho {place}:\n\n{list}\n\nCác phiên Claude mới sẽ nhận thay đổi.") } else { format!("Orrery will run, for {place}:\n\n{list}\n\nNew Claude sessions pick up the change.") };
        (title.to_owned(), detail, self.pick("Run", "Chạy"))
    }

    pub fn deepseek(&self, plan: &Value) -> (String, String, String) {
        let kind = plan["kind"].as_str().unwrap_or("install");
        let commands: Vec<String> = plan["commands"].as_array().map(|a| a.iter().filter_map(Value::as_str).map(clip).collect()).unwrap_or_default();
        let cmds = commands.join("\n");
        let vi = self.vi;
        match kind {
            "logout" => (
                self.pick("Log out of DeepSeek?", "Đăng xuất DeepSeek?"),
                self.pick(
                    "Codewhale deletes the saved DeepSeek API key (codewhale auth clear --provider deepseek). DeepSeek panes stop working until you add a key again.",
                    "Codewhale xoá API key DeepSeek đã lưu (codewhale auth clear --provider deepseek). Pane DeepSeek sẽ không chạy được cho tới khi bạn thêm key lại.",
                ),
                self.pick("Log out", "Đăng xuất"),
            ),
            "sync" => {
                let files: Vec<String> = plan["files"]
                    .as_array()
                    .map(|a| {
                        a.iter()
                            .map(|f| {
                                let create = f["action"] == "create";
                                let verb = match (create, vi) {
                                    (true, false) => "Create",
                                    (false, false) => "Update",
                                    (true, true) => "Tạo",
                                    (false, true) => "Cập nhật",
                                };
                                format!("{verb} {}", f["file"].as_str().unwrap_or(""))
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                let servers: Vec<&str> = plan["mcpAdded"].as_array().map(|a| a.iter().filter_map(Value::as_str).collect()).unwrap_or_default();
                let outbox = plan["outboxAdded"] == Value::Bool(true);
                let mut detail = files.join("\n");
                if !servers.is_empty() {
                    let head = if vi { "MCP server thêm vào ~/.codewhale/mcp.json" } else { "MCP servers added to ~/.codewhale/mcp.json" };
                    detail.push_str(&format!("\n\n{head}: {}", servers.join(", ")));
                }
                if outbox {
                    detail.push_str(if vi {
                        "\n\nconfig.toml: bật lifecycle outbox ([lifecycle_outbox] path = ~/.codewhale/notifications/outbox.jsonl) để Monitor hiện trạng thái phiên DeepSeek."
                    } else {
                        "\n\nconfig.toml: turn on the lifecycle outbox ([lifecycle_outbox] path = ~/.codewhale/notifications/outbox.jsonl) so the Monitor shows DeepSeek session status."
                    });
                }
                detail.push_str(if vi {
                    "\n\nMỗi file đã có được sao lưu một lần thành <file>.orrery-backup. Trong AGENTS.md chỉ khối Orrery thay đổi."
                } else {
                    "\n\nEach file that already exists is backed up once as <file>.orrery-backup. In AGENTS.md only the Orrery block changes."
                });
                (self.pick("Copy rules, skills and MCP servers from Claude?", "Chép rule, skill và MCP server từ Claude?"), detail, self.pick("Write files", "Ghi file"))
            }
            "update" => (
                self.pick("Update the DeepSeek CLI (Codewhale)?", "Cập nhật DeepSeek CLI (Codewhale)?"),
                if vi {
                    format!("Orrery sẽ chạy:\n\n{cmds}\n\nPane DeepSeek đang mở vẫn dùng bản cũ cho tới khi khởi động lại.")
                } else {
                    format!("Orrery will run:\n\n{cmds}\n\nDeepSeek panes that are open keep the old version until they restart.")
                },
                self.pick("Update", "Cập nhật"),
            ),
            _ => {
                let download = kind == "download";
                let title = if download { self.pick("Download the Codewhale program?", "Tải chương trình Codewhale?") } else { self.pick("Install the DeepSeek CLI (Codewhale)?", "Cài DeepSeek CLI (Codewhale)?") };
                let detail = if vi {
                    format!("Orrery sẽ chạy:\n\n{cmds}\n\nLần chạy codewhale đầu tiên sẽ tải chương trình (khoảng 130 MB) từ GitHub Releases.")
                } else {
                    format!("Orrery will run:\n\n{cmds}\n\nThe first run of codewhale downloads its program (about 130 MB) from GitHub Releases.")
                };
                let ok = if download { self.pick("Download", "Tải về") } else { self.pick("Install", "Cài") };
                (title, detail, ok)
            }
        }
    }

    pub fn prompts(&self, save: bool) -> (String, String) {
        let title = if save { self.pick("Export prompt library", "Xuất thư viện prompt") } else { self.pick("Import prompt library", "Nhập thư viện prompt") };
        (title, self.pick("Orrery prompts", "Prompt Orrery"))
    }
}
