# RELEASE 4.0.9

Release 2026-11-23 (tag v4.0.9): Sương kính dùng panel đặc, terminal không mất nội dung khi thu nhỏ, gói v109 (thông báo gộp, phím tắt, chat view, share mode, góp ý diff), bỏ Electron.

## Tiếng Việt

### Sửa lỗi
- Sương kính dùng panel đặc: bỏ blur làm cửa sổ mờ và đứng khi đổi size nhiều lần. Bỏ công tắc Giảm trong suốt và thanh Độ trong kính của theme tự chọn.
- Thu nhỏ Orrery không còn bóp terminal còn 1 cột làm mất nội dung agent đã in.
- Điện thoại không còn đổi cỡ terminal trên máy; terminal điện thoại vừa bề ngang, pinch để zoom.

### Mới
- Gộp thông báo dồn dập thành 1, kèm badge trên taskbar / Dock.
- Ctrl+Tab đổi pane theo lần dùng gần nhất; đổi được phím tắt trong Cài đặt.
- Thanh quota báo giờ sẽ hết theo tốc độ đang dùng.
- Góp ý trên từng dòng diff, gửi sang agent thành prompt điền sẵn.
- Lệnh shell chạy lâu báo khi xong.
- Commit và push cảnh báo trước khi lộ secret ra ngoài.
- Xem session Claude / Codex dạng chat ngay trên máy.
- Chế độ chia sẻ che path, email, mã ghép khi chia sẻ màn hình.
- Tìm trong mọi terminal bằng Ctrl+Alt+F.
- Footer hiện % CPU và % RAM của máy.
- Menu project gọn hơn; kéo để sắp xếp project trên rail.
- Theme cũ gom vào mục Xem tất cả.

### Bên trong
- Bỏ hẳn Electron và collector Node; app chỉ chạy trên Tauri + Rust.
- Văn phòng chỉ tải khi bật, mở app nhẹ hơn.

## English

### Fixes
- Mist uses solid panels: no more blur that froze the window after many resizes. The Reduce transparency switch and the custom theme glass clarity slider are gone.
- Minimizing Orrery no longer squeezes terminals to one column and wipes what agents printed.
- Phone remote never resizes the desktop terminal; the phone terminal fits the width, pinch to zoom.

### New
- One grouped notification per burst, with a badge on the taskbar / Dock.
- Ctrl+Tab switches panes by recent use; shortcuts can be changed in Settings.
- Quota bar shows when the limit will run out at the current pace.
- Comment on diff lines and send them to the agent as a prefilled prompt.
- Long shell commands tell you when they finish.
- Commit and push warn before a secret leaves the machine.
- Chat view for Claude / Codex sessions on the desktop.
- Share mode hides paths, emails and pairing codes while you share the screen.
- Search across every terminal with Ctrl+Alt+F.
- Footer shows machine CPU and RAM.
- Shorter project menu; drag to reorder projects in the rail.
- Older themes moved behind View all.

### Under the hood
- Electron and the Node collector are gone; the app runs on Tauri and Rust only.
- Office loads only when you turn it on, so the app opens lighter.

## Chưa có trong bản này
- macOS v110 P0.5 (PATH login shell, menu app, Move to Applications, Dock bounce, toast có nút) chưa thử trên Mac.
