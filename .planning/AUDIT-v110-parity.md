# AUDIT v110 P0: parity Electron + collector Node → Tauri + Rust

Gói P0 của `.planning/PLAN-v110-modules.md`, 2026-11-22, chỉ đọc. Kế thừa `.planning/AUDIT-v104-tauri-parity.md` (bảng 44 dòng theo tính năng, WP-104k) và kiểm lại bằng grep thật trên nhánh `feat/v98-pwa`. Bản này thêm: phía collector, các hook `Deps` của core-serve, phụ thuộc build / script / CI còn trỏ vào `apps/desktop`, ước lượng dòng bỏ được.

Kết luận nhanh: 403 / 403 kênh `ccm:*` trong `apps/desktop/src/preload.ts` đều có trong Rust (`apps/shell/src` + `apps/core/crates`, đếm bằng `rg -o "ccm:[a-z0-9:_.-]+"` rồi `comm`). Lúc chạy Tauri không gọi Node. Phần thiếu thật còn 6 mục (cuối file).

Trạng thái: **có** (Rust làm đủ) / **khác** (làm khác có chủ ý) / **thiếu** (Electron hoặc collector có, Tauri chưa có) / **Office** (chỉ còn dùng khi `CCM_CORE=node` cho Office) / **chết** (chỉ phục vụ đường Node hoặc Electron, xoá được ngay).

## 1. Electron main `apps/desktop/src/**` (55 022 dòng TS, bỏ test)

| Module (dòng) | Rust (crate / file) | Trạng thái | Ghi chú |
|---|---|---|---|
| `main.ts` (2872), `pages.ts`, `appearance.ts`, `layouts.ts`, `editors.ts`, `shortcuts.ts`, `pick-start.ts` | `apps/shell/src/{main,window,tray,lifecycle,appearance,config}.rs`; core-desk `appearance.rs`, `layouts.rs`, `editors.rs`, `pages.rs` | có / khác | Không có trang intro (dialog `fail()`); `shortcuts.ts` dọn `.lnk` của bản NSIS Electron → chết |
| `preload.ts` (510) | `apps/shell/src/shim.js` sinh bởi `scripts/gen-desk-shim.mjs` | có, **nguồn build** | Phải dời trước khi xoá (mục 3) |
| `collector-host.ts`, `core-serve-host.ts`, `coreProcsEnv.ts`, `host-rpc.ts` | `apps/shell/src/{serve,stdio,collector}.rs` chạy `core_serve::run` nhúng | khác | Host RPC tắt trên Tauri (`CCM_HOST_RPC` off) → xem thiếu #1 |
| `strings.ts` (639) | `strings.rs`, `desk_text.rs`, `preview_text.rs`, `host_extra_text.rs`, `disk_*_text.rs` | có | |
| `openLink.ts`, `orreryApi.ts` | `links.rs`, `ccm.rs` (`ccm:orrery-reply`), `mcp.rs`, `host_telegram.rs` (`shell.ask … "ccm:orrery"`) | có | |
| `mcpDesk.ts`, `mcpDeskHosts.ts` | core-desk `mcp_desk/*` | có | |
| `pty/` (4246: host, broker, supervisor, presets, shellInit, shim, git, owners, mcp, mcpTools/) | core-pty (`orrery-core` pty), `apps/shell/src/{pty,presets,shim,pty_branch,worktree,wtcopy,mcp}.rs`, core-desk `mcp/*` | có / khác | Thiếu OSC 633 + bash `--init-file` (thiếu #3). `pty/mcp.ts` + `pty/mcpTools/*` (1 900 dòng) là **nguồn** của `core-desk/src/mcp/tools.json` qua `scripts/gen-mcp-tools.mjs` |
| `preview/` (6521) | core-desk `preview/*` (36 file), `apps/shell/src/preview_{host,ipc,serve,watch,access,grants,files}.rs`, `files_ipc.rs` | có | |
| `run/` (5566) | core-desk `run/{android,flutter}/*`, `apps/shell/src/host_run.rs` | có | |
| `telegram/` (4318) | core-desk `telegram/*` (+ `mt/` MTProto Rust), `host_telegram.rs`, `episodes.rs` | khác | `bigUpload` / `user` trả failed (v104 dòng 24) |
| `devices/` (3817), `apk/`, `capture/`, `phone/` | core-desk `devices/*`, `apk/*`, `capture/*`, `phone/*`; `host_devices.rs`, `host_extra.rs`, `capture.rs`, `shot.rs` | có / khác | |
| `layout/` (3779) | core-desk `layout/*`, `host_run.rs` `layout.*`, `resources.rs` `apply_layout_env` | có | |
| `flutterPreview/` (1326) | core-desk `flutter_preview/*` | có | |
| `extensions/` (1046) | core-desk `extensions/*`, `host_extra.rs` | có | |
| `github/` (360) | core-desk `github/*`, `vault.rs`, `legacy.rs` | khác | Keyring AES-GCM, đọc được token Electron cũ |
| `diskMap/`, `diskClean/`, `diskApps/`, `diskMove/`, `diskRam/`, `organize/`, `disk/` (8 477) | crate core-disk; `apps/shell/src/disk_{map,clean,clean_dev,clean_sys,apps,apps_left,apps_reg,move,move_tree,ram,organize*,snap,snap_report,guard,host,agent}.rs`; core-desk `disk/` | có | |
| `buildClean/`, `libs/`, `releaseCheck/`, `agentData/`, `agentStatus/`, `resources/`, `power/`, `backup/`, `restore/`, `update/`, `deepseek/`, `claudeHooks/` | core-desk cùng tên (`build_clean`, `libs`, `release_check`, `agent_data`, `agent_status`, `resources.rs`, `power`, `backup`, `restore.rs`, `update`, `deepseek`, `claude_hooks`); `power.rs`, `update_host.rs`, `hosts.rs` | có | DeepSeek tạm dừng ở cả hai vỏ |
| `bridge.ts`, `bridgeRust.ts`, `statusline.ts`, `hooks.ts`, `claudeSettings.ts` | core-desk `bridge/{install,settings}.rs`, crate core-bridge (`orrery-bridge`), `lifecycle.rs` `migrate_bridges` | có | |
| `serve-toast.ts`, `menuToast.ts` | `toast.rs`, `menu_toast.rs`, `mac_note.rs` | có (P0.5: macOS toast menu có nút qua `tauri-plugin-notification`) | v104 dòng 33, 34 |
| `attention.ts` (dock badge / bounce) | `apps/shell/src/badge.rs` (badge, F2) + `attention.rs` (bounce Critical, P0.5) | có | |
| `mac.ts` (menu app, PATH login shell, Move to Applications) | `apps/shell/src/{mac_env,mac_menu,mac_move}.rs` (P0.5) | có (chưa compile trên Mac) | thiếu #2 đã làm |
| `bugReport.ts`, `capture.ts`, `webstorageExport.ts` | `bug_report.rs`, `capture.rs`, `webstorage.rs` | có / khác | Log chỉ `shell.log` + `pty.log` |
| `desk/` (forward, host) | `apps/shell/src/{desk,hosts}.rs` | có | |
| `types/` | | chết | |

## 2. Collector Node `apps/collector/src/**` (14 680 dòng TS)

| Module | Rust | Trạng thái |
|---|---|---|
| `index.ts`, `server.ts`, `store.ts`, `config.ts`, `datadir.ts`, `json-file.ts` | core-serve (`lib.rs`, `runtime.rs`, `dispatch.rs`, `config.rs`), core-http (`server.rs`, `ws.rs`, `cookie.rs`, `csp.rs`), core-store (`store/`, `persist/datadir.rs`, `persist/json_file.rs`) | có |
| `monitor.ts` (39 K), `registry.ts`, `repo.ts`, `peers.ts`, `gitstat.ts`, `jobs.ts`, `job-stop.ts`, `model-catalog.ts`, `tool-spans.ts`, `spanMirror.ts` | core-monitor (`monitor/*`, `registry.rs`, `repo.rs`, `peers.rs`, `gitstat.rs`, `jobs.rs`, `job_stop.rs`, `catalog.rs`, `spans.rs`) | có |
| `tracker.ts`, `trackerView.ts`, `transcript.ts`, `tailer.ts`, `usage.ts` | core-transcript (`tracker/*`, `extract.rs`, `tail.rs`, `index.rs`), core-monitor `tracker.rs`, `usage.rs` | có |
| `aliases.ts`, `favorites.ts`, `history.ts`, `snooze.ts`, `forecast.ts`, `response.ts`, `daystart.ts`, `timeline.ts` (status heatmap) | core-store `persist/{aliases,favorites,history,snooze,forecast,response,daystart,timeline}.rs` | có |
| `hooks.ts`, `statusline.ts` (đọc side file) | core-transcript `sidefiles/*`, core-monitor `sidefiles.rs` | có |
| `process.ts`, `win-procs.ts`, `ps-host.ts` | core-procs, core-monitor `process.rs` | có (`ps-host.ts`, `win-procs.ts` chết) |
| `resources/`, `sensitive/`, `prompt-search.ts` | core-sources `resources/*`, `sensitive/*`, `prompt_search/*` | có |
| `codex/`, `deepseek/` | core-monitor `codex/*`, `deepseek/*`; core-transcript `codex/*`, `sidefiles/outbox.rs` | có |
| `toast.ts` (37.8 K), `toast-electron.ts` | core-toast + `apps/shell/src/toast.rs` | có (Windows) |
| `coreProcs.ts`, `coreTracker.ts`, `coreTranscript.ts`, `coreSideFiles.ts`, `coreFallback.ts`, `nodeSideFiles.ts`, `procSource.ts`, `sideFileSource.ts`, `trackerSource.ts`, `usageSource.ts` | | chết (cầu Node → `orrery-core` cho các cờ `CCM_CORE_*`) |
| `focus.ts`, `focus-mac.ts`, `osascript.ts`, `launcher.ts`, `opener.ts`, `folder-pick.ts`, `win-exe.ts` | core-monitor `native/` `NativeDeps` (P0.5) dùng khi không có host RPC; picker = Tauri dialog (`apps/shell/src/folder_pick.rs`) | có (thiếu #1 đã làm) |
| `office/` (hub, model, runtime, locale), timelapse `timeline/<day>.jsonl` | `core-serve/src/dispatch.rs:105` in "Office needs CCM_CORE=node" | **Office** |
| `claude-settings.ts` (đọc `model`) | core-monitor `catalog.rs` / `shared.rs` | có |

## 3. Tauri còn lấy gì từ `apps/desktop` (phải dời trước khi xoá)

| File | Ai đọc | Chỗ mới đề xuất |
|---|---|---|
| `apps/desktop/package.json` (`version` 4.0.7) | `apps/shell/build.rs:12`, `apps/shell/src/config.rs:189` (test `include_str!`), `apps/shell/tauri.conf.json:4`, `apps/web/vite.config.ts:7` (`__APP_VERSION__`), `apps/web/src/lib/changelog.test.ts:2`, `scripts/dist-tauri.mjs:86`, `scripts/measure.mjs:91`, `scripts/perf-bench.mjs:667`, `apps/shell/e2e/{v97-remote,v97-device}.cjs`, mọi `apps/desktop/e2e/*.cjs` (`require('../package.json')`, 20 file), `.github/workflows/desktop.yml:43` | `apps/shell/package.json` (workspace `@ccm/shell`, chỉ `version` + script `e2e`) hoặc `version` ở root `package.json`; đề xuất `apps/shell/package.json` để `tauri.conf.json` trỏ `./package.json` |
| `apps/desktop/src/preload.ts` | `scripts/gen-desk-shim.mjs:7` | `apps/shell/shim/preload.ts` (chỉ import `electron` giả do esbuild thay) |
| `apps/desktop/src/pty/mcp.ts`, `pty/mcpTools/*`, `pty/mcpRust.ts` (type) | `scripts/gen-mcp-tools.mjs:10-11` → `core-desk/src/mcp/tools.json` | Cách 1 (gọn): coi `tools.json` là nguồn, bỏ `gen-mcp-tools.mjs`. Cách 2: dời phần khai báo tool sang `apps/shell/shim/mcpTools/` (bỏ phần handler Node) |
| `apps/desktop/daemon-kit.mjs` | `scripts/dist-tauri.mjs:6`, `scripts/kit-release.mjs:6`, `.github/workflows/layout-daemon.yml:16` | `scripts/lib/daemon-kit.mjs` |
| `apps/desktop/test/fixtures/pty-control.json` | `apps/core/crates/core-pty/src/control.rs:200` | `apps/core/crates/core-pty/tests/fixtures/` |
| `apps/desktop/test/fixtures/fake-layout-daemon.cjs` | `apps/core/crates/core-desk/tests/layout.rs:24` | `apps/core/crates/core-desk/tests/fixtures/` |
| `apps/desktop/e2e/*.cjs` (100 file, 18 067 dòng) + `e2e/fixtures/` + `cdp.cjs` + `run.cjs` | Tauri E2E mặc định (`run.cjs` chạy `apps/shell/target/debug/orrery-shell`), `apps/shell/e2e/{t1,v97-remote,v97-device}.cjs` require `apps/desktop/e2e/cdp.cjs`, `scripts/qa-fast.mjs:85` | `apps/shell/e2e/` (gộp với 3 file đang có); bỏ 4 file chỉ Electron `v74`, `v75`, `v76b`, `v77a` (+ nhánh `--shell=electron` trong `cdp.cjs`, `run.cjs`) sau khi chủ nhân duyệt |
| Test vitest `apps/desktop/test/*.twin.test.ts`, `codex-parity*` | so TS với Rust | Bỏ cùng TS (Rust đã có test riêng) |

## 4. Script / env / docs / CLAUDE.md phải sửa khi xoá

- `package.json` root: `dev` (đang chạy `@ccm/collector --dev`) → `cargo run -p core-bin -- serve --dev` hoặc `orrery-core serve --dev`; bỏ `desktop`, `desktop:dev`, `dist:electron*`; `shell:dev` giữ; `workspaces` bỏ `apps/desktop`, `apps/collector`; `allowScripts` bỏ `electron-winstaller`.
- `scripts/dist.mjs` (nhánh `--electron`), `scripts/qa-fast.mjs` (`WS` bỏ 2 workspace, `e2eFile` chỉ `apps/shell/e2e`, cờ `--electron`, regex dòng 81 `apps/desktop/e2e`), `scripts/demo-fixture.mjs:233`, `scripts/perf-bench.mjs` (`profileCollector`, `roleOf` theo Electron), `scripts/measure.mjs` (`apps/desktop/stage`), `scripts/trace-capture.mjs`, `scripts/trace-lockstats.mjs`, `scripts/trace-store-hook.mjs`, `scripts/usage-corpus.mjs` (đều chạy `apps/collector/src/index.ts`: bỏ hoặc chuyển sang `orrery-core serve`), `scripts/hook-bridge.mjs` + `scripts/statusline-bridge.mjs` (bản Node cũ, `orrery-bridge` thay; Rust chỉ nhận chuỗi `node "...mjs"` để migrate, không đọc file).
- `.github/workflows/desktop.yml` (input `electron`, `electron_files`, node-pty mac), `test.yml:38` (comment `layout-daemon.int.test.ts` thuộc `apps/desktop/test`), `layout-daemon.yml:16`.
- Env bỏ: `CCM_CORE`, `CCM_CORE_PTY`, `CCM_CORE_PROCS`, `CCM_CORE_USAGE`, `CCM_CORE_TRANSCRIPT`, `CCM_CORE_CODEX`, `CCM_CORE_SIDEFILES` (giá trị `node`), `CCM_TOAST_BACKEND=electron`, `CCM_ELECTRON_USER_DATA` (`cdp.cjs:83`), `CCM_E2E_SHELL`. Giữ `CCM_CORE_BIN`, `CCM_SERVE_STATS`.
- `CLAUDE.md`: mục Commands (`npm run dev`, `npm test -w @ccm/collector`, `desktop`, `desktop:dev`, `dist:win` / `dist:mac` mô tả Electron, `npm run e2e -w @ccm/desktop`), dòng env dài (dòng 26), Architecture (`apps/collector`, `apps/desktop`), mọi câu "Office needs `node`". Xem đề xuất gọn ở `.planning/AUDIT-v110-index.md` mục 4.
- `docs/claude-code-integration.md` (226 K; §11j shell, §11k macOS, §323 focus, §331 opener nói tới `apps/collector/src/*.ts`), `docs/codex-integration.md`, `docs/deepseek-integration.md`: đổi đường dẫn TS sang Rust.
- `.code_index/collector.md`, `desktop*.md` (4 file, 139 KB) → `archive/`.
- Comment trỏ TS: `apps/web/src/lib/projects.ts:4,24`, `lib/preview.ts:40,78,90,95`, `lib/changelog.ts:13`, `apps/layout-daemon/src/test/.../RealProjectTest.kt:46`, `packages/shared/src/types.ts:512` (không sửa comment cũ theo rule; chỉ ghi để biết đường dẫn sẽ chết).
- `packages/shared`: phần chỉ collector / desktop dùng (`office` hub types, `mcpDesk`, `orreryApi` phía main) kiểm lại sau khi xoá; Rust test đọc `packages/shared/src/*` (`core-store/tests/shared_ts.rs`, `core-transcript/tests/tool_summary.rs`) nên giữ package.

## 5. Ước lượng dòng TS bỏ được

| Phần | Dòng |
|---|---|
| `apps/desktop/src` (trừ `preload.ts` 510 và `pty/mcp*.ts` + `mcpTools/` khoảng 1 900 nếu giữ cách 2) | khoảng 52 600 (55 022 nếu chọn cách 1) |
| `apps/collector/src` | 14 680 |
| Test vitest `apps/desktop/test` (200 file) + `apps/collector/test` (62 file) | 38 839 + 12 267 = 51 106 |
| E2E chỉ Electron (`v74`, `v75`, `v76b`, `v77a`, `desktop-only`, `core-procs`) | khoảng 1 760 |
| `apps/desktop/{build.mjs,after-pack.cjs}` + `electron-builder.yml` | khoảng 180 |
| **Tổng** | khoảng **67 000 dòng source + 52 800 dòng test** (TS 56 % → khoảng 45 % toàn repo) |

Dependency npm bỏ: `electron` 44, `electron-builder`, `@lydell/node-pty`, `sql.js`, `telegram` (gramjs) (`apps/desktop/package.json`).

## 6. Thiếu, cần làm (hoặc chủ nhân duyệt bỏ) trước khi xoá

1. **[ĐÃ LÀM P0.5, 2026-11-22]** `core-monitor/src/native/` `NativeDeps` + `apps/shell/src/folder_pick.rs`, E2E `v110-external-open`. **Mở / focus ngoài app trên Tauri**: `terminal.focusWindow` (focus cửa sổ terminal ngoài), `terminal.open` / `favorite.open` (mở thư mục bằng VS Code / Explorer / terminal ngoài), `folder.pick` qua WS. Trên Tauri đều trả `unsupported` vì `SystemDeps` không cài 4 hook (`core-monitor/src/monitor/deps.rs:24-35`). Web vẫn gọi: `stores/connection.ts:525,552,565,588`, `useAgentActions.ts:91`, `CompactView.vue:107`, `FavoritesList.vue`. Cách làm: cài `NativeDeps` trong vỏ (đã có `editors.rs`, dialog Tauri `blocking_pick_folder`) truyền vào `core_serve` thay `SystemDeps`; port `focus.ts` (PowerShell `SetForegroundWindow`) + `focus-mac.ts` (osascript), `launcher.ts`, `opener.ts`. Hoặc duyệt bỏ và ẩn nút trên Tauri.
2. **[ĐÃ LÀM P0.5]** `mac_env.rs`, `mac_menu.rs`, `mac_move.rs` (unit test chạy trên Windows, chưa compile trên Mac). **macOS `mac.ts`**: PATH của login shell khi mở từ Finder / Dock (thiếu thì `claude`, `git`, `node` có thể không tìm thấy trong pane và trong run), menu app, Move to Applications.
3. **OSC 633 + bash `--init-file`** (shell marks v73, `run.cjs` bước bị tắt trên Tauri).
4. **[ĐÃ LÀM P0.5]** `attention.rs` (shell đặt `CCM_TOAST_BACKEND=electron` trên Mac). **Dock bounce / badge macOS**: `badge.rs` (v109) có badge, chưa có bounce `request_user_attention`; kiểm lại khi v109 xong.
5. **[ĐÃ LÀM P0.5]** `mac_note.rs` qua `tauri-plugin-notification` 2.8.1 (2+ nút gom vào menu "Options" của macOS, không xoá được thông báo đã hiện). **Toast menu có nút trên macOS** (`menu_toast.rs` mới có Windows).
6. **Office**: chỉ chạy với collector Node. Office tạm dừng từ 2026-10-28 → đề xuất duyệt bỏ timelapse phía server khi xoá collector; P2 vẫn tách module web. Bật lại sau này thì port `office/` sang core-monitor.

Ngoài ra (không chặn xoá): dời 2 fixture + `preload.ts` + `daemon-kit.mjs` + version (mục 3), chọn cách xử lý `gen-mcp-tools.mjs`, bỏ 4 E2E chỉ Electron.
