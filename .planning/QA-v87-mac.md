# QA v87 trên macOS — bản trung gian (PTY mặc định Rust)

Mặc định `CCM_CORE_PTY` = `rust` (đổi 2026-11-09, chờ soak macOS). Core Rust không khởi động được thì app tự dùng Node host và hiện notification "Terminals use the Node host" **1 lần**.

## Cài 1 lần trên mac

1. Command Line Tools (nếu chưa có): `xcode-select --install`.
2. Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh` (chọn mặc định), rồi `source "$HOME/.cargo/env"`, kiểm `cargo --version`. `apps/core/rust-toolchain.toml` dùng `stable` nên rustup tự tải đúng bản.
3. Chạy từ source: đóng app Orrery bản cài trước (single-instance, port 4317), rồi:
   ```
   git pull
   npm install
   npm run desktop
   ```
   `build.mjs` tự build `apps/core`. Thấy cảnh báo `apps/core was not built` = Rust chưa cài / chưa vào PATH: mở terminal mới rồi chạy lại. Không cần build bản cài mac.
4. Hoặc lấy bản dựng sẵn: GitHub Actions → workflow `desktop` → Run workflow → artifact `.dmg` (arm64 / x64). Bản ký ad-hoc: chuột phải → Open lần đầu.

## Các bước kiểm

Đánh dấu từng dòng; lỗi thì ghi số bước.

1. Mở app, Settings → Diagnostics → Copy: dòng đầu có `terminal host: rust`. Nếu ghi `node (rust core unusable, fell back)` thì Rust không chạy: gửi log (mục cuối).
2. Workspace: mở pane **shell**; gõ `echo hi`, `ls`, `vim` rồi `:q` (phím + màu đúng, không trễ).
3. Mở pane **Claude** và pane **Codex**: gõ prompt ngắn, có chữ trả về, Claude hiện đúng trong Monitor kèm subagent.
4. Resize: kéo cửa sổ và chia pane (split) — nội dung xterm co giãn, không vỡ dòng.
5. `cd` sang folder khác (có repo git): tiêu đề / branch của pane đổi theo (OSC 7); `git checkout -b x` đổi branch.
6. Đóng pane: `ps aux | rg "<tên lệnh vừa chạy>"` không còn process (ví dụ `sleep 999 &` rồi đóng pane).
7. Quit app (Cmd+Q) khi còn 2 pane: hộp xác nhận ghi đúng "Quit and close 2 terminals?" và đủ 2 dòng; Cancel thì app và pane còn; quit thật xong chạy `ps aux | rg orrery-core` **không còn dòng nào** (và không còn shell / claude con mồ côi).
8. `--resume`: đóng 1 pane Claude đã có hội thoại, dùng Undo close (hoặc mở lại session cũ): lệnh có `--resume <id>`, hội thoại về lại.
9. Kill core thử: `pkill -f orrery-core` khi có pane: pane báo lỗi / thoát, app không treo; mở pane mới thì chạy lại bình thường.
10. Monitor: session Claude / Codex đang chạy hiện đúng, subagent đúng cây.
11. Usage: tab Usage, heatmap hiện số khớp dự kiến, không treo khi mở lần đầu.
12. Port / process panel: liệt kê process và cổng đang nghe (ví dụ chạy `python3 -m http.server 8000`) có dòng 8000 kèm tên process.

## Tắt từng cờ nếu lỗi

Đặt biến rồi mở lại app (chạy từ terminal: `CCM_CORE_PTY=node npm run desktop`):

| Cờ | Tác dụng |
|---|---|
| `CCM_CORE_PTY=node` | Terminal dùng host Node + node-pty như trước |
| `CCM_CORE_USAGE=node` | Quét usage bằng Node |
| `CCM_CORE_TRANSCRIPT=node` | Theo dõi transcript bằng Node |
| `CCM_CORE_PROCS=node` | Bảng process / port bằng Node (PowerShell / `ps` + `lsof`) |

Bản đang chạy xem ở dòng `terminal host:` trong Diagnostics và dòng đầu `pty.log`. Cờ usage / transcript chỉ đổi mặc định ở bản sau; bản này nếu chưa mặc định Rust thì không cần đặt.

## Log gửi lại khi lỗi

- Settings → Diagnostics → Copy (đã che token / đường dẫn home), dán vào tin nhắn.
- `~/Library/Logs/Agent Monitor/pty.log` (và `collector.log`).
- Kết quả `ps aux | rg "orrery-core|Orrery"` và số bước lỗi ở trên.
