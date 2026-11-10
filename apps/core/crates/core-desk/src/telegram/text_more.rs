use super::text::esc;

pub fn label(vi: bool, kind: &str) -> &'static str {
    match (kind, vi) {
        ("ap", false) => "✅ Allow",
        ("ap", true) => "✅ Cho phép",
        ("dn", false) => "❌ Deny",
        ("dn", true) => "❌ Từ chối",
        ("go", false) => "▶️ Continue",
        ("go", true) => "▶️ Tiếp tục",
        ("ts", false) => "🧪 Run tests",
        ("ts", true) => "🧪 Chạy test",
        ("df", false) => "📄 Show diff",
        ("df", true) => "📄 Xem diff",
        ("st", false) => "⏹ Stop",
        ("st", true) => "⏹ Dừng",
        ("rs", _) => "↩️",
        ("rn", _) => "▶️",
        ("nw", false) => "➕ New session",
        ("nw", true) => "➕ Mở session mới",
        ("np", _) => "📨",
        ("lt", _) => "💬",
        ("tl", _) => "📜",
        ("lk", _) => "🔗",
        ("lp", _) | ("pj", _) => "📁",
        ("bg", false) => "▶️ Run all",
        ("bg", true) => "▶️ Chạy hết",
        ("bx", false) => "🗑 Skip",
        ("bx", true) => "🗑 Bỏ qua",
        ("qa", _) => "🔹",
        ("ul", false) => "✂️ Disconnect",
        ("ul", true) => "✂️ Ngắt",
        ("es", _) => "⎋ Esc",
        ("cp", _) => "🗜 Compact",
        ("cl", false) => "🗑 Close",
        ("cl", true) => "🗑 Đóng",
        ("fc", false) => "🪟 Focus",
        ("fc", true) => "🪟 Mở",
        _ => "",
    }
}

pub const KINDS: [&str; 23] = ["ap", "dn", "go", "ts", "df", "st", "rs", "rn", "nw", "np", "lt", "tl", "lk", "lp", "bg", "bx", "qa", "ul", "es", "cp", "cl", "fc", "pj"];

pub fn tail_button(vi: bool) -> &'static str {
    if vi { "20 dòng cuối" } else { "last 20 lines" }
}

pub fn plan_label(vi: bool, ap: bool) -> &'static str {
    match (vi, ap) {
        (false, true) => "✅ Approve plan",
        (false, false) => "❌ Reject plan",
        (true, true) => "✅ Duyệt plan",
        (true, false) => "❌ Từ chối plan",
    }
}

fn pick_what(vi: bool, w: &str) -> &'static str {
    match (vi, w) {
        (false, "send") => "Send to which one?",
        (false, "stop") => "Stop which one?",
        (false, "tail") => "Terminal of which one?",
        (false, "last") => "Last reply of which one?",
        (false, "compact") => "Compact which one? A press asks to compact it.",
        (false, "close") => "Close which one? A press closes it now.",
        (false, _) => "Show which one?",
        (true, "send") => "Gửi vào session nào?",
        (true, "stop") => "Dừng session nào?",
        (true, "tail") => "Xem terminal của session nào?",
        (true, "last") => "Xem câu trả lời cuối của session nào?",
        (true, "compact") => "Compact session nào? Bấm là bắt đầu compact.",
        (true, "close") => "Đóng session nào? Bấm là đóng ngay.",
        (true, _) => "Mở session nào lên?",
    }
}

pub fn pick(vi: bool, c: usize, w: &str) -> String {
    if vi { format!("Project đang có {c} session mở. {}", pick_what(vi, w)) } else { format!("This project has {c} open sessions. {}", pick_what(vi, w)) }
}

pub fn pin_line(vi: bool, n: &str, s: &str, tool: Option<&str>, min: Option<i64>) -> String {
    let icon = match s {
        "working" => "🟢",
        "waiting" => "🟡",
        "idle" => "⚪",
        "closed" => "⚫",
        _ => "❔",
    };
    let word = match (vi, s) {
        (false, "working") => "running",
        (false, "waiting") => "waiting for you",
        (false, "idle") => "idle",
        (false, "closed") => "closed",
        (false, _) => "unknown",
        (true, "working") => "đang chạy",
        (true, "waiting") => "chờ bạn",
        (true, "idle") => "rảnh",
        (true, "closed") => "đã đóng",
        (true, _) => "không rõ",
    };
    let t = tool.filter(|t| !t.is_empty()).map(|t| format!(" · {}", esc(t))).unwrap_or_default();
    let m = min.map(|m| if vi { format!(" · {m} phút") } else { format!(" · {m} min") }).unwrap_or_default();
    format!("{icon} {} — {word}{t}{m}", esc(n))
}

pub fn more(vi: bool, key: &str) -> &'static str {
    match (vi, key) {
        (false, "resumeNone") => "No ended session of this project to resume.",
        (false, "resumePick") => "Pick a session to resume:",
        (false, "runNone") => "This project has no Run config that can be started from Telegram (shared or imported configs need a confirmation on the machine).",
        (false, "runPick") => "Pick a Run config:",
        (false, "allNone") => "No agent session of this project could take it (none open, or each has a message waiting).",
        (false, "lastNone") => "No finished reply of that session yet.",
        (false, "linkPick") => "This topic is not connected yet. Connect it to an open session (🔗) or a whole project (📁):",
        (false, "pinEmpty") => "No session connected to this topic.",
        (true, "resumeNone") => "Project này không có session đã kết thúc để mở lại.",
        (true, "resumePick") => "Chọn session để mở lại:",
        (true, "runNone") => "Project này không có Run config chạy được từ Telegram (config chia sẻ / import cần xác nhận trên máy).",
        (true, "runPick") => "Chọn Run config:",
        (true, "allNone") => "Không session nào của project nhận được (chưa mở, hoặc đều đang có tin chờ gửi).",
        (true, "lastNone") => "Session này chưa có câu trả lời xong nào.",
        (true, "linkPick") => "Topic này chưa nối với gì. Nối với session đang mở (🔗) hoặc cả project (📁):",
        (true, "pinEmpty") => "Không có session nào nối với topic này.",
        _ => "",
    }
}

pub fn more1(vi: bool, key: &str, n: &str) -> String {
    let n = esc(n);
    match (vi, key) {
        (false, "opened") => format!("🆕 Opened a new Claude session in <b>{n}</b>."),
        (false, "resumed") => format!("↩️ Resumed <b>{n}</b>."),
        (false, "runNoMatch") => format!("No Run config named “{n}”."),
        (false, "runStarted") => format!("▶️ Starting <b>{n}</b>…"),
        (false, "runFail") => format!("Could not start: {n}"),
        (false, "linked") => format!("🔗 This topic is now connected to <b>{n}</b>. Tag me here to talk to it."),
        (false, "unlinked") => format!("✂️ Disconnected this topic from <b>{n}</b>."),
        (false, "ended") => format!("⏹ Session <b>{n}</b> ended, so this topic was disconnected. Reopen it (↩️) or connect another session (🔗):"),
        (false, "cleared") => format!("🧹 <b>{n}</b> started a new session (/clear), so this topic was disconnected. Connect the new one (🔗):"),
        (false, "last") => format!("💬 Last reply of <b>{n}</b>"),
        (false, "pinHead") | (true, "pinHead") => format!("📌 <b>{n}</b>"),
        (true, "opened") => format!("🆕 Đã mở session Claude mới ở <b>{n}</b>."),
        (true, "resumed") => format!("↩️ Đã mở lại <b>{n}</b>."),
        (true, "runNoMatch") => format!("Không có Run config tên “{n}”."),
        (true, "runStarted") => format!("▶️ Đang chạy <b>{n}</b>…"),
        (true, "runFail") => format!("Không chạy được: {n}"),
        (true, "linked") => format!("🔗 Đã nối topic này với <b>{n}</b>. Tag bot ở đây để làm việc với nó."),
        (true, "unlinked") => format!("✂️ Đã ngắt topic này khỏi <b>{n}</b>."),
        (true, "ended") => format!("⏹ Session <b>{n}</b> đã kết thúc nên topic này đã được ngắt. Mở lại (↩️) hoặc nối session khác (🔗):"),
        (true, "cleared") => format!("🧹 <b>{n}</b> đã sang session mới (/clear) nên topic này đã được ngắt. Nối session mới (🔗):"),
        (true, "last") => format!("💬 Câu trả lời cuối của <b>{n}</b>"),
        _ => String::new(),
    }
}

pub fn new_pick(vi: bool, c: usize, p: bool) -> String {
    if vi {
        format!("Project đang có {c} session mở. {}", if p { "Mở session mới, hay gửi prompt vào session đang có?" } else { "Vẫn mở thêm session mới?" })
    } else {
        format!("This project already has {c} open session{}. {}", if c > 1 { "s" } else { "" }, if p { "Open a new one, or send the prompt to one of them?" } else { "Open another one?" })
    }
}

pub fn run_shot(vi: bool, c: &str, d: &str) -> String {
    if vi { format!("📱 <b>{}</b> đang chạy trên {}", esc(c), esc(d)) } else { format!("📱 <b>{}</b> is running on {}", esc(c), esc(d)) }
}

pub fn all_sent(vi: bool, names: &[String]) -> String {
    let list = names.iter().map(|x| format!("<b>{}</b>", esc(x))).collect::<Vec<_>>().join(", ");
    if vi { format!("📨 Đã gửi vào {list}.") } else { format!("📨 Sent to {list}.") }
}

pub fn project_pick(vi: bool, p: bool) -> String {
    if vi { format!("Mở session mới ở project nào?{}", if p { " Prompt đi kèm." } else { "" }) } else { format!("Which project should the new session open in?{}", if p { " The prompt goes with it." } else { "" }) }
}

pub fn where_session(vi: bool, n: &str, p: &str, open: bool, keep: bool) -> String {
    if vi {
        format!("🔗 Topic này đang nối với session <b>{}</b> — {} trong Orrery.\n<code>{}</code>{}", esc(n), if open { "đang mở" } else { "chưa mở" }, esc(p), if keep { "\nGiữ nối cả khi session kết thúc." } else { "" })
    } else {
        format!("🔗 This topic is connected to session <b>{}</b> — {} in Orrery.\n<code>{}</code>{}", esc(n), if open { "open" } else { "not open" }, esc(p), if keep { "\nKept after the session ends." } else { "" })
    }
}

pub fn where_project(vi: bool, l: &str, p: &str) -> String {
    if vi { format!("📁 Topic này đang nối với project <b>{}</b>; tin nhắn vào các session của project.\n<code>{}</code>", esc(l), esc(p)) } else { format!("📁 This topic is connected to project <b>{}</b>; messages go to its sessions.\n<code>{}</code>", esc(l), esc(p)) }
}

pub fn backlog_head(vi: bool, n: usize) -> String {
    if vi { format!("🕘 Có {n} lệnh gửi lúc Orrery tắt:") } else { format!("🕘 {n} command{} sent while Orrery was off:", if n > 1 { "s were" } else { " was" }) }
}

pub fn backlog_skipped(vi: bool, n: usize) -> String {
    if vi { format!("🗑 Đã bỏ qua {n} lệnh.") } else { format!("🗑 Skipped {n} command{}.", if n > 1 { "s" } else { "" }) }
}

pub fn prompt(vi: bool, kind: &str) -> &'static str {
    match (vi, kind) {
        (false, "go") => "Continue.",
        (false, _) => "Run the tests of this project and report the result.",
        (true, "go") => "Tiếp tục.",
        (true, _) => "Chạy test của project và báo kết quả.",
    }
}

fn only_commands(t: &str) -> bool {
    let mut rest = t;
    if rest.is_empty() {
        return false;
    }
    while !rest.is_empty() {
        let Some(r) = rest.strip_prefix('/') else { return false };
        let end = r.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == ':' || c == '.' || c == '-')).unwrap_or(r.len());
        if end == 0 {
            return false;
        }
        rest = r[end..].trim_start_matches(|c: char| c.is_whitespace());
    }
    true
}

fn raw_command(t: &str) -> bool {
    let low = t.to_ascii_lowercase();
    ["/compact", "/clear"].iter().any(|c| low.strip_prefix(c).is_some_and(|r| r.is_empty() || r.starts_with(|ch: char| ch.is_whitespace())))
}

pub fn via_telegram(text: &str, vi: bool) -> String {
    let t = super::jsx::trim(text);
    if text.is_empty() || only_commands(t) || raw_command(t) {
        return text.to_string();
    }
    let note = if vi { "(Tin này gửi từ Telegram. Xong việc, gửi kết quả bằng tool telegram_send, kèm file nếu được yêu cầu, ví dụ APK đã build.)" } else { "(Sent from Telegram. When done, send the result with the telegram_send tool, attaching the file if one was asked for, e.g. a built APK.)" };
    format!("{text}\n\n{note}")
}

pub fn reply_text(vi: bool, key: &str, l: &str) -> String {
    match (vi, key) {
        (false, "sent") => format!("✓ {l} — sent to the session."),
        (false, "typed") => format!("✓ {l} — typed into the session; it is not idle, so it was not submitted."),
        (false, "already") => format!("Already answered: {l}"),
        (false, "closed") => "That session is closed; nothing was typed.".into(),
        (false, _) => "That session already has a prompt waiting; try again in a moment.".into(),
        (true, "sent") => format!("✓ {l} — đã gửi vào session."),
        (true, "typed") => format!("✓ {l} — đã gõ vào session; session đang bận nên chưa gửi."),
        (true, "already") => format!("Đã trả lời rồi: {l}"),
        (true, "closed") => "Session đó đã đóng; không gõ gì.".into(),
        (true, _) => "Session đó đang có prompt chờ gửi, chủ nhân thử lại sau giây lát.".into(),
    }
}

pub fn file_kind(vi: bool, kind: &str) -> String {
    match (vi, kind) {
        (false, "photo") => "a photo",
        (false, "voice") => "a voice message",
        (false, "audio") => "an audio file",
        (false, "video") => "a video",
        (false, "document") => "a file",
        (false, "album") => "several files",
        (true, "photo") => "ảnh",
        (true, "voice") => "tin nhắn thoại",
        (true, "audio") => "file âm thanh",
        (true, "video") => "video",
        (true, "document") => "file",
        (true, "album") => "nhiều file",
        (_, other) => return other.to_string(),
    }
    .to_string()
}
