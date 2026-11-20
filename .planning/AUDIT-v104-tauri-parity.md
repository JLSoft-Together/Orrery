# AUDIT v104: Tauri (`apps/shell`) so với Electron (`apps/desktop/src`)

Gói WP-104k, 2026-11-20. Nguồn: `.code_index/desktop*.md`, `.code_index/shell.md`, `.planning/HANDOFF-2026-11-11-1017-tauri-gaps.md`, grep thật trong `apps/shell/src` + `apps/core/crates`. Chủ nhân chốt: v104 chỉ kiểm + E2E mặc định Tauri, KHÔNG xoá Electron (xoá ở WP-104l sau khi checklist Mac đạt).

Cột Trạng thái: **đủ** (Tauri làm đủ như Electron) / **khác** (làm khác có chủ ý, hành vi gần như nhau) / **thiếu** (Electron có, Tauri chưa có).
Cột "Rust còn gọi TS?": Rust có phải chạy hoặc đọc file TS lúc chạy (runtime), lúc build hoặc trong test không. Không dòng nào ở runtime: vỏ Tauri không có Node (core-serve, core-desk, core-pty, core-toast link thẳng hoặc chạy `orrery-core`).
Cột "macOS đã test?": `cargo test` + `cargo build` shell trên Mac chạy 2026-11-11 (189 test pass, `QA-v87-mac-result.md` cho phần core Rust); WKWebView không có CDP nên `cdp.cjs` chưa chạy được trên Mac, mọi mục "chưa chạy thật" nằm trong `.planning/QA-mac-checklist.md` mục Tauri.

## 1. Bảng tính năng

| # | Tính năng | File Electron (`apps/desktop/src/`) | File Tauri (`apps/shell/src/`) | Trạng thái | Rust còn gọi TS? | macOS đã test? |
|---|---|---|---|---|---|---|
| 1 | Khởi động, single-instance, data dir, key app | `main.ts`, `collector-host.ts`, `core-serve-host.ts` | `main.rs`, `config.rs`, `serve.rs`, `collector.rs`, `stdio.rs`, `log.rs` | đủ | Không | build + cargo test; chạy thật chưa |
| 2 | Collector Node + fallback `CCM_CORE*=node` | `collector-host.ts`, `core-serve-host.ts`, `coreProcsEnv.ts`, `pty/supervisor.ts` | không có (cố ý: `core_serve::run` chạy thẳng trong process, không fallback Node) | khác | Không | n/a |
| 3 | Cửa sổ chính, bounds, zoom, nền, frameless, nút cửa sổ tự vẽ | `main.ts` `baseWindow`, `appearance.ts` | `window.rs`, `header.js`, `appearance.rs`, `ccm.rs` (`shell:window`, `ccm:zoom`, `ccm:appearance`) | đủ | Không | chưa chạy thật (traffic lights, kéo header) |
| 4 | Cửa sổ compact, tray, chạy cùng Windows | `main.ts` | `window.rs`, `tray.rs` | đủ | Không | chưa chạy thật |
| 5 | Trang khởi động / lỗi (golem intro, retry) | `pages.ts` | `main.rs` `fail()` hộp thoại lỗi, không có trang intro | khác | Không | n/a |
| 6 | Chuỗi native en / vi | `strings.ts` | `strings.rs`, `desk_text.rs`, `preview_text.rs`, `host_extra_text.rs`, `disk_*_text.rs` | đủ | Không | không cần |
| 7 | Preload `window.ccmDesktop` | `preload.ts` | `shim.js` sinh bởi `scripts/gen-desk-shim.mjs` (bundle `preload.ts`), `ccm.rs`, `shim.rs` | đủ | Có, lúc build: `preload.ts` là nguồn của `shim.js` | cargo test có `--check` shim khớp |
| 8 | Deep link `agent-monitor://` (open / focus / snooze) | `openLink.ts`, `main.ts` `registerLinks` | `links.rs`, plugin deep-link + single-instance | đủ | Không | chưa chạy thật (`open-url` macOS) |
| 9 | Status line bridge + hook bridge | `bridge.ts`, `bridgeRust.ts`, `statusline.ts`, `hooks.ts`, `claudeSettings.ts`, `claudeHooks/` | `lifecycle.rs` `migrate_bridges`, `host_extra.rs` (`statusline.*`, `hooks.*`, `claudeHooks.*`), core-desk `bridge/`, `orrery-bridge` | đủ | Không (chuỗi `node "...mjs"` chỉ để nhận ra lệnh cũ khi migrate) | chưa chạy thật |
| 10 | Terminal PTY host + broker | `pty/host.ts`, `pty/broker.ts`, `pty/supervisor.ts`, `pty/owners.ts` | `pty.rs` (spawn `orrery-core` pty), `presets.rs`, `shim.rs`, `pty_branch.rs`, `pty_*` control | đủ | Không | core-pty E2E mac chạy với Electron; Tauri chưa chạy thật |
| 11 | Preset shell, shim `claude` / `codex`, branch pane | `pty/presets.ts`, `pty/shellInit.ts`, `pty/git.ts` | `presets.rs`, `shim.rs`, `pty_branch.rs` | khác: thiếu OSC 633 shell integration (nonce) và bash `--init-file` re-prepend shim | Không | chưa |
| 12 | Worktree pane | `pty/git.ts`, `pty/broker.ts` `answerWorktree` | `worktree.rs`, `wtcopy.rs` | đủ | Không | chưa |
| 13 | MCP `orrery` (19 tool) | `pty/mcp.ts`, `pty/mcpRust.ts`, `mcpDesk.ts`, `mcpDeskHosts.ts`, `orreryApi.ts` | `mcp.rs` (`McpServer` + `PaneHost` trong process) | đủ (DeepSeek luôn tắt: `hand_back` / `open_session deepseek` không có) | Không | chưa |
| 14 | Cổng thư mục, grants, full access | `preview/ipc.ts`, `preview/grants.ts` | `preview_host.rs`, `preview_ipc.rs`, `preview_access.rs`, `preview_grants.rs` | đủ | Không | cargo test; symlink `/tmp` mac đã sửa ở phía core |
| 15 | Preview server, file channel (`preview-url`, `open-path`, `reveal`, pick) | `preview/server.ts`, `preview/files.ts` | `preview_serve.rs`, `files_ipc.rs`, `preview_files.rs` | đủ | Không | chưa |
| 16 | Explorer, watcher, Changes, Git Log, branches, push | `preview/explorer.ts`, `changes.ts`, `log.ts`, `branches.ts`, `watch.ts`, `gitExec.ts`, `listFiles.ts` | `preview_watch.rs`, core-desk `preview/*` (qua host `preview.*`) | đủ | Không | chưa |
| 17 | Ghi file, editor dirty, auto save | `preview/fileWrite.ts`, `editors.ts` | core-desk `preview/file_write`, trang gọi `ccm:file-write` | đủ | Không | chưa |
| 18 | Run Android / Flutter | `run/*` | `host_run.rs` + core-desk `run` | đủ | Không (spawn `adb`, `gradlew`, `flutter` ngoài, không TS) | chưa |
| 19 | XML Layout Preview | `layout/*` | `host_run.rs` `layout.confirm` / `layout.urlFor`, `resources.rs` `apply_layout_env`, core-desk `layout`, daemon Kotlin | đủ | Không (JVM daemon, không TS); test core-desk `tests/layout.rs` dùng `apps/desktop/test/fixtures/fake-layout-daemon.cjs` | chưa |
| 20 | Flutter widget preview | `flutterPreview/*` | core-desk `fwp` | đủ | Không | chưa |
| 21 | Extensions (MCP / plugin) | `extensions/*` | `host_extra.rs` `extensions.confirm` + core-desk `extensions` | đủ | Không | chưa |
| 22 | GitHub accounts + keychain | `github/*`, `safeStorage` | `vault.rs`, `legacy.rs`, core-desk `github` | khác: master key AES-256-GCM trong keyring, đọc được token Electron `v10` cũ | Không | chưa (Keychain `Orrery Safe Storage`) |
| 23 | Devices adb, devfs, apk, release, capture, phone | `devices/*`, `apk/`, `capture/`, `phone/` | `host_devices.rs`, `host_extra.rs` (`phone.*`) + core-desk | khác: `device.used` không kèm tên AVD, `ccm:devfs-progress` gửi mọi cửa sổ, confirm luôn kiểu Warning | Không | chưa |
| 24 | Telegram, episode, tick | `telegram/*`, `phone/` | `host_telegram.rs`, `episodes.rs`, `lifecycle.rs` | khác: `bigUpload` / `user` trả failed (Tauri dùng MTProto Rust) | Không | chưa |
| 25 | Backup, restore, vòng đời run | `backup/*`, `restore/*` | `desk.rs`, `hosts.rs`, `host_extra.rs`, `lifecycle.rs` | đủ | Không | chưa |
| 26 | Update (NSIS / dmg) | `update/*` | `update_host.rs` | đủ | Không | chưa (`hdiutil`, `mac_script`) |
| 27 | Disk manager (v101) | `disk*/`, `organize/`, `buildClean/`, `libs/` | `disk_*.rs`, `hosts.rs` | đủ | Không | chưa (RAM, quét `/Volumes`) |
| 28 | Resources, power, agent data, agent status, release check | `resources/`, `power/`, `agentData/`, `agentStatus/`, `releaseCheck/` | `hosts.rs`, `resources.rs`, core-desk | đủ | Không | chưa |
| 29 | Browser pane (`<webview>`) | `main.ts` `hardenWebviews` (partition `persist:ccm-preview`) | `guest.rs`, `guest_ipc.rs`, `guest.js`, `webview.js`, `native.rs` | khác: child webview chung profile với cửa sổ chính (mất cô lập partition) | Không | chưa (chụp mac `takeSnapshot` chưa biên dịch) |
| 30 | Capture dust, ảnh clipboard, capture-guest | `capture.ts`, `main.ts` | `capture.rs`, `shot.rs`, `native.rs` | đủ | Không | chưa |
| 31 | Kéo thả đường dẫn file từ OS | `preload.ts` `webUtils.getPathForFile` | `drop_paths.rs`, `shim.js` `pathsForFiles` | khác: bất đồng bộ, `getPathForFile` đồng bộ trả `''` | Không | chưa (`NSPasteboardNameDrag`) |
| 32 | Log, báo cáo lỗi (zip) | `main.ts` `logReport`, `bugReport.ts` | `files_ipc.rs` `ccm:logs`, `bug_report.rs` | khác: log chỉ gồm `shell.log` + `pty.log` | Không | chưa |
| 33 | Toast OS Windows (chip, nút trả lời) | `serve-toast.ts` (vẽ bằng `apps/collector/src/toast.ts`) | `toast.rs` (WinRT qua crate `windows`, cùng XML với script PowerShell), `episodes.rs` thread `host-messages`; core-serve `ServeConfig.host_events` bật khi chạy nhúng (WP-104m) | đủ trên Windows (macOS: xem dòng 35) | Không (Electron vẫn gọi `collector/src/toast.ts`) | không áp dụng (toast Windows) |
| 34 | Toast menu v100 (nút trả lời khi Orrery không focus) | `menuToast.ts` | `menu_toast.rs` (`Core` port `createMenuToasts`), kênh `ccm:menu-toast-sync` (`ccm.rs`), link `agent-monitor://menu` (`links.rs` `Link::Menu`) -> `ccm:menu-toast` (WP-104m) | đủ trên Windows; khác: macOS chưa có toast menu (Electron mac dùng `Notification` có action) | Không | chưa (mục Tauri WP-104m trong checklist Mac) |
| 35 | Dock badge / bounce macOS (`CCM_TOAST_BACKEND=electron`) | `attention.ts` | không có | **thiếu** (chỉ macOS) | Không | n/a |
| 36 | Mac: menu app, PATH login shell, "Move to Applications" | `mac.ts` | không có (`tray.rs` chỉ có menu tray) | **thiếu** (chỉ macOS) | Không | n/a |
| 37 | Office timelapse (Habblaud) | collector `office/` | khoá trên Tauri (cần collector Node); Office đang tạm dừng từ 2026-10-28 | **thiếu** có chủ ý | Cần TS (collector Node) nếu bật lại | n/a |
| 38 | DeepSeek (Codewhale) | `deepseek/*` | `host_extra.rs` `deepseek.confirm`; panes DeepSeek tắt mặc định từ 2026-11-03 | khác: tính năng tạm dừng ở cả hai vỏ | Không | n/a |
| 39 | Remote v97 / v98 (điện thoại, relay, push FCM / Web Push) | không có | `remote/*` | Tauri-only | Không | chưa (mục Remote trong checklist Mac) |
| 40 | Nhập localStorage Electron lần đầu | `webstorageExport.ts` | `webstorage.rs` | đủ | Không (đọc `webstorage-export.json` hoặc leveldb) | chưa |
| 41 | Đóng gói + cài đặt | `build.mjs`, electron-builder, `scripts/dist.mjs` | `scripts/dist-tauri.mjs`, `tauri.release.json`, `nsis/hooks.nsh` | đủ | Không | chưa (`dmg`, ký ad-hoc) |
| 43 | Giữ máy thức khi agent chạy (keep awake, badge `[data-awake]`) | `power/ipc.ts` `awakeControl` nối `main.ts` (tin `busy` của collector -> `desk:awake-busy`, `powerMonitor` -> `desk:awake-power`, thoát -> `desk:awake-dispose`) | `power.rs` (tin `busy` -> `desk:awake-busy`, đọc pin mỗi 30 s -> `desk:awake-power`, `Shell::shutdown` -> `desk:awake-dispose`, dừng collector -> busy 0) + core-desk `power/` (WP-104m) | khác: đổi nguồn điện thấy sau max 30 s (Electron có event `powerMonitor`) | Không | chưa (`caffeinate`, `pmset`) |
| 44 | Deep link tới instance đang chạy (single-instance) | `main.ts` `second-instance` | `main.rs` plugin single-instance, `links.rs` | đủ trong app; E2E không kiểm được: chế độ E2E (`config::e2e_isolated`) tắt single-instance nên instance thứ 2 không chuyển link vào instance E2E (và ở máy có bản dev khác thì link tới nhầm bản đó) | Không | chưa |
| 42 | E2E driver | `e2e/cdp.cjs` (Electron `--remote-debugging-port`) | cùng `cdp.cjs`, WebView2 `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS` | đủ trên Windows (mặc định từ WP-104k); Mac không có CDP trên WKWebView | Không | không chạy được: WKWebView |

Không còn dòng "chưa rõ".

## 2. Rust còn đụng vào file TS ở đâu (cho WP-104l)

Lúc chạy: không có. Vỏ Tauri, `orrery-core`, `orrery-bridge` không gọi Node, không đọc `apps/desktop/src/**` hay `apps/collector/**`.

Lúc build / test (không được xoá khi dọn `apps/desktop`):
| File TS / JSON | Ai đọc | Ghi chú |
|---|---|---|
| `apps/desktop/package.json` | `apps/shell/build.rs`, `apps/shell/src/config.rs` `include_str!`, `apps/shell/tauri.conf.json` `version` | version app; GIỮ |
| `apps/desktop/src/preload.ts` | `scripts/gen-desk-shim.mjs` (bundle ra `apps/shell/src/shim.js`) | GIỮ, kèm mọi import của nó |
| `apps/desktop/test/fixtures/pty-control.json` | `apps/core/crates/core-pty/src/control.rs:200` `include_str!` | chuyển fixture sang `apps/core` trước khi xoá `apps/desktop/test` |
| `apps/desktop/test/fixtures/fake-layout-daemon.cjs` | `apps/core/crates/core-desk/tests/layout.rs:24` | như trên |
| `packages/shared/src/*`, `packages/shared/test/toolSummary.cases.json` | `core-store/tests/shared_ts.rs`, `core-transcript/tests/tool_summary.rs` | `packages/shared` còn; không đụng |
| `apps/layout-daemon/kit.json`, `init/ccm-project-model.gradle` | `resources.rs`, `tauri.release.json` | không đụng |

Chỉ Electron gọi TS ở runtime và sẽ mất khi xoá: `apps/collector/src/toast.ts` qua `serve-toast.ts` / `menuToast.ts` (dòng 33, 34; Tauri làm lại bằng Rust ở WP-104m), collector Node (`CCM_CORE*=node`, `CCM_CORE=node`, Office).

## 3. Việc phải làm trước WP-104l (từ các dòng "thiếu")

1. ~~Toast OS trên Tauri (dòng 33, 34)~~: xong ở WP-104m trên Windows (`toast.rs`, `menu_toast.rs`), xoá `apps/collector/src/toast.ts` không còn làm mất toast của vỏ Tauri. Còn macOS: toast menu có nút (dòng 34) và dock badge (dòng 35). Keep awake (dòng 43) cũng xong ở WP-104m. WP-104m sửa thêm lỗi dòng 3: thu nhỏ cửa sổ Windows co webview về 144x19 px làm PTY resize còn khoảng 1 cột (`window.rs` `keep_size_when_minimized`).
2. macOS (dòng 35, 36): dock badge / bounce, menu app, PATH login shell, Move to Applications chưa có; ghi vào `.planning/QA-mac-checklist.md`.
3. Chuyển 2 fixture ở mục 2 sang `apps/core` trước khi xoá `apps/desktop/test`.
4. Dòng 11: OSC 633 + bash `--init-file` (v73 bước "shell marks" tắt trên Tauri vì thiếu cái này).
5. Dòng 43: nối keep awake: sink `episodes.rs` nhận tin `busy` của core-serve (hiện ghi ra stdout) -> `desk:awake-busy`, power change -> `desk:awake-power`, `Shell::shutdown` -> `desk:awake-dispose`; core-serve phải phát `busy` khi `CCM_HOST_RPC` tắt.

## 4. E2E Tauri (WP-104k)

Kết quả từng file: `.planning/wp/v104/WP-104k-report.md`. File chỉ chạy được trên Electron (`run.cjs` `ELECTRON_ONLY`, tự đặt `CCM_E2E_SHELL=electron`): v74, v75, v76, v76b, v77a (patch `dialog.showMessageBox` qua Node inspector của main; v76 còn thiếu keep awake, dòng 43). Bước bỏ trên Tauri: desktop-only + v49-open (link qua instance thứ 2, dòng 44), v73 "shell marks" (dòng 11).
