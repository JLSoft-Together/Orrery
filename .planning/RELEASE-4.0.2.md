# Orrery 4.0.2

## English
- New default theme **Ngân hà (Galaxy)**: deep black with faint static stars behind the panels (Settings → Appearance → Show stars). Light systems get **Orrery** light; **Orrery** dark and **Console** stay available. Every install moves to the new default once.
- Pane colours follow the agent: Claude orange, Codex purple, shell teal. A status bar at the bottom shows running / waiting agents, dev ports, branch and context; on Monitor it also shows tokens used today.
- **Monitor › Board**: richer cards (running tool, branch, context, Open in Workspace / View diff), empty columns fold into strips, "Done today" timeline, group by status, project or agent.
- **Monitor › Orbit** is a star map: one system per project, each session a sun, subagents as planets, sessions that message each other joined by a dashed line, waiting sessions in yellow.
- Workspace: running subagents and a context meter in the pane header, a card with the files of the last turn, tabs grouped by project with "+N", focus mode (Ctrl+.), a launcher on new tabs, and Explorer marks the files agents are editing or reading.
- A starting animation shows while a Claude or Codex pane boots.
- Settings → Appearance: large theme cards with a Workspace preview and agent colours; Light / Dark / Follow system.
- Per-session **MCP & plugins** sheet (pane ⋯ menu, Monitor card menu): turn MCP servers, connectors and plugins off for one project, then restart the session with `--resume`.
- **Context warning**: token chip on panes and cards, a toast once a session passes 500K (Settings → Reminders, 0 = off) with a Compact… button.
- **Bug report** in Settings → About: one .zip (max 50 MB) with logs, crash reports and UI errors, secrets and home path masked.
- Fixes: Vietnamese typing in terminal panes on Windows and macOS; Windows drag files from Explorer into a pane inserts `@path` again; XML Preview "Fit" scales on first render.

## Tiếng Việt
- Theme mặc định mới **Ngân hà**: đen sâu, sao mờ tĩnh sau các panel (Settings → Giao diện → Hiện sao). Máy để chế độ sáng dùng **Orrery** sáng; **Orrery** tối và **Console** vẫn chọn được. Cập nhật xong mọi máy chuyển sang mặc định mới một lần.
- Màu pane theo agent: Claude cam, Codex tím, shell xanh ngọc. Thanh trạng thái ở đáy hiện agent đang chạy / đang chờ, cổng dev, nhánh và context; ở Giám sát có thêm token đã dùng hôm nay.
- **Giám sát › Bảng**: thẻ đủ thông tin (tool đang chạy, nhánh, context, Mở trong Workspace / Xem diff), cột rỗng thu thành dải, dòng thời gian "Xong hôm nay", nhóm theo trạng thái / project / agent.
- **Giám sát › Quỹ đạo** thành bản đồ sao: mỗi project một hệ, mỗi session một mặt trời, subagent là hành tinh, session nhắn nhau nối bằng đường đứt, session đang chờ màu vàng.
- Workspace: header pane hiện subagent đang chạy và thước context, thẻ liệt kê file của lượt vừa rồi, tab nhóm theo project kèm "+N", chế độ tập trung (Ctrl+.), launcher khi mở tab mới, Explorer đánh dấu file agent đang sửa / đọc.
- Pane Claude / Codex có hiệu ứng "đang khởi động" trong lúc agent mở lên.
- Settings → Giao diện: thẻ theme lớn có preview Workspace và màu agent; Sáng / Tối / Theo hệ thống.
- Bảng **MCP & plugin** theo session (menu ⋯ của pane, menu thẻ Giám sát): tắt MCP server, connector, plugin cho riêng một project, rồi khởi động lại session bằng `--resume`.
- **Cảnh báo context**: chip số token trên pane và thẻ, toast 1 lần khi session vượt 500K (Settings → Nhắc nhở, 0 = tắt) kèm nút Compact….
- **Báo lỗi** trong Settings → Giới thiệu: 1 file .zip (max 50 MB) gồm log, crash, lỗi giao diện, đã che secret và đường dẫn home.
- Sửa lỗi: gõ tiếng Việt trong terminal trên Windows và macOS; Windows kéo file từ Explorer vào pane lại ra `@path`; XML Preview "Fit" đúng tỉ lệ ngay lần render đầu.
