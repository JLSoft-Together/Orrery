# Release 3.4.0 (2026-11-08)

## English
- Agents can open a new pane and start its prompt themselves: Orrery MCP `open_session` with `submit: true` (Claude and Codex; links and shell panes never submit)
- `open_session` returns the pane id, ready for `send_to_session`, `close_session` and the new `wait_session`
- New MCP tool `wait_session`: waits up to 10 minutes until a pane is idle or closed, without reading files in a loop
- `list_sessions` shows when a Claude pane hit its usage limit (and when it resets) and since when a pane is idle
- Workflow status: any workflow can write `.orrery/status.json` (phase, progress, note) in the pane folder; it shows as a chip on the pane header and on the Mission Control card
- `run_log` filters lines by a regex and pages through new lines with a cursor, so log checks read only what is new
- Telegram: agents can attach reply buttons to a message; your tap is typed into that pane and sent only when the pane is idle (works with receive mode off)
- Resources: system services such as AnyDesk no longer show as an agent's servers after a reboot, so `free_port` is never offered for them

## Tiếng Việt
- Agent mở pane mới và tự gửi prompt luôn: MCP Orrery `open_session` với `submit: true` (Claude và Codex; link và pane shell không bao giờ tự gửi)
- `open_session` trả id pane, dùng thẳng cho `send_to_session`, `close_session` và tool mới `wait_session`
- Tool MCP mới `wait_session`: chờ max 10 phút tới khi pane rảnh hoặc đóng, không cần đọc file liên tục
- `list_sessions` báo pane Claude đang chạm usage limit (và lúc nào reset), pane rảnh từ lúc nào
- Trạng thái workflow: workflow bất kỳ ghi `.orrery/status.json` (phase, tiến độ, ghi chú) trong thư mục pane, hiện thành chip trên header pane và thẻ Mission Control
- `run_log` lọc dòng theo regex và đọc tiếp bằng cursor, kiểm log chỉ đọc phần mới
- Telegram: agent gắn button trả lời vào tin nhắn; bấm là chữ vào đúng pane đó, chỉ tự gửi khi pane rảnh (chạy cả khi tắt chế độ nhận tin)
- Tài nguyên: service hệ thống như AnyDesk không còn bị tính là server của agent sau khi khởi động lại máy, nên `free_port` không còn gợi ý tắt chúng
