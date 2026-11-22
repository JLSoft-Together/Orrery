# RELEASE next (số version chủ nhân đặt)

Dòng changelog chờ (đưa vào `apps/web/src/features/update/lib/changelog.ts` cùng lúc bump `apps/shell/package.json` + `package-lock.json`, test `changelog.test.ts` đòi version đầu = version shell; mỗi dòng ≤ 100 ký tự):
- en: `The classic UI will be removed in 4.1.0` / vi: `Giao diện cũ sẽ bỏ ở bản 4.1.0` (WP-104i)
- en: `Inbox: one chip for menus, permissions, questions, same-file edits, errors, notes and done; Ctrl+Shift+A opens it, Ctrl+Alt+Q jumps to the next item` / vi: `Inbox gộp: 1 chip cho menu, xin quyền, câu hỏi, đụng file, lỗi, ghi chú và việc xong; Ctrl+Shift+A mở, Ctrl+Alt+Q tới mục kế` (WP-104e, cắt còn ≤100 ký tự khi đưa vào)
- en: `Mist uses solid panels: no more blur that froze the window after many resizes` / vi: `Sương kính dùng panel đặc: bỏ blur làm cửa sổ mờ, đứng khi đổi size nhiều lần`
- Các gói v104 khác (Inbox gộp, cảnh báo đụng file, Settings search, terminal điện thoại không mất dòng, DeepSeek tách module): viết dòng khi release.

## v109 (commit b913af1e..6f4a0428)
- en: `Phone remote never resizes the desktop terminal; phone terminal fits the width, pinch to zoom` / vi: `Điện thoại không còn đổi cỡ terminal trên máy; terminal điện thoại vừa bề ngang, pinch để zoom`
- en: `One grouped notification per burst, with a badge on the taskbar / Dock` / vi: `Gộp thông báo dồn dập thành 1, kèm badge trên taskbar / Dock`
- en: `Ctrl+Tab switches panes by recent use; shortcuts can be changed in Settings` / vi: `Ctrl+Tab đổi pane theo lần dùng gần nhất; đổi được phím tắt trong Settings`
- en: `Quota bar shows when the limit will run out at the current pace` / vi: `Thanh quota báo giờ sẽ hết theo tốc độ đang dùng`
- en: `Comment on diff lines and send them to the agent as a prefilled prompt` / vi: `Góp ý trên từng dòng diff, gửi sang agent thành prompt điền sẵn`
- en: `Long shell commands tell you when they finish (shell marks)` / vi: `Lệnh shell chạy lâu báo khi xong (shell marks)`
- en: `Commit and push warn before a secret leaves the machine` / vi: `Commit và push cảnh báo trước khi lộ secret ra ngoài`
- en: `Chat view for Claude / Codex sessions on the desktop` / vi: `Xem session Claude / Codex dạng chat ngay trên máy`
- en: `Share mode hides paths, emails and pairing codes while you share the screen` / vi: `Chế độ chia sẻ che path, email, mã ghép khi chia sẻ màn hình`
- en: `Search across every terminal with Ctrl+Alt+F` / vi: `Tìm trong mọi terminal bằng Ctrl+Alt+F`
- en: `Footer shows machine CPU and RAM` / vi: `Footer hiện % CPU và % RAM của máy`
- en: `Shorter project menu; drag to reorder projects in the rail` / vi: `Menu project gọn hơn; kéo để sắp xếp project trên rail`
- en: `Older themes moved behind View all` / vi: `Theme cũ gom vào mục Xem tất cả`

## v110 (chưa commit)
- en: `Electron and the Node collector are gone; the app runs only on the Tauri shell and Rust core` / vi: `Bỏ hẳn Electron và collector Node; app chỉ chạy trên Tauri + Rust`
- en: `Office loads only when you turn it on (smaller first load)` / vi: `Văn phòng chỉ tải khi bật, mở app nhẹ hơn`
- en: `Office is not available in this build` / vi: `Văn phòng chưa có trên bản này` (dòng Settings; chủ nhân chọn có đưa vào changelog không)
- Mac: P0.5 (PATH login shell, menu app, Move to Applications, Dock bounce, toast có nút) chưa chạy thử trên Mac, chỉ ghi dòng khi đã thử.
