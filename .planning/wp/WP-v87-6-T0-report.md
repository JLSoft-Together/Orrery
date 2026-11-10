## WP-v87-6-T0 — vỏ Tauri `apps/shell` (khung) + cookie auth

Vỏ Tauri chạy được trên Windows: core-serve trong process, terminal qua `orrery-core`, header tự vẽ, tray, single-instance + deep link, autostart, notification, zoom, shim `window.ccmDesktop` sinh tự động. E2E trên Tauri: **p2 15/15 PASS**, **agent-map 34/36** (2 lỗi clipboard, xem dưới).

### Đã làm
1. **Cargo workspace riêng `apps/shell`** (`[workspace]` trống, `Cargo.lock` riêng lấy từ spike rồi bổ sung, target `apps/shell/target`). Tauri ghim `=2.12.5` (`tray-icon`, `unstable`), `tauri-build =2.7.4`; plugin single-instance (+deep-link), deep-link, autostart, notification, dialog, opener. Link `core-serve` qua path; không đụng `apps/core/Cargo.toml` / `Cargo.lock`.
2. **core-serve trong process** (`serve.rs`, `stdio.rs`): thread `core-serve` chạy `core_serve::run(cfg)`; env như Electron đặt (`CCM_APP_KEY` 32 byte ngẫu nhiên, `CCM_PORT` env hoặc 4317, `CCM_DATA_DIR`, `CCM_WEB_DIST`), `CCM_HOST_RPC` tắt (SystemDeps). `core_serve::run` đọc lệnh điều khiển từ stdin của process và EOF = tắt (GUI / stdin `ignore` → tắt ngay) → shell thay stdin bằng pipe của mình (Windows `CreatePipe` + `SetStdHandle`, macOS `pipe` + `dup2`), lúc thoát ghi `{"type":"shutdown"}` (chờ max 4 s). Cổng đã có Orrery khác trả `/health` → dừng, báo lỗi, không chạm app đó. Đề xuất WP sau: thêm `core_serve::run_with(cfg, control_rx, emit)` để bỏ đổi stdin (core-serve ngoài phạm vi T0).
3. **core-pty: spawn binary `orrery-core`** (`pty.rs`), cùng giao thức NDJSON với `pty/supervisor.ts` `RustHost`. Lý do không link thẳng: `server::run` dùng stdin/stdout của process (stdin đã giao cho serve), `serve()` private, `adopt_self` đưa cả process vào job object (kéo WebView2 theo), pty crash không làm sập app, giữ đúng đường Electron đã có E2E. Resolve preset tối thiểu trong Rust (`presets.rs`): shell / claude / codex (deepseek tắt), cwd thật + không UNC, max 16 pane, `resume` / `session` chỉ UUID, `child_env` bỏ `CCM_*` / `ORRERY_*` / `WEBVIEW2_*` / biến session agent; `cwd` → `cwd-ok`; `wt` → `wt.error git` (chưa port). Chưa có: MCP `orrery` (D8), shim claude/codex, OSC 633, zsh ZDOTDIR, nhánh git của pane.
4. **Cửa sổ** (`window.rs`, `header.js`): Windows `decorations(false)` + nút min / max-restore / close tự vẽ theo `--ccm-ink` (close hover đỏ), `data-tauri-drag-region` gắn lên header + phần tử không tương tác; mac `TitleBarStyle::Overlay` + `hidden_title`. Close → ẩn vào tray (lần đầu: notification). Chặn điều hướng ra ngoài origin app (mở trình duyệt ngoài), `window.open` URL app → cửa sổ `compact`. Lưu / phục hồi vị trí, kích thước, maximize. Zoom Ctrl/⌘ `=` `-` `0` + `ccm:zoom` (bước 0,5, -3..3, lưu `shell-appearance.json`, áp lại mỗi lần load).
5. **Tray** (`tray.rs`): như `main.ts:578-620` (Mở dashboard, Cửa sổ gọn, Chạy cùng Windows — chỉ bản release, Mở thư mục log, Thoát), chữ vi/en theo `ccm:locale`.
6. **Single-instance + deep link** (`links.rs`): `agent-monitor://open|focus|snooze` theo `openLink.ts` / `main.ts openLink`: open → max 10 / phút, hỏi trang qua `ccm:orrery` (`session.open`, caller `url`, max 5 s), lỗi → dialog; focus → `ccm:focus` rồi `POST /focus`, cuối cùng `ccm:select`; snooze → `GET /snooze`. Dev không đăng ký scheme vào registry (chỉ nhận qua argv của instance thứ hai; `tauri.conf.json` khai báo scheme cho bundler T2).
7. **Cookie auth** (`core-http/src/cookie.rs`, `server.rs`): `GET /?k=<key>` đúng → 302 về `/` (giữ query khác, vd `?mode=compact`) + `Set-Cookie: ccm_app=<key>; HttpOnly; SameSite=Strict; Path=/`, `no-store`, `no-referrer`; sai → 403 không cookie. Cookie đúng = header `x-ccm-app` cho HTTP và WS upgrade; header cũ giữ nguyên cho Electron; `/health` vẫn không cần key.
8. **Shim** `scripts/gen-desk-shim.mjs` → `apps/shell/src/shim.js`: esbuild bundle chính `preload.ts` với module `electron` giả (`invoke/send` → `invoke('ccm', {channel, args})`, `on` → `plugin:event|listen`, `sendSync` → hằng `ccm:core-serve`=`rust`, `ccm:deepseek-enabled`=false), nên tên method / event / cách gói tham số giữ y preload, `desktop.ts` web không đổi (`hasTerminals` true vì có `ptyEndpoint`). `--check` báo shim cũ. Init script chỉ chạy trên origin app. Command `ccm`: chỉ nhận webview cùng origin; T0 tự làm `pty-endpoint`, `pick-folder`, `open-external`, `zoom`, `window-bg`, `appearance`, `locale`, `login-item`, `orrery-reply`, `focus-reply`, `shell:window`; kênh khác → `not ported: <channel>`. Capability remote `http://127.0.0.1:<port>/*` thêm lúc chạy (chỉ đúng cổng của app).
9. **Dev / E2E**: `npm run shell:dev` (build web + gen shim + `cargo run -j 4`). Bản debug không có `CCM_PORT` → cổng 4419, không có `CCM_DATA_DIR` → `%TEMP%\orrery-shell-dev`, WebView2 profile `<data>\shell-webview` (hoặc `CCM_SHELL_WEBVIEW_DIR`). Identifier `dev.ccm.orrery.shell` (T2 chốt). `cdp.cjs`: `CCM_E2E_SHELL=tauri` chạy `apps/shell/target/debug/orrery-shell.exe` (hoặc `CCM_E2E_SHELL_EXE`) với `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=<dbg> …`, cùng port / data / APPDATA tạm.
10. `apps/web` không sửa (header tự vẽ làm hoàn toàn bằng init script).

### File
- Mới: `apps/shell/{Cargo.toml, Cargo.lock, build.rs, tauri.conf.json, capabilities/default.json, icons/*, ui/index.html}`, `apps/shell/src/{main, config, serve, stdio, pty, presets, window, tray, links, ccm, appearance, state, strings}.rs`, `apps/shell/src/{header.js, shim.js}`, `scripts/gen-desk-shim.mjs`, `apps/core/crates/core-http/src/cookie.rs`, `apps/core/crates/core-http/tests/cookie.rs`, `.code_index/shell.md`.
- Sửa: `apps/core/crates/core-http/src/{lib.rs, server.rs, http.rs (302 Found)}`, `apps/desktop/e2e/cdp.cjs`, `package.json` (`shell:dev`), `.gitignore` (`apps/shell/target/`, `apps/shell/gen/`), `.code_index/{core,_meta}.md`, `.planning/PLAN-v87-step6-tauri.md` (§4 ghi chú T0).
- Không comment trong code, file dài nhất `pty.rs` 261 dòng; code mac viết theo `cfg(target_os = "macos")`, chưa biên dịch trên mac.

### Test (số thật)
- `cargo test --locked -j 4 -p core-http` (apps/core): **73 passed** (9 suite; mới: 5 unit `cookie.rs`, 2 `tests/cookie.rs`). `cargo clippy --locked -j 4 -p core-http --all-targets -- -D warnings`: sạch.
- `cargo test -j 4` (apps/shell): **24 passed**. `cargo clippy -j 4 --all-targets -- -D warnings`: sạch. `cargo build -j 4` (dev) 3 phút 19 giây lần đầu.
- `node scripts/gen-desk-shim.mjs --check`: shim up to date.
- E2E Tauri (`CCM_E2E_SHELL=tauri`, port 4399, CDP 9333, data tạm):
  - **p2: 15/15 PASS** — endpoint pty, tab Workspace, shell pane chạy, gõ `echo ("CCM"+"E2E"+(40+2))` ra `CCME2E42`, phát hiện socket bị chiếm + reconnect, owners, pane claude chạy + link được terminal monitor + header trạng thái, nút +, popover, đóng pane kill process.
  - **agent-map: 34/36** — FAIL 2: "Copy ▸ Session ID puts the session id on the clipboard" và "Copied status line" (bước đầu tiên, menu node bấm qua `.click()` trong `Runtime.evaluate`). Cùng thao tác ở thẻ session và Ctrl+C trên node đều PASS → nghi `navigator.clipboard` của WebView2 cần user activation / quyền clipboard mà Electron cấp sẵn qua permission handler. Để T1/T2: cấp quyền clipboard cho origin app (WebView2 `SetPermissionState`) rồi chạy lại.
  - Ảnh: `%TEMP%\e2e-tauri-p2\*.png`, `%TEMP%\e2e-tauri-map\*.png` (header tự vẽ hiển thị đúng, nút min/max/close bên phải).
- Sau E2E: không còn process `orrery-shell` / `orrery-core` của con (stop = `taskkill /T` theo pid con tự spawn).

### RAM rảnh
Bản **debug**, 1 cửa sổ Monitor, chưa mở terminal, sau 25 s: **169,7 MB private**, 8 process (orrery-shell 38,0 + WebView2 130,6 + conhost 1,1; conhost chỉ có ở bản debug console). Electron cùng loại 274,3 MB (docs/perf/v87-step1-win32.json). Bản release + đo chuẩn để T3.

### Chưa kiểm / còn lại
- Kéo cửa sổ bằng drag region, snap layout flyout khi hover nút max, double-click maximize: cần chuột thật, không giả lập trên máy đang dùng.
- Toast của serve (cần host RPC) và quit confirmation (census pty, editor dirty) chưa port; status line command để trống (D1 `orrery-bridge`).
- `core-desk` chưa link (D0 đang làm song song): kênh chưa port trả lỗi `not ported`; khi D0 xong nối `dispatch` vào nhánh `_` của `ccm.rs`.
- macOS: chưa biên dịch / chạy (chỉ Windows); CDP mac không có.
- Máy: CPU 16 % lúc bắt đầu, mỗi lúc 1 việc nặng (cargo `-j 4`). Không commit / push / checkout; không chạm port 4317 / data thật; không đăng ký protocol.

STATUS: DONE
