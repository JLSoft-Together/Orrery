# shell: vỏ Tauri `apps/shell`

Cargo workspace riêng (`apps/shell/Cargo.toml`, target `apps/shell/target`), link `core-serve`, `core-desk` bằng path. Đường vào `apps/shell/src/main.rs`.

## Khởi động

1. `apps/shell/src/config.rs::ShellConfig` (app key ngẫu nhiên 32 byte, cổng; bản debug mặc định 4419 + data `%TEMP%/orrery-shell-dev`).
2. `apps/shell/src/serve.rs::Serve` chạy `core_serve::run_with` cùng process (`/health` đã trả lời thì báo `PortInUse`, không chạm app khác).
3. `apps/shell/src/desk.rs` dựng `Desk::standard`, `lifecycle.rs::boot`, `episodes.rs`, `mcp.rs::start`.
4. `apps/shell/src/window.rs::create_main` mở `http://127.0.0.1:<port>/?k=<key>` (core-http đổi sang cookie), `init_script` nạp shim + `header.js` + `webview.js`.
5. Close cửa sổ chính = ẩn vào tray (`tray.rs`); thoát thật qua `tray::quit` -> `lifecycle::clean`.
6. Tray chờ bạn: web gửi `ccm:tray-waiting` -> `tray_waiting.rs::set` (max 8 dòng, 60 ký tự) -> `tray::refresh` dựng lại menu + tooltip; bấm dòng `wait:<terminal>` -> `tray_waiting::open` (ask `ccm:focus`).

## File chính

| File | Vai trò |
|---|---|
| `apps/shell/src/ccm.rs::ccm` | command duy nhất cho trang: `ccm(channel, args)`, chỉ webview cùng origin app |
| `apps/shell/shim/preload.ts` | nguồn shim `window.ccmDesktop` (bundle bởi `scripts/gen-desk-shim.mjs` thành `apps/shell/src/shim.js`) |
| `apps/shell/src/shim.rs` | nhúng shim vào init script |
| `apps/shell/src/header.js` | nút min / max / close tự vẽ (Windows), vùng kéo |
| `apps/shell/src/guest.rs`, `guest_ipc.rs`, `guest.js`, `webview.js` | child webview cho Browser pane (API kiểu `<webview>`) |
| `apps/shell/src/pty.rs`, `presets.rs`, `pty_branch.rs`, `shell_init.rs` | spawn `orrery-core` pty, preset claude / codex / shell, nhánh git của pane, OSC 633 |
| `apps/shell/src/links.rs::handle` | deep link `agent-monitor://open|focus|snooze` |
| `apps/shell/src/vault.rs` | keychain: master key AES-256-GCM, seal `o1:` |
| `apps/shell/src/update_host.rs` | cập nhật (nsis / mac dmg) |
| `apps/shell/src/log.rs` | `shell.log` trong data dir (> 2 MB thì xoay) |
| `apps/shell/src/legacy.rs`, `webstorage.rs` | nhập dữ liệu / localStorage từ bản Electron cũ |
| `apps/shell/src/mac_env.rs`, `mac_menu.rs`, `mac_move.rs` | macOS: PATH login shell, menu app, Move to Applications |
| `apps/shell/tauri.conf.json`, `tauri.release.json`, `nsis/hooks.nsh` | cấu hình đóng gói |

Host chi tiết: `shell-hosts.md`; disk: `shell-disk.md`; remote điện thoại: `shell-remote.md`.

## Chạy / test / phát hành

- Dev: `npm run shell:dev` (build web + shim + `cargo run -j 4`); `cargo test -j 4` trong `apps/shell`.
- E2E: `node apps/shell/e2e/run.cjs` hoặc `npm run e2e -w @ccm/shell`; chạy tay một file: `node apps/shell/e2e/<tên>.cjs`. Hạ tầng `apps/shell/e2e/cdp.cjs` (CDP qua WebView2, log `shell.log` / `shell.err` trong `CCM_E2E_OUT`).
- Phát hành: `npm run dist` -> `scripts/dist-tauri.mjs` (NSIS Windows, dmg macOS, chỉ build trên đúng OS); version ở `apps/shell/package.json` (`build.rs` đưa vào `ORRERY_VERSION`).
- macOS: nhiều đoạn viết nhưng CHƯA compile trên Mac (xem `.planning/PLAN-v110-modules.md` P0.5).

## Bất biến

- Trang app mới có `window.ccmDesktop`; child webview chỉ có command `guest`, không gọi được `ccm`.
- `on_navigation` của child chỉ http(s) và không phải origin app; app tự không cho mở `file:`.
- Devtools chỉ khi bản debug hoặc `CCM_E2E_DEVTOOLS=1`.
- Windows: `window.rs::keep_size_when_minimized` giữ webview chính size thường lúc minimize (`Fit::Keep`, `set_size` làm lệch tỉ lệ autoresize của tauri-runtime-wry), nên `Resized` đầu tiên sau restore phải `set_bounds` lại đúng client (`Fit::Fill`); thiếu bước này webview phình ~9x / 45x, UI bị che, không click được. E2E `window-restore`.
- Dev không bao giờ đụng cổng 4317 / `%LOCALAPPDATA%\ccm` của bản cài.

## Liên kết

- Plan: `.planning/PLAN-v87-step6-*.md`, `PLAN-v110-modules.md`. Env: `docs/env.md`.
- v112d `presets.rs::launch_flags` + `AgentOpts.{model,permission,effort}`: spawn msg `launch` chỉ thành cờ `--model/--permission-mode/--effort` (claude) hoặc `-m/-s` (codex) khi qua whitelist; resume không nhận cờ.
