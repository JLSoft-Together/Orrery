## WP-v87-3-leak: tìm nguyên nhân process con sót sau test parity PTY (Codex)
- Hiện tượng: 2026-11-09 có 48 process `node.exe -e "process.stdout.write('up\n');setInterval(()=>{},1000);process.stdin.on('data',()=>{})"` (child của `apps/desktop/test/pty-parity/script.ts`) còn sống sau các lượt chạy 12:41–12:59; parent đã chết. Không rõ do host Node (`apps/desktop/src/pty/host.ts`, node-pty) hay Rust core (`apps/core`, Job Object) hay do harness (`apps/desktop/test/pty-parity/harness.ts`).
- Việc:
  1. Trước mỗi lượt đếm process khớp chữ ký trên (PowerShell `Get-CimInstance Win32_Process -Filter "Name='node.exe'"`, lọc `CommandLine -like "*stdout.write('up*setInterval(()=>{},1000)*"`).
  2. Chạy `npm test -w @ccm/desktop -- test/pty-parity --maxWorkers=2` 3 lần chỉ host Node; đếm sau mỗi lượt. Rồi 3 lần với `CCM_PARITY_FULL=1 CCM_CORE_BIN=<abs>\apps\core\target\release\orrery-core.exe`; đếm. Thêm 1 lượt cố ý ngắt giữa chừng (timeout ngắn / kill vitest theo pid của chính nó) để xem đường dọn khi test bị huỷ.
  3. Xác định kịch bản (tên scenario trong `script.ts`) nào để sót: chạy từng scenario (`-t <tên>`) nếu cần.
  4. Viết test tái hiện (fail khi còn sót) nếu làm được; đề xuất cách sửa (file:line) nhưng KHÔNG sửa code ngoài `apps/desktop/test/pty-parity/**`. Lỗi nằm trong harness thì được sửa harness.
  5. Cuối cùng dọn mọi process khớp chữ ký do chính lượt này tạo (theo pid đã ghi).
- Luật: không git commit/push; không comment trong code; tìm bằng `rg`; KHÔNG chạm app Orrery đang chạy (port 4317, `%APPDATA%\Agent Monitor`, `%LOCALAPPDATA%\ccm`); KHÔNG `taskkill /IM` / `Stop-Process -Name`; chỉ kill theo pid tự ghi. Không sửa `apps/core/**`, `apps/desktop/src/**`, `apps/collector/**` (một WP khác đang sửa).
- Báo cáo: `.planning/wp/WP-v87-3-leak-report.md` (tiếng Việt, ≤ 20 dòng: bảng số process sót theo lượt Node / Rust / huỷ giữa chừng, scenario gây sót, nguyên nhân, đề xuất sửa file:line), dòng cuối `STATUS: DONE|PARTIAL|BLOCKED`.

## Nhật ký
