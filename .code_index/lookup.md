# lookup: việc -> chỗ bắt đầu

Đọc file này trước mọi task, rồi mở 1-2 file module được trỏ tới. Path tính từ gốc repo. Kiểm: `node scripts/index-check.mjs`.

## Vỏ, kênh, giao thức

| Việc | Bắt đầu ở |
|---|---|
| Thêm kênh `ccm:*` từ trang | `apps/shell/src/ccm.rs::ccm` -> `apps/shell/src/desk.rs::call_inside` -> `apps/core/crates/core-desk/src/lib.rs::dispatch_with`; handler `desk.on(...)` trong module; type phía web `apps/web/src/core/lib/desktop.ts`; shim `scripts/gen-desk-shim.mjs` |
| Thêm message WS client -> server | `packages/shared/src/types.ts::ClientMessage` + `apps/core/crates/core-http/src/client/mod.rs::ClientMessage` + `apps/core/crates/core-serve/src/dispatch.rs` |
| Thêm event server -> client | `packages/shared/src/types.ts::MonitorEvent` + `apps/core/crates/core-store/src/event.rs` + `apps/web/src/app/stores/connection.ts::useConnectionStore` |
| Thêm tool MCP `orrery` | `apps/core/crates/core-desk/src/mcp/tools.json` (đọc ở `mcp/defs.rs`), handler `apps/core/crates/core-desk/src/mcp/t_open.rs` (hoặc `t_*.rs` khác), vỏ `apps/shell/src/mcp.rs::start` |
| Deep link `agent-monitor://` | `apps/shell/src/links.rs::handle` |
| Cửa sổ chính, init script | `apps/shell/src/window.rs::create_main`, `window.rs::init_script` |
| Serve nhúng khởi động | `apps/core/crates/core-serve/src/lib.rs::run` (gọi từ `apps/shell/src/serve.rs`) |
| Env / cổng / app key | `apps/core/crates/core-serve/src/config.rs::ServeConfig`, `apps/shell/src/config.rs::ShellConfig`, `docs/env.md` |
| Child webview của Browser pane | `apps/shell/src/guest.rs`, `apps/shell/src/guest.js` |
| Keychain / token niêm phong | `apps/shell/src/vault.rs`, `apps/core/crates/core-desk/src/keychain.rs` |
| Cập nhật app | `apps/shell/src/update_host.rs`, `apps/core/crates/core-desk/src/update/mod.rs` |
| Đóng gói / phát hành | `scripts/dist-tauri.mjs`, `apps/shell/tauri.conf.json`, `apps/shell/nsis/hooks.nsh` |

## Terminal, quan sát, toast

| Việc | Bắt đầu ở |
|---|---|
| Spawn pane terminal | `apps/shell/src/pty.rs`, `apps/shell/src/presets.rs`, `apps/core/crates/core-pty/src/hub/spawn.rs` |
| Viewer / owner input, claim | `apps/core/crates/core-pty/src/hub/viewers.rs` |
| Lệnh shell xong / lỗi (OSC 633) | `apps/core/crates/core-pty/src/marks.rs::OscScanner`, `apps/web/src/features/shellDone/useShellDone.ts` |
| Nhận diện session Claude / Codex | `apps/core/crates/core-monitor/src/monitor/reconcile.rs`, `apps/core/crates/core-monitor/src/registry.rs::read_registry` |
| Đọc transcript, status, context % | `apps/core/crates/core-transcript/src/tracker/mod.rs`, `apps/core/crates/core-monitor/src/tracker.rs::MonTracker` |
| Usage / chi phí | `apps/core/crates/core-monitor/src/usage.rs`, `apps/web/src/features/usage/stores/usage.ts` |
| Quy tắc toast (waiting, done, stuck) | `apps/core/crates/core-toast/src/toaster/observe.rs::observe` |
| Mask secret | `apps/core/crates/core-transcript/src/mask.rs`, `packages/shared/src/secretMask.ts` |
| Process table, ports | `apps/core/crates/core-procs/src/lib.rs` |

## Web

| Việc | Bắt đầu ở |
|---|---|
| State pane / tab / split | `apps/web/src/features/workspace/stores/workspace.ts::useWorkspaceStore` |
| Terminal pane (xterm) | `apps/web/src/features/workspace/components/TermPane.vue`, `lib/ptyClient.ts` cùng thư mục feature |
| Kết nối WS, snapshot / event | `apps/web/src/app/stores/connection.ts::useConnectionStore` |
| Feature mới nghe event | `apps/web/src/core/stores/connectionEvents.ts` |
| Settings lưu `ccm.*` | `apps/web/src/core/stores/settings.ts::useSettingsStore` |
| Thêm trang Settings | `apps/web/src/features/settings/components/index.ts` |
| Chọn theme lúc mở app | `apps/web/src/core/lib/themes.ts::startTheme` (khớp `apps/web/index.html` `pick`) |
| Token màu / shape | `apps/web/src/style.css`, `apps/web/src/themes.css`, `.planning/COLOR-SYSTEM.md` |
| Thêm chuỗi i18n | `apps/web/src/i18n/en.ts`, `apps/web/src/i18n/vi.ts` (hoặc `i18n/` của feature, import tĩnh trong hai file này) |
| Đọc menu lựa chọn Claude trên màn terminal | `apps/web/src/features/click-menu/lib/termMenu.ts::parseTermMenu` |
| Ctrl+K | `apps/web/src/features/palette/components/CommandPalette.vue` |
| Agent Map / Quỹ đạo | `apps/web/src/features/map/components/AgentMap.vue`, `apps/web/src/features/map/components/OrbitStarMap.vue` |
| UI next (rail, canvas, glass) | `apps/web/src/features/next/components/NextShell.vue` |
| Diff / Changes / Git Log | `apps/web/src/features/git/components/ChangesPane.vue`, `apps/web/src/features/git/components/log/LogPane.vue` |
| Editor file | `apps/web/src/features/files/components/FilePane.vue` |
| Dời file web, sửa import | `scripts/web-move.mjs`; luật tầng `apps/web/src/importRules.test.ts` |

## Desk (Rust) theo miền

| Việc | Bắt đầu ở |
|---|---|
| Quyền đường dẫn (gate) | `apps/core/crates/core-desk/src/preview/gate.rs`, `apps/shell/src/preview_host.rs` |
| Git (changes, log, commit, branch) | `apps/core/crates/core-desk/src/preview/ipc_git.rs` |
| Chặn commit / push có token | `apps/core/crates/core-desk/src/preview/secrets.rs::scan_staged` |
| Run Android / Flutter | `apps/core/crates/core-desk/src/run/manager.rs`, `apps/core/crates/core-desk/src/run/ipc.rs` |
| XML Layout preview | `apps/core/crates/core-desk/src/layout/daemon.rs`, `apps/layout-daemon/README.md` |
| adb / thiết bị | `apps/core/crates/core-desk/src/devices/manager.rs::DeviceManager` |
| Telegram nhận lệnh | `apps/core/crates/core-desk/src/telegram/inbound.rs` |
| Quét ổ, index, tìm | `apps/core/crates/core-disk/src/walk/mod.rs`, `apps/core/crates/core-disk/src/index.rs` |
| Xoá file an toàn (Disk) | `apps/shell/src/disk_guard.rs` |

## Remote điện thoại

| Việc | Bắt đầu ở |
|---|---|
| Gateway desktop | `apps/shell/src/remote/mod.rs`, `apps/shell/src/remote/session.rs` |
| Push (FCM / Web Push) | `apps/shell/src/remote/push.rs`, `apps/shell/src/remote/webpush.rs` |
| Giao thức Noise | `apps/core/crates/core-remote-proto/src/lib.rs` |
| PWA phiên Noise | `apps/pwa/src/remote/session.ts` |
| Relay | `apps/relay/src/room.ts` |
| App Flutter | `apps/mobile/.code_index/mobile.md` |

## Test, QA, tài liệu

| Việc | Bắt đầu ở |
|---|---|
| Gate nhanh theo diff | `scripts/qa-fast.mjs` (`npm run qa`), map E2E ở `E2E_MAP` trong file đó |
| Thêm / chạy E2E | `apps/shell/e2e/cdp.cjs` (hạ tầng), `apps/shell/e2e/run.cjs` (chạy hết) |
| Gate điện thoại | `scripts/qa-remote.mjs` |
| Kiểm index này | `scripts/index-check.mjs` |
| Fixture session giả | `scripts/demo-fixture.mjs` |
| Nguồn dữ liệu Claude Code / Codex | `docs/claude-code-integration.md`, `docs/codex-integration.md` |
| Ngoại lệ chủ nhân đã duyệt | `docs/core-principle.md` |
