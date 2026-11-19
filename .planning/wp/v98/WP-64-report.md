# WP-64 report: `^C` không ngắt lệnh trong pane shell (Windows)

## Lớp gây lỗi

Lớp core-pty / tiến trình: tiến trình `orrery-core` thừa kế cờ "bỏ qua Ctrl+C" (`SetConsoleCtrlHandler(NULL, TRUE)` của cha) và truyền xuống mọi tiến trình con tạo trong ConPTY. Child (ping, PowerShell...) không nhận `CTRL_C_EVENT`, nên `0x03` vào ConPTY vẫn không ngắt được. Byte `0x03` không bị biến đổi / lọc ở web, remote hay core.

Bằng chứng (test Rust, ghi `0x03` thẳng vào writer của `pty_win::spawn`):
- Trước sửa: `ping -t 127.0.0.1` và `powershell Start-Sleep 60` không thoát sau 3 s (cả hai test fail). Đã thử bỏ cờ `PSEUDOCONSOLE_WIN32_INPUT_MODE` và đổi `0x03` thành chuỗi win32-input `ESC[67;46;3;1;8;1_`: vẫn fail, loại hai giả thuyết đó.
- Sau khi gọi `SetConsoleCtrlHandler(None, 0)` trước `CreateProcessW`: cả hai thoát trong ngưỡng 3 s với byte `0x03` thô, không cần đổi byte.

Lớp (a) bàn phím thật trong dev app và (c) từ web app chưa chạy lại: cả ba đường cùng đi qua `pty_win::spawn` của core, và WP-58a đã ghi desktop Ctrl+C qua CDP cũng hỏng, khớp nguyên nhân này. Đường remote (`apps/shell/src/remote`) không bị sửa.

## Sửa

`apps/core/crates/core-pty/src/pty_win.rs`: đầu `spawn` gọi `SetConsoleCtrlHandler(None, 0)` (import thêm từ `windows_sys::Win32::System::Console`). Cờ này chỉ bật lại xử lý Ctrl+C mặc định cho process core và con cháu; core là server nền không có console riêng nên không ảnh hưởng gì khác.

Test hồi quy mới trong cùng file (`mod tests`): `ctrl_c_interrupts_ping`, `ctrl_c_interrupts_powershell_sleep`.

Pane Claude / Codex: không đổi byte ghi vào PTY, chỉ child nay nhận `CTRL_C_EVENT` đúng như terminal thường. Claude Code đặt raw mode (tắt processed input) nên vẫn nhận `0x03` như ký tự và xử lý "huỷ" như cũ. Chưa gửi prompt vào Claude thật.

## Test

- `cargo test -j 4 -p core-pty`: 48 passed, 0 failed (gồm 2 test mới).
- `cargo test -j 4 -p core-bin`: 45 passed, 3 ignored (9 suites).
- `cargo test -j 4 --bin orrery-shell pty`: 10 passed (shell không phụ thuộc core-pty, chỉ kiểm không hỏng).

## Bước kiểm tay cho chủ nhân

Cần build lại `orrery-core` (dev app dùng bản đã build, chưa rebuild trong WP này).
1. Desktop: mở pane shell, chạy `ping -t 127.0.0.1`, bấm Ctrl+C: ping phải dừng và hiện thống kê; thử `Start-Sleep 60`.
2. Điện thoại / web app: cùng lệnh, bấm nút `^C` trên KeyBar.
3. Pane Claude: Ctrl+C vẫn huỷ / xoá dòng như trước.

## Rủi ro

- Chỉ Windows; bản unix (`pty_unix.rs`) không đổi.
- Chưa xác định chính xác ai bật cờ ignore ở cha (shell Tauri hay launcher); sửa ở core đủ cho mọi cha.
