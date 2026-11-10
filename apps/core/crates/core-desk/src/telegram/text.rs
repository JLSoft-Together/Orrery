pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

pub fn done(vi: bool, n: &str, w: i64) -> String {
    match (vi, w) {
        (false, 0) => format!("✅ <b>{n}</b> finished"),
        (false, w) => format!("✅ <b>{n}</b> finished after {w} min"),
        (true, 0) => format!("✅ <b>{n}</b> đã xong"),
        (true, w) => format!("✅ <b>{n}</b> đã xong sau {w} phút"),
    }
}

pub fn waiting(vi: bool, n: &str) -> String {
    if vi { format!("⏳ <b>{n}</b> đang chờ chủ nhân trả lời") } else { format!("⏳ <b>{n}</b> is waiting for your input") }
}

pub fn question(vi: bool, n: &str) -> String {
    if vi { format!("❓ <b>{n}</b> đang hỏi chủ nhân") } else { format!("❓ <b>{n}</b> is asking you a question") }
}

pub fn permission(vi: bool, n: &str, tool: Option<&str>) -> String {
    let t = tool.filter(|t| !t.is_empty()).map(|t| format!(": {t}")).unwrap_or_default();
    if vi { format!("🔐 <b>{n}</b> cần duyệt{t}") } else { format!("🔐 <b>{n}</b> needs approval{t}") }
}

pub fn report(n: &str) -> String {
    format!("📝 <b>{n}</b>")
}

pub fn test(vi: bool) -> &'static str {
    if vi { "🤖 Thử từ Orrery: báo Telegram vào topic này đang chạy." } else { "🤖 Test from Orrery: Telegram reports work in this topic." }
}

pub fn big_file(vi: bool, f: &str, mb: &str) -> String {
    if vi { format!("📦 {f} ({mb} MB) vượt giới hạn gửi file của bot Telegram, file vẫn nằm trên máy.") } else { format!("📦 {f} ({mb} MB) is larger than Telegram allows bots to send; it stays on the machine.") }
}

pub fn big_uploading(vi: bool, f: &str, mb: &str, pct: Option<i64>) -> String {
    let p = pct.map(|p| format!(" {p}%")).unwrap_or_default();
    if vi { format!("⏫ Đang gửi {f} ({mb} MB)…{p}") } else { format!("⏫ Uploading {f} ({mb} MB)…{p}") }
}

pub fn big_done(vi: bool, f: &str, mb: &str) -> String {
    if vi { format!("✅ Đã gửi {f} ({mb} MB).") } else { format!("✅ Sent {f} ({mb} MB).") }
}

pub fn big_failed(vi: bool, f: &str, why: &str) -> String {
    let why = match (vi, why) {
        (false, "auth") => "api_id / api_hash or the bot token was refused (Settings › Telegram › Large files)",
        (false, "chat") => "the bot cannot post in this topic",
        (false, "network") => "Telegram could not be reached",
        (false, _) => "upload failed",
        (true, "auth") => "api_id / api_hash hoặc token bot bị từ chối (Settings › Telegram › File lớn)",
        (true, "chat") => "bot không gửi được vào topic này",
        (true, "network") => "không kết nối được Telegram",
        (true, _) => "upload lỗi",
    };
    if vi { format!("⚠️ Không gửi được {f}: {why}. File vẫn nằm trên máy.") } else { format!("⚠️ Could not send {f}: {why}. The file stays on the machine.") }
}

pub fn digest_head(vi: bool, name: &str, n: i64, at: &str) -> String {
    if vi { format!("✅ <b>{name}</b> · xong {n} lần · {at}") } else { format!("✅ <b>{name}</b> · done {n} time{} · {at}", if n == 1 { "" } else { "s" }) }
}

pub fn paired(vi: bool, n: &str) -> String {
    if vi { format!("🔗 Đã ghép với {}. Từ giờ chỉ chủ nhân gửi được vào session.", esc(n)) } else { format!("🔗 Paired with {}. Only you can send to sessions now.", esc(n)) }
}

pub fn stranger(vi: bool, n: &str) -> String {
    if vi { format!("Xin lỗi bạn, tôi chỉ trả lời chủ nhân {}, gửi thêm tiền để thương lượng 🐮", esc(n)) } else { format!("Sorry, I only answer my owner {}. Send more money to negotiate 🐮", esc(n)) }
}

pub fn help(vi: bool, bot: &str) -> String {
    let b = esc(bot);
    let lines: Vec<String> = if vi {
        vec![
            format!("🤖 <b>Lệnh của @{b}</b>"),
            format!("Tag bot trong topic của project rồi gõ lệnh. Ví dụ: <code>@{b} new viết test cho login</code>"),
            String::new(),
            "💬 <b>Giao việc</b>".into(),
            "<code>nội dung</code> — gửi vào session của project".into(),
            "reply vào tin báo của bot — gửi đúng session đó".into(),
            "<code>all &lt;nội dung&gt;</code> — gửi vào mọi session của project".into(),
            "ảnh / file kèm caption tag bot — lưu vào project".into(),
            String::new(),
            "🧭 <b>Session</b>".into(),
            "<code>status</code> — các session đang mở của project".into(),
            "<code>new [prompt]</code> — mở session Claude mới".into(),
            "<code>resume</code> — mở lại session đã kết thúc".into(),
            "<code>last</code> — câu trả lời cuối của session".into(),
            "<code>tail [số dòng]</code> — các dòng cuối của terminal session (mặc định 30, tối đa 80)".into(),
            "<code>stop</code> — dừng việc đang chạy (Esc)".into(),
            "<code>compact</code> — Compact của Orrery: ghi file handoff, /compact rồi làm tiếp (có hỏi xác nhận). <code>/compact ghi chú</code> và <code>/clear</code> vẫn gõ nguyên vào session".into(),
            "<code>close</code> — đóng pane của session (có hỏi xác nhận, chỉ chủ nhân)".into(),
            "<code>focus</code> — đưa Orrery lên trước, ở session đó".into(),
            "<code>where</code> — topic này đang nối với gì".into(),
            String::new(),
            "📱 <b>App</b>".into(),
            "<code>run [config]</code> — chạy app theo Run config".into(),
            "<code>screen [serial]</code> — chụp màn hình máy Android".into(),
        ]
    } else {
        vec![
            format!("🤖 <b>Commands of @{b}</b>"),
            format!("Mention the bot in a project topic, then type. Example: <code>@{b} new write login tests</code>"),
            String::new(),
            "💬 <b>Send work</b>".into(),
            "<code>your message</code> — to the project's session".into(),
            "reply to a bot report — to that exact session".into(),
            "<code>all &lt;message&gt;</code> — to every session of the project".into(),
            "photo / file with a caption mentioning the bot — saved into the project".into(),
            String::new(),
            "🧭 <b>Sessions</b>".into(),
            "<code>status</code> — open sessions of the project".into(),
            "<code>new [prompt]</code> — open a new Claude session".into(),
            "<code>resume</code> — reopen a finished session".into(),
            "<code>last</code> — last reply of the session".into(),
            "<code>tail [lines]</code> — last lines of the session terminal (default 30, max 80)".into(),
            "<code>stop</code> — stop the running work (Esc)".into(),
            "<code>compact</code> — Orrery Compact: handoff file, /compact, then resume (asks to confirm). <code>/compact notes</code> and <code>/clear</code> are typed into the session as they are".into(),
            "<code>close</code> — close the session pane (asks to confirm, owner only)".into(),
            "<code>focus</code> — bring Orrery to the front on that session".into(),
            "<code>where</code> — what this topic is connected to".into(),
            String::new(),
            "📱 <b>App</b>".into(),
            "<code>run [config]</code> — run the app with a Run config".into(),
            "<code>screen [serial]</code> — screenshot of the Android device".into(),
        ]
    };
    lines.join("\n")
}

pub fn inbound(vi: bool, key: &str) -> &'static str {
    match (vi, key) {
        (false, "noTopic") => "This topic is not assigned to a project in Orrery.",
        (false, "readOnly") => "You can only view here: status, last, tail, help.",
        (false, "rate") => "Too many messages in the last minute; wait a little.",
        (false, "none") => "No session of this project is open in an Orrery Workspace pane (sessions in other terminals cannot be written to).",
        (false, "open") => "That session is already open (in an Orrery pane or another terminal).",
        (false, "gone") => "That session is not open in Orrery any more.",
        (false, "unavailable") => "Orrery is not ready (window closed?).",
        (false, "statusEmpty") => "No session of this project is open in Orrery.",
        (true, "noTopic") => "Topic này chưa được gán project trong Orrery.",
        (true, "readOnly") => "Bạn chỉ có quyền xem: status, last, tail, help.",
        (true, "rate") => "Gửi nhanh quá, chủ nhân chờ chút nhé.",
        (true, "none") => "Không có session nào của project này mở trong pane Workspace của Orrery (session ở terminal khác thì không gửi vào được).",
        (true, "open") => "Session đó đang mở rồi (trong pane Orrery hoặc terminal khác).",
        (true, "gone") => "Session đó không còn mở trong Orrery.",
        (true, "unavailable") => "Orrery chưa sẵn sàng (cửa sổ đang đóng?).",
        (true, "statusEmpty") => "Không có session nào của project này mở trong Orrery.",
        _ => "",
    }
}

pub fn sent(vi: bool, n: &str) -> String {
    if vi { format!("📨 Đã gửi vào <b>{}</b>.", esc(n)) } else { format!("📨 Sent to <b>{}</b>.", esc(n)) }
}

pub fn queued(vi: bool, n: &str) -> String {
    if vi { format!("⏳ <b>{}</b> đang làm, xong việc sẽ gửi tiếp.", esc(n)) } else { format!("⏳ <b>{}</b> is busy; the message is sent when it finishes.", esc(n)) }
}

pub fn many(vi: bool, l: &str) -> String {
    if vi { format!("Project này có nhiều session, chủ nhân reply vào tin báo của session muốn gửi:\n{l}") } else { format!("Several sessions of this project are open; reply to a report of the one you mean:\n{l}") }
}

pub fn busy(vi: bool, n: &str) -> String {
    if vi { format!("<b>{}</b> đã có một tin đang chờ gửi.", esc(n)) } else { format!("<b>{}</b> already has a message waiting to be sent.", esc(n)) }
}

pub fn status(vi: bool, l: &str) -> String {
    if vi { format!("Các session của project:\n{l}") } else { format!("Sessions of this project:\n{l}") }
}

pub fn pane_status(status: &str, vi: bool) -> String {
    if !vi {
        return status.to_string();
    }
    match status {
        "working" => "đang làm",
        "idle" => "rảnh",
        "waiting" => "đang chờ",
        "stale" => "mất kết nối",
        "unknown" => "không rõ",
        other => other,
    }
    .to_string()
}

pub fn tail_head(vi: bool, n: &str) -> String {
    if vi { format!("📜 Terminal của <b>{}</b>", esc(n)) } else { format!("📜 Terminal of <b>{}</b>", esc(n)) }
}

pub fn tail_empty(vi: bool) -> &'static str {
    if vi { "Màn hình terminal đang trống." } else { "The terminal screen is empty." }
}

pub const MENU_EN: [(&str, &str); 14] = [
    ("status", "Open sessions of this project"),
    ("new", "Open a new Claude session"),
    ("stop", "Stop the running work (Esc)"),
    ("compact", "Compact the session (handoff, /compact, resume)"),
    ("close", "Close the session pane"),
    ("focus", "Bring Orrery to the front on the session"),
    ("last", "Last reply of the session"),
    ("tail", "Last lines of the session terminal"),
    ("resume", "Reopen a finished session"),
    ("where", "What this topic is connected to"),
    ("run", "Run the app with a Run config"),
    ("all", "Send a message to every session"),
    ("screen", "Screenshot of the Android device"),
    ("help", "List the commands"),
];

pub const MENU_VI: [(&str, &str); 14] = [
    ("status", "Các session đang mở của project"),
    ("new", "Mở session Claude mới"),
    ("stop", "Dừng việc đang chạy (Esc)"),
    ("compact", "Compact session (handoff, /compact, làm tiếp)"),
    ("close", "Đóng pane của session"),
    ("focus", "Đưa Orrery lên trước ở session"),
    ("last", "Câu trả lời cuối của session"),
    ("tail", "Các dòng cuối của terminal session"),
    ("resume", "Mở lại session đã kết thúc"),
    ("where", "Topic này đang nối với gì"),
    ("run", "Chạy app theo Run config"),
    ("all", "Gửi tin vào mọi session"),
    ("screen", "Chụp màn hình máy Android"),
    ("help", "Danh sách lệnh"),
];
