use super::text::esc;
use serde_json::Value;

pub fn act(vi: bool, key: &str) -> &'static str {
    match (vi, key) {
        (false, "notOwner") => "Only the paired account can use these buttons.",
        (false, "notAllowed") => "Your access does not cover this button.",
        (false, "expired") => "This button is no longer valid.",
        (false, "state") => "That session is not in that state any more.",
        (false, "keysBusy") => "A previous button of that session is still being typed; try again in a moment.",
        (false, "screenNone") => "No Android device is connected to Orrery.",
        (false, "screenFail") => "Orrery could not take the screenshot.",
        (false, "fileTooBig") => "Telegram lets bots download files up to 20 MB only.",
        (false, "fileFail") => "Orrery could not save that file.",
        (false, "diffClean") => "No uncommitted changes.",
        (false, "diffNotRepo") => "That project is not a git repository.",
        (false, "diffFail") => "Orrery could not build the diff.",
        (false, "summaryEmpty") => "No session activity today.",
        (true, "notOwner") => "Chỉ tài khoản đã ghép mới dùng được các nút này.",
        (true, "notAllowed") => "Bạn không có quyền dùng nút này.",
        (true, "expired") => "Nút này không còn hiệu lực.",
        (true, "state") => "Session đó không còn ở trạng thái này nữa.",
        (true, "keysBusy") => "Nút bấm trước của session đó còn đang được gõ, chủ nhân thử lại sau giây lát.",
        (true, "screenNone") => "Không có máy Android nào kết nối với Orrery.",
        (true, "screenFail") => "Orrery không chụp được màn hình.",
        (true, "fileTooBig") => "Bot Telegram chỉ tải được file tối đa 20 MB.",
        (true, "fileFail") => "Orrery không lưu được file đó.",
        (true, "diffClean") => "Không có thay đổi chưa commit.",
        (true, "diffNotRepo") => "Project này không phải git repo.",
        (true, "diffFail") => "Orrery không tạo được diff.",
        (true, "summaryEmpty") => "Hôm nay không có session nào chạy.",
        _ => "",
    }
}

pub fn act1(vi: bool, key: &str, n: &str) -> String {
    let n = esc(n);
    match (vi, key) {
        (false, "approved") => format!("✅ Allowed in <b>{n}</b>."),
        (false, "denied") => format!("❌ Denied in <b>{n}</b>."),
        (false, "stopAsk") => format!("Stop the agent in <b>{n}</b>?"),
        (false, "sessionClosed") => format!("Session <b>{n}</b> of this topic is not open in Orrery. Send <code>resume</code> to reopen it."),
        (false, "stopped") => format!("⏹ Sent Esc to <b>{n}</b>."),
        (false, "escaped") => format!("⎋ Sent Esc to <b>{n}</b>."),
        (false, "closeAsk") => format!("Close <b>{n}</b> in Orrery? The pane closes now; reopening it is done on the machine (undo close)."),
        (false, "closed") => format!("🗑 Closed <b>{n}</b>."),
        (false, "compactAsk") => format!("Compact <b>{n}</b>? Orrery asks it to write a handoff, runs /compact, then continues from the handoff (it submits prompts)."),
        (false, "compacted") => format!("🗜 Compact started in <b>{n}</b>."),
        (false, "focused") => format!("🪟 Orrery is in front, on <b>{n}</b>."),
        (false, "screenCaption") | (true, "screenCaption") => format!("📱 {n}"),
        (false, "summaryHead") => format!("📊 <b>Day summary {n}</b>"),
        (true, "approved") => format!("✅ Đã cho phép ở <b>{n}</b>."),
        (true, "denied") => format!("❌ Đã từ chối ở <b>{n}</b>."),
        (true, "stopAsk") => format!("Dừng agent ở <b>{n}</b>?"),
        (true, "sessionClosed") => format!("Session <b>{n}</b> gắn với topic này chưa mở trong Orrery. Gõ <code>resume</code> để mở lại."),
        (true, "stopped") => format!("⏹ Đã gửi Esc vào <b>{n}</b>."),
        (true, "escaped") => format!("⎋ Đã gửi Esc vào <b>{n}</b>."),
        (true, "closeAsk") => format!("Đóng <b>{n}</b> trong Orrery? Pane đóng ngay; mở lại phải làm trên máy (hoàn tác đóng)."),
        (true, "closed") => format!("🗑 Đã đóng <b>{n}</b>."),
        (true, "compactAsk") => format!("Compact <b>{n}</b>? Orrery bảo nó ghi handoff, chạy /compact rồi làm tiếp từ handoff (có gửi prompt)."),
        (true, "compacted") => format!("🗜 Đã bắt đầu compact ở <b>{n}</b>."),
        (true, "focused") => format!("🪟 Orrery đã lên trước, ở <b>{n}</b>."),
        (true, "summaryHead") => format!("📊 <b>Tổng kết ngày {n}</b>"),
        _ => String::new(),
    }
}

pub fn chose(vi: bool, n: &str, l: &str) -> String {
    if vi { format!("🔹 Đã chọn “{}” ở <b>{}</b>.", esc(l), esc(n)) } else { format!("🔹 Chose “{}” in <b>{}</b>.", esc(l), esc(n)) }
}

pub fn image_prompt(vi: bool, n: usize) -> String {
    match (vi, n > 1) {
        (false, true) => format!("The user sent {n} images over Telegram (attached)."),
        (false, false) => "The user sent an image over Telegram (attached).".into(),
        (true, true) => format!("Chủ nhân gửi {n} ảnh qua Telegram (đã đính kèm)."),
        (true, false) => "Chủ nhân gửi một ảnh qua Telegram (đã đính kèm).".into(),
    }
}

pub fn file_prompt(vi: bool, k: &str, f: &str, c: &str) -> String {
    let tail = if c.is_empty() { String::new() } else { format!("\n{c}") };
    if vi { format!("Chủ nhân gửi {k} qua Telegram, đã lưu ở {f}.{tail}") } else { format!("The user sent a {k} over Telegram, saved at {f}.{tail}") }
}

pub fn diff_caption(vi: bool, n: &str, f: usize) -> String {
    if vi { format!("📄 <b>{}</b>: {f} file thay đổi", esc(n)) } else { format!("📄 <b>{}</b>: {f} file(s) changed", esc(n)) }
}

pub fn limit(vi: bool, i: Option<&Value>) -> String {
    let Some(i) = i else { return if vi { "⚠️ Hạn mức Claude sắp hết.".into() } else { "⚠️ Claude usage limit is about to run out.".into() } };
    let window = i.get("window").and_then(Value::as_str);
    let reset = i.get("reset").and_then(Value::as_str);
    let hit = i.get("hit") == Some(&Value::Bool(true));
    let used = i.get("usedPct").filter(|v| !v.is_null()).map(|v| crate::js::number(v.as_f64().unwrap_or(0.0)));
    if vi {
        let win = match window {
            Some("seven_day") => "7 ngày",
            Some("five_hour") => "5 giờ",
            _ => "",
        };
        let reset = reset.map(|r| format!(", reset lúc {r}")).unwrap_or_default();
        let w = if win.is_empty() { String::new() } else { format!(" ({win})") };
        if hit {
            return format!("⛔ Đã chạm hạn mức Claude{w}{reset}.");
        }
        return format!("⚠️ Hạn mức Claude{w} sắp hết{}{reset}.", used.map(|u| format!(": đã dùng {u}%")).unwrap_or_default());
    }
    let win = match window {
        Some("seven_day") => "7-day",
        Some("five_hour") => "5-hour",
        _ => "",
    };
    let reset = reset.map(|r| format!(", resets at {r}")).unwrap_or_default();
    let w = if win.is_empty() { String::new() } else { format!("{win} ") };
    if hit {
        return format!("⛔ Claude {w}usage limit reached{reset}.");
    }
    format!("⚠️ Claude {w}usage limit is about to run out{}{reset}.", used.map(|u| format!(": {u}% used")).unwrap_or_default())
}

pub fn disk(vi: bool, l: &str, f: &str) -> String {
    if vi { format!("💾 Ổ {} sắp đầy: còn {}.", esc(l), esc(f)) } else { format!("💾 Disk {} is almost full: {} left.", esc(l), esc(f)) }
}

pub fn run_failed(vi: bool, c: &str, code: &str) -> String {
    if vi { format!("❌ <b>{}</b> lỗi: {}", esc(c), esc(code)) } else { format!("❌ <b>{}</b> failed: {}", esc(c), esc(code)) }
}

pub fn summary_line(vi: bool, n: &str, c: &super::html::DayCounts) -> String {
    let min = super::jsx::round(c.worked_ms / 60_000.0);
    if vi {
        format!("• <b>{}</b>: {} lần xong, {min} phút, {} lần chờ, {} lần xin quyền{}", esc(n), c.done, c.waits, c.permissions, if c.run_fails > 0 { format!(", {} lần build lỗi", c.run_fails) } else { String::new() })
    } else {
        format!("• <b>{}</b>: {} finished, {min} min, {} waits, {} approvals{}", esc(n), c.done, c.waits, c.permissions, if c.run_fails > 0 { format!(", {} build failures", c.run_fails) } else { String::new() })
    }
}
