# core-disk: quét ổ, index RAM, tìm kiếm, dupes, MFT

Đường vào: `apps/core/crates/core-disk/src/lib.rs`; chạy bằng `orrery-core disk` (NDJSON, `cli.rs`) hoặc nhúng qua `apps/shell/src/disk_host.rs`.

## File chính

| File | Vai trò |
|---|---|
| `apps/core/crates/core-disk/src/walk/mod.rs` | `scan`: pool luồng, `Level` (light / balanced / max), `Control` (pause / cancel), `Progress` |
| `apps/core/crates/core-disk/src/walk/win.rs` | `FindFirstFileExW` trên `\\?\`, hạ ưu tiên luồng |
| `apps/core/crates/core-disk/src/walk/mac.rs` | `list_bulk` (`getattrlistbulk`), trả `None` thì rơi về `unix.rs::list` |
| `apps/core/crates/core-disk/src/walk/unix.rs` | `list`: `read_dir` + `meta.blocks() * 512` |
| `apps/core/crates/core-disk/src/mft/mod.rs` | "Quét siêu nhanh" đọc MFT NTFS (một helper UAC chạy một lần) |
| `apps/core/crates/core-disk/src/index.rs` | `Node` 48 B, `Index`, `Builder`, merge, forget |
| `apps/core/crates/core-disk/src/search.rs` | `Query`, tìm song song, bỏ dấu (`fold.rs`) |
| `apps/core/crates/core-disk/src/dupes.rs` | file trùng (BLAKE3) |
| `apps/core/crates/core-disk/src/store.rs` | `save` / `load` index nén ở `CCM_DATA_DIR/disk/` |
| `apps/core/crates/core-disk/src/serve.rs` | `Engine` + `dispatch` + vòng NDJSON |
| `apps/core/crates/core-disk/src/volumes.rs` | danh sách ổ |

## Phía vỏ (`shell-disk.md`)

`apps/shell/src/disk_map.rs` (scan / index / search / treemap), `disk_clean*.rs` (dọn dev / system / Docker), `disk_organize*.rs` (Downloads + Undo), `disk_move*.rs` (dời thư mục, junction), `disk_apps*.rs` (gỡ app), `disk_ram.rs`, `disk_snap*.rs`, `disk_guard.rs` (kiểm đường dẫn trước khi xoá).

## Bất biến

- Không follow link / reparse point (trừ tag cloud), không qua ổ khác trừ khi `crossMounts`, không mở file cloud-only.
- Hardlink tính 1 lần; size là size trên đĩa (làm tròn cluster).
- Index vượt `u32::MAX` node thì scan lỗi `TooLarge`, giữ index cũ. Cancel giữ nguyên index cũ.
- Mọi xoá đi qua `disk_guard.rs` (đường dẫn canonical) và dialog native, Recycle Bin / Trash mặc định.
- Ổ mạng không gọi API có thể treo (`GetVolumeInformationW`).

## Liên kết

- Web: `web-disk.md`. Plan: `.planning/PLAN-v101-disk-manager.md`. Bản chi tiết cũ: `.code_index/archive/core-disk-v101.md`. E2E: `v101-*.cjs`.
