# core-desk: kênh `ccm:*` của trang (tổng quan)

Đường vào: `apps/core/crates/core-desk/src/lib.rs` (`Desk::standard`, `Desk::on`, `Desk::dispatch_with`, `Ctx`, `DeskEnv`, `DeskIo`). Lib thuần, không phụ thuộc Tauri. Vỏ gọi qua `apps/shell/src/desk.rs`.

## Luồng một kênh

Trang `window.ccmDesktop.invoke(channel, ...)` (shim `apps/shell/shim/preload.ts`, sinh bởi `scripts/gen-desk-shim.mjs`) -> command `ccm` (`apps/shell/src/ccm.rs`) -> `desk.rs::call_inside` -> `Desk::dispatch_with` -> handler đăng ký bằng `desk.on(MODULE, "ccm:...", |ctx, args| ...)`. Gọi ngược vỏ: `ctx.host(method, params)` -> `apps/shell/src/hosts.rs`. Event ra trang: `ctx.emit` / `Emitter`.

## Bảng module -> file index

| Thư mục (`apps/core/crates/core-desk/src/`) | Kênh / việc | Xem |
|---|---|---|
| `preview/` | Explorer, đọc / ghi file, Git (changes, log, branches, commit, worktree, checkpoints), search, secrets, i18n scan, scripts | `core-desk-preview.md` |
| `run/`, `layout/`, `flutter_preview/` | Run pane, XML Layout / Drawable / Flutter preview | `core-desk-run.md` |
| `devices/`, `apk/`, `capture/`, `phone/` | adb, emulator, devfs, APK size, quay màn hình, push điện thoại | `core-desk-devices.md` |
| `telegram/` (+ `mt/`) | bot, inbound, báo cáo, nút trả lời | `core-desk-telegram.md` |
| `mcp/`, `mcp_desk/` | MCP server `orrery`, tool, screenshot | `core-desk-mcp.md` |
| `disk/` | Disk host (mở file, không gian) | `core-disk.md` |
| `backup/`, `restore.rs`, `appearance.rs` | backup / khôi phục prompt, giao diện | tại file |
| `build_clean/`, `release_check/`, `libs/` | dọn build, kiểm phát hành, kiểm phiên bản lib | tại thư mục |
| `claude_hooks/`, `extensions/`, `bridge/`, `agent_status/`, `agent_data/`, `deepseek/` | cài hook / MCP / plugin, statusline, dữ liệu agent | `web-settings.md` (phía trang) |
| `github/` | tài khoản GitHub (token niêm phong qua `keychain.rs`) | `web-settings.md` |
| `update/`, `power/`, `resources.rs`, `pages.rs`, `editors.rs`, `layouts.rs` | cập nhật, keep awake, Resources, trang phụ | tại file |

## Bất biến

- Mọi kênh cần quyền file đi qua gate `preview/gate.rs` (root đã grant, realpath, không UNC, không gốc ổ).
- Xác nhận phá huỷ (xoá, cài MCP / plugin, gỡ) luôn qua dialog native của vỏ (`ctx.host("*.confirm")`), kể cả E2E (`CCM_E2E_CONFIRM`).
- Token (GitHub, Telegram) không bao giờ về trang; `keychain.rs` chỉ seal / unseal ở vỏ.

## Liên kết

- Thêm kênh: viết handler trong module, `register` ở `Desk::standard`, thêm vào danh sách kênh của vỏ nếu vỏ tự xử lý (`apps/shell/src/ccm.rs`), type phía web ở `packages/shared/src/*.ts` và `apps/web/src/core/lib/desktop.ts`.
- Plan: `.planning/PLAN-v87-step6-*.md`. Test: `apps/core/crates/core-desk/tests/`.
