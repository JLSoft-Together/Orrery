# HANDOFF 2026-11-11 02:30 — Orrery 4.0.0 (Tauri + Rust)

## Trạng thái
- `master` đã push (tới `33584b3`), tag `v4.0.0` đã push, CI `desktop` run 42883690014 build Windows + mac Tauri.
- Release `layout-daemon-v1` (kit XML Preview, `--latest=false`) đã đăng; `latest` vẫn là v3.4.0.
- Release v4.0.0 ĐÃ ĐĂNG (latest) 02:50: https://github.com/JLSoft-Together/Orrery/releases/tag/v4.0.0 — feed cũ `Claude-Code-Monitor/releases/latest` trả v4.0.0 + digest. Windows Setup 19 MB, dmg 24 / 25 MB.
- Test: core / shell / desktop / web / typecheck sạch; E2E Electron rust 19/22 (v73, v76, run fail y hệt node = không phải Rust; v74 đã sửa `f0fd7eb`); smoke Tauri mac OK (health, WS, thoát sạch); đo perf mac `docs/perf/4.0.0-*.json`.
- Updater 3.4.0 → 4.0.0: code tag v3.4.0 chọn đúng `Orrery-4.0.0-arm64/x64.dmg`, `Orrery-Setup-4.0.0.exe`; Portable không lên được (không có asset).

## Việc tiếp (chủ nhân)
1. Cài `Orrery-Setup-4.0.0.exe` từ draft trên 1 máy Windows có 3.4: kiểm gỡ Electron cũ, giữ data, autostart, token GitHub, Telegram đăng nhập lại.
2. Mở dmg mac xem UI (Bảng, Quỹ đạo Q2/Q3, panel lệnh, Workspace terminal); macOS hỏi Keychain "Orrery Safe Storage" 1 lần.
3. Lỗi trên Windows → sửa nhanh, bản 4.0.1 (user 3.4 đã nhận 4.0.0 qua updater).

## Còn treo
- E2E `--shell=tauri` trên Windows (T3); OSC 633 trên Tauri; bash shim `--init-file`; Office khoá trên Tauri; W4 dời nút Office (chủ nhân chưa trả lời có trả về chỗ cũ không).
