# DECISIONS

- 2026-11-04 · MCP `telegram_receive on` phải qua hộp xác nhận native trên máy (agent không tự bật nhận lệnh từ xa được); `off` / `status` và `telegram_notify` không cần · lý do: review WP-1 (prompt injection có thể tự bật nhận lệnh). Main quyết, báo chủ nhân.
- 2026-11-04 · Chủ nhân cho phép tự test Telegram bằng bản dev (`npm run build` web + desktop, electron stage với user-data-dir / CCM_DATA_DIR tạm, cổng 4319, CDP 9333) · lý do: chủ nhân nói 'con tự test đi nhé'.
- 2026-11-05 · Người lạ tag / trả lời bot: bot trả câu cố định "Xin lỗi bạn, tôi chỉ trả lời chủ nhân <tên>, gửi thêm tiền để thương lượng 🐮" (tối đa 5 câu/phút, không bao giờ chuyển vào session, không tốn token) — thay cho "bỏ qua im lặng" của 2026-11-04 · lý do: chủ nhân yêu cầu.
- 2026-11-05 · Kết nối Telegram theo session (menu chuột phải session ở rail → "Kết nối Telegram…", chỉ session này hoặc cả project) · tuyến session gắn theo Claude session id (giữ qua resume), trang đẩy bảng pane ↔ session ↔ terminal sang main (`ccm:telegram-panes`) · lý do: chủ nhân yêu cầu.
- 2026-11-05 · Thành viên Telegram (WP-D, main chốt theo đề xuất, chủ nhân đổi được): mức `read` chỉ status / last / tail / help + nút 💬 📜 📄; mức `full` mọi lệnh + nút trừ Cho phép / Từ chối quyền tool, nối topic, lệnh lúc app tắt, ghép bot (chỉ chủ nhân); prompt của thành viên mang tên đã lưu trong app, không lấy tên Telegram (chống mạo danh).

- 2026-11-09 · Office (Habblaud) mặc định TẮT, bật trong Settings · chủ nhân hầu như không dùng; tắt thì collector không chạy OfficeHub/timelapse (tiết kiệm CPU/disk), web không load Habblaud.
- 2026-11-09 · PLAN-v87 lõi Rust: đồng ý cài Rust + CI macOS; bỏ blur trên toolbar/sidebar (GPU); chuyển MCP orrery từ PTY host sang Electron main ở bước 2; giảm cả size cài đặt (bước 1b, mạnh nhất ở bước 6 Tauri).
- 2026-11-09 · Tauri không Node sidecar, port hết sang Rust rồi mới bỏ Electron · chủ nhân muốn size bộ cài nhỏ nhất (~20–35 MB) · .planning/PLAN-v87-tauri-options.md
