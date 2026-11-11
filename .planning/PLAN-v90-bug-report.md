# PLAN v90 — Gửi log báo lỗi (gói .zip)

Chủ nhân 2026-11-11: "trong setting thêm phần gửi log báo lỗi: tổng hợp log, crash, lỗi log để user
down gửi mình fix", "log max 50MB".

## Chốt (con tự quyết, chủ nhân sửa được)
- Settings → Giới thiệu: section **Báo lỗi** — danh sách sẽ gom (tên + dung lượng), checkbox
  "Kèm file crash dump (có thể chứa dữ liệu bộ nhớ)" mặc định **tắt**, nút **Lưu gói báo lỗi…** → hộp thoại
  lưu `orrery-bugreport-<yyyymmdd-hhmm>.zip` vào Downloads, xong mở thư mục. Không upload đi đâu.
- Gói **max 50MB** (tính trên dữ liệu chưa nén). Vượt → cắt phần đầu log (giữ phần cuối mới nhất), bỏ dump
  cũ trước; `manifest.txt` ghi rõ file nào bị cắt / bỏ.
- Nội dung:
  - `report.txt`: bản report Diagnostics hiện có (version, OS, arch, locale, port, chẩn đoán trang).
  - `logs/`: mọi `*.log` trong data dir (`shell.log`, `shell.old.log`, `pty.log`, …) đầy đủ, đã mask.
  - `crashes/`:
    - Windows: event `Application Error` (1000) + `Windows Error Reporting` (1001) có `Orrery.exe` /
      `orrery-core.exe` / `orrery-shell.exe` 30 ngày (đọc bằng `wevtutil qe Application … /f:text`), các
      `Report.wer` trong `%LOCALAPPDATA%\Microsoft\Windows\WER\ReportArchive|ReportQueue\AppCrash_{Orrery,orrery}*`;
      dump `%LOCALAPPDATA%\CrashDumps\{Orrery,orrery}*.dmp` chỉ khi tick checkbox.
    - macOS: `~/Library/Logs/DiagnosticReports/{Orrery,orrery-core,orrery-shell}*.{ips,crash}` 30 ngày.
  - `page-errors.txt`: lỗi phía giao diện (window `error`, `unhandledrejection`, `console.error`) max 300
    dòng gần nhất trong phiên, giữ trong RAM.
- Mask: `mask_hook_text` (secret → ••••, home → `~`) cho mọi file text. Không bao giờ gom transcript / nội dung session.

- Chủ nhân thêm: phát hiện lỗi → toast nhẹ chỉ đường vào Settings gửi log. Crash lần chạy trước (tên crash/dump
  mới so với `ccm.bugReport.seen`) → 1 toast lúc mở app; lỗi giao diện chưa bắt (`error` / `unhandledrejection`,
  bỏ `ResizeObserver loop`) → max 1 toast / phiên, không trong 3s đầu. Switch "Nhắc gửi log khi có lỗi" mặc định bật.

## Contract
- `ccm:bug-report-preview()` → `{ ok: true, items: [{ name, bytes, kind: 'log'|'crash'|'dump'|'report' }], limit: 52428800 }`
- `ccm:bug-report-save({ page: string, errors: string, dumps: boolean })` →
  `{ ok: true, path, bytes, files: number, cut: string[], skipped: string[] } | { ok: false, code: 'cancel'|'io' }`
- preload `bugReportPreview()`, `bugReportSave(req)`; web `lib/desktop.ts` types `BugReportItem`, `BugReportPreview`, `BugReportSave*`.

## Gói
- WP-A backend: Tauri shell (`files_ipc.rs` hoặc module mới `bug_report.rs`, crate `zip` deflate), Electron
  mirror (`main.ts` cạnh `ccm:logs`), preload + shim, test Rust + Vitest.
- WP-B web: `lib/errorLog.ts` (ring buffer + install ở `main.ts`), section Settings, i18n, test, E2E `v90-bug-report.cjs`.
