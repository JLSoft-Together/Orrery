# shell-hosts: host callback, preview, episodes, lifecycle

Đường vào: `apps/shell/src/hosts.rs::call` (định tuyến `ctx.host(method, ...)` của core-desk theo prefix: `preview.`, `devices.`, `run.`, `layout.`, `telegram.`, `backup.`, `keychain.`...).

## File chính

| File | Vai trò |
|---|---|
| `apps/shell/src/hosts.rs` | bảng host đứng (không cần cửa sổ gọi) |
| `apps/shell/src/desk_text.rs`, `strings.rs` | chuỗi en / vi cho dialog native |
| `apps/shell/src/host_devices.rs` | `devices.confirm`, `devices.owner`, `devices.use`, `devfs.*`, `capture.*` |
| `apps/shell/src/host_run.rs` | `run.owner`, `run.confirm`, `layout.confirm`, `layout.urlFor`, `gate.*` |
| `apps/shell/src/host_extra.rs`, `host_telegram.rs`, `host_extra_text.rs` | confirm lẻ (extensions, hooks, github, resources, deepseek), Telegram, restore, phone idle |
| `apps/shell/src/preview_host.rs` | callback `preview.*` (gate đường dẫn) |
| `apps/shell/src/preview_ipc.rs`, `preview_access.rs`, `preview_grants.rs` | grant thư mục, full access, hộp thoại xin quyền |
| `apps/shell/src/preview_files.rs`, `preview_watch.rs`, `preview_serve.rs`, `files_ipc.rs` | list file, watcher, server ảnh, mở / reveal file |
| `apps/shell/src/episodes.rs` | episode Telegram / phone từ sink của core-serve |
| `apps/shell/src/lifecycle.rs` | `boot`, `clean`, `migrate_bridges` |
| `apps/shell/src/power.rs`, `badge.rs`, `attention.rs` | keep awake, badge, Dock bounce |
| `apps/shell/src/bug_report.rs`, `capture.rs`, `shot.rs` | bug report, chụp màn |
| `apps/shell/src/worktree.rs`, `wtcopy.rs` | worktree + sao chép |
| `apps/shell/src/folder_pick.rs`, `drop_paths.rs` | chọn thư mục, kéo thả |

## Bất biến

- Mọi dialog xác nhận đi qua `hosts::confirm` để `CCM_E2E_CONFIRM` điều khiển được; kind lạ trả `false`.
- Thiếu arm `preview.` thì Explorer / Git / search mất quyền (xem `core-desk-preview.md`).
- Path hợp lệ khi tuyệt đối, canonicalize, nằm trong 1 root đã grant (so cả component, Windows không phân biệt hoa thường).
- Không bắn đôi episode khi có main + compact: nguồn là sink của core-serve.

## Liên kết

- Core-desk: `core-desk.md`. Plan: `.planning/PLAN-v87-step6-*.md`.
