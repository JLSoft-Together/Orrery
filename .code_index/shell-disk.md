# shell-disk: host Disk (Rust trong vỏ)

Đường vào: `apps/shell/src/disk_host.rs` (kênh `ccm:disk-*`), engine ở `apps/core/crates/core-disk` (xem `core-disk.md`).

## File chính

| File | Vai trò |
|---|---|
| `apps/shell/src/disk_map.rs` | scan, index, search, treemap / sunburst, MFT helper |
| `apps/shell/src/disk_clean.rs`, `disk_clean_dev.rs`, `disk_clean_sys.rs` | Clean up: rác dev, hệ thống, SDK / emulator, Docker |
| `apps/shell/src/disk_clean_text.rs` | chuỗi dialog dọn |
| `apps/shell/src/disk_organize.rs`, `disk_organize_rules.rs`, `disk_organize_log.rs` | Organize Downloads + Undo |
| `apps/shell/src/disk_move.rs`, `disk_move_tree.rs` | dời thư mục sang ổ khác, để lại junction |
| `apps/shell/src/disk_apps.rs`, `disk_apps_reg.rs`, `disk_apps_left.rs` | Apps: danh sách, gỡ, phần dư |
| `apps/shell/src/disk_ram.rs` | RAM theo process |
| `apps/shell/src/disk_snap.rs`, `disk_snap_report.rs` | snapshot so sánh, báo cáo Telegram |
| `apps/shell/src/disk_guard.rs` | chặn đường dẫn xoá (canonical, không gốc ổ / hệ thống) |
| `apps/shell/src/disk_agent.rs` | MCP `disk_search`, `disk_usage`, `disk_propose_delete` |
| `apps/shell/src/disk_pick.rs`, `disk_text.rs` | chọn thư mục, chuỗi |

## Bất biến

- Mọi xoá qua `disk_guard.rs` + dialog native, Recycle Bin / Trash mặc định; file cloud-only không bao giờ mở.
- Rescan nền khi idle chạy ở mức `light`, bỏ khi chủ nhân đang dùng máy.
- Test nằm cạnh file (`disk_*_tests.rs`), không test trên ổ thật.

## Liên kết

- Web: `web-disk.md`. Plan: `.planning/PLAN-v101-disk-manager.md`. E2E: `v101-*.cjs`.
