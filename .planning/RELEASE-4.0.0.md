# Release 4.0.0 (2026-11-11)

Orrery 4 runs entirely on Rust: Tauri shell + `orrery-core`, no Electron and no Node in the installer. / Orrery 4 chạy hoàn toàn bằng Rust: vỏ Tauri + `orrery-core`, bộ cài không còn Electron và Node.

## English
- Monitor has a new Board tab: four columns (Needs you, Running, Idle, Done today) with one card per session; Alt+1 / Alt+2 switch to Orbit
- New Orbit tab: sessions on priority rings around you (needs you inside, running in the middle, idle outside), a detail panel, and a focus view of one system with tool arcs and a tool feed (Enter in, Esc back)
- Running commands panel at the bottom of Monitor: every running tool with its full command (secrets masked, home shown as ~), how long it has run and a button that opens the pane; toggle it with the bar or the ` key
- Agents show by icon (Claude, Codex, DeepSeek) instead of a text label on Board cards
- New lightweight app: Orrery now runs on Tauri with a Rust core instead of Electron and Node; the installer is several times smaller and the app uses less RAM. Settings, layouts and GitHub tokens carry over; the old Electron app is removed on Windows, your data stays
- Telegram: sign in to your Telegram account again once (Settings → Telegram) to scan topics and send large files; the bot keeps working without it
- The portable Windows build is gone: use the Setup installer
- Lighter UI under load: events are batched per frame, off-screen cards and map nodes skip layout, hidden panes pause their output and clocks
- Office (pixel-art room view) is not in this version; it comes back later
- No more blur in toolbars, menus and dialogs (less GPU work)
- Guide: Monitor and Workspace tours are three short steps each
- XML Layout Preview is installed only when you want it: Settings → Integrations, or the Install button in an XML pane
- Linux builds are dropped (Windows and macOS only)
- Fixes: a shell server is named after the process that listens (no more `cut` / `tee`), subagent start time is exact, macOS paths under /tmp and /var are handled

## Tiếng Việt
- Monitor có tab Bảng mới: 4 cột (Cần anh, Đang chạy, Rảnh, Xong hôm nay), mỗi session 1 thẻ; Alt+1 / Alt+2 chuyển sang Quỹ đạo
- Tab Quỹ đạo mới: session xếp trên các vòng ưu tiên quanh bạn (cần anh ở trong, đang chạy ở giữa, rảnh ở ngoài), panel chi tiết, và màn tiêu điểm 1 hệ có vệt tool và dòng tool (Enter vào, Esc ra)
- Panel "Lệnh đang chạy" ở dưới Monitor: mọi tool đang chạy kèm lệnh đầy đủ (che secret, home hiện là ~), thời gian đã chạy và nút mở pane; bấm thanh hoặc phím ` để ẩn / hiện
- Agent trên thẻ Bảng hiện bằng icon (Claude, Codex, DeepSeek), không ghi chữ
- App mới nhẹ hơn: Orrery chạy trên Tauri với core Rust thay Electron + Node; bộ cài nhỏ hơn nhiều lần, app ít RAM hơn. Settings, layout và token GitHub được giữ; trên Windows bản Electron cũ tự gỡ, dữ liệu giữ nguyên
- Telegram: đăng nhập lại tài khoản Telegram 1 lần (Settings → Telegram) để quét topic và gửi file lớn; bot vẫn chạy khi chưa đăng nhập lại
- Bỏ bản Windows portable: dùng bộ cài Setup
- UI nhẹ hơn khi tải nặng: gộp event theo frame, thẻ và node ngoài màn hình không layout, pane ẩn tạm dừng output và đồng hồ
- Bản này chưa có Office (phòng pixel-art), bản sau mới có lại
- Bỏ blur ở toolbar, menu, dialog (đỡ GPU)
- Hướng dẫn: tour Monitor và Workspace còn 3 step ngắn mỗi tour
- XML Layout Preview chỉ cài khi cần: Settings → Integrations, hoặc nút Cài trong pane XML
- Bỏ bản Linux (chỉ còn Windows và macOS)
- Sửa: server chạy từ shell mang tên process đang listen (không còn hiện `cut` / `tee`), giờ bắt đầu subagent chính xác, đường dẫn /tmp và /var trên macOS đúng
