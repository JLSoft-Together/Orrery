# PROGRESS 2026-11-11 — vá các phần chưa port sang Tauri (4.0.x)

Lỗi gốc: Explorer, Git Log không đọc được trên 4.0.x. `core-desk` gọi ngược host `preview.*`, vỏ Tauri `apps/shell/src/hosts.rs` `call` không có → `Err("no host for …")`. Quét thêm: ~60 host call và 16 kênh trang chưa có handler.

Chủ nhân chốt: làm hết, test trên mac, xong chủ nhân test máy Windows.

## Bảng WP

| WP | Phạm vi | File sở hữu | Trạng thái |
|---|---|---|---|
| 1 | host `preview.*` (18) | `preview_host.rs`, `preview_text.rs`, `preview_host_tests.rs`, `pty.rs` `live_roots` | xong, `cargo test` shell 115 pass |
| 1b | kênh `grant-folder`, `request-access`, `forget-grants`, `full-access`, `list-files`, `watch`/`unwatch`, `changes-watch`/`unwatch`, `project-root` | `preview_ipc.rs`, `preview_access.rs`, `preview_grants.rs`, `preview_files.rs`, `preview_watch.rs`, `ccm.rs`, `window.rs` | xong, shell 127 pass; thêm dep `notify` 8.2 (đã có trong lock) |
| 2 | `run.*`, `layout.urlFor`/`confirm`, `gate.dir` | `host_run.rs`, `host_run_tests.rs` | xong, 11 pass; `run.owner` = `<label>:<key>`, `gate.dir` / `gate.file`, server ảnh nhỏ cho `layout.urlFor` (gói 5 gộp vào server preview chung) |
| 3 | `devices.*`, `devfs.*`, `apk.pick`, `release.pick`, `capture.*` | `host_devices.rs`, `host_devices_tests.rs` | xong, 8/8 pass; main thêm `window.rs` `release_owners` (nhả `desk:dev-release-owner` + `desk:run-release-owner` prefix `<label>:` khi reload / đóng cửa sổ) |
| 4 | `telegram.*`, confirm extensions/hooks/claudeHooks/statusline/github/resources/deepseek, `restore.*`, `disk.open`, `phone.*`, soát `mcp.*` | `host_extra.rs`, `host_telegram.rs`, `host_extra_text.rs`, `host_extra_tests.rs` | xong, 16/16 pass; `mcp.*` không cần (Tauri MCP chạy trong `mcp.rs`) |
| 6 | nối `desk:tg-tick` / `tg-handle` / `tg-alert` / `phone-episode`, `run-boot` / `run-clean`, `bridge-migrate`, cho phép project khi Telegram start run | `episodes.rs`, `lifecycle.rs` (mới), `desk.rs` `call_inside_allowing`, `state.rs` `before_stop`, `serve.rs`, `core-serve/src/lib.rs` `run_with` | xong, shell 189 pass |
| 5 | kênh `preview-url`, `open-path`, `reveal`, `pick-file(s)`, `logs` | `files_ipc.rs`, `preview_serve.rs` (+tests), `ccm.rs`; gộp server ảnh của `host_run.rs` | xong, shell 175 pass |
| QA | test tự động: shell 189, core-desk 376, core-serve, web 1914/1915 (1 test `usePaneAttachments` chập chờn khi chạy cả bộ, chạy riêng pass), shim up to date, `cargo build` debug OK. E2E `--shell=tauri` không chạy được trên mac (WKWebView không có CDP, chỉ WebView2) | chờ chủ nhân click thử mac + Windows |

`hosts::call` đã nối sẵn `host_run` / `host_devices` / `host_extra` (stub trả `None`) để 3 gói chạy song song không sửa chung `hosts.rs` / `main.rs`.

## UI
- Nút `ccm-btn-primary` / `ccm-btn-danger` thiếu class gốc `ccm-btn` (không padding / height / radius / cỡ chữ): sửa 8 file (`XmlPreviewSettings`, `DrawablePreview`, `FlutterPreviewPanel`, `LayoutPreview`, `PermAdvisorDialog`, `StringsDialog`, `AgentDataSettings`, `ResourcesDialog`).

## QA Windows (2026-11-11)
- Sửa trong lúc QA (chưa commit): nhánh git cho pane (`pty_branch.rs`), CSP `connect-src` thêm `ipc:` / `http://ipc.localhost`, `disable_drag_drop_handler` (`window.rs` / `guest.rs`), `procs.rs` `alive` bỏ pid < 1, `apps/core/.gitattributes` `*.dart eol=lf`, test chạy được trên Windows.
- E2E `run.cjs --shell=tauri` lượt 2: 1064 PASS / 27 FAIL, `electron.err` không còn `no host for` / `not ported`.
- Phân loại 27 fail:
  - Script chỉ chạy được trên Electron (không sửa app): `v74`, `v75`, `v76b`, `v77a`, `v76` "block next time" (patch `dialog.showMessageBox` qua Node inspector của main), `v49-open` (spawn Electron làm second instance).
  - Electron cũng fail (script cũ): `p5`, `v67-ide`, `agent-map`, `v25` main agent node (Orbit thay Sky từ v88), `v76` resources / sensitive / keep awake, `v79-shots`.
  - Chập chờn: `v70-tools` (chạy riêng 43/43), `tabs-cwd` (17/17).
  - Lỗi Tauri thật, chưa sửa:
    1. `v25` drop file từ OS: shim `webUtils.getPathForFile` trả `""` → không ra `@path`. Web gọi đồng bộ (`TermPane.vue`, `DeviceFilesDialog.vue`). Hướng sửa: API bất đồng bộ, Windows dùng `chrome.webview.postMessageWithAdditionalObjects` → host đọc `CoreWebView2File.Path`; macOS chưa có cách.
    2. `layout-speed` fit mode: frame đầu render 1080 px thay vì đúng cỡ đang hiện. Rust giữ nguyên `scale`; nghi `box.width` = 0 lúc render đầu trên WebView2 (`LayoutPreview.vue` `wantScale`), watch chỉ render lại khi cần nét hơn. Chưa probe.
    3. `v73` pane PowerShell exit 3 không có dấu ✗: gap OSC 633 trên Tauri (đã biết từ 4.0.0).
- 2026-11-11 ~04:20 app Orrery 4.0.1 tự tắt (pty.log `shutdown`, mở lại 04:21:09) trong lúc chạy E2E Electron so sánh `agent-map` → phiên Claude chết theo. Chạy lại 5 script Electron sau đó không bị tắt; chưa rõ nguyên nhân.
