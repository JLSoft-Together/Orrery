# lookup: việc -> chỗ bắt đầu

Đọc file này trước mọi task, rồi mở 1-2 file module được trỏ tới (`README.md`). Path tính từ gốc repo. Kiểm: `node scripts/index-check.mjs`.

## Vỏ, kênh, giao thức

| Việc | Bắt đầu ở |
|---|---|
| Thêm kênh `ccm:*` từ trang | `apps/shell/src/ccm.rs::ccm` -> `apps/shell/src/desk.rs::call_inside` -> `core-desk/src/lib.rs::dispatch_with`; type `apps/web/src/core/lib/desktop.ts`; shim `scripts/gen-desk-shim.mjs` |
| Message WS client -> server / event server -> client | `packages/shared/src/types.ts::ClientMessage` / `::MonitorEvent`, `apps/core/crates/core-serve/src/dispatch.rs` |
| Thêm tool MCP `orrery` | `apps/core/crates/core-desk/src/mcp/tools.json`, handler `mcp/t_*.rs` (vd `t_todo.rs`), vỏ `apps/shell/src/mcp.rs::start` |
| Deep link, cửa sổ chính, thu nhỏ | `apps/shell/src/links.rs::handle`, `apps/shell/src/window.rs::create_main`, `window.rs::minimized_size` |
| Env / cổng / app key | `apps/core/crates/core-serve/src/config.rs::ServeConfig`, `apps/shell/src/config.rs::ShellConfig`, `docs/env.md` |
| Child webview Browser pane, keychain | `apps/shell/src/guest.rs`, `apps/shell/src/vault.rs`, `apps/core/crates/core-desk/src/keychain.rs` |
| Cập nhật app, đóng gói | `apps/shell/src/update_host.rs`, `scripts/dist-tauri.mjs`, `apps/shell/tauri.conf.json` |

## Terminal, quan sát, toast, tray

| Việc | Bắt đầu ở |
|---|---|
| Spawn pane terminal | `apps/shell/src/pty.rs`, `apps/shell/src/presets.rs`, `apps/core/crates/core-pty/src/hub/spawn.rs` |
| Viewer / owner input, claim | `apps/core/crates/core-pty/src/hub/viewers.rs` |
| Lệnh shell xong / lỗi (OSC 633) | `apps/core/crates/core-pty/src/marks.rs::OscScanner`, `apps/web/src/features/shellDone/useShellDone.ts` |
| Dev server URL -> preview, chip header | `apps/web/src/features/workspace/lib/devServerUrl.ts`, `lib/devChipGate.ts` |
| Lịch sử lệnh (Ctrl+R), mốc terminal | `apps/core/crates/core-desk/src/cmdhist.rs`, `apps/web/src/features/workspace/lib/cmdHistory.ts`, `lib/termBookmarks.ts` |
| Nhận diện session, transcript, usage | `apps/core/crates/core-monitor/src/monitor/reconcile.rs`, `core-transcript/src/tracker/mod.rs`, `core-monitor/src/usage.rs` |
| Quy tắc toast (waiting, done, stuck) | `apps/core/crates/core-toast/src/toaster/observe.rs::observe` |
| Tray: danh sách chờ bạn | `apps/shell/src/tray_waiting.rs`, `apps/web/src/features/notify/trayWaiting.ts` |
| Thông báo theo mức từng project | `apps/web/src/features/notify/projectLevel.ts`, `stores/projectNotify.ts` |
| Mask secret, process table | `apps/core/crates/core-transcript/src/mask.rs`, `apps/core/crates/core-procs/src/lib.rs` |

## Web

| Việc | Bắt đầu ở |
|---|---|
| State pane / tab / split | `apps/web/src/features/workspace/stores/workspace.ts::useWorkspaceStore` |
| Terminal pane (xterm) | `apps/web/src/features/workspace/components/TermPane.vue`, `lib/ptyClient.ts` |
| Footer chip, checklist cài đặt | `features/workspace/lib/footerChips.ts::planFooterChips`, `features/settings/lib/setupChecklist.ts` |
| Header pane, tín hiệu tab | `features/workspace/lib/paneHeadActions.ts`, `lib/tabSignals.ts` |
| Launcher, mẫu khởi chạy | `features/workspace/lib/launchTemplates.ts`, `components/LaunchPicker.vue` |
| Pick phần tử preview | `apps/web/src/core/lib/preview.ts`, `features/agent-tools/lib/elementCrop.ts` |
| Gửi đoạn code đang chọn cho agent | `features/files/lib/sendSelection.ts`, `components/CodeEditor.vue` |
| Rail lọc session, menu Nhóm | `features/next/components/rail/LiveRail.vue`, `features/next/lib/railLive.ts` |
| Empty state (Inbox, Board, Changes) | `apps/web/src/core/components/EmptyState.vue` |
| Kết nối WS, event, Settings | `apps/web/src/app/stores/connection.ts::useConnectionStore`, `core/stores/settings.ts`, `features/settings/components/index.ts` |
| Theme, token màu / shape | `apps/web/src/core/lib/themes.ts::startTheme`, `apps/web/src/style.css`, `apps/web/src/themes.css` |
| Chuỗi i18n | `apps/web/src/i18n/en.ts`, `apps/web/src/i18n/vi.ts` (hoặc `i18n/` của feature) |
| Menu Claude / Codex trên terminal | `features/click-menu/lib/termMenu.ts::parseTermMenu` |
| Ctrl+K, Agent Map, UI next | `features/palette/components/CommandPalette.vue`, `features/map/components/AgentMap.vue`, `features/next/components/NextShell.vue` |
| Dời file web, sửa import | `scripts/web-move.mjs`; luật tầng `apps/web/src/importRules.test.ts` |

## Git, PR, worktree, chat

| Việc | Bắt đầu ở |
|---|---|
| Diff / Changes / Git Log | `features/git/components/ChangesPane.vue`, `features/git/components/log/LogPane.vue` |
| Git (changes, log, commit, branch) | `apps/core/crates/core-desk/src/preview/ipc_git.rs`; gate `preview/gate.rs`; chặn token `preview/secrets.rs::scan_staged` |
| PR / CI / merge trong Git Log | `core-desk/src/github/` (`ipc.rs`, `pulls.rs`, `ci_watch.rs`, `api.rs`); `features/git/components/PrPanel.vue`, `lib/prView.ts`, `stores/ci.ts` |
| CI xong gửi Telegram | `apps/core/crates/core-desk/src/telegram/ci.rs` |
| Stash, hoàn tác commit cuối | `core-desk/src/preview/stash.rs`, `features/git/lib/stash.ts`, `components/StashDialog.vue`, `log/StashMenu.vue` |
| Worktree setup / dọn sau merge | `core-desk/src/preview/wtconfig.rs`, `features/git/lib/worktreeFlow.ts::cleanupWorktree`, `lib/askCleanup.ts` |
| Checklist trước merge, todo MCP | `core-desk/src/preview/checklist.rs`, `mcp/t_todo.rs`, `features/git/components/WorktreeChecklist.vue`, `lib/checklist.ts::runCheck` |
| Commit -> session (chip Git Log) | `features/chat/commitSessions.ts`, `features/chat/CommitChips.vue`, `core-transcript/src/chat/edits.rs` |
| Xuất session ra Markdown | `core-transcript/src/chat/export.rs::export`, `features/chat/export.ts::runExport`, `ChatExportMenu.vue` |

## Desk (Rust) theo miền

| Việc | Bắt đầu ở |
|---|---|
| Run Android / Flutter, lịch sử log Run | `core-desk/src/run/manager.rs`, `run/history.rs::RunHistory`, `apps/web/src/features/run/components/RunHistoryDialog.vue` |
| XML Layout preview | `core-desk/src/layout/daemon.rs`, `apps/layout-daemon/README.md` |
| adb / thiết bị | `core-desk/src/devices/manager.rs::DeviceManager` |
| Telegram nhận lệnh | `core-desk/src/telegram/inbound.rs` |
| Quét ổ, index, xoá an toàn (Disk) | `core-disk/src/walk/mod.rs`, `apps/shell/src/disk_guard.rs` |

## Remote điện thoại

| Việc | Bắt đầu ở |
|---|---|
| Gateway desktop, push | `apps/shell/src/remote/mod.rs`, `remote/push.rs`, `remote/webpush.rs` |
| Giao thức Noise | `apps/core/crates/core-remote-proto/src/lib.rs` |
| PWA, relay, app Flutter | `apps/pwa/src/remote/session.ts`, `apps/relay/src/room.ts`, `apps/mobile/.code_index/mobile.md` |

## Test, QA, tài liệu

| Việc | Bắt đầu ở |
|---|---|
| Gate nhanh theo diff | `scripts/qa-fast.mjs` (`npm run qa`), map `E2E_MAP` trong file đó |
| Thêm / chạy E2E | `apps/shell/e2e/cdp.cjs` (hạ tầng), `apps/shell/e2e/run.cjs` (chạy hết) |
| Gate điện thoại, fixture giả | `scripts/qa-remote.mjs`, `scripts/demo-fixture.mjs` |
| Nguồn dữ liệu, ngoại lệ đã duyệt | `docs/claude-code-integration.md`, `docs/codex-integration.md`, `docs/core-principle.md` |
